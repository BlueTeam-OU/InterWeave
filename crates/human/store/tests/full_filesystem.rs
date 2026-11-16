// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Stage 18's S10, the store's half (`tests/SECURITY-MATRIX.md` row
//! S21-10), on a filesystem that is really full: unread content arriving
//! until the medium answers `ENOSPC`. `storage_failure.rs` fills a page
//! quota, which SQLite answers as it answers `ENOSPC`; here the kernel
//! refuses the write, so the store meets the failure where a full disk
//! puts it, in the WAL's frames or its checkpoint, not at a page count.
//!
//! The medium is a tmpfs capped by `size=`, which only a mount namespace
//! of the test's own can make without root: the test re-runs itself as a
//! child under `unshare -rm` (an unprivileged user plus mount namespace)
//! that mounts it, and reads the child's verdict. Where that namespace
//! cannot be made the test FAILS, never skips (the reason CI lifts the
//! `AppArmor` restriction before its tests, `.github/workflows/ci.yml`).
//! The mount lives and dies with the child, so the child also proves the
//! recovery: it remounts the same filesystem larger, as a person freeing
//! the disk would give it room, and opens the store again on it.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::path::Path;
use std::process::Command;

use interweave_human_core::retention::StorageHealth;
use interweave_human_store::{
    AppMessageId, HumanStore, InboundOrigin, NewInbound, StoreError, StoreOptions,
};
use interweave_transport_api::TransportIdentity;

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const TEST: &str = "unread_content_on_a_full_filesystem_is_refused_degrades_the_store_and_keeps_every_acknowledged_row";
const CHILD_ENV: &str = "INTERWEAVE_FULL_FILESYSTEM_MOUNT";
const CHILD_OK: &str = "FULL-FILESYSTEM-CHILD-OK";
const OFFERED: u32 = 24;
/// The child's root, then the capped mount in it, then the test binary
/// run again there. An unprivileged user namespace cannot map the host's
/// root, so the host's `/` and `/home` read there as owned by the
/// overflow uid, and the store's ancestor rule (ADR-0028 A 2026-10-08)
/// refuses every directory beneath them -- rightly. So the child pivots
/// into a tmpfs the namespace owns, the system directories and the test
/// binary's directory bind-mounted into it, as
/// `tests/support/src/e2e/netns.rs` does: every ancestor of the store's
/// file is then the namespace's own. `$1` is the new root's mount point,
/// `$2` the binary's directory, `$3` the binary. The mount's cap, 1 MiB,
/// is room for the store's schema and some maximal messages (48 KiB
/// each), not for the twenty-four the child offers.
const NEW_ROOT: &str = r#"
set -e
NR="$1"
mount -t tmpfs -o mode=0755 none "$NR"
for d in usr etc proc sys dev tmp; do mkdir -p "$NR/$d"; done
for l in bin sbin lib lib64; do
  if [ -L "/$l" ]; then ln -s "$(readlink "/$l")" "$NR/$l"
  elif [ -d "/$l" ]; then mkdir -p "$NR/$l"; mount --rbind "/$l" "$NR/$l"; fi
done
for d in usr etc dev proc sys; do mount --rbind "/$d" "$NR/$d"; done
chmod 1777 "$NR/tmp"
mkdir -p "$NR$2"; mount --rbind "$2" "$NR$2"
mkdir -m 0700 "$NR/full"
mount -t tmpfs -o size=1m,mode=0700 none "$NR/full"
mount --make-rprivate "$NR"
mkdir "$NR/old"; cd "$NR"; pivot_root . old; cd /; umount -l /old; rmdir /old
INTERWEAVE_FULL_FILESYSTEM_MOUNT=/full exec "$3" --exact "$IW_TEST" --nocapture --test-threads 1
"#;

fn id(n: u32) -> AppMessageId {
    AppMessageId::parse(format!("{n:032x}")).expect("canonical")
}

fn inbound(n: u32, payload: Vec<u8>) -> NewInbound {
    NewInbound {
        app_message_id: id(n),
        origin: InboundOrigin {
            peer: TransportIdentity::parse(PEER).expect("canonical"),
            endpoint: None,
            channel: None,
        },
        media_type: None,
        payload,
        received_at: 2_000,
        kind: interweave_human_store::InboundKind::Text,
    }
}

fn maximal(n: u32) -> Vec<u8> {
    let mut payload = vec![0_u8; interweave_transport_api::MAX_PAYLOAD_BYTES];
    payload[..4].copy_from_slice(&n.to_be_bytes());
    payload
}

/// The half inside the namespace, on the capped mount.
fn on_the_full_mount(mount: &Path) {
    // THE PREMISE: the directory is a filesystem of its own, capped, so
    // the refusal below is the kernel's and not the store's quota.
    let mut over = std::fs::File::create(mount.join("over")).expect("create");
    let filled = std::io::Write::write_all(&mut over, &vec![0_u8; 2 * 1024 * 1024])
        .expect_err("2 MiB does not fit a 1 MiB mount");
    assert_eq!(
        filled.raw_os_error(),
        Some(28),
        "the mount answers ENOSPC: {filled}"
    );
    drop(over);
    std::fs::remove_file(mount.join("over")).expect("remove");

    let dir = mount.join("state");
    let path = dir.join("human.sqlite3");
    let mut store = HumanStore::open(&path, StoreOptions::default()).expect("opens");
    let mut acknowledged = Vec::new();
    let mut refused = None;
    for n in 1..=OFFERED {
        match store.commit_unread_inbound(&inbound(n, maximal(n))) {
            Ok(_) => acknowledged.push(n),
            Err(e) => {
                refused = Some((n, e));
                break;
            }
        }
    }
    let (refused_n, refused) = refused.expect("24 x 48 KiB cannot fit a 1 MiB mount");
    assert!(
        !acknowledged.is_empty(),
        "the mount admits a message first, so the store filled rather than never fitting"
    );
    match &refused {
        StoreError::Sql(e) => assert_eq!(
            e.sqlite_error_code(),
            Some(rusqlite::ErrorCode::DiskFull),
            "the refusal is the full disk's: {e}"
        ),
        other => panic!("the refusal is the medium's own: {other:?}"),
    }

    // THE CLAIM: a full filesystem degrades the store, so the human
    // endpoint stops presenting as durable and takes nothing more.
    assert_eq!(store.health(), StorageHealth::Degraded);
    assert!(
        store
            .health()
            .degraded_response()
            .is_some_and(|r| r.release_human_endpoint && r.suspend_broadcast_joins),
    );
    let next = store
        .commit_unread_inbound(&inbound(100, b"tiny".to_vec()))
        .expect_err("a degraded store takes no unread content, however small");
    assert!(matches!(next, StoreError::Degraded), "{next:?}");
    // While the disk is still full, the probe does not report recovery.
    assert!(
        store.recheck_health().is_err(),
        "the probe fails while the filesystem is full"
    );
    drop(store);

    // NEVER SILENTLY: with room made, the same file holds every
    // acknowledged message, byte for byte, and nothing of the refused one.
    // Room is the filesystem grown, not a file freed on it: when the disk
    // filled, the WAL held most of it (measured: 750 KB of WAL beside a
    // 266 KB database on the 1 MiB mount), and the recovery probe needs
    // room beside both. `--options-mode ignore`, since mount(8) otherwise
    // repeats the options it reads back, among them the host uid the
    // namespace cannot name ("tmpfs: Invalid uid", measured).
    let grown = Command::new("mount")
        .args(["--options-mode", "ignore", "-o", "remount,size=4m"])
        .arg(mount)
        .status()
        .expect("mount runs");
    assert!(grown.success(), "the mount grows: {grown}");
    let mut store = HumanStore::open(&path, StoreOptions::default()).expect("reopens");
    let held: Vec<u32> = store
        .unread_inbound()
        .expect("read")
        .iter()
        .map(|m| {
            let n = u32::from_be_bytes(m.payload[..4].try_into().expect("four bytes"));
            assert_eq!(m.payload, maximal(n), "message {n} is intact");
            assert_eq!(m.app_message_id, id(n));
            n
        })
        .collect();
    assert_eq!(
        held, acknowledged,
        "every acknowledged message, and only those"
    );
    assert!(!held.contains(&refused_n));
    assert_eq!(
        store.recheck_health().expect("room again"),
        StorageHealth::Healthy,
        "with room made the probe reports recovery"
    );
    println!("{CHILD_OK}");
}

#[test]
fn unread_content_on_a_full_filesystem_is_refused_degrades_the_store_and_keeps_every_acknowledged_row()
 {
    if let Some(mount) = std::env::var_os(CHILD_ENV) {
        on_the_full_mount(Path::new(&mount));
        return;
    }

    let dir = common::private_tempdir();
    let new_root = dir.path().join("root");
    std::fs::create_dir(&new_root).expect("the new root's mount point");
    let me = std::env::current_exe().expect("this test binary");
    let bin_dir = me.parent().expect("the binary's directory");
    let output = Command::new("unshare")
        .args([
            "-rm",
            "--propagation",
            "private",
            "sh",
            "-c",
            NEW_ROOT,
            "sh",
        ])
        .arg(&new_root)
        .arg(bin_dir)
        .arg(&me)
        .env("IW_TEST", TEST)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "cannot run `unshare -rm` ({e}). This test needs an unprivileged user plus \
                 mount namespace to mount a capped tmpfs; it fails rather than skips where \
                 that is unavailable."
            )
        });
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains(CHILD_OK),
        "the child on the full mount failed ({}). If the namespace or the mount could not \
         be made, user namespaces are unavailable here and this test cannot ask its \
         question.\n--- stdout\n{stdout}\n--- stderr\n{stderr}",
        output.status
    );
}

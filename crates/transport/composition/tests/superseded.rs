// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Admin writes across ports, through a running composition (LOCAL-IPC.md
//! §Cancellation mapping and request concurrency): a write from a port
//! admitted before the one whose write for the same key was last applied
//! is refused `superseded`, nothing written, and audited -- for a trust
//! set by peer, and for the three bootstrap-address writes by address --
//! each beside its control: the same write from the later port, or for
//! another key, applies. And the stated limit, pinned so a change is
//! noticed: a READ on the later port orders nothing, so the older port's
//! write after it applies.
//!
//! Its own test binary: a GLOBAL subscriber, since the driver logs from
//! the runtime's worker threads.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use interweave_local_client_api::{
    AddressChange, AddressRefusal, AddressesChange, AdminBinding as _, AdminCapability,
    AdminPort as _, TrustChange, TrustRefusal,
};
use interweave_profile_config::ProfileConfig;
use interweave_profile_config::bootstrap_overlay::BOOTSTRAP_OVERLAY_FILE;
use interweave_profile_config::trust_overlay::TRUST_OVERLAY_FILE;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use interweave_transport_composition::{ComposedRuntime, CompositionOptions, InProcessAdmin};

fn id() -> (ProfileIdentity, TransportIdentity) {
    let identity = ProfileIdentity::generate();
    let peer = identity.transport_identity().expect("peer id");
    (identity, peer)
}

fn profile() -> ProfileConfig {
    serde_norway::from_str(
        "schema_version: 2
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  entries:
    - id: human
      enabled: true
      advertise: false
discovery:
  providers: []
",
    )
    .expect("the document parses")
}

/// A runtime keeping both overlays in a fresh state directory.
async fn runtime() -> (tempfile::TempDir, ComposedRuntime) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    chmod(dir.path(), 0o700);
    let at = |name: &str| -> PathBuf { dir.path().join(name) };
    let options = CompositionOptions {
        trust_overlay_file: Some(at(TRUST_OVERLAY_FILE)),
        bootstrap_overlay_file: Some(at(BOOTSTRAP_OVERLAY_FILE)),
        ..CompositionOptions::default()
    };
    let (identity, _) = id();
    let runtime = ComposedRuntime::start(&identity, &profile(), options)
        .await
        .expect("composes");
    (dir, runtime)
}

fn chmod(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

async fn port(runtime: &ComposedRuntime) -> InProcessAdmin {
    runtime
        .sessions()
        .admin([AdminCapability::Trust, AdminCapability::Peers].into())
        .await
        .expect("a port")
}

async fn allowed(port: &InProcessAdmin, peer: &TransportIdentity) -> bool {
    port.trust()
        .await
        .expect("the policy")
        .peers()
        .any(|p| p == peer)
}

async fn holds(port: &InProcessAdmin, address: &str) -> bool {
    port.bootstrap_addresses()
        .await
        .expect("the list")
        .iter()
        .any(|row| row.address == address)
}

/// A private-range address for `peer`, distinct for each `port`.
fn address(port: u16, peer: &TransportIdentity) -> String {
    format!("/ip4/192.168.77.1/tcp/{port}/p2p/{}", peer.as_str())
}

/// A, admitted first, gave up; B, admitted after it, allowed the peer.
/// A's revocation arriving after B's set is refused and audited, and the
/// peer stays allowed. The controls: A's set for another peer applies,
/// and B's own later revocation applies.
#[tokio::test(flavor = "current_thread")]
async fn a_set_from_an_older_port_after_a_later_ports_set_is_superseded() {
    let _ = logged();
    let (_dir, runtime) = runtime().await;
    let a = port(&runtime).await;
    let b = port(&runtime).await;
    let (_, peer) = id();
    let (_, other) = id();

    assert_eq!(b.set_trust(peer.clone(), true).await, Ok(TrustChange::Set));
    assert_eq!(
        a.set_trust(peer.clone(), false).await,
        Ok(TrustChange::Refused(TrustRefusal::Superseded))
    );
    assert!(allowed(&b, &peer).await, "B's set stands");
    assert_eq!(
        a.set_trust(other.clone(), true).await,
        Ok(TrustChange::Set),
        "another peer"
    );
    assert_eq!(
        b.set_trust(peer.clone(), false).await,
        Ok(TrustChange::Set),
        "B again"
    );
    assert!(!allowed(&b, &peer).await);
    // An `unchanged` set stands as well, so it orders: B allowing a peer
    // A already allowed still refuses A's later revocation.
    assert_eq!(b.set_trust(other.clone(), true).await, Ok(TrustChange::Set));
    assert_eq!(
        a.set_trust(other.clone(), false).await,
        Ok(TrustChange::Refused(TrustRefusal::Superseded))
    );
    runtime.stop().await.expect("stops");

    let text = logged();
    let audit = audit_of(&text, "admin.trust.set", peer.as_str());
    assert_eq!(audit.len(), 3, "{text}");
    assert!(audit[0].contains("outcome=\"changed\""), "{}", audit[0]);
    assert!(audit[1].contains("outcome=\"superseded\""), "{}", audit[1]);
    assert!(audit[1].contains("allowed=false"), "{}", audit[1]);
    assert!(audit[2].contains("outcome=\"changed\""), "{}", audit[2]);
}

/// THE STATED LIMIT: a read orders nothing. B reads the policy; A's set
/// arriving after the read applies, so a client that lost an answer
/// re-reads and treats what it showed meanwhile as pending.
#[tokio::test(flavor = "current_thread")]
async fn a_read_on_a_later_port_orders_nothing() {
    let (_dir, runtime) = runtime().await;
    let a = port(&runtime).await;
    let b = port(&runtime).await;
    let (_, peer) = id();
    assert!(!allowed(&b, &peer).await, "B reads: not allowed");
    assert_eq!(a.set_trust(peer.clone(), true).await, Ok(TrustChange::Set));
    assert!(
        allowed(&b, &peer).await,
        "A's set took effect after B's read"
    );
    assert!(!holds(&b, &address(4001, &peer)).await);
    assert_eq!(
        a.add_bootstrap_address(address(4001, &peer)).await,
        Ok(AddressChange::Added)
    );
    runtime.stop().await.expect("stops");
}

/// The address writes, keyed by address: B's add stands against A's
/// removal and against A's list naming it -- refused naming that line,
/// nothing written, its other line included -- and each is audited. The
/// controls: A's add of another address applies, and B's own removal
/// applies.
#[tokio::test(flavor = "current_thread")]
async fn an_address_write_from_an_older_port_is_superseded_by_address() {
    let _ = logged();
    let (_dir, runtime) = runtime().await;
    let a = port(&runtime).await;
    let b = port(&runtime).await;
    let (_, peer) = id();
    let held = address(4101, &peer);
    let other = address(4102, &peer);

    assert_eq!(
        b.add_bootstrap_address(held.clone()).await,
        Ok(AddressChange::Added)
    );
    match a.remove_bootstrap_address(held.clone()).await {
        Ok(AddressChange::Refused {
            reason: AddressRefusal::Superseded,
            ..
        }) => {}
        other => panic!("A's removal: {other:?}"),
    }
    assert!(holds(&b, &held).await, "B's add stands");
    match a
        .add_bootstrap_addresses(vec![other.clone(), held.clone()])
        .await
    {
        Ok(AddressesChange::Refused {
            reason: AddressRefusal::Superseded,
            line: Some(1),
            ..
        }) => {}
        other => panic!("A's list: {other:?}"),
    }
    assert!(!holds(&b, &other).await, "all or none: nothing written");
    assert_eq!(
        a.add_bootstrap_address(other.clone()).await,
        Ok(AddressChange::Added)
    );
    assert_eq!(
        b.remove_bootstrap_address(held.clone()).await,
        Ok(AddressChange::Removed)
    );
    assert!(!holds(&b, &held).await);
    runtime.stop().await.expect("stops");

    let text = logged();
    let removal = audit_of(&text, "admin.peers.remove_address", &held);
    assert_eq!(removal.len(), 1, "{text}");
    assert!(
        removal[0].contains("outcome=\"superseded\""),
        "{}",
        removal[0]
    );
    let listed = audit_of(&text, "admin.peers.add_addresses", &held);
    assert_eq!(listed.len(), 1, "{text}");
    assert!(
        listed[0].contains("outcome=\"superseded\""),
        "{}",
        listed[0]
    );
    assert!(
        audit_of(&text, "admin.peers", &other).is_empty(),
        "an applied address write is not audited"
    );
}

fn logged() -> String {
    static LOG: std::sync::OnceLock<Arc<Mutex<Vec<u8>>>> = std::sync::OnceLock::new();
    let log = LOG.get_or_init(|| {
        let log = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || Writer(Arc::clone(&sink)))
            .with_ansi(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("the one subscriber");
        log
    });
    String::from_utf8(log.lock().expect("lock").clone()).expect("utf-8")
}

/// The audit lines of `method` that name `subject`, in order: each
/// test's own, since every test names keys of its own.
fn audit_of<'a>(text: &'a str, method: &str, subject: &str) -> Vec<&'a str> {
    text.lines()
        .filter(|l| {
            l.contains("interweave::audit")
                && l.contains(method)
                && l.contains("outcome=")
                && l.contains(subject)
        })
        .collect()
}

struct Writer(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("lock").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

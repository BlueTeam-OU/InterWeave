// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Two Platform-tests lines of plan §20 on a device, through the app
//! itself rather than the cases' runner: the app's own `MainActivity`
//! and `NetworkService`, started by `am start` on a profile the cases'
//! `provision` wrote, with D, a desktop daemon on this host, as its one
//! trusted peer over adb's reversed port.
//!
//! - `an_arrival_while_no_window_has_focus_posts_a_count_notice`: with
//!   the notification shade over the window, a message D sends is posted
//!   as the `messages` channel's notice, its count and never its text;
//!   the window taking focus back withdraws it
//!   (`human-client-android.md`, "Notifications").
//! - `a_process_death_restarts_the_service_into_the_same_store`: a
//!   message committed unread, then the process crashed; the platform
//!   restarts the service with no person present, its runtime takes a
//!   second message from D, and the store holds both unread. The
//!   example's profile is `stay-reachable`, so the restart is the
//!   foreground service's (`device-runs/2026-10-20-gate-b.md` ran it by
//!   hand). ActivityManager lengthens the restart delay of a service
//!   that crashed minutes before, past `PATIENCE`: a rerun waits a few
//!   minutes after the last (seen on the A40, 2026-10-22).
//!
//! Device only (`#[ignore]`d): the line is the platform's, and the host
//! stand-in has no notification manager and no process to kill. Run as
//! `src/adb.rs` says, after `installDebug installDebugAndroidTest`. Each
//! case writes the app's profile, so it is for a phone whose data may be
//! replaced (`pm clear` first on one with a user-presence profile, whose
//! seed no instrumentation can unwrap).
//!
//! What this does NOT prove: a radio network (adb's reversed port
//! carries D), the person's view of the notice (its text is read from
//! `dumpsys notification`, not seen), and, under API 33, the
//! `POST_NOTIFICATIONS` grant, which those levels do not have.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;
use std::time::{Duration, Instant};

use interweave_android_e2e_cases::{Recording, Tap, chat_text, desktop_message_id};
use interweave_android_e2e_tests::adb::{AdbDevice, PACKAGE};
use interweave_android_e2e_tests::{Desktop, Device, PATIENCE, free_port, human};
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{
    ClientConfig, ClientEvent, Destination, OutboundStatus, SessionState, TransportClient,
};
use interweave_human_ui_model::{UiText, placeholder_en};
use interweave_ipc_client::IpcBinding;
use interweave_test_support::e2e;
use interweave_transport_api::{MAX_PAYLOAD_BYTES, TransportIdentity};

/// Where an `embedded-android` profile listens: a wildcard only.
const WILDCARD: &str = "/ip4/0.0.0.0/tcp/0";

/// The app's notice of waiting messages: `NetworkService.kt`'s
/// `MESSAGES_ID` on its `MESSAGES_CHANNEL`.
const NOTICE_ID: &str = "id=2 ";
const NOTICE_CHANNEL: &str = "channel=messages";

/// The human store, under the app's data directory: the embedded
/// profile's `human_dir` (`ProfilePaths`) and `STORE_FILE`.
const STORE: &str = "interweave/state/profiles/human-android/human/human.sqlite";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
async fn an_arrival_while_no_window_has_focus_posts_a_count_notice() {
    let (mut phone, d, mut side) = started().await;
    let log = || format!("D:\n{}\nphone:\n{}", d.log(), phone.log());

    shade(true);
    until(&log, "the window giving up focus", || !app_focused());
    assert!(notice().is_none(), "control: no notice before the arrival");

    let text = chat_text(1, "D", false);
    side.send(&phone.peer(), 1, &text).await;
    side.until_accepted(&log, 1).await;
    let posted = until_some(&log, "the notice posted", notice);
    assert!(
        posted.contains(NOTICE_CHANNEL),
        "the messages channel: {posted}"
    );
    // The dump carries the notice's words, so their absence below is
    // read, not assumed.
    let title = placeholder_en::text(UiText::NewMessage);
    assert!(
        posted.contains(&format!("android.title=String ({title})")),
        "the count's title, {title:?}, in the dump: {posted}"
    );
    assert!(
        !posted.contains(&text) && !posted.contains("from **D**"),
        "a count, never the text: {posted}"
    );

    // The window back in front: what waited is read there, and the
    // notice goes.
    shade(false);
    until(&log, "the window taking focus back", app_focused);
    until(&log, "the notice withdrawn", || notice().is_none());
    phone.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
async fn a_process_death_restarts_the_service_into_the_same_store() {
    let (mut phone, d, mut side) = started().await;
    let log = || format!("D:\n{}\nphone:\n{}", d.log(), phone.log());

    // Committed unread: posted only after the commit.
    adb(&["shell", "input", "keyevent", "KEYCODE_HOME"]);
    until(&log, "the window leaving", || !app_focused());
    side.send(&phone.peer(), 1, &chat_text(1, "D", false)).await;
    side.until_accepted(&log, 1).await;
    until_some(&log, "the first notice", notice);

    let before = app_pid().expect("the app's process");
    adb(&["shell", "am", "crash", PACKAGE]);
    let after = until_some(&log, "the service restarted in a new process", || {
        app_pid().filter(|p| *p != before)
    });

    side.send(&phone.peer(), 2, &chat_text(2, "D", false)).await;
    side.until_accepted(&log, 2).await;
    until_some(&log, "the restarted process's notice", notice);
    assert_eq!(
        app_pid(),
        Some(after),
        "no person and no second restart: the same process took it"
    );

    // Read at rest: no writer while the store's two files are copied.
    phone.stop();
    let copy = private_dir();
    let db = copy.path().join("human.sqlite");
    private_file(&db, &phone.pull(STORE));
    if let Some(wal) = pull_if_any(&format!("{STORE}-wal")) {
        private_file(&copy.path().join("human.sqlite-wal"), &wal);
    }
    let unread: Vec<String> = HumanStore::open(&db, StoreOptions::default())
        .expect("the copy opens")
        .unread_inbound()
        .expect("its unread rows")
        .into_iter()
        .map(|r| r.app_message_id.as_str().to_owned())
        .collect();
    for serial in [1, 2] {
        assert!(
            unread.contains(&desktop_message_id(serial)),
            "message {serial} unread in the store after the restart: {unread:?}"
        );
    }
}

/// The phone provisioned with D as its one trusted peer, the app started
/// and in front, and D's session ready.
async fn started() -> (AdbDevice, Desktop, Side) {
    let mut phone = AdbDevice::connect();
    if sdk() >= 33 {
        let granted = adb(&["shell", "dumpsys", "package", PACKAGE]);
        assert!(
            granted.contains("android.permission.POST_NOTIFICATIONS: granted=true"),
            "API 33+: grant the app's notifications first, as a person does"
        );
    }
    let mut d = Desktop::new();
    let d_port = free_port(std::net::Ipv4Addr::LOCALHOST);
    let d_listen = format!("/ip4/127.0.0.1/tcp/{d_port}");
    d.start(&e2e::example(
        "human-desktop.yaml",
        &phone.peer(),
        &d_listen,
        None,
    ))
    .await;
    phone.reach(d_port);
    phone.start(&e2e::example(
        "human-android.yaml",
        &d.peer,
        WILDCARD,
        Some(&format!("{d_listen}/p2p/{}", d.peer.as_str())),
    ));
    adb(&[
        "shell",
        "am",
        "start",
        "-W",
        "-n",
        &format!("{PACKAGE}/.MainActivity"),
    ]);
    let mut side = Side::new(d.binding());
    let log = || format!("D:\n{}\nphone:\n{}", d.log(), phone.log());
    until(&log, "the app in front", app_focused);
    side.until(&log, "D's session ready", Side::is_ready).await;
    (phone, d, side)
}

/// D's person: the desktop client's facade over D's daemon.
struct Side {
    client: TransportClient<Recording<IpcBinding>, IpcBinding>,
    accepted: Vec<String>,
    clock: Instant,
    _store_dir: tempfile::TempDir,
}

impl Side {
    fn new(binding: IpcBinding) -> Self {
        let store_dir = private_dir();
        let store = HumanStore::open(
            &store_dir.path().join("human.sqlite"),
            StoreOptions::default(),
        )
        .expect("a store");
        let client = TransportClient::new(
            Recording::new(binding.clone(), Tap::default()),
            binding,
            store,
            ClientConfig {
                client_kind: "human-client".to_owned(),
                endpoints: vec![human()],
                channels: Vec::new(),
                max_payload_bytes: MAX_PAYLOAD_BYTES,
            },
            Box::new(wall_ms),
            0,
        )
        .expect("an empty store's pending rows");
        Self {
            client,
            accepted: Vec::new(),
            clock: Instant::now(),
            _store_dir: store_dir,
        }
    }

    async fn step(&mut self) {
        let now = u64::try_from(self.clock.elapsed().as_millis()).expect("ms");
        Box::pin(self.client.tick(now)).await;
        let _ = Box::pin(self.client.drain(64, now)).await;
        while let Some(event) = self.client.next_event() {
            if let ClientEvent::Outbound(update) = event
                && matches!(update.status, OutboundStatus::Accepted { .. })
            {
                self.accepted
                    .push(update.app_message_id.as_str().to_owned());
            }
        }
    }

    fn is_ready(&self) -> bool {
        matches!(self.client.session_state(), SessionState::Ready { .. })
    }

    async fn send(&mut self, to: &TransportIdentity, serial: u8, text: &str) {
        let envelope = HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: desktop_message_id(serial),
            text: text.to_owned(),
            reply_to: None,
            sent_at_ms: Some(wall_ms()),
            from_endpoint: Some(human()),
        };
        let destination = Destination::Direct {
            peer: to.clone(),
            endpoint: Some(human()),
        };
        Box::pin(self.client.send(destination, &envelope, 0))
            .await
            .expect("the facade commits it");
    }

    /// Step until the phone's endpoint queue admitted message `serial`.
    async fn until_accepted(&mut self, log: &impl Fn() -> String, serial: u8) {
        let id = desktop_message_id(serial);
        self.until(log, &format!("message {serial} accepted"), |s| {
            s.accepted.contains(&id)
        })
        .await;
    }

    async fn until(&mut self, log: &impl Fn() -> String, what: &str, done: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            self.step().await;
            if done(self) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "{what} did not happen in time\n{}",
                log()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

/// Poll `done` until it holds, failing with `log` when `PATIENCE` runs
/// out. The device's state is read by `adb`, a second or so a round.
fn until(log: &impl Fn() -> String, what: &str, done: impl Fn() -> bool) {
    until_some(log, what, || done().then_some(()));
}

fn until_some<T>(log: &impl Fn() -> String, what: &str, found: impl Fn() -> Option<T>) -> T {
    let deadline = Instant::now() + PATIENCE;
    loop {
        if let Some(t) = found() {
            return t;
        }
        assert!(
            Instant::now() < deadline,
            "{what} did not happen in time\n{}",
            log()
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// `adb <args>`'s output; a failing `adb` fails the test.
fn adb(args: &[&str]) -> String {
    let out = Command::new("adb").args(args).output().expect("adb");
    assert!(
        out.status.success(),
        "adb {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn sdk() -> u32 {
    adb(&["shell", "getprop", "ro.build.version.sdk"])
        .trim()
        .parse()
        .expect("an API level")
}

/// The notification shade over the window, or put away.
fn shade(open: bool) {
    let verb = if open {
        "expand-notifications"
    } else {
        "collapse"
    };
    adb(&["shell", "cmd", "statusbar", verb]);
}

/// Whether the app's window has the platform's input focus.
fn app_focused() -> bool {
    adb(&["shell", "dumpsys", "window"])
        .lines()
        .any(|l| l.contains("mCurrentFocus") && l.contains(&format!("{PACKAGE}/")))
}

fn app_pid() -> Option<u32> {
    let out = Command::new("adb")
        .args(["shell", "pidof", PACKAGE])
        .output()
        .expect("adb");
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// The app's posted notice of waiting messages, as `dumpsys
/// notification --noredact` prints its record: from its own
/// `NotificationRecord(` line to the next one.
fn notice() -> Option<String> {
    let dump = adb(&["shell", "dumpsys", "notification", "--noredact"]);
    let mut records = dump.split("NotificationRecord(").skip(1);
    records
        .find(|r| r.contains(&format!("pkg={PACKAGE} ")) && r.contains(NOTICE_ID))
        .map(str::to_owned)
}

/// A file under the app's data directory read as the app, or `None`
/// when there is none.
fn pull_if_any(path: &str) -> Option<Vec<u8>> {
    let out = Command::new("adb")
        .args(["exec-out", "run-as", PACKAGE, "cat", path])
        .output()
        .expect("adb");
    (out.status.success() && out.stderr.is_empty() && !out.stdout.is_empty()).then_some(out.stdout)
}

/// A scratch directory the store accepts: owner-only at creation,
/// whatever the umask (ADR-0028 A 2026-10-08).
fn private_dir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("a private directory")
}

/// `bytes` written owner-only, as the store requires of its files.
fn private_file(path: &std::path::Path, bytes: &[u8]) {
    std::fs::write(path, bytes).expect("a copy");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).expect("owner-only");
}

fn wall_ms() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock")
            .as_millis(),
    )
    .expect("a Unix time in ms")
}

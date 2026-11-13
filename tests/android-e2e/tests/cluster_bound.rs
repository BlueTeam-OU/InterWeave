// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The cluster bound on a phone (HUMAN-CHAT.md's Bounds; TM-58): a peer's
//! character -- one grapheme cluster -- at the 32-code-point bound and
//! past it, drawn by the app's own window and renderer wherever it draws
//! a peer's text. The desktop's text shaper counts a cluster's code points
//! in eight bits, so a character of 256 panicked it under the release
//! profile's overflow checks (`crates/human/ui-slint/tests/cluster_draw.rs`);
//! the phone draws with a renderer of its own, which only a phone can
//! show (it shapes with the same parley, the A40 found on 2026-11-13).
//!
//! D is a desktop daemon on this host's loopback, the phone's only route
//! to it the port adb reverses (`src/adb.rs`), as in `long_message.rs`.
//! The app must hold no contact, as after a fresh install: a contact a
//! run before left of the same name or key puts a note before the second
//! card's Update contact, and on the A40 (2026-11-13) focus did not reach
//! that button past the note's link, so the case could not accept it.
//! That was the proposal row's clip, which Slint's TAB stops at; the row
//! now clips only while a swipe moves it (ui-slint `tests/cards.rs`,
//! `every_control_of_a_proposal_taller_than_the_window_is_a_tab_stop`),
//! not yet run on a phone, and the case's steps still expect no note.
//! D's person sends:
//!
//! 1. one body holding a character of 32 code points, one of 33 and one
//!    of 256 -- drawn in the conversation and as its line in Chats;
//! 2. a card whose name is one character of 32 code points, and then one
//!    whose name is one of 64, the most a card name holds (64 scalars):
//!    each name drawn in its card's proposal in the conversation, then
//!    accepted, so it is drawn as the conversation's title; the second's
//!    also as its row's name in Chats.
//!
//! After each step the app's own log is read for a panic, and its process
//! for the pid it had (`pidof`). Both are needed: a shaper's panic on the
//! view's thread ends the window and not the process, whose service runs
//! on (a build without the replacement, on the A40, 2026-11-13: parley's
//! `attempt to add with overflow`, the same pid after it). `INTERWEAVE_ANDROID_SHOTS` names a directory each step's
//! screen is saved to as a PNG, for the person who checks what was drawn
//! at the bound (the character whole) and past it (one U+FFFD): the
//! trace names kinds, never text, so the screen is the only witness of
//! the glyphs. The build under test is the one installed is the run
//! file's to show (the installed `base.apk`'s sha256 beside the built
//! one's), not this case's.
//!
//! What this does NOT prove: that the replacement is what keeps the
//! phone's renderer whole -- a build without it is the control for that,
//! and it is a probe, not this case; Chats drawn again from the store, which
//! holds neither the body once read nor the card once accepted; Show source and the reader, which
//! the host tests hold to the same replacement; a name drawn in a
//! notification (a card is never announced); and `TalkBack`.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use ed25519_dalek::Signer as _;
use interweave_android_e2e_cases::keys;
use interweave_android_e2e_tests::adb::{AdbDevice, PACKAGE};
use interweave_android_e2e_tests::long::envelope;
use interweave_android_e2e_tests::screen::{Screens, Step};
use interweave_android_e2e_tests::{Desktop, Device, PATIENCE, free_port, human};
use interweave_human_chat_protocol::{CardDraft, CardRoute, MAX_CARD_NAME_SCALARS, build_card};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{
    ClientConfig, ClientEvent, Destination, OutboundStatus, SessionState, TransportClient,
};
use interweave_ipc_client::IpcBinding;
use interweave_test_support::e2e::example;
use interweave_transport_api::{MAX_PAYLOAD_BYTES, TransportIdentity};

/// Where the app listens: an `embedded-android` profile listens on a
/// wildcard only.
const WILDCARD: &str = "/ip4/0.0.0.0/tcp/0";

/// HUMAN-CHAT.md's cluster bound: the most code points a drawn character
/// holds (ui-model's `cluster::CLUSTER_CODE_POINTS`).
const BOUND: usize = 32;

/// How long a step waits after D's message is accepted at the app before
/// its screen is read: the app draws on its next render, which the trace
/// does not name for a row.
const DRAWN_AFTER: Duration = Duration::from_secs(3);

/// One character of `n` code points: `base` and `n - 1` combining acute
/// accents, which make no precomposed character with `x` or with each
/// other, so a card name's NFC leaves it as it is.
fn character(base: char, n: usize) -> String {
    std::iter::once(base)
        .chain(std::iter::repeat_n('\u{301}', n - 1))
        .collect()
}

/// D's person, sending: the desktop's facade over D's IPC with a store of
/// its own, and each of its rows' latest status.
struct Sender {
    client: TransportClient<IpcBinding, IpcBinding>,
    outbound: BTreeMap<String, OutboundStatus>,
    _store_dir: tempfile::TempDir,
}

impl Sender {
    fn new(binding: IpcBinding) -> Self {
        let store_dir = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .expect("a store directory");
        let store = HumanStore::open(
            &store_dir.path().join("state/human.sqlite"),
            StoreOptions::default(),
        )
        .expect("a store");
        let client = TransportClient::new(
            binding.clone(),
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
            outbound: BTreeMap::new(),
            _store_dir: store_dir,
        }
    }

    async fn step(&mut self) {
        Box::pin(self.client.tick(wall_ms())).await;
        Box::pin(self.client.drain(64, wall_ms())).await;
        while let Some(event) = self.client.next_event() {
            if let ClientEvent::Outbound(update) = event {
                self.outbound
                    .insert(update.app_message_id.as_str().to_owned(), update.status);
            }
        }
    }

    /// Step until `done`, or fail naming `what` with `log`.
    async fn until(&mut self, what: &str, log: impl Fn() -> String, done: impl Fn(&Self) -> bool) {
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

    /// Step until the row `id` is accepted at the app's human endpoint:
    /// the app's store holds it.
    async fn accepted(&mut self, id: String, log: impl Fn() -> String) {
        self.until(&format!("{id} accepted at the app"), log, |s| {
            matches!(
                s.outbound.get(&id),
                Some(OutboundStatus::Accepted { endpoint }) if endpoint == &human()
            )
        })
        .await;
    }

    /// `text` to `to`'s human endpoint, accepted there.
    async fn send(
        &mut self,
        to: &TransportIdentity,
        serial: u8,
        text: String,
        log: impl Fn() -> String,
    ) {
        let mut message = envelope(text);
        message.app_message_id = format!("{serial:032x}");
        message.sent_at_ms = Some(wall_ms());
        let destination = Destination::Direct {
            peer: to.clone(),
            endpoint: Some(human()),
        };
        Box::pin(self.client.send(destination, &message, 0))
            .await
            .expect("the facade commits it");
        self.accepted(message.app_message_id, log).await;
    }

    /// A card named `name`, signed for D's route to `to`'s human
    /// endpoint, accepted there.
    async fn send_card(
        &mut self,
        to: &TransportIdentity,
        serial: u8,
        name: &str,
        log: impl Fn() -> String,
    ) {
        let own = self
            .client
            .local_peer()
            .cloned()
            .expect("D's session says its PeerId");
        // Test-only key material: a fixed seed, never a real identity.
        let key = ed25519_dalek::SigningKey::from_bytes(&[58; 32]);
        let id = format!("{serial:032x}");
        let card = build_card(
            &id,
            CardDraft {
                name,
                surname: None,
                photo: None,
            },
            key.verifying_key().to_bytes(),
            CardRoute {
                source_peer: &own,
                source_endpoint: &human(),
            },
            Some(wall_ms()),
            |m| key.sign(m).to_bytes(),
        )
        .expect("a name within the card's bounds");
        Box::pin(self.client.send_card(to.clone(), human(), &card, 0))
            .await
            .expect("the facade commits it");
        self.accepted(id, log).await;
    }
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

/// `step` passed, or the test fails with it and the trace's tail.
fn passed(step: &Step, screens: &Screens) {
    assert_eq!(
        step.get(keys::RESULT).and_then(|v| v.as_str()),
        Some(keys::PASS),
        "{step:?}\nthe trace's tail: {:#?}\nthe app: {:#?}",
        &screens.trace()[screens.trace().len().saturating_sub(30)..],
        screens.app_log()
    );
}

/// The app's process id, if it runs.
fn pid(serial: &str) -> Option<String> {
    let out = Command::new("adb")
        .args(["-s", serial, "shell", "pidof", PACKAGE])
        .output()
        .expect("adb");
    let pid = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (!pid.is_empty()).then_some(pid)
}

/// The phone's screen now, to `dir/<name>.png`.
fn shot(serial: &str, dir: &Path, name: &str) {
    let out = Command::new("adb")
        .args(["-s", serial, "exec-out", "screencap", "-p"])
        .output()
        .expect("adb");
    assert!(
        out.status.success(),
        "screencap: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::write(dir.join(format!("{name}.png")), out.stdout).expect("the shot written");
}

/// The lines of the app's log tag since the launch cleared the log that
/// say a thread panicked (the native library's panic hook writes them
/// there, `apps/human-android`'s jni.rs).
fn panics(serial: &str) -> Vec<String> {
    let out = Command::new("adb")
        .args([
            "-s",
            serial,
            "logcat",
            "-d",
            "-v",
            "raw",
            "-s",
            "interweave:*",
        ])
        .output()
        .expect("adb");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("panic"))
        .map(str::to_owned)
        .collect()
}

/// What a step leaves to read: no panic in the app's log and its process
/// still the one it was, after [`DRAWN_AFTER`], and its screen saved as
/// `name`.
struct Witness {
    serial: String,
    shots: Option<PathBuf>,
}

impl Witness {
    fn drawn(&self, name: &str, before: &str, screens: &Screens) {
        std::thread::sleep(DRAWN_AFTER);
        if let Some(dir) = &self.shots {
            shot(&self.serial, dir, name);
        }
        let panicked = panics(&self.serial);
        assert!(
            panicked.is_empty(),
            "{name}: the app panicked while drawing: {panicked:#?}"
        );
        let now = pid(&self.serial);
        assert_eq!(
            now.as_deref(),
            Some(before),
            "{name}: the app's process was {before} and is now {now:?}: it ended while drawing\nthe app: {:#?}",
            screens.app_log()
        );
        println!("{name}: drawn, process {before} still running");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device with the app's debug and androidTest builds: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
async fn on_a_device_a_character_at_the_cluster_bound_and_past_it_is_drawn_everywhere() {
    let serial = std::env::var("ANDROID_SERIAL").expect("ANDROID_SERIAL names the phone");
    let witness = Witness {
        serial: serial.clone(),
        shots: std::env::var_os("INTERWEAVE_ANDROID_SHOTS").map(PathBuf::from),
    };
    let mut phone = AdbDevice::connect();
    let mut d = Desktop::new();
    let (d_peer, phone_peer) = (d.peer.clone(), phone.peer());
    let d_port = free_port(Ipv4Addr::LOCALHOST);
    let d_listen = format!("/ip4/127.0.0.1/tcp/{d_port}");
    d.start(&example("human-desktop.yaml", &phone_peer, &d_listen, None))
        .await;
    phone.reach(d_port);
    phone.start(&example(
        "human-android.yaml",
        &d_peer,
        WILDCARD,
        Some(&format!("{d_listen}/p2p/{}", d_peer.as_str())),
    ));
    phone.stop();
    let mut sender = Sender::new(d.binding());
    sender
        .until(
            "D's session ready",
            || d.log(),
            |s| matches!(s.client.session_state(), SessionState::Ready { .. }),
        )
        .await;
    let mut screens = Screens::new(&serial);
    let launched = tokio::task::block_in_place(|| screens.launch(PATIENCE));
    passed(&launched, &screens);
    let app = pid(&serial).expect("the app runs after its launch");

    // 1. The body, in Chats and then in its conversation.
    let body = format!(
        "at {} one past {} far past {} end",
        character('a', BOUND),
        character('a', BOUND + 1),
        character('a', 256)
    );
    sender.send(&phone_peer, 1, body, || d.log()).await;
    tokio::task::block_in_place(|| {
        witness.drawn("1-chats-body", &app, &screens);
        let s = screens.open_conversation(PATIENCE);
        passed(&s, &screens);
        witness.drawn("2-conversation-body", &app, &screens);
    });

    // 2. The cards' names, in their proposals, each then accepted.
    let at = character('x', BOUND);
    let past = character('x', MAX_CARD_NAME_SCALARS);
    sender.send_card(&phone_peer, 2, &at, || d.log()).await;
    tokio::task::block_in_place(|| {
        witness.drawn("3-proposal-name-at", &app, &screens);
        let s = screens.accept_card(PATIENCE);
        passed(&s, &screens);
        witness.drawn("4-title-name-at", &app, &screens);
    });
    sender.send_card(&phone_peer, 3, &past, || d.log()).await;
    tokio::task::block_in_place(|| {
        witness.drawn("5-proposal-name-past", &app, &screens);
        let s = screens.accept_card(PATIENCE);
        passed(&s, &screens);
        witness.drawn("6-title-name-past", &app, &screens);
        let s = screens.back_to_conversations(PATIENCE);
        passed(&s, &screens);
        witness.drawn("7-chats-name-past", &app, &screens);
    });

    Box::pin(sender.client.close()).await;
    phone.stop();
    d.stop().await;
}

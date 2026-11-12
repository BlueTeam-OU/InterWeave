// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The reader's times on a phone (design/2026-11-04-long-message.md A5):
//! each long shape (`src/long.rs`, the largest whose envelope fits) sent
//! by a desktop daemon to the app as a person runs it, its row's Read the
//! whole message pressed by key, and the debug build's trace read for
//! the time from that press to the reader's first frame, and from End to
//! the frame that shows the list at its end, formatted and then as
//! source. A5's figures are 0.5 s and 1 s on the A40; the case reports
//! every time beside them and fails only on a step that does not
//! happen, since a figure the build cannot reach goes back to
//! ux-designer rather than failing the build.
//!
//! D is a desktop daemon on this host's loopback, the phone's only route
//! to it the port adb reverses (`src/adb.rs`); the app is provisioned
//! through its instrumentation with D as its one peer, then started from
//! the launcher, so what receives and draws is the app's own process and
//! window, not a case's. The shapes go one at a time, each once the last
//! is measured: the newest row's Read the whole message is then the one
//! TAB reaches that has not been opened yet.
//!
//! `INTERWEAVE_ANDROID_SHOTS`, when set, names a directory each End's
//! frame is saved to as a PNG, for the person who checks that the last
//! line is the one drawn: the trace's `at-end` says the list stands at
//! its end, not what the screen shows there.
//!
//! What this does NOT prove: the optimised (release) build's times -- the
//! trace is the debug build's, whose native library runs at the release
//! opt-level with debug assertions on (`device-runs/2026-11-05-samsung-
//! a40.md`); the key's way to the view, before the press is timed; that
//! scrolling shows no blank screen or seam (a person looks, A5's other
//! half); the reader opened from a row in source mode; `TalkBack` (A6, A8,
//! blocked on the accessibility adapter); and every other acceptance
//! row of section 10.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use interweave_android_e2e_cases::keys;
use interweave_android_e2e_tests::adb::AdbDevice;
use interweave_android_e2e_tests::long::{envelope, shapes};
use interweave_android_e2e_tests::screen::{Screens, Step};
use interweave_android_e2e_tests::{Desktop, Device, PATIENCE, free_port, human};
use interweave_human_chat_protocol::render;
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{
    ClientConfig, ClientEvent, Destination, OutboundStatus, SessionState, TransportClient,
};
use interweave_human_ui_model::{UiText, cut, fill, placeholder_en};
use interweave_ipc_client::IpcBinding;
use interweave_test_support::e2e::example;
use interweave_transport_api::{MAX_PAYLOAD_BYTES, TransportIdentity};

/// Where the app listens: an `embedded-android` profile listens on a
/// wildcard only.
const WILDCARD: &str = "/ip4/0.0.0.0/tcp/0";

/// A5's figures.
const OPEN_FIGURE: Duration = Duration::from_millis(500);
const END_FIGURE: Duration = Duration::from_secs(1);

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

    /// `text` to `to`'s human endpoint, committed and then accepted
    /// there: the app's store holds it.
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
        let id = message.app_message_id;
        self.until(&format!("message {serial} accepted at the app"), log, |s| {
            matches!(
                s.outbound.get(&id),
                Some(OutboundStatus::Accepted { endpoint }) if endpoint == &human()
            )
        })
        .await;
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

/// A step's `field` as a count of milliseconds.
fn ms(step: &Step, field: &str) -> Duration {
    let ms = step
        .get(field)
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("no {field} in {step:?}"));
    Duration::from_millis(ms)
}

/// Whether the app's row cuts `text` drawn formatted, as the view asks
/// the model (ui-slint's `drawn_body`, with its image text): a row it
/// shows whole has no Read the whole message to press.
fn cut_in_row(text: &str) -> bool {
    let image = |alt: &str| {
        if alt.trim().is_empty() {
            placeholder_en::text(UiText::ImageNotShownWithoutAlt).to_owned()
        } else {
            fill(placeholder_en::text(UiText::ImageNotShown), &[("alt", alt)])
        }
    };
    cut::formatted(&render(text), &image)
        .shown_percent
        .is_some()
}

/// A shape's line of A5's table: its times beside the figures, or that
/// the row shows it whole.
fn report((name, chars, times): &Row) {
    let Some((open, ends)) = times else {
        println!("{name}\t{chars}\twhole in the row: nothing to open");
        return;
    };
    let over = |t: &Duration, figure: Duration| if *t > figure { " OVER" } else { "" };
    println!(
        "{name}\t{chars}\t{open:?}{}\t{:?}{} ({:?})\t{:?}{} ({:?})",
        over(open, OPEN_FIGURE),
        ends[0].0,
        over(&ends[0].0, END_FIGURE),
        ends[0].1,
        ends[1].0,
        over(&ends[1].0, END_FIGURE),
        ends[1].1,
    );
}

/// A shape's name, characters, and its open and End times with End's
/// frame counts, when the row cuts it.
type Row = (
    &'static str,
    usize,
    Option<(Duration, Vec<(Duration, Option<serde_json::Value>)>)>,
);

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

/// The case's two ends: D on this host's loopback, the app provisioned
/// with D as its one peer and started from the launcher, and D's person
/// ready to send.
struct Rig {
    serial: String,
    phone: AdbDevice,
    d: Desktop,
    phone_peer: TransportIdentity,
    sender: Sender,
    screens: Screens,
}

impl Rig {
    async fn start() -> Self {
        let serial = std::env::var("ANDROID_SERIAL").expect("ANDROID_SERIAL names the phone");
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
        Self {
            serial,
            phone,
            d,
            phone_peer,
            sender,
            screens,
        }
    }

    async fn stop(mut sender: Sender, mut phone: AdbDevice, mut d: Desktop) {
        Box::pin(sender.client.close()).await;
        phone.stop();
        d.stop().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device with the app's debug and androidTest builds: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
async fn on_a_device_the_reader_of_each_long_shape_is_timed_open_and_at_its_end() {
    let shots = std::env::var_os("INTERWEAVE_ANDROID_SHOTS").map(PathBuf::from);
    let Rig {
        serial,
        phone,
        d,
        phone_peer,
        mut sender,
        mut screens,
    } = Rig::start().await;

    let mut opened: Vec<String> = Vec::new();
    // Each shape's line is printed once it is measured, so a later
    // shape's failure keeps the times already taken.
    println!(
        "A5 on {serial}: shape, characters, open (figure {OPEN_FIGURE:?}), End formatted and source (figure {END_FIGURE:?}), frames"
    );
    for (serial_no, (name, text)) in (1u8..).zip(shapes()) {
        let chars = text.chars().count();
        let cut = cut_in_row(&text);
        sender.send(&phone_peer, serial_no, text, || d.log()).await;
        let row = tokio::task::block_in_place(|| {
            let frame = |what: &str| {
                if let Some(dir) = &shots {
                    let name = format!("{serial_no:02}-{name}-{what}").replace(' ', "-");
                    shot(&serial, dir, &name);
                }
            };
            if opened.is_empty() {
                let s = screens.open_conversation(PATIENCE);
                frame("conversation");
                passed(&s, &screens);
            }
            if !cut {
                frame("whole-in-row");
                return (name, chars, None);
            }
            let open = screens.read_whole(&opened, PATIENCE);
            if open.get(keys::RESULT).and_then(|v| v.as_str()) != Some(keys::PASS) {
                frame("read-whole-failed");
            }
            passed(&open, &screens);
            let read = open
                .get("read")
                .and_then(|v| v.as_str())
                .expect("the row's focus key")
                .to_owned();
            let mut ends = Vec::new();
            for form in ["formatted", "source"] {
                if form == "source" {
                    let s = screens.reader_switch();
                    passed(&s, &screens);
                }
                let end = screens.reader_end(PATIENCE);
                passed(&end, &screens);
                assert_eq!(
                    end.get("form").and_then(|v| v.as_str()),
                    Some(form),
                    "{name}: End measured the other form: {end:?}"
                );
                assert_eq!(
                    end.get("at-end").and_then(|v| v.as_str()),
                    Some("yes"),
                    // The trace waits 20 s for the end (ui-slint's
                    // `END_PATIENCE`): a slower End is reported with its
                    // time below, so `no` is an end that never came, or
                    // a reader closed first.
                    "{name}, {form}: no frame showed the list at its end within 20 s: {end:?}"
                );
                frame(&format!("{form}-end"));
                ends.push((ms(&end, "ms"), end.get("frames").cloned()));
            }
            let s = screens.close_reader(PATIENCE);
            passed(&s, &screens);
            opened.push(read);
            (name, chars, Some((ms(&open, "ms"), ends)))
        });
        report(&row);
    }

    Rig::stop(sender, phone, d).await;
}

/// When Back reaches the reader after Read the whole message: ux-
/// designer's figure for the A/B of 2026-11-11, about 0.2 s, inside the
/// list's opening on the A40 and before its bar is due at 0.5 s.
const BACK_AFTER: Duration = Duration::from_millis(200);

/// The same past the first go and the bar: design/2026-11-04-long-message.md
/// section 5.4's run of about 0.6 s, on the shapes the A40 opens in about
/// 0.5 and 0.75 s (device-runs/2026-11-12-samsung-a40-pixel-9a.md).
const BACK_PAST_FIRST_GO: Duration = Duration::from_millis(600);

/// A person who presses Back while the reader opens: the list shape sent
/// as in the timed case, its Read the whole message pressed, and Back
/// [`BACK_AFTER`] later. The reader closes to its conversation with focus
/// back on the row's Read the whole message, so the press was not lost;
/// the case prints whether the reader's rows were drawn before it closed.
///
/// With `INTERWEAVE_ANDROID_SHOTS` set, the screen is recorded across the
/// two keys to `back-<shape>-<ms>ms.mp4` there, for the person who
/// checks whether an opening bar showed: the trace says which page was
/// drawn, not what the screen showed on it.
///
/// What this does NOT prove: the gap between the two keys as the view
/// saw them -- it is the phone shell's, give or take the two `input`
/// commands' start-up -- or when the opening yielded to the Back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device with the app's debug and androidTest builds: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
async fn on_a_device_back_while_the_reader_opens_closes_it_to_its_conversation() {
    back_while_opening(&["list"], BACK_AFTER).await;
}

/// The same with Back [`BACK_PAST_FIRST_GO`] after the press: on heading
/// autolinks and reference link, the shapes section 5.4 names, and on
/// the two stress shapes. Alone in a conversation the first two open
/// within about 0.4 s on the A40 (2026-11-12), so their Back lands after
/// the rows are in; the stress shapes' openings run past the first go
/// and the bar, so theirs lands inside the opening. Each closes to its
/// conversation with focus back on its row. The case
/// prints each key's phone time beside the trace's (`enter-at`,
/// `back-at`), from which the run's Back-to-close is read against the
/// logcat, and the recording shows whether the bar was up.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device with the app's debug and androidTest builds: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
async fn on_a_device_back_past_the_first_go_closes_the_reader_to_its_conversation() {
    back_while_opening(
        &[
            "heading autolinks",
            "reference link",
            "distinct headings",
            "distinct links",
        ],
        BACK_PAST_FIRST_GO,
    )
    .await;
}

/// Each of `names`' shapes sent in turn, its Read the whole message
/// pressed and Back `after` later ([`Screens::read_whole_then_back`]).
async fn back_while_opening(names: &[&str], after: Duration) {
    let shots = std::env::var_os("INTERWEAVE_ANDROID_SHOTS").map(PathBuf::from);
    let Rig {
        serial,
        phone,
        d,
        phone_peer,
        mut sender,
        mut screens,
    } = Rig::start().await;
    let mut opened: Vec<String> = Vec::new();
    for (serial_no, name) in (1u8..).zip(names) {
        let (name, text) = shapes()
            .into_iter()
            .find(|(shape, _)| shape == name)
            .unwrap_or_else(|| panic!("the {name} shape"));
        assert!(cut_in_row(&text), "the {name} shape is cut in its row");
        sender.send(&phone_peer, serial_no, text, || d.log()).await;
        let back = tokio::task::block_in_place(|| {
            if opened.is_empty() {
                let s = screens.open_conversation(PATIENCE);
                passed(&s, &screens);
            }
            let on_phone = "/data/local/tmp/interweave-back-during-opening.mp4";
            let mut recording = None;
            let back = screens.read_whole_then_back(&opened, after, PATIENCE, || {
                recording = shots.as_ref().map(|_| {
                    let child = Command::new("adb")
                        .args([
                            "-s",
                            &serial,
                            "shell",
                            "screenrecord",
                            "--time-limit",
                            "4",
                            on_phone,
                        ])
                        .spawn()
                        .expect("adb screenrecord");
                    // screenrecord takes a moment to start its encoder.
                    std::thread::sleep(Duration::from_secs(1));
                    child
                });
            });
            if let (Some(mut child), Some(dir)) = (recording, &shots) {
                child.wait().expect("screenrecord ended");
                let file = format!("back-{name}-{}ms.mp4", after.as_millis()).replace(' ', "-");
                let pulled = Command::new("adb")
                    .args(["-s", &serial, "pull", on_phone])
                    .arg(dir.join(file))
                    .status()
                    .expect("adb pull");
                let removed = Command::new("adb")
                    .args(["-s", &serial, "shell", "rm", "-f", on_phone])
                    .status()
                    .expect("adb rm");
                assert!(
                    pulled.success() && removed.success(),
                    "the recording pulled and removed"
                );
            }
            back
        });
        passed(&back, &screens);
        let field = |key: &str| back.get(key).and_then(|v| v.as_str()).unwrap_or("?");
        println!(
            "Back {after:?} after Read the whole message on {name}: closed to its conversation; rows drawn before it closed: {}; Enter launched {}, Back launched {} (phone time)",
            field("drawn"),
            field("enter-at"),
            field("back-at"),
        );
        opened.push(field("read").to_owned());
    }
    Rig::stop(sender, phone, d).await;
}

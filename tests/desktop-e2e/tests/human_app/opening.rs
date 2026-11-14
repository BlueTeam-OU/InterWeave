// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A5's single Backs while the reader opens, on the desktop, where Back is
//! Escape (design/2026-11-04-long-message.md A5 and section 5.4). The runs
//! of two Backs are Android's alone (A5): a second Escape in the
//! conversation shows no step to observe.
//!
//! The shapes are the Android case's (`tests/android-e2e/src/long.rs`),
//! each at its largest, each in a conversation of its own so its row's
//! Open the whole message is the only one shown. The row's Open the whole
//! message is reached by keyboard and pressed by Return, as on the phone
//! by Enter, and the Escape is a person's key, both through XTEST
//! ([`display::Hand`]), each stamped on the host's monotonic clock the
//! moment before it is sent. The client is a test build of
//! `human-desktop` with its `ui-trace` feature, whose window writes what
//! it shows to stderr in kinds, each line stamped on the same clock
//! (`apps/human-desktop/src/trace.rs`). The rounds are then judged as the
//! phone's are (`tests/android-e2e/src/back_run.rs`): void when the first
//! screen was drawn before the Escape, otherwise a pass only when the
//! pages were the reader then the conversation, the close within the
//! limit, and the row's Open the whole message holds the focus after it,
//! as the trace's last focus line says.
//!
//! What this does NOT prove: the times are the press's send and the
//! window's trace line, which is logged when the page changed, a frame
//! before the screen shows it; the first screen is no frame either, since
//! the desktop's software renderer tells none: its line (`reader=drawn
//! .. frame=untold`) is when the rows were handed to the window, before
//! the frame that draws them, so a round is void on it early, never
//! passed on it; that no frame showed the opening bar (no
//! frame is read here); the shipped build, which carries no trace; and
//! any window system but the X server the case runs on.

use std::path::PathBuf;
use std::time::Duration;

use interweave_android_e2e_tests::back_run::{self, Run, Times, Verdict};
use interweave_android_e2e_tests::{long, screen};
use interweave_human_ui_model::{UiText, placeholder_en, short_peer};
use interweave_transport_api::TransportIdentity;

use crate::a11y::{Bus, Window};
use crate::display::{self, ESCAPE, Hand, RETURN};
use crate::harness::{self as app, App, until_lease};
use crate::world::{Seeded, World, seed_inbound, two_daemons};

/// The variable naming the traced client: `human-desktop` built with its
/// `ui-trace` feature, optimised, as A5 asks of its figures.
const TRACED: &str = "INTERWEAVE_TRACED_DESKTOP";

/// A5's closing limits: inside the first turn the reader closes within
/// 1 s of the Back, past it within 0.15 s.
const CLOSE_IN_FIRST_TURN: Duration = Duration::from_secs(1);
const CLOSE_PAST_FIRST_TURN: Duration = Duration::from_millis(150);

/// The Escape past the first turn, while the bar shows: A5's about 0.6 s.
const BACK_PAST_FIRST_TURN: Duration = Duration::from_millis(600);

/// The stress shapes (`long.rs`), whose openings run past the first turn
/// on the A40; A5 runs the Escape at 0.6 s on them alone.
const STRESS: [&str; 2] = ["distinct headings", "distinct links"];

/// The shape whose Escape comes inside the first turn.
const LIST: &str = "list";

/// How many steps back from the message field Open the whole message may
/// be: past every link control a row's preview carries, with room.
const STEPS_BACK: usize = 60;

/// How long a key is given to move the focus before the trace is read.
const KEY_SETTLE: Duration = Duration::from_millis(150);

/// Plain opens of the list shape its first-screen time is read from: the
/// first open of a session measures cold (136 ms against 27 ms after it,
/// on this host's first run), and A5's Escape at half that time must
/// come before the warm first screen the rounds meet.
const PLAIN_OPENS: usize = 3;

/// Rounds of each run, as on the phone.
const ROUNDS: usize = 3;

/// How long a round waits for the close after the Escape before it reads
/// the trace: far past both limits, so a late close is measured, not
/// missed.
const SETTLE: Duration = Duration::from_secs(3);

/// The conversations' peers, one a shape; none needs a daemon, since the
/// store does not ask who is reachable.
const C: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";
const D: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

fn traced_binary() -> PathBuf {
    let bin = std::env::var_os(TRACED).map_or_else(
        || {
            panic!(
                "{TRACED} is not set: build the traced client with `cargo build --release \
                 -p interweave-human-desktop --features ui-trace --target-dir target/traced` \
                 and name target/traced/release/human-desktop"
            )
        },
        PathBuf::from,
    );
    assert!(
        bin.is_file(),
        "{TRACED} names {}, which is no file",
        bin.display()
    );
    bin
}

/// The trace's lines in a traced client's stderr, each with its time in
/// ms on the monotonic clock, from `from_ms` on. A stamped line whose
/// time cannot be read is an error, never skipped: skipping one would
/// move every later line onto its neighbour's time.
fn trace_since(log: &str, from_ms: u64) -> Result<Vec<(u64, String)>, String> {
    const STAMP: &str = "human-desktop: mono-us=";
    let mut out = Vec::new();
    for l in log.lines() {
        let Some(rest) = l.strip_prefix(STAMP) else {
            continue;
        };
        let (at, line) = rest
            .split_once(' ')
            .and_then(|(at, line)| Some((at.parse::<u64>().ok()?, line)))
            .ok_or_else(|| format!("a trace line with no monotonic time: {l:?}"))?;
        let at = at / 1000;
        if at >= from_ms && line.starts_with(screen::TRACE) {
            out.push((at, line.to_owned()));
        }
    }
    Ok(out)
}

/// One shape's conversation: its peer and its body.
struct Shape {
    name: &'static str,
    peer: TransportIdentity,
}

/// The client on A, a traced build, with the three shapes each seeded
/// unread in a conversation of its own.
struct Opened<'b> {
    client: App,
    window: Window<'b>,
    hand: Hand,
    shapes: Vec<Shape>,
}

impl Opened<'_> {
    fn pid(&self) -> u32 {
        self.client.child.id()
    }

    /// The last focus key the trace said since byte `mark` of the log.
    fn focus_since(&self, mark: usize) -> Option<String> {
        let log = self.client.log();
        let said = format!("{}focus=", screen::TRACE);
        log.get(mark..)?
            .lines()
            .rev()
            .find_map(|l| Some(l.split_at(l.find(&said)? + said.len()).1.trim().to_owned()))
    }

    /// `shape`'s conversation shown and its row's Open the whole message
    /// focused, reached as a keyboard reaches it: from the message field,
    /// back along the focus order past the row's link controls (section 9:
    /// Tab reaches it after the bubble and before the links). A row with
    /// many link controls is taller than the list, which opens on its end,
    /// so the control stands above it in sight of no pointer until focus
    /// reveals it. (A wheel turned through XTEST did not move that list on
    /// this display on 2026-11-14, and nothing showed it moves any list
    /// here, so it is not used: whether a person's wheel does is not
    /// what this case asks.) Its focus key, `read:<handle>`.
    async fn focus_open(&self, shape: &Shape) -> String {
        let title = short_peer(shape.peer.as_str());
        let row = self
            .window
            .until(
                &format!("Chats' row for {}", shape.name),
                |e| {
                    e.role == "list item"
                        && e.name == title
                        && e.actions.iter().any(|a| a == "click")
                },
                || self.client.log(),
            )
            .await;
        self.window.activate(&row, "click").await;
        let field = self
            .window
            .until(
                "the message field",
                |e| e.role == "entry" && e.name == placeholder_en::text(UiText::Composer),
                || self.client.log(),
            )
            .await;
        let (x, y, w, h) = self.window.extents(&field).await;
        let mark = self.client.log().len();
        tokio::task::block_in_place(|| {
            self.hand.click(self.pid(), x + w / 2, y + h / 2);
            for _ in 0..STEPS_BACK {
                std::thread::sleep(KEY_SETTLE);
                if let Some(key) = self.focus_since(mark).filter(|k| k.starts_with("read:")) {
                    return key;
                }
                self.hand.shift_tab();
            }
            panic!(
                "{}'s Open the whole message never took the focus in {STEPS_BACK} steps back: {}",
                shape.name,
                self.client.log()
            )
        })
    }

    /// Open the whole message pressed by Return on `shape`'s row, then
    /// Escape `after` later: the round's times, its trace and whether the
    /// row's Open the whole message holds the focus once it settled.
    /// `after` `None` presses Escape only once the first screen is drawn:
    /// the plain open the list shape's first-screen time is read from.
    async fn round(
        &self,
        shape: &Shape,
        after: Option<Duration>,
    ) -> (Times, Vec<(u64, String)>, bool) {
        let read = self.focus_open(shape).await;
        let (open, escape) = tokio::task::block_in_place(|| {
            let open = self.hand.key(RETURN);
            let escape = if let Some(after) = after {
                let due = open + u64::try_from(after.as_micros()).expect("a short wait");
                std::thread::sleep(Duration::from_micros(
                    due.saturating_sub(display::mono_us()),
                ));
                self.hand.key(ESCAPE)
            } else {
                let deadline = std::time::Instant::now() + SETTLE * 10;
                let drawn = || {
                    trace_since(&self.client.log(), open / 1000)
                        .unwrap_or_default()
                        .iter()
                        .any(|(_, l)| l.contains("reader=drawn "))
                };
                while !drawn() {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the reader never drew: {}",
                        self.client.log()
                    );
                    std::thread::sleep(Duration::from_millis(20));
                }
                self.hand.key(ESCAPE)
            };
            std::thread::sleep(SETTLE);
            (open, escape)
        });
        let trace = trace_since(&self.client.log(), open / 1000)
            .unwrap_or_else(|e| panic!("{e}\n{}", self.client.log()));
        let sent = [
            ("ENTER".to_owned(), Some(open / 1000)),
            ("BACK".to_owned(), Some(escape / 1000)),
        ];
        // The focus the trace said last: where it settled after the close.
        let focused = trace
            .iter()
            .rev()
            .find_map(|(_, l)| l.strip_prefix(screen::TRACE)?.strip_prefix("focus="))
            == Some(read.as_str());
        (back_run::times(&sent, &trace), trace, focused)
    }
}

/// A person who presses Escape while the reader opens, on the traced
/// client: inside the first turn on the list shape, at about half its
/// first-screen time as this run measures it, closing within 1 s; and at
/// 0.6 s on each stress shape, closing within 0.15 s. Each round passes
/// when the reader closes to its conversation within its limit with the
/// focus on the row's Open the whole message; void when the first screen
/// was drawn before the Escape. Prints each round's Escape to the close
/// and how long the first screen showed; fails naming every failed and
/// every void round.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "a timing run on a traced build: INTERWEAVE_TRACED_DESKTOP names it (see traced_binary)"]
async fn escape_while_the_reader_opens_closes_it_to_its_conversation() {
    let binary = traced_binary();
    let _focus = display::exclusive().await;
    let world = two_daemons().await;
    let opened_bus = Bus::connect().await;
    let opened = start(&world, &opened_bus, &binary).await;

    let (mut failed, mut void) = (Vec::new(), Vec::new());
    let mut judged = |what: String, run: Run, t: &Times, trace: &[(u64, String)], focused: bool| {
        let verdict = match back_run::judge(run, t, trace) {
            Verdict::Pass if !focused => {
                Verdict::Fail("the focus is not on the row's Open the whole message".to_owned())
            }
            v => v,
        };
        let field = |v: Option<u64>| v.map_or("-".to_owned(), |v| v.to_string());
        let result = match &verdict {
            Verdict::Pass => "pass".to_owned(),
            Verdict::Fail(why) => format!("FAIL: {why}"),
            Verdict::Void(why) => format!("{}: {why}", back_run::VOID),
        };
        println!(
            "{what}: {result}; Escape to close {} ms, first screen shown {} ms; ms: open sent {}, Escape sent {}, first screen {}, closed {}",
            field(t.back_to_close()),
            field(t.first_screen_shown()),
            field(t.enter),
            field(t.back),
            field(t.drawn),
            field(t.closed),
        );
        match verdict {
            Verdict::Pass => {}
            Verdict::Fail(why) => failed.push(format!("{what}: {why}")),
            Verdict::Void(why) => void.push(format!("{what}: {why}")),
        }
    };

    let list = opened
        .shapes
        .iter()
        .find(|s| s.name == LIST)
        .expect("the list shape");
    let mut plain = Vec::new();
    for _ in 0..PLAIN_OPENS {
        let (t, _, _) = opened.round(list, None).await;
        plain.push(
            t.drawn
                .and_then(|d| d.checked_sub(t.enter?))
                .expect("the plain open's first screen"),
        );
    }
    let first_screen = *plain.iter().min().expect("a plain open");
    println!(
        "list: first screen {plain:?} ms after the press on plain opens; the least, {first_screen} ms, is halved"
    );
    let half = Duration::from_millis(first_screen / 2);
    for n in 1..=ROUNDS {
        let (t, trace, focused) = opened.round(list, Some(half)).await;
        judged(
            format!("{LIST}, Escape {half:?} after the press, round {n}"),
            Run::Single {
                within: CLOSE_IN_FIRST_TURN,
            },
            &t,
            &trace,
            focused,
        );
    }
    for name in STRESS {
        let shape = opened
            .shapes
            .iter()
            .find(|s| s.name == name)
            .expect("a stress shape");
        for n in 1..=ROUNDS {
            let (t, trace, focused) = opened.round(shape, Some(BACK_PAST_FIRST_TURN)).await;
            judged(
                format!("{name}, Escape {BACK_PAST_FIRST_TURN:?} after the press, round {n}"),
                Run::Single {
                    within: CLOSE_PAST_FIRST_TURN,
                },
                &t,
                &trace,
                focused,
            );
        }
    }
    drop(judged);
    let mut client = opened.client;
    let _ = client.child.kill();
    let _ = client.child.wait();
    if let Some(failure) = back_run::case_failure(&failed, &void, None) {
        panic!("{failure}");
    }
}

/// The traced client on `world`'s A, the three shapes seeded unread
/// before it starts, its window on the bus and given the focus.
async fn start<'b>(world: &World, bus: &'b Bus, binary: &std::path::Path) -> Opened<'b> {
    let bodies = long::shapes();
    let peers = [
        world.b_peer.clone(),
        TransportIdentity::parse(C).expect("a PeerId"),
        TransportIdentity::parse(D).expect("a PeerId"),
    ];
    let names = [LIST, STRESS[0], STRESS[1]];
    let texts: Vec<&str> = names
        .iter()
        .map(|name| {
            bodies.iter().find(|(n, _)| n == name).map_or_else(
                || panic!("long.rs has no {name}"),
                |(_, body)| body.as_str(),
            )
        })
        .collect();
    let seeded: Vec<Seeded<'_>> = peers
        .iter()
        .zip(&texts)
        .zip(1_u64..)
        .map(|((peer, text), serial)| Seeded {
            peer,
            serial,
            text: Some(text),
            received_at: 10_000 + serial,
            kept: false,
        })
        .collect();
    seed_inbound(&world.a, &seeded);
    let client = app::start_binary(binary, &world.a, &[]);
    until_lease(&world.a.binding(), true, &client).await;
    let window = bus.window_of(client.child.id(), || client.log()).await;
    display::focus(client.child.id(), || client.log()).await;
    let shapes = names
        .into_iter()
        .zip(peers)
        .map(|(name, peer)| Shape { name, peer })
        .collect();
    Opened {
        client,
        window,
        hand: Hand::new(),
        shapes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamped_line_is_read_in_ms_from_the_round_on() {
        let log = "human-desktop: the daemon answers\n\
                   human-desktop: mono-us=1234999000 ui-trace page=conversation\n\
                   human-desktop: mono-us=1235004017 ui-trace page=reader\n\
                   human-desktop: mono-us=1235412900 ui-trace reader=drawn ms=412\n";
        assert_eq!(
            trace_since(log, 1_235_000).expect("readable"),
            [
                (1_235_004, "ui-trace page=reader".to_owned()),
                (1_235_412, "ui-trace reader=drawn ms=412".to_owned()),
            ]
        );
    }

    #[test]
    fn a_stamped_line_with_no_time_is_an_error() {
        let log = "human-desktop: mono-us=soon ui-trace page=reader\n";
        assert!(trace_since(log, 0).is_err());
    }
}

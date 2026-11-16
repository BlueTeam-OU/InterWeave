// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! TM-45 (`tests/SECURITY-MATRIX.md`): the lines `App::pump` writes about
//! a failure are a class and nothing else. The desktop's stderr is the
//! client's only log, and a message body, a card's name or surname, or an
//! endpoint must never reach it. Each case plants such words in the store
//! and the node, forces one failure path through the real view on the
//! testing backend, the facade thread and a real store (the app-core test
//! hooks fail its reads), reads the line through the app's log sink
//! ([`App::with_log`]), and holds it to its exact class and every line
//! to none of the planted words.
//!
//! `App::pump` writes seven failure-path lines. Held here: three of them
//! -- "the message store's pending rows could not be read", "the
//! transport side has stopped" and a command's "<name> failed: <class>"
//! (as a page read and as a mark-read) -- and two session lines, ready
//! and refused. What it does NOT reach, for want of a hook on the host:
//! the other four failure-path lines -- "the message store could not be
//! listed" (the start's own store reads), "unread messages could not be
//! read again", "cards or contacts could not be read" and "a card was
//! accepted and its share could not be recorded yet". Their text is
//! fixed or carries the `{why:?}` of a `Failure`, a fieldless class enum
//! (`app-core`'s `protocol.rs`). Nor the session lines "reconnecting,
//! attempt N", "storage degraded" and "closed"; `session_class`'s unit
//! test in `app.rs` holds the reconnecting one's text, and the other two
//! are fixed. The daemon's log is p2p-network-dev's.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use interweave_human_app_core::test_hooks::{Fault, StoreFaults};
use interweave_human_app_core::{FacadeSide, Opener};
use interweave_human_desktop::app::{App, SlintSurface};
use interweave_human_desktop::facade_thread::FacadeThread;
use interweave_human_store::{HumanStore, RowId, StoreError, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{ConversationKey, RuntimeHost};
use interweave_human_ui_slint::View;
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::ComponentHandle as _;

type Desktop = App<SlintSurface, NoLinks>;

/// Words that appear nowhere but in the content they are planted in, so
/// a match in a log line can only be that content.
const BODY: &str = "tm45logbodyquokka";
const NAME: &str = "Tm45Lognameaxolotl";
const SURNAME: &str = "Tm45Logsurnamepangolin";
const ENDPOINT: &str = "tm45logendpointcapybara";

const PLANTED: [&str; 4] = [BODY, NAME, SURNAME, ENDPOINT];

thread_local! {
    /// The lines this test's thread wrote: the pump runs on it, and each
    /// `#[test]` has a thread of its own.
    static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn record(line: &str) {
    LINES.with(|lines| lines.borrow_mut().push(line.to_owned()));
}

fn lines() -> Vec<String> {
    LINES.with(|lines| lines.borrow().clone())
}

fn human() -> EndpointId {
    EndpointId::parse(ENDPOINT).expect("endpoint")
}

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer")
}

fn node(enabled: bool) -> FakeConfig {
    let mut endpoint = FakeEndpoint::open(human(), false);
    endpoint.enabled = enabled;
    FakeConfig {
        peer: peer(),
        endpoints: vec![endpoint],
        default_endpoint: Some(human()),
        queue_bound: 16,
    }
}

fn direct(from: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: from.clone(),
        endpoint: Some(human()),
    }
}

struct NoLinks;

impl Opener for NoLinks {
    fn open(&mut self, _destination: &str) {
        panic!("no link is activated in these tests");
    }

    fn ask_network_access(&mut self) {
        panic!("the desktop root never reports network access withheld");
    }

    fn authenticate(&mut self) {
        panic!("the desktop root never reports authentication needed");
    }
}

/// A store holding `count` unread text rows from `from`, each carrying
/// the planted body word, and a contact on that route with the planted
/// name and surname.
fn planted(from: &TransportIdentity, count: u32) -> HumanStore {
    use interweave_human_store::{AppMessageId, InboundKind, InboundOrigin, NewInbound};
    use interweave_transport_api::MediaType;
    let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
    for n in 1..=count {
        let id = format!("{n:032x}");
        store
            .commit_unread_inbound(&NewInbound {
                app_message_id: AppMessageId::parse(id.clone()).expect("an id"),
                origin: InboundOrigin {
                    peer: from.clone(),
                    endpoint: Some(human()),
                    channel: None,
                },
                media_type: Some(
                    MediaType::parse("application/vnd.interweave-human-chat+json;v=2")
                        .expect("media"),
                ),
                payload: format!(
                    r#"{{"v":2,"kind":"text","app_message_id":"{id}","text":"{BODY} {n}"}}"#
                )
                .into_bytes(),
                received_at: u64::from(n),
                kind: InboundKind::Text,
            })
            .expect("committed");
    }
    store
        .accept_card(
            &interweave_human_store::AcceptedCard {
                peer: from,
                endpoint: &human(),
                name: NAME,
                surname: Some(SURNAME),
                identity_key: [7; 32],
                photo: None,
            },
            1,
        )
        .expect("a contact");
    store
}

/// How the desktop is built: whether the facade's `make` fails, and
/// whether the node lets the client lease its endpoint.
struct Build {
    make_fails: bool,
    endpoint_enabled: bool,
}

const WORKING: Build = Build {
    make_fails: false,
    endpoint_enabled: true,
};

fn desktop(store: HumanStore, faults: &Arc<StoreFaults>, build: &Build) -> Desktop {
    desktop_with_probe(store, faults, build, Arc::new(AtomicBool::new(false)))
}

/// [`desktop`], the facade thread's daemon probe panicking once `die` is
/// set: the thread then ends the way a crashed one does, with the start
/// done and a conversation shown (`FacadeThread::close` consumes the
/// thread, so the app could not send a command after it).
fn desktop_with_probe(
    store: HumanStore,
    faults: &Arc<StoreFaults>,
    build: &Build,
    die: Arc<AtomicBool>,
) -> Desktop {
    let (a, _b) = FakeNetwork::pair(node(build.endpoint_enabled), node(true));
    let faults = Arc::clone(faults);
    let make_fails = build.make_fails;
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    view.window().show().expect("shown");
    let thread = FacadeThread::spawn(
        move || {
            if make_fails {
                return Err(StoreError::Degraded);
            }
            let client = TransportClient::new(
                a.clone(),
                a,
                store,
                ClientConfig {
                    client_kind: "human-client".to_owned(),
                    endpoints: vec![human()],
                    channels: vec![],
                    max_payload_bytes: 49_152,
                },
                Box::new(|| 1_786_600_000_000),
                0,
            )
            .expect("facade");
            Ok(FacadeSide::new(client, || 1_786_600_000_000).with_store_faults(faults))
        },
        move || {
            assert!(!die.load(Ordering::SeqCst), "the facade thread dies here");
            Ok(true)
        },
        Arc::new(|| {}),
        |_| {},
    )
    .expect("the facade thread");
    App::new(SlintSurface(view), NoLinks, thread).with_log(record)
}

fn view(app: &Desktop) -> &View {
    &app.side().surface().0
}

/// A few frames of the toolkit's work, each frame's layout forced by a
/// query of the element tree (the list reports what is in view only once
/// laid out), then one turn of the app.
fn turn(app: &mut Desktop) {
    for _ in 0..2 {
        slint::platform::update_timers_and_animations();
        let _ = i_slint_backend_testing::ElementQuery::from_root(view(app).window()).find_first();
    }
    app.pump();
}

/// Turn until `done`, or fail after a while.
fn until(app: &mut Desktop, what: &str, done: impl Fn(&Desktop) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(app) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}; the log so far: {:?}",
            lines()
        );
        turn(app);
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Turn until a line starting with `class` is written, and return it: the
/// control that the path ran.
fn until_line(app: &mut Desktop, class: &str) -> String {
    until(app, class, |_| lines().iter().any(|l| l.starts_with(class)));
    lines()
        .into_iter()
        .find(|l| l.starts_with(class))
        .expect("the line")
}

/// No line of the log holds any planted word.
fn assert_content_free() {
    for line in lines() {
        for word in PLANTED {
            assert!(
                !line.to_lowercase().contains(&word.to_lowercase()),
                "planted {word:?} in the log line {line:?}"
            );
        }
    }
}

/// The conversation with `from` selected, once its summary is shown.
fn select(app: &mut Desktop, from: &TransportIdentity) {
    let key = direct(from);
    until(app, "the conversation's summary", |app| {
        app.side()
            .model()
            .conversations()
            .iter()
            .any(|c| c.key == key)
    });
    app.side_mut().surface_mut().0.select(key);
}

/// A session leased: the line names the class and the time, and never
/// the endpoint the lease is on (planted as `ENDPOINT`). Mutation:
/// `session_class` (`app.rs`) naming the state with `{state:?}`, whose
/// `Ready` carries the endpoint -- the exact-line assertion and the
/// planted-word scan both fail.
#[test]
fn a_ready_session_is_logged_without_its_endpoint() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    let mut app = desktop(planted(&from, 1), &faults, &WORKING);
    let line = until_line(&mut app, "session ");
    assert!(
        line.starts_with("session ready at ") && line.ends_with(" ms"),
        "{line:?}"
    );
    assert_content_free();
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

/// A refused session is logged by the refusal's class. Mutation: the
/// `Refused` arm of `session_class` (`app.rs`) naming `{state:?}`
/// (`Refused { problem: .. }`): the exact-line assertion fails.
#[test]
fn a_refused_session_is_logged_by_its_class() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    let build = Build {
        make_fails: false,
        endpoint_enabled: false,
    };
    let mut app = desktop(planted(&from, 1), &faults, &build);
    let line = until_line(&mut app, "session refused");
    assert!(
        line.starts_with("session refused: NotAvailableToThisClient at ") && line.ends_with(" ms"),
        "{line:?}"
    );
    assert_content_free();
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

/// The facade that cannot be built because the store's pending rows
/// cannot be read: one class line, and nothing of what the store holds.
/// Mutation: the line (`app.rs`, `FromFacade::FacadeFailed`) saying what
/// the model holds, `{:?}` of its conversations' summaries -- the
/// exact-line assertion fails.
#[test]
fn a_facade_that_cannot_be_built_is_logged_as_that_class_alone() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    let build = Build {
        make_fails: true,
        endpoint_enabled: true,
    };
    let mut app = desktop(planted(&from, 1), &faults, &build);
    let line = until_line(&mut app, "the message store's pending rows");
    assert_eq!(line, "the message store's pending rows could not be read");
    assert_content_free();
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

/// A command handed to a facade thread that has ended: the line says the
/// transport side has stopped, and not which command. The thread is made
/// to end (its daemon probe panics) after the start, with a conversation
/// shown; opening it then asks for a page, which nobody takes. Mutation:
/// the line (`app.rs`, `facade.command` returning false) naming the
/// command with `{command:?}` -- the exact-line assertion fails.
#[test]
fn a_command_for_a_stopped_facade_is_logged_as_that_class_alone() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    let die = Arc::new(AtomicBool::new(false));
    let mut app = desktop_with_probe(planted(&from, 3), &faults, &WORKING, Arc::clone(&die));
    let key = direct(&from);
    until(&mut app, "the conversation's summary", |app| {
        app.side()
            .model()
            .conversations()
            .iter()
            .any(|c| c.key == key)
    });
    die.store(true, Ordering::SeqCst);
    // The probe runs once a second: turn past it, the thread ended.
    let end = Instant::now() + Duration::from_millis(2_500);
    while Instant::now() < end {
        turn(&mut app);
        std::thread::sleep(Duration::from_millis(10));
    }
    app.side_mut().surface_mut().0.select(key);
    let line = until_line(&mut app, "the transport side has stopped");
    assert_eq!(line, "the transport side has stopped");
    assert_content_free();
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

/// A page read that fails is a command that failed: named by its kind and
/// the failure's class, with no row text and no peer. Mutation: the line
/// (`app.rs`, `Problem::Command`) printing `{command:?}` in place of the
/// command's name -- the exact-line assertion fails.
#[test]
fn a_failed_page_read_is_logged_by_the_command_and_the_class() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    faults.set_pages(Fault::Fail);
    let mut app = desktop(planted(&from, 3), &faults, &WORKING);
    select(&mut app, &from);
    let line = until_line(&mut app, "fetch page failed");
    assert_eq!(line, "fetch page failed: StorageUnavailable");
    assert_content_free();
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

/// A read the store refuses for one row is a command that failed: the
/// line names the command and the class, and nothing of the row.
/// Mutation: the line (`app.rs`, `Problem::Command`) printing the row's
/// content -- the exact-line assertion fails.
#[test]
fn a_refused_mark_read_is_logged_by_the_command_and_the_class() {
    let from = peer();
    let faults = Arc::new(StoreFaults::default());
    for row in 1..=3 {
        faults.refuse_row(RowId::from_stored(row));
    }
    let mut app = desktop(planted(&from, 3), &faults, &WORKING);
    view(&app).set_window_focused(true);
    select(&mut app, &from);
    let line = until_line(&mut app, "mark read failed");
    assert_eq!(line, "mark read failed: StorageUnavailable");
    assert_content_free();
    assert!(app.close(Duration::from_secs(5)), "the facade thread ended");
}

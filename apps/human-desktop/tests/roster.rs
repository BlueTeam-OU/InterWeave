// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The roster on the facade thread, headless (plan §30): the window is
//! told the identities held before any update, and a switch crosses to
//! the facade thread, which brings the identity in front and remembers
//! it. Two are held here over the fake node, composed by hand; what the
//! desktop holds is the profile's held list (`ipc.rs`).
//!
//! A switch a person makes is driven through the headless window's
//! switch and switcher rows, by their accessible actions.
//!
//! What this does NOT prove: a real daemon, a real window, a pointer or
//! keyboard press (the desktop e2e suite's).

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use interweave_human_app_core::{Command, FacadeSide, Opener, Roster, RosterConfig};
use interweave_human_desktop::app::{App, SlintSurface};
use interweave_human_desktop::facade_thread::{FacadeFault, FacadeThread};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{
    ConversationKey, IdentityKey, IdentityState, RuntimeHost, isolated,
};
use interweave_human_ui_slint::View;
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::EndpointId;
use slint::ComponentHandle as _;

fn human() -> EndpointId {
    endpoint_id("human")
}

fn second() -> EndpointId {
    endpoint_id("second")
}

fn node(endpoints: &[EndpointId]) -> FakeConfig {
    FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer"),
        endpoints: endpoints
            .iter()
            .map(|e| FakeEndpoint::open(e.clone(), false))
            .collect(),
        default_endpoint: endpoints.first().cloned(),
        queue_bound: 16,
    }
}

struct NoLinks;

impl Opener for NoLinks {
    fn open(&mut self, _destination: &str) {}
    fn ask_network_access(&mut self) {}
    fn authenticate(&mut self) {}
}

/// The window learns the identities held and the one in front from the
/// facade thread, the model shown so far becoming the front's; a switch
/// it makes reaches the roster, which writes the last-front row, and the
/// model of the identity left is kept. A switch to the identity in front
/// sends nothing. The held identities' rows reach every model.
/// Mutations: `App::pump` dropping `FromFacade::Held` -- the models hold
/// none; the facade thread not handling the switch
/// (`ToFacade::Front` ignored) -- the row is never written; the held
/// identities never set (`FacadeThread::held`) -- the window never
/// names a front.
#[test]
fn the_window_is_told_the_identities_and_a_switch_reaches_the_roster() {
    let (a, _b) = FakeNetwork::pair(node(&[human(), second()]), node(&[human()]));
    let store = HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store");
    let rows = store.handle_speaking_as(human());
    let node = a.clone();
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Daemon).expect("a window");
    let thread = FacadeThread::spawn(
        move || {
            Roster::new(
                RosterConfig {
                    client_kind: "human-client".to_owned(),
                    endpoints: vec![human(), second()],
                    channels: vec![],
                    max_payload_bytes: 49_152,
                    others: vec![second()],
                    default: None,
                    aliases: std::collections::BTreeMap::new(),
                },
                store,
                |_| (node.clone(), node.clone()),
                || 1_786_600_000_000,
                0,
            )
            .map_err(FacadeFault::from)
        },
        || Ok(true),
        Arc::new(|| {}),
        |_| {},
    )
    .expect("the facade thread");
    let mut app = App::new(SlintSurface(view), NoLinks, thread);

    let wait = common::Wait::on("the identities held");
    while app.side().front().is_none() || a.open_sessions() < 2 {
        wait.check();
        app.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(app.side().front(), Some(&human()));
    assert!(app.side().model_of(&second()).is_some(), "its own model");
    // The rows the facade thread reads reach every model, each in front
    // of its own.
    let wait = common::Wait::on("the held identities told");
    let told = |app: &App<SlintSurface, NoLinks>, e: &EndpointId| {
        app.side()
            .model_of(e)
            .map_or(0, |m| m.identities().held().len())
    };
    while told(&app, &human()) < 2 || told(&app, &second()) < 2 {
        wait.check();
        app.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
    let shown = app.side().model_of(&second()).expect("held").identities();
    assert_eq!(
        shown
            .front()
            .map(interweave_human_ui_model::IdentityKey::as_str),
        Some(second().as_str())
    );

    assert!(!app.switch(&human()), "in front already");
    assert!(app.switch(&second()));
    assert_eq!(app.side().front(), Some(&second()));
    let wait = common::Wait::on("the row the roster writes");
    while rows.last_front().expect("read") != Some(second()) {
        wait.check();
        app.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(app.side().model_of(&human()).is_some(), "kept behind");
    assert!(app.close(Duration::from_secs(5)));
}

static FRONT_REPORTS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// A last-front row the roster cannot read is reported once, by class
/// and without the row's content, and the start goes on without it --
/// here with no default, so the first is in front and the line names
/// neither (#769 r2 F4); a readable row reports nothing (#769 r1 F2).
/// Mutation:
/// `FacadeThread` not asking `front_unread` -- no line; the line
/// carrying `{:?}` of the error -- the exact-line assertion fails.
#[test]
fn an_unreadable_last_front_row_is_reported_once_by_class() {
    let planted = |row: Option<&str>| {
        let dir = common::private_tempdir();
        let path = dir.path().join("state").join("human.sqlite3");
        let store = HumanStore::open(&path, &human(), StoreOptions::default()).expect("opens");
        if let Some(row) = row {
            rusqlite::Connection::open(&path)
                .expect("a second connection")
                .execute(
                    "INSERT INTO settings (key, value) VALUES ('last_front', ?1)",
                    [row],
                )
                .expect("a row");
        }
        (dir, store)
    };
    let run = |row: Option<&str>| {
        FRONT_REPORTS.lock().expect("reports").clear();
        let (a, _b) = FakeNetwork::pair(node(&[human()]), node(&[human()]));
        let (_dir, store) = planted(row);
        let thread = FacadeThread::spawn(
            move || {
                Roster::new(
                    RosterConfig {
                        client_kind: "human-client".to_owned(),
                        endpoints: vec![human()],
                        channels: vec![],
                        max_payload_bytes: 49_152,
                        others: vec![],
                        default: None,
                        aliases: std::collections::BTreeMap::new(),
                    },
                    store,
                    |_| (a.clone(), a.clone()),
                    || 1_786_600_000_000,
                    0,
                )
                .map_err(FacadeFault::from)
            },
            || Ok(true),
            Arc::new(|| {}),
            |line| FRONT_REPORTS.lock().expect("reports").push(line.to_owned()),
        )
        .expect("the facade thread");
        // The report is made before the identities are named.
        let wait = common::Wait::on("the identities held");
        while thread.held().is_none() {
            wait.check();
            std::thread::sleep(Duration::from_millis(10));
        }
        let reports = FRONT_REPORTS.lock().expect("reports").clone();
        assert!(thread.close(Duration::from_secs(5)));
        reports
    };
    assert_eq!(
        run(Some("human")),
        ["the identity last in front could not be read; the start went on without it"],
        "a row this build does not write"
    );
    assert!(run(None).is_empty(), "no row, no report");
}

/// The desktop over the roster, two identities held: the first aliased
/// `home`, the second `work`, so the switch and the switcher's rows name
/// them; its window shown wide.
fn held_app(a: &FakeNode, store: HumanStore) -> App<SlintSurface, NoLinks> {
    let node = a.clone();
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 800.0));
    view.window().show().expect("shown");
    let thread = FacadeThread::spawn(
        move || {
            Roster::new(
                RosterConfig {
                    client_kind: "human-client".to_owned(),
                    endpoints: vec![human(), second()],
                    channels: vec![],
                    max_payload_bytes: 49_152,
                    others: vec![second()],
                    default: None,
                    aliases: std::collections::BTreeMap::from([
                        (human(), "home".to_owned()),
                        (second(), "work".to_owned()),
                    ]),
                },
                store,
                |_| (node.clone(), node.clone()),
                || 1_786_600_000_000,
                0,
            )
            .map_err(FacadeFault::from)
        },
        || Ok(true),
        Arc::new(|| {}),
        |_| {},
    )
    .expect("the facade thread");
    let mut app = App::new(SlintSurface(view), NoLinks, thread);
    pump_until(&mut app, "both identities leased and told", |app| {
        a.open_sessions() == 2
            && [human(), second()].iter().all(|e| {
                app.side()
                    .model_of(e)
                    .is_some_and(|m| m.identities().held().len() == 2)
            })
    });
    app
}

fn pump_until(
    app: &mut App<SlintSurface, NoLinks>,
    what: &str,
    done: impl Fn(&App<SlintSurface, NoLinks>) -> bool,
) {
    let wait = common::Wait::on(what);
    while !done(app) {
        wait.check();
        app.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn view_of(app: &App<SlintSurface, NoLinks>) -> &View {
    &app.side().surface().0
}

fn labelled(app: &App<SlintSurface, NoLinks>, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view_of(app).window(), label).collect()
}

fn one(app: &App<SlintSurface, NoLinks>, label: &str) -> ElementHandle {
    let found = labelled(app, label);
    assert_eq!(found.len(), 1, "one element labelled {label:?}");
    found.into_iter().next().expect("one")
}

fn switch_label(alias: &str) -> String {
    format!("Identity: {}. Switch identity", isolated(alias))
}

fn focus_id(app: &App<SlintSurface, NoLinks>) -> String {
    view_of(app).window().get_focus_id().to_string()
}

/// A row the person chooses in the switcher reaches the roster: the
/// switch opens the switcher, the row brings its identity in front, the
/// window names it with Switch back on the switch's snackbar, and the
/// roster writes the last-front row. Mutations: `App::pump` not taking
/// the identity acts -- the switcher never opens; the pump's crossings
/// dropped -- the row is never written.
#[test]
fn a_row_chosen_in_the_switcher_reaches_the_roster_and_the_last_front_row() {
    let (a, _b) = FakeNetwork::pair(node(&[human(), second()]), node(&[human()]));
    let store = HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store");
    let rows = store.handle_speaking_as(human());
    let mut app = held_app(&a, store);
    app.pump();

    one(&app, &switch_label("home")).invoke_accessible_default_action();
    pump_until(&mut app, "the switcher open", |app| {
        view_of(app).window().get_id_switcher().open
    });
    one(&app, "work").invoke_accessible_default_action();
    pump_until(&mut app, "the second in front", |app| {
        app.side().front() == Some(&second())
    });
    app.pump();
    let shown = app.side().model().identities();
    assert_eq!(
        shown.front().map(IdentityKey::as_str),
        Some(second().as_str())
    );
    assert!(shown.switch_snackbar().is_some(), "Switch back offered");
    assert!(!view_of(&app).window().get_id_switcher().open, "closed");
    one(&app, &switch_label("work"));
    one(&app, "Switch back");

    let wait = common::Wait::on("the row the roster writes");
    while rows.last_front().expect("read") != Some(second()) {
        wait.check();
        app.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(app.close(Duration::from_secs(5)));
}

/// Focus on Send when the identity in front stops being able to send
/// goes to the composer's field (identities §5.2, §11 F21): over the
/// roster, a conversation shown with focus on Send, the daemon stops,
/// the identity in front is told starting, and the next turn moves
/// focus. Mutation: `App::pump` dropping `FromFacade::Held` -- the
/// model never hears the state and focus stays on Send.
#[test]
fn send_disabled_by_the_front_identitys_state_moves_focus_to_the_field() {
    let (a, b) = FakeNetwork::pair(node(&[human(), second()]), node(&[human()]));
    let store = HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store");
    let mut app = held_app(&a, store);

    // Bob sends to the first identity from the other node.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let client = TransportClient::new(
        b.clone(),
        b.clone(),
        HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store"),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![human()],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| 1_786_600_000_000),
        0,
    )
    .expect("bob");
    let mut bob = FacadeSide::new(client, || 1_786_600_000_000);
    runtime.block_on(async {
        let _ = bob.turn(0).await;
        let _ = bob
            .execute(
                Command::Send {
                    key: ConversationKey::Direct {
                        peer: a.peer().clone(),
                        endpoint: Some(human()),
                    },
                    draft: "hello".to_owned(),
                },
                1,
            )
            .await;
        let _ = bob.turn(1).await;
    });
    pump_until(&mut app, "bob's conversation", |app| {
        app.side().model().conversations().len() == 1
    });
    let key = app.side().model().conversations()[0].key.clone();
    app.side_mut().surface_mut().0.select(key);
    app.pump();
    view_of(&app).window().set_draft("hi".into());
    app.pump();
    let wait = common::Wait::on("focus on Send");
    while focus_id(&app) != "send" {
        wait.check();
        let text: slint::SharedString = slint::platform::Key::Tab.into();
        let window = view_of(&app).window().window();
        window.dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
        window.dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
    }
    assert_eq!(
        app.side().model().identities().held()[0].state,
        IdentityState::Ready,
        "the premise: it sends"
    );

    a.stop();
    pump_until(&mut app, "the first told starting", |app| {
        matches!(
            app.side().model().identities().held()[0].state,
            IdentityState::Starting { .. }
        )
    });
    app.pump();
    // The view moves focus from a timer, which the window's event loop
    // runs; headless, the test runs them.
    for _ in 0..3 {
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(2));
        slint::platform::update_timers_and_animations();
    }
    assert_eq!(focus_id(&app), "composer", "F21");
    assert!(app.close(Duration::from_secs(5)));
}

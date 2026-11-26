// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The roster on the facade thread, headless (plan §30): the window is
//! told the identities held before any update, and a switch crosses to
//! the facade thread, which brings the identity in front and remembers
//! it. The desktop composes the first identity alone in this batch; two
//! are held here to see the crossing work.
//!
//! What this does NOT prove: a switch a person makes (nothing reaches one
//! in this batch), a real daemon, a real window.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use interweave_human_app_core::{Opener, Roster, RosterConfig};
use interweave_human_desktop::app::{App, SlintSurface};
use interweave_human_desktop::facade_thread::{FacadeFault, FacadeThread};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_ui_model::RuntimeHost;
use interweave_human_ui_slint::View;
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::EndpointId;

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
/// sends nothing. Mutations: the facade thread not handling the switch
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

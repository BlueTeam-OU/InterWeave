// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The start-up refusal screen of the shipped client (plan §18 clause
//! (4), the decision of 2026-10-20), on a real display and read over
//! AT-SPI as a screen reader is handed it: a refusal a person can act on
//! is shown, not only written to stderr, its actions are reached through
//! their accessible actions, and Close keeps the exit class.
//!
//! The release rows (`release.rs`) carry the store from a newer build and
//! the profile a newer build marked; these carry the window already open
//! and a Try again that opens the app once the store is put right. What
//! they do NOT prove: raising the window that is already open (not
//! built: no channel to it exists), and anything a sighted person judges
//! of the screen's look, which is the design review's.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use interweave_human_store::SCHEMA_VERSION;
use interweave_human_ui_model::{UiText, placeholder_en};

use crate::a11y::{Bus, Element};
use crate::common::{Home, PATIENCE, example, free_port, stranger};
use crate::display;
use crate::harness::{self as app, human_lease, until_lease};
use crate::world::app_store;

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

fn home() -> Home {
    let home = Home::new("human-desktop");
    home.write_key();
    let listen = format!("/ip4/127.0.0.1/tcp/{}", free_port(Ipv4Addr::LOCALHOST));
    home.write_config(&example("human-desktop.yaml", &stranger(), &listen, None));
    home
}

fn button(label: &'static str) -> impl Fn(&Element) -> bool {
    move |e: &Element| e.role == "button" && e.name == label && e.actions == ["click"]
}

fn named(label: &'static str) -> impl Fn(&Element) -> bool {
    move |e: &Element| e.name == label
}

/// Write `version` into the store's header under its key (ADR-0060), the
/// WAL folded in first so the header is the one read.
fn set_store_version(home: &Home, version: i64) {
    let conn = interweave_human_store::at_rest::open_writable_for_tests(
        &home.paths,
        &interweave_human_store::key_custody::FilesystemCustody,
    )
    .expect("the store opens under its key");
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpointed");
    conn.pragma_update(None, "user_version", version)
        .expect("the header");
}

/// The store's folder as a name -> (bytes, mode) map: what a write,
/// a new companion file or a changed mode would show.
fn folder(store: &Path) -> BTreeMap<String, (Vec<u8>, u32)> {
    let dir = store.parent().expect("the store's folder");
    std::fs::read_dir(dir)
        .expect("the folder reads")
        .map(|entry| {
            let entry = entry.expect("an entry");
            let path = entry.path();
            let mode = entry.metadata().expect("its metadata").permissions().mode();
            let bytes = if path.is_file() {
                std::fs::read(&path).expect("its bytes")
            } else {
                Vec::new()
            };
            (
                entry.file_name().to_string_lossy().into_owned(),
                (bytes, mode),
            )
        })
        .collect()
}

/// A second window for a profile whose window is open: the screen says
/// so, offers Close and nothing else, and Close ends it with
/// `EX_TEMPFAIL`; the first window keeps the endpoint.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_window_is_refused_on_a_screen_that_only_closes() {
    let _focus = display::exclusive().await;
    let home = home();
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let bus = Bus::connect().await;
    let mut first = app::start(&home);
    until_lease(&home.binding(), true, &first).await;

    let mut second = app::start(&home);
    let screen = bus.window_of(second.child.id(), || second.log()).await;
    screen
        .until("the heading", named(text(UiText::RefusalHeading)), || {
            second.log()
        })
        .await;
    screen
        .until("why", named(text(UiText::RefusalAlreadyOpen)), || {
            second.log()
        })
        .await;
    let close = screen
        .until("Close", button(text(UiText::Close)), || second.log())
        .await;
    let tree = screen.read().await;
    assert!(
        !tree.iter().any(|e| e.name == text(UiText::TryAgain)),
        "no Try again: {}",
        crate::a11y::describe(&tree)
    );
    assert!(
        second.log().contains("already open"),
        "the stderr line is kept: {}",
        second.log()
    );

    screen.activate(&close, "click").await;
    let status = second.exit_within(PATIENCE);
    assert_eq!(status.code(), Some(75), "EX_TEMPFAIL: {}", second.log());
    assert!(
        human_lease(&home.binding()).await.is_some(),
        "the first window keeps the endpoint"
    );
    assert!(first.terminate().success(), "{}", first.log());
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}

/// A store this build cannot read: the screen names it and offers Try
/// again; trying while it is still unreadable says so to the screen
/// reader, takes nothing and writes nothing in the store's folder, and a
/// second window started meanwhile is refused as already open; once the
/// file is put right, Try again opens the app in the same process, which
/// takes the endpoint.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn try_again_opens_the_app_once_the_store_is_put_right() {
    let _focus = display::exclusive().await;
    let home = home();
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let bus = Bus::connect().await;
    // A first run creates the store at this build's version.
    let mut client = app::start(&home);
    until_lease(&home.binding(), true, &client).await;
    assert!(client.terminate().success(), "{}", client.log());
    let store = app_store(&home);
    set_store_version(&home, SCHEMA_VERSION + 1);

    let mut client = app::start(&home);
    let screen = bus.window_of(client.child.id(), || client.log()).await;
    screen
        .until("why", named(text(UiText::RefusalStoreRecovery)), || {
            client.log()
        })
        .await;
    let path = screen
        .until("the file", named(text(UiText::RefusalPath)), || {
            client.log()
        })
        .await;
    assert_eq!(
        screen.text_of(&path).await,
        store.display().to_string(),
        "the store's path, whole"
    );
    screen
        .until("Close", button(text(UiText::Close)), || client.log())
        .await;
    let retry = screen
        .until("Try again", button(text(UiText::TryAgain)), || client.log())
        .await;
    assert!(human_lease(&home.binding()).await.is_none());

    let before = folder(&store);
    let mut heard = screen.announcements().await;
    screen.activate(&retry, "click").await;
    heard
        .until(text(UiText::RefusalStillRefused), || client.log())
        .await;
    assert!(
        human_lease(&home.binding()).await.is_none(),
        "a refused Try again takes nothing"
    );
    assert_eq!(
        client
            .log()
            .matches("cannot be read by this version")
            .count(),
        2,
        "each refusal is a stderr line: {}",
        client.log()
    );
    assert_eq!(
        folder(&store),
        before,
        "a Try again refused for the store's header writes nothing in its folder"
    );

    // The lock is held through the screen (run.rs): a second window
    // started now is refused as already open, not let at the store.
    let mut second = app::start(&home);
    let other = bus.window_of(second.child.id(), || second.log()).await;
    let close = other
        .until("Close", button(text(UiText::Close)), || second.log())
        .await;
    other
        .until("why", named(text(UiText::RefusalAlreadyOpen)), || {
            second.log()
        })
        .await;
    other.activate(&close, "click").await;
    assert_eq!(
        second.exit_within(PATIENCE).code(),
        Some(75),
        "EX_TEMPFAIL while the first sits on its screen: {}",
        second.log()
    );

    set_store_version(&home, SCHEMA_VERSION);
    let retry = screen
        .until("Try again", button(text(UiText::TryAgain)), || client.log())
        .await;
    screen.activate(&retry, "click").await;
    until_lease(&home.binding(), true, &client).await;
    assert!(client.running(), "the app runs on: {}", client.log());
    assert!(client.terminate().success(), "{}", client.log());
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}

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

/// Every file under `dir`, recursively, with its bytes: what a discard
/// must leave as it was.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(tree(&path));
        } else if let Ok(bytes) = std::fs::read(&path) {
            out.insert(path.display().to_string(), bytes);
        }
    }
    out
}

/// ADR-0060 decision 6 on the desktop, through the screen a person sees:
/// a store whose key file is gone opens nothing and reaches recovery mode;
/// Delete stored data… asks first, Cancel removes nothing, and the
/// confirmed discard recreates the store and opens the app with its
/// notice -- the identity, the identity's own key, the profile and the
/// trust all as they were (plan §29 (b); human-client-ui.md §13).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_store_whose_key_is_lost_is_discarded_and_recreated_on_the_persons_word() {
    use interweave_human_store::{identity_key, store_key};
    let _focus = display::exclusive().await;
    let home = home();
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let bus = Bus::connect().await;
    let mut client = app::start(&home);
    until_lease(&home.binding(), true, &client).await;
    assert!(client.terminate().success(), "{}", client.log());
    // A trust change, so the trust overlay exists to be compared.
    crate::package::transportctl_ok(&home, &["trust", "allow", stranger().as_str()], "");

    std::fs::remove_file(store_key::key_path(&home.paths)).expect("the key file goes");
    let store = app_store(&home);
    let kept = |home: &Home| {
        let mut kept = tree(
            home.paths
                .config_file()
                .parent()
                .expect("the profile's folder"),
        );
        kept.extend(tree(
            &home.paths.state_dir().join(identity_key::IDENTITY_KEYS_DIR),
        ));
        for overlay in [
            interweave_profile_config::trust_overlay::TRUST_OVERLAY_FILE,
            interweave_profile_config::endpoint_overlay::ENDPOINT_OVERLAY_FILE,
            interweave_profile_config::profile_format::PROFILE_FORMAT_FILE,
        ] {
            let path = home.paths.state_dir().join(overlay);
            if let Ok(bytes) = std::fs::read(&path) {
                kept.insert(path.display().to_string(), bytes);
            }
        }
        kept.insert(
            "identity".to_owned(),
            std::fs::read(home.paths.identity_file()).expect("the identity"),
        );
        kept
    };
    let before = kept(&home);
    assert!(
        before
            .keys()
            .any(|k| k.ends_with(interweave_profile_config::trust_overlay::TRUST_OVERLAY_FILE)),
        "the control: the trust overlay is there to compare"
    );
    // The store's files: the folder without the single-instance lock,
    // which each start rewrites with its own process.
    let store_files = |store: &Path| {
        let mut files = folder(store);
        files.retain(|name, _| name != interweave_profile_config::lock::HUMAN_CLIENT_LOCK_FILE);
        files
    };
    let human_before = store_files(&store);

    let mut client = app::start(&home);
    let screen = bus.window_of(client.child.id(), || client.log()).await;
    screen
        .until("why", named(text(UiText::RefusalStoreRecovery)), || {
            client.log()
        })
        .await;
    assert!(
        human_lease(&home.binding()).await.is_none(),
        "the store opened nothing"
    );
    let discard = screen
        .until(
            "Delete stored data…",
            button(text(UiText::DiscardStore)),
            || client.log(),
        )
        .await;
    screen.activate(&discard, "click").await;
    screen
        .until(
            "the loss",
            named(text(UiText::DiscardLosesContacts)),
            || client.log(),
        )
        .await;
    let cancel = screen
        .until("Cancel", button(text(UiText::Cancel)), || client.log())
        .await;
    screen.activate(&cancel, "click").await;
    let discard = screen
        .until(
            "Delete stored data…",
            button(text(UiText::DiscardStore)),
            || client.log(),
        )
        .await;
    assert!(
        store_files(&store) == human_before,
        "Cancel removed nothing"
    );

    screen.activate(&discard, "click").await;
    let confirm = screen
        .until(
            "Delete stored data",
            button(text(UiText::DiscardStoreConfirm)),
            || client.log(),
        )
        .await;
    screen.activate(&confirm, "click").await;
    until_lease(&home.binding(), true, &client).await;
    let window = bus.window_of(client.child.id(), || client.log()).await;
    window
        .until("the notice", named(text(UiText::StoreDiscarded)), || {
            client.log()
        })
        .await;
    assert!(client.running(), "the app runs: {}", client.log());
    assert!(
        store_key::exists(&home.paths).expect("readable"),
        "a new key"
    );
    assert!(
        kept(&home) == before,
        "the identity, its key, the profile and the trust are as they were"
    );
    assert!(client.terminate().success(), "{}", client.log());
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}

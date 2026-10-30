// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The windowed start on the shipped client, read over AT-SPI as a screen
//! reader reads it (design/2026-10-27-start-window.md section 10): a store
//! filled before the start, the badges and previews Chats shows from its
//! summaries alone, the opening at the newest rows with the line counting
//! what was not loaded, and the unread rows nothing loaded surviving a
//! restart. What needs a held or failed read -- placement, the failure
//! row, the bounds -- is the headless suite's (`apps/human-desktop/tests/
//! window_faults.rs`), which a shipped binary cannot be given.

use std::time::Duration;

use interweave_human_ui_model::{UiText, fill, placeholder_en, short_peer};
use interweave_transport_api::TransportIdentity;

use crate::a11y::{Bus, Element, Window, describe};
use crate::display;
use crate::harness::{self as app, App, until_lease};
use crate::world::{Seeded, World, rows, seed_inbound, two_daemons, until_rows};

/// A's unread backlog: more than the opening's one page of 64 records.
const A_UNREAD: u64 = 80;
/// The rows the opening's page brings: the facade's page of records
/// (app-core `listing::PAGE_RECORDS`).
const PAGE: u64 = 64;

/// The second conversation's peer, "B" in the design's store: no daemon
/// of the world, since the store does not ask who is reachable.
const B: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

fn text(key: UiText) -> &'static str {
    placeholder_en::text(key)
}

fn b_peer() -> TransportIdentity {
    TransportIdentity::parse(B).expect("a PeerId")
}

/// The design's store: A with [`A_UNREAD`] unread rows, the newest last;
/// B with two read-and-kept rows and one unread row newer than both.
fn the_design_store(world: &World) {
    let a = world.b_peer.clone();
    let b = b_peer();
    let mut seeded: Vec<Seeded<'_>> = Vec::new();
    let texts: Vec<String> = (1..=A_UNREAD).map(|n| format!("A {n}")).collect();
    for (n, text) in (1..=A_UNREAD).zip(&texts) {
        seeded.push(Seeded {
            peer: &a,
            serial: n,
            text: Some(text),
            received_at: 10_000 + n,
            kept: false,
        });
    }
    for (serial, text, at, kept) in [
        (101, "B kept first", 500, true),
        (102, "B kept second", 600, true),
        (103, "B newest", 700, false),
    ] {
        seeded.push(Seeded {
            peer: &b,
            serial,
            text: Some(text),
            received_at: at,
            kept,
        });
    }
    seed_inbound(&world.a, &seeded);
}

/// The client on `world`'s side A: started, holding its lease, its window
/// on the AT-SPI bus and given the focus.
async fn started<'b>(world: &World, bus: &'b Bus) -> (App, Window<'b>) {
    let client = app::start(&world.a);
    until_lease(&world.a.binding(), true, &client).await;
    let window = bus.window_of(client.child.id(), || client.log()).await;
    display::focus(client.child.id(), || client.log()).await;
    (client, window)
}

/// Chats' row for the conversation with `peer`: titled by its short
/// `PeerId` (human-client-ui.md section 4), selectable and pressable.
fn chat_row(peer: &TransportIdentity) -> impl Fn(&Element) -> bool + '_ {
    move |e| {
        e.role == "list item"
            && e.name == short_peer(peer.as_str())
            && e.actions.iter().any(|a| a == "click")
    }
}

/// `peer`'s row once its description says `count` unread.
async fn until_badge(
    window: &Window<'_>,
    client: &App,
    peer: &TransportIdentity,
    count: u64,
) -> Element {
    let unread = fill(text(UiText::UnreadCount), &[("count", &count.to_string())]);
    let row = chat_row(peer);
    window
        .until(
            &format!("{} unread on {}'s row", count, short_peer(peer.as_str())),
            |e| row(e) && e.description.contains(&unread),
            || client.log(),
        )
        .await
}

/// Any `EarlierUnread` line.
fn any_earlier_unread(e: &Element) -> bool {
    let template = text(UiText::EarlierUnread);
    let head = &template[..template.find('{').expect("a count")];
    e.name.starts_with(head)
}

/// Any message item whose body is `body`: its label is `UiText::Item`,
/// "{author}: {body}. {status}, {time}".
fn message(body: &str) -> impl Fn(&Element) -> bool + '_ {
    move |e| {
        e.role == "list item" && e.actions == ["click"] && e.name.contains(&format!(": {body}. "))
    }
}

/// Items 1 and 2: Chats lists both conversations from the summaries --
/// A's badge its whole unread count, B's 1, B's preview its newest row --
/// and a restart with Chats shown reads nothing that changes them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chats_lists_every_conversation_from_its_summary_and_a_restart_changes_nothing() {
    let _focus = display::exclusive().await;
    let world = two_daemons().await;
    the_design_store(&world);
    let bus = Bus::connect().await;
    let b = b_peer();

    for start in ["the first start", "a restart"] {
        let (mut client, window) = started(&world, &bus).await;
        until_badge(&window, &client, &world.b_peer, A_UNREAD).await;
        let b_row = until_badge(&window, &client, &b, 1).await;
        assert!(
            b_row.description.contains("B newest"),
            "{start}: B's preview is its newest row: {:?}",
            b_row.description
        );
        // Nothing was opened, so nothing was read.
        assert_eq!(
            rows(&world.a, "unread_inbound"),
            usize::try_from(A_UNREAD + 1).expect("small"),
            "{start}"
        );
        assert!(client.terminate().success(), "{}", client.log());
    }
}

/// Items 3 and 9: A opens at its newest row, one page of it, with
/// nothing announced; what it loaded is read, so A's badge counts the
/// unread rows the opening did not load -- the line's count, which is
/// out of view above the page; after a restart those rows are still
/// unread, and A's badge counts them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_conversation_opens_at_its_newest_and_its_unloaded_unread_rows_survive() {
    let _focus = display::exclusive().await;
    let world = two_daemons().await;
    the_design_store(&world);
    let bus = Bus::connect().await;

    let (mut client, window) = started(&world, &bus).await;
    let row = until_badge(&window, &client, &world.b_peer, A_UNREAD).await;
    let mut heard = window.announcements().await;
    window.activate(&row, "click").await;
    let newest = window
        .until("A's newest row", message(&format!("A {A_UNREAD}")), || {
            client.log()
        })
        .await;
    // The list exports the rows in view, so the page's size is read
    // from the set each item says it is in, not by finding its oldest
    // row; and the line above the page is out of view, so its count is
    // read where Chats repeats it, as A's badge (design section 10,
    // item 3: "A's badge on Chats equals it").
    assert_eq!(
        newest.attributes.get("posinset").map(String::as_str),
        Some(PAGE.to_string().as_str()),
        "the newest row is the last of one page"
    );
    let tree = window.read().await;
    let sets: Vec<&String> = tree
        .iter()
        .filter(|e| e.role == "label")
        .filter_map(|e| e.attributes.get("setsize"))
        .collect();
    assert!(
        !sets.is_empty() && sets.iter().all(|n| **n == PAGE.to_string()),
        "one page, from the newest:\n{}",
        describe(&tree)
    );
    // The window has the focus: what it loaded is read, and A's badge is
    // the unread rows the opening did not load.
    let not_loaded = A_UNREAD - PAGE;
    until_badge(&window, &client, &world.b_peer, not_loaded).await;
    until_rows(
        &world.a,
        "unread_inbound",
        usize::try_from(not_loaded + 1).expect("small"),
        || client.log(),
    )
    .await;
    let said = heard.within(Duration::from_secs(1)).await;
    assert!(said.is_empty(), "the opening announced {said:?}");
    assert!(client.terminate().success(), "{}", client.log());

    let (mut again, window) = started(&world, &bus).await;
    until_badge(&window, &again, &world.b_peer, not_loaded).await;
    assert!(again.terminate().success(), "{}", again.log());
}

/// Item 13: a row no build can show is counted on no line and no badge
/// and has no Chats row of its own; Chats ends with how many there are;
/// and a conversation whose newest stored row is one shows the newest
/// that decodes as its preview.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rows_the_app_cannot_show_are_counted_apart_and_never_previewed() {
    let _focus = display::exclusive().await;
    let world = two_daemons().await;
    let a = world.b_peer.clone();
    let b = b_peer();
    // A third conversation, with no row that decodes.
    let c = TransportIdentity::parse("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN")
        .expect("a PeerId");
    let row = |peer, serial, text, received_at| Seeded {
        peer,
        serial,
        text,
        received_at,
        kept: false,
    };
    seed_inbound(
        &world.a,
        &[
            row(&a, 1, None, 100),
            row(&a, 2, Some("A first"), 200),
            row(&a, 3, Some("A second"), 300),
            row(&c, 4, None, 400),
            row(&b, 5, Some("B decodes"), 500),
            row(&b, 6, None, 600),
        ],
    );
    let bus = Bus::connect().await;
    let (mut client, window) = started(&world, &bus).await;
    let b_row = until_badge(&window, &client, &b, 1).await;
    assert!(
        b_row.description.contains("B decodes"),
        "B's preview is its newest row that decodes: {:?}",
        b_row.description
    );
    let a_row = until_badge(&window, &client, &a, 2).await;
    let not_shown = fill(text(UiText::RowsNotShown), &[("count", "3")]);
    window
        .until(
            "Chats' count of rows it cannot show",
            |e| e.name == not_shown,
            || client.log(),
        )
        .await;
    let tree = window.read().await;
    assert!(
        !tree.iter().any(chat_row(&c)),
        "a conversation of rows it cannot show has no row:\n{}",
        describe(&tree)
    );

    // Opened, A loads every row it can show at once: no line counts the
    // one it cannot, and once read the badge is 0.
    window.activate(&a_row, "click").await;
    window
        .until("A's oldest row", message("A first"), || client.log())
        .await;
    // B's two rows and the two no build can show stay unread.
    until_rows(&world.a, "unread_inbound", 4, || client.log()).await;
    let read = chat_row(&a);
    window
        .until(
            "A's row, nothing unread",
            |e| read(e) && !e.description.contains(" unread"),
            || client.log(),
        )
        .await;
    let tree = window.read().await;
    assert!(
        !tree.iter().any(any_earlier_unread),
        "the row it cannot show is on no line:\n{}",
        describe(&tree)
    );
    assert!(client.terminate().success(), "{}", client.log());
}

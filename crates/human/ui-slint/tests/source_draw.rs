// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Show source with a character past the cluster bound, drawn by the
//! desktop's own window and software renderer, whose text shaper counts
//! a character's code points in eight bits (HUMAN-CHAT.md, Bounds:
//! "Show source draws a replaced cluster as the body does"). `views.rs`
//! holds the source's text to one U+FFFD on the testing backend, which
//! shapes no text; here the same menu entry is chosen in a real window
//! and the source drawn, the test finding the drawn source's text by the
//! label it speaks. With the replacement taken out the shaper overflows
//! under the overflow checks this test runs with, and it does so at the
//! first render, before the menu opens: the row measures its source
//! pieces with the code font whether or not the source is shown, so the
//! measuring text shapes the same character the drawn source would.
//! One test in this binary, since a process holds one window's event
//! loop.
//!
//! What this cannot prove: the phone's renderer
//! (`tests/android-e2e/tests/cluster_bound.rs`, which did not draw Show
//! source), and the reader's source, which the long-message tests reach
//! on the testing backend only.

#![cfg(feature = "desktop")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use i_slint_backend_testing::{AccessibleRole, ElementQuery};
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::{ConversationKey, RuntimeHost, UiModel, UiText, placeholder_en};
use interweave_human_ui_slint::View;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::{ComponentHandle as _, Model as _};
use unicode_segmentation::UnicodeSegmentation as _;

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn received(row: i64, from: &TransportIdentity, text: &str) -> Received {
    Received {
        row: RowId::from_stored(row),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: human(),
        },
        envelope: HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{:032x}", row + 1_000),
            text: text.to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: u64::try_from(row).expect("positive"),
    }
}

/// What the drawn texts speak, the source's among them once it is shown.
fn spoken_texts(view: &View) -> Vec<String> {
    ElementQuery::from_root(view.window())
        .match_accessible_role(AccessibleRole::Text)
        .find_all()
        .into_iter()
        .filter_map(|e| e.accessible_label().map(|l| l.to_string()))
        .collect()
}

fn draw(view: &View, what: &str) {
    slint::platform::update_timers_and_animations();
    let _ = view.window().window().take_snapshot().expect(what);
}

/// A letter and 255 combining marks in a body with a link whose label
/// would hold it: Show source chosen from the row's menu draws the
/// source with the character as one U+FFFD.
#[test]
fn show_source_draws_a_character_past_the_cluster_bound_in_a_real_window() {
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    view.window().show().expect("shown");
    let from = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let past = format!("a{}", "\u{301}".repeat(255));
    assert_eq!(
        past.graphemes(true)
            .map(|c| c.chars().count())
            .collect::<Vec<_>>(),
        [256],
        "the premise: one character of 256 code points by UAX #29"
    );
    let source = format!("x {past} y [see](https://example.org/{past})");
    let mut model = UiModel::new();
    model.received(received(1, &from, &source));
    view.render(&model);
    view.select(ConversationKey::Direct {
        peer: from,
        endpoint: Some(human()),
    });
    let _ = view.take_events(&model);
    view.render(&model);
    draw(&view, "the conversation drawn");

    let show = placeholder_en::text(UiText::ShowSource);
    let replaced = placeholder_en::text(UiText::ReplacedCharacter);
    let row = view
        .window()
        .get_messages()
        .row_data(0)
        .expect("the message row");
    let spoken = interweave_human_ui_model::cluster::spoken(row.label.as_str(), replaced);
    // The source's pieces as the row would speak them once drawn; the
    // row holds them before the menu is chosen, so the control is that
    // no drawn text speaks them yet.
    let source_spoken: Vec<String> = row
        .source_pieces
        .iter()
        .map(|p| interweave_human_ui_model::cluster::spoken(p.as_str(), replaced).into_owned())
        .collect();
    assert!(!source_spoken.is_empty(), "the source has pieces");
    let before = spoken_texts(&view);
    assert!(
        !source_spoken.iter().any(|s| before.contains(s)),
        "the source is not drawn before Show source is chosen"
    );
    ElementQuery::from_root(view.window())
        .match_accessible_role(AccessibleRole::ListItem)
        .find_all()
        .into_iter()
        .find(|e| e.accessible_label().as_deref() == Some(&*spoken))
        .expect("the row's item")
        .invoke_accessible_default_action();
    draw(&view, "the menu drawn");
    ElementQuery::from_root(view.window())
        .find_all()
        .into_iter()
        .find(|e| {
            e.accessible_label().as_deref() == Some(show)
                && e.accessible_description().as_deref() == Some(&*spoken)
        })
        .expect("the menu's Show source")
        .invoke_accessible_default_action();
    // The row hears the menu's choice through a change callback, which
    // runs on the event loop's turn; the snapshot draws what it shows.
    draw(&view, "the source drawn");

    let pieces = view
        .window()
        .get_messages()
        .row_data(0)
        .expect("the message row")
        .source_pieces;
    assert_eq!(
        pieces.iter().map(|p| p.to_string()).collect::<String>(),
        source.replace(&past, "\u{FFFD}"),
        "the source is shown, the character as one U+FFFD"
    );
    let after = spoken_texts(&view);
    assert!(
        source_spoken.iter().all(|s| after.contains(s)),
        "the source drawn, its texts {source_spoken:?} among {after:?}"
    );
}

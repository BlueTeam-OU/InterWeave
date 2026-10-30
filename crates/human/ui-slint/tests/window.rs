// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The views over the windowed mode (design/2026-10-27-start-window.md):
//! Chats from the conversations' summaries, the open conversation's window
//! and its marks, through Slint's testing backend.
//!
//! What this cannot prove, stated once: that a platform adapter exports
//! this tree, how a screen reader speaks it, Slint's text shaping on a
//! real display (where a right-to-left preview's ellipsis lands), the
//! platform's reduced-motion setting, or anything a store, a facade or a
//! phone does: the model here is driven by hand.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use i_slint_backend_testing::ElementHandle;
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::preview::cut;
use interweave_human_ui_model::{
    ConversationKey, NewestRow, RuntimeHost, StoredSummary, Table, UiModel, UiText, fill,
    placeholder_en,
};
use interweave_human_ui_slint::View;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::ComponentHandle as _;

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn direct(peer: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(human()),
    }
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

fn view() -> View {
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    view.set_clock(|| 60_000, |_| 0);
    view.window().show().expect("shown");
    view
}

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

fn all(view: &View) -> Vec<ElementHandle> {
    i_slint_backend_testing::ElementQuery::from_root(view.window()).find_all()
}

fn described(view: &View, description: &str) -> usize {
    all(view)
        .iter()
        .filter(|e| e.accessible_description().as_deref() == Some(description))
        .count()
}

fn labelled(view: &View, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label).collect()
}

/// A Chats row's description for one unread message previewed as
/// `preview`.
fn one_unread(preview: &str) -> String {
    let one = fill(text(UiText::UnreadCount), &[("count", "1")]);
    fill(
        text(UiText::ConversationDescription),
        &[
            ("kind", text(UiText::DirectConversation)),
            ("unread", &one),
            ("preview", preview),
            ("time", "00:00"),
        ],
    )
}

/// A Chats preview is the model's cut (section 3): 160 characters and the
/// ellipsis, on a window wide enough to show more, and the row's
/// description reads the same cut. Mutation: the view's own first line,
/// uncut.
#[test]
fn a_chats_preview_is_cut_at_its_bound_and_read_with_the_same_cut() {
    let mut view = view();
    let mut model = UiModel::new();
    let long = "a".repeat(300);
    model.received(received(1, &peer(), &long));
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(4000.0, 800.0));
    view.render(&model);
    let shown = format!("{}\u{2026}", "a".repeat(160));
    assert_eq!(described(&view, &one_unread(&shown)), 1);
    assert_eq!(described(&view, &one_unread(&long)), 0, "never uncut");
}

/// A conversation the start summarised and holds no row of is listed with
/// its stored preview and the store's count (sections 3 and 6). Mutation:
/// Chats listing held rows alone.
#[test]
fn a_summarised_conversation_is_listed_with_its_stored_preview_and_badge() {
    let mut view = view();
    let mut model = UiModel::new();
    let from = peer();
    model.started(
        Vec::new(),
        vec![StoredSummary {
            conversation: direct(&from),
            unread: 1,
            newest: Some(NewestRow {
                table: Table::Unread,
                row: RowId::from_stored(7),
                received_at: 7,
                preview: cut("from the store"),
                card: false,
            }),
        }],
        Some(RowId::from_stored(7)),
    );
    assert!(model.messages(&direct(&from)).is_empty(), "no row held");
    view.render(&model);
    assert_eq!(described(&view, &one_unread("from the store")), 1);
}

/// Chats ends with the count of stored rows this version cannot show,
/// one and many in their own words, and the line goes at zero (section
/// 6.1); it is read after the last conversation (section 8). Mutations:
/// the line never drawn; drawn at zero.
#[test]
fn rows_not_shown_end_chats_and_go_at_zero() {
    let mut view = view();
    let mut model = UiModel::new();
    model.received(received(1, &peer(), "hello"));
    let one = text(UiText::RowsNotShownOne);
    let three = fill(text(UiText::RowsNotShown), &[("count", "3")]);
    view.render(&model);
    assert!(labelled(&view, one).is_empty(), "nothing hidden, no line");
    assert_eq!(view.window().get_rows_not_shown(), "", "no line at all");

    model.set_rows_not_shown(1);
    view.render(&model);
    assert_eq!(labelled(&view, one).len(), 1);

    model.set_rows_not_shown(3);
    view.render(&model);
    assert!(labelled(&view, one).is_empty());
    let line = labelled(&view, &three);
    assert_eq!(line.len(), 1);
    let elements = all(&view);
    let at = |wanted: &ElementHandle| {
        elements
            .iter()
            .position(|e| e.accessible_label() == wanted.accessible_label())
            .expect("in the tree")
    };
    let row = elements
        .iter()
        .position(|e| e.accessible_description().as_deref() == Some(one_unread("hello").as_str()))
        .expect("the conversation");
    assert!(at(&line[0]) > row, "read after the last conversation");

    model.set_rows_not_shown(0);
    view.render(&model);
    assert!(labelled(&view, &three).is_empty(), "gone at zero");
    assert_eq!(view.window().get_rows_not_shown(), "", "no line at all");
}

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

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::ElementHandle;
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::preview::cut;
use interweave_human_ui_model::{
    ConversationKey, Edge, ListedInbound, NewestRow, PageEnd, PageRows, RuntimeHost, StoredSummary,
    Table, UiModel, UiText, WindowEvent, WindowInput, fill, placeholder_en,
};
use interweave_human_ui_slint::View;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::{ComponentHandle as _, Model as _};

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

/// A windowed model whose one conversation, from `from`, the store says
/// holds `unread` unread rows, shown and opened in `view`, its first page
/// asked and not yet answered.
fn opened(view: &mut View, from: &TransportIdentity, unread: u64) -> UiModel {
    let mut model = UiModel::new();
    model.started(
        Vec::new(),
        vec![StoredSummary {
            conversation: direct(from),
            unread,
            newest: Some(NewestRow {
                table: Table::Unread,
                row: RowId::from_stored(1_000),
                received_at: 1_000,
                preview: cut("newest"),
                card: false,
            }),
        }],
        Some(RowId::from_stored(i64::MAX)),
    );
    view.select(direct(from));
    let _ = view.take_events(&model);
    model.window_input(WindowInput::Opened(direct(from)));
    view.render(&model);
    model
}

/// The page asked, answered with rows `rows` of `from`, more remaining.
fn page(model: &mut UiModel, from: &TransportIdentity, rows: std::ops::RangeInclusive<i64>) {
    let mut asks = model.take_page_asks();
    assert_eq!(asks.len(), 1, "{asks:?}");
    let ask = asks.remove(0);
    let unread = rows
        .map(|n| {
            let r = received(n, from, &format!("message {n}"));
            ListedInbound {
                row: r.row,
                origin: r.origin,
                envelope: r.envelope,
                received_at: r.received_at,
            }
        })
        .collect();
    model.page_arrived(PageRows {
        ask,
        unread,
        kept: Vec::new(),
        proposals: Vec::new(),
        end: PageEnd::More,
    });
}

/// A clock a test steps.
fn stepped(view: &mut View) -> Rc<Cell<Instant>> {
    let at = Rc::new(Cell::new(Instant::now()));
    let read = Rc::clone(&at);
    view.set_monotonic(move || read.get());
    at
}

fn position(elements: &[ElementHandle], wanted: impl Fn(&ElementHandle) -> bool) -> usize {
    elements.iter().position(wanted).expect("in the tree")
}

/// The progress row shows only once its page has taken 250 ms (section
/// 5), named for a screen reader. Mutation: shown at once.
#[test]
fn the_progress_row_shows_only_after_250_ms() {
    let mut view = view();
    let at = stepped(&mut view);
    let from = peer();
    let model = opened(&mut view, &from, 5);
    let loading = text(UiText::EarlierLoading);
    assert!(labelled(&view, loading).is_empty(), "not at once");
    at.set(at.get() + Duration::from_millis(249));
    view.render(&model);
    assert!(labelled(&view, loading).is_empty(), "not at 249 ms");
    at.set(at.get() + Duration::from_millis(1));
    view.render(&model);
    assert_eq!(labelled(&view, loading).len(), 1, "at 250 ms");
}

/// A page that could not be read shows the failure row at the top --
/// its text, then Try again, then the line, then the rows (section 8) --
/// announced once as the edge fails, not on each render; Try again
/// reports the edge's retry. Mutations: announced on every render while
/// failed; Try again reporting nothing.
#[test]
fn a_failed_page_is_announced_once_and_try_again_reports_its_retry() {
    let mut view = view();
    let from = peer();
    let mut model = opened(&mut view, &from, 20);
    page(&mut model, &from, 11..=20);
    model.window_input(WindowInput::Reached {
        conversation: direct(&from),
        edge: Edge::Top,
    });
    let ask = model.take_page_asks().remove(0);
    model.page_failed(&ask);
    view.render(&model);
    let failed = text(UiText::EarlierLoadFailed);
    let slots = |view: &View| {
        (
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        )
    };
    let said = slots(&view);
    assert!(
        said.0.contains(failed) || said.1.contains(failed),
        "announced: {said:?}"
    );
    view.render(&model);
    assert_eq!(slots(&view), said, "once, not again on the next render");

    let elements = all(&view);
    let line = fill(text(UiText::EarlierUnread), &[("count", "10")]);
    let text_at = position(&elements, |e| {
        e.accessible_label().as_deref() == Some(failed)
    });
    let button_at = position(&elements, |e| {
        e.accessible_label().as_deref() == Some(text(UiText::TryAgain))
    });
    let line_at = position(&elements, |e| {
        e.accessible_label().as_deref() == Some(line.as_str())
    });
    let row_at = position(&elements, |e| {
        e.accessible_label()
            .is_some_and(|l| l.contains("message 11"))
    });
    assert!(text_at < button_at && button_at < line_at && line_at < row_at);

    labelled(&view, text(UiText::TryAgain))[0].invoke_accessible_default_action();
    assert_eq!(
        view.take_window_events(),
        [WindowEvent::Input(WindowInput::TryAgain {
            conversation: direct(&from),
            edge: Edge::Top,
        })]
    );
}

/// The listed mode draws no window mark: every row is held. Mutation:
/// marks drawn whatever the mode.
#[test]
fn a_listed_model_draws_no_marks() {
    let mut view = view();
    let mut model = UiModel::new();
    let from = peer();
    model.received(received(1, &from, "hello"));
    view.select(direct(&from));
    let _ = view.take_events(&model);
    view.render(&model);
    assert_eq!(view.window().get_window_marks().row_count(), 0);
}

/// A pressed Try again leaves its row in place, inert, while the read is
/// on its way: a second press reports nothing, and the progress row takes
/// its place only at 250 ms. A second failure is announced again
/// (section 5, "A pressed row"). Mutations: the row taken away at the
/// press; a second press reported; a repeat failure not announced.
#[test]
fn a_pressed_try_again_stays_inert_until_the_progress_row_and_a_repeat_failure_is_announced() {
    let mut view = view();
    let at = stepped(&mut view);
    let from = peer();
    let mut model = opened(&mut view, &from, 20);
    page(&mut model, &from, 11..=20);
    model.window_input(WindowInput::Reached {
        conversation: direct(&from),
        edge: Edge::Top,
    });
    let ask = model.take_page_asks().remove(0);
    model.page_failed(&ask);
    view.render(&model);
    let failed = text(UiText::EarlierLoadFailed);
    let try_again = text(UiText::TryAgain);
    labelled(&view, try_again)[0].invoke_accessible_default_action();
    for event in view.take_window_events() {
        model.window_event(event);
    }
    let retry = model.take_page_asks().remove(0);
    view.render(&model);
    assert_eq!(labelled(&view, failed).len(), 1, "the row stays");
    labelled(&view, try_again)[0].invoke_accessible_default_action();
    assert!(
        view.take_window_events().is_empty(),
        "inert while in flight"
    );

    at.set(at.get() + Duration::from_millis(250));
    view.render(&model);
    assert!(labelled(&view, failed).is_empty(), "replaced");
    assert_eq!(labelled(&view, text(UiText::EarlierLoading)).len(), 1);

    let before = (
        view.window().get_announcement_a().to_string(),
        view.window().get_announcement_b().to_string(),
    );
    model.page_failed(&retry);
    view.render(&model);
    let after = (
        view.window().get_announcement_a().to_string(),
        view.window().get_announcement_b().to_string(),
    );
    assert_ne!(after, before, "the repeat is announced");
    assert!(after.0.contains(failed) || after.1.contains(failed));
}

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
    SETTLE_WINDOW.with(|w| *w.borrow_mut() = Some(view.window().as_weak()));
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
    view.render(&model);
    apply(view, &mut model);
    model
}

/// A few frames' worth of the toolkit's work: the testing backend draws
/// nothing, so each frame's layout is forced by reading where the
/// elements stand, as a draw would, before the next frame's change
/// handlers run.
fn settle() {
    for _ in 0..4 {
        slint::platform::update_timers_and_animations();
        if let Some(view) =
            SETTLE_WINDOW.with(|w| w.borrow().as_ref().and_then(slint::Weak::upgrade))
        {
            for element in i_slint_backend_testing::ElementQuery::from_root(&view).find_all() {
                let _ = element.absolute_position();
            }
        }
    }
}

thread_local! {
    /// The window [`settle`] lays out.
    static SETTLE_WINDOW: std::cell::RefCell<Option<slint::Weak<interweave_human_ui_slint::AppWindow>>> =
        const { std::cell::RefCell::new(None) };
}

/// The root's part: the view's window reports applied, then drawn.
fn apply(view: &mut View, model: &mut UiModel) {
    settle();
    for event in view.take_window_events() {
        model.window_event(event);
    }
    view.render(model);
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
        retries(&mut view),
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
    let elements = all(&view);
    let line = fill(text(UiText::EarlierUnread), &[("count", "10")]);
    assert!(
        position(&elements, |e| e.accessible_label().as_deref()
            == Some(failed))
            < position(&elements, |e| e.accessible_label().as_deref()
                == Some(line.as_str())),
        "where it stood: before the line (section 8)"
    );
    labelled(&view, try_again)[0].invoke_accessible_default_action();
    assert!(retries(&mut view).is_empty(), "inert while in flight");

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

fn retries(view: &mut View) -> Vec<WindowEvent> {
    view.take_window_events()
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                WindowEvent::Input(WindowInput::TryAgain { .. } | WindowInput::ShowAnyway { .. })
            )
        })
        .collect()
}

fn conversation_events(view: &mut View) -> Vec<WindowEvent> {
    view.take_window_events()
        .into_iter()
        .filter(|e| !matches!(e, WindowEvent::Input(WindowInput::Place { .. })))
        .collect()
}

/// The window opens with the conversation shown and closes when another
/// is shown, or Chats takes a narrow window's place (section 7.1's U5);
/// the listed mode reports nothing. Mutations: no opening reported; the
/// one before never closed; reports in the listed mode.
#[test]
fn the_window_follows_the_conversation_shown() {
    let mut view = view();
    let (a, b) = (peer(), peer());
    let mut model = UiModel::new();
    model.started(Vec::new(), Vec::new(), None);
    view.select(direct(&a));
    let _ = view.take_events(&model);
    view.render(&model);
    assert_eq!(
        conversation_events(&mut view),
        [WindowEvent::Input(WindowInput::Opened(direct(&a)))]
    );
    view.select(direct(&b));
    let _ = view.take_events(&model);
    view.render(&model);
    assert_eq!(
        conversation_events(&mut view),
        [
            WindowEvent::Input(WindowInput::Closed(direct(&a))),
            WindowEvent::Input(WindowInput::Opened(direct(&b))),
        ]
    );
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    view.window().set_list_shown(true);
    slint::platform::update_timers_and_animations();
    view.render(&model);
    assert_eq!(
        conversation_events(&mut view),
        [WindowEvent::Input(WindowInput::Closed(direct(&b)))],
        "Chats in its place"
    );
}

/// The listed mode reports nothing of a window: every row is held.
/// Mutation: reports whatever the mode.
#[test]
fn a_listed_model_reports_no_window() {
    let mut view = view();
    let mut model = UiModel::new();
    let from = peer();
    model.received(received(1, &from, "hello"));
    view.select(direct(&from));
    let _ = view.take_events(&model);
    view.render(&model);
    slint::platform::update_timers_and_animations();
    assert!(view.take_window_events().is_empty());
}

/// A conversation of 60 rows scrolled to its newest, its top left.
fn tall(view: &mut View) -> (TransportIdentity, UiModel) {
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    let from = peer();
    let mut model = opened(view, &from, 100);
    page(&mut model, &from, 41..=100);
    view.render(&model);
    apply(view, &mut model);
    scroll(view, -10_000.0);
    apply(view, &mut model);
    (from, model)
}

fn scroll(view: &View, delta_y: f32) {
    view.window()
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
            position: slint::LogicalPosition::new(700.0, 300.0),
            delta_x: 0.0,
            delta_y,
        });
    slint::platform::update_timers_and_animations();
}

/// Scrolling the list's top into view reaches the top edge, once however
/// long the person stays there; scrolling away leaves it. A long scroll
/// is one report of the person's place, not one a frame. Mutations: the
/// top never reported; reported on every take; each move its own place.
#[test]
fn scrolling_the_top_into_view_reaches_it_once_and_away_leaves_it() {
    let mut view = view();
    let (from, _model) = tall(&mut view);
    let top = |e: &WindowEvent| match e {
        WindowEvent::Input(WindowInput::Reached {
            edge: Edge::Top, ..
        }) => 1,
        WindowEvent::Input(WindowInput::Left {
            edge: Edge::Top, ..
        }) => -1,
        _ => 0,
    };
    for _ in 0..20 {
        scroll(&view, 400.0);
    }
    let events = view.take_window_events();
    assert_eq!(
        events
            .iter()
            .map(top)
            .collect::<Vec<_>>()
            .iter()
            .filter(|n| **n == 1)
            .count(),
        1,
        "{events:?}"
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, WindowEvent::Input(WindowInput::Place { .. })))
            .count(),
        1,
        "one place for the whole scroll"
    );
    let again = view.take_window_events();
    assert!(again.is_empty(), "nothing again, the top once: {again:?}");
    scroll(&view, -10_000.0);
    let events = view.take_window_events();
    assert!(events.iter().any(|e| top(e) == -1), "left: {events:?}");
    let _ = from;
}

/// Shift+Tab until focus is on `key`, as a person moves it; false when
/// it never gets there.
fn backtab_to(view: &View, key: &str) -> bool {
    let back: slint::SharedString = slint::platform::Key::Backtab.into();
    for _ in 0..2_000 {
        if view.window().get_focus_id() == key {
            return true;
        }
        view.window()
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: back.clone() });
        view.window()
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: back.clone() });
    }
    false
}

/// Moving focus onto the top's line, by Tab, reaches the top edge: every
/// gesture has a non-gesture equivalent (section 8). Mutation: focus never
/// reaching an edge.
#[test]
fn shift_tab_onto_the_line_reaches_the_top() {
    let mut view = view();
    let (_from, _model) = tall(&mut view);
    let back: slint::SharedString = slint::platform::Key::Backtab.into();
    let mut reached = false;
    for _ in 0..2_000 {
        view.window()
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: back.clone() });
        view.window()
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: back.clone() });
        if view.window().get_focus_id() == "earlier-unread:t" {
            reached = view.take_window_events().iter().any(|e| {
                matches!(
                    e,
                    WindowEvent::Input(WindowInput::Reached {
                        edge: Edge::Top,
                        ..
                    })
                )
            });
            break;
        }
    }
    assert!(reached, "focus on the line reaches the top");
}

/// The body of the first message row in view: the list makes only the
/// rows in its view.
fn first_in_view(view: &View) -> String {
    all(view)
        .into_iter()
        .filter_map(|e| e.accessible_label())
        .find_map(|l| {
            l.split(": ")
                .nth(1)
                .filter(|b| b.starts_with("message "))
                .map(|b| b.split('.').next().unwrap_or(b).to_owned())
        })
        .expect("a message in view")
}

/// Where the message whose label holds `text` stands on screen.
fn y_of(view: &View, text: &str) -> f32 {
    all(view)
        .into_iter()
        .find(|e| e.accessible_label().is_some_and(|l| l.contains(text)))
        .map(|e| e.absolute_position().y)
        .expect("drawn")
}

/// A page of older rows arriving above the person's place leaves the row
/// they were looking at where it was on screen (section 5, "neither the
/// scroll position nor focus moves"). Mutation: the place not kept --
/// the row pushed down by the page's height.
#[test]
fn a_page_arriving_above_leaves_the_row_in_view_where_it_was() {
    let mut view = view();
    let (from, mut model) = tall(&mut view);
    scroll(&view, 10_000.0);
    apply(&mut view, &mut model);
    let row = first_in_view(&view);
    assert_eq!(row, "message 41", "control: at the top");
    let before = y_of(&view, &row);
    page(&mut model, &from, 1..=40);
    view.render(&model);
    apply(&mut view, &mut model);
    settle();
    let after = y_of(&view, &row);
    assert!(
        (after - before).abs() <= 1.0,
        "the row stays: {before} then {after}"
    );
}

/// A message arriving at the end while the person is up in the history
/// behaves as today: nothing in view moves (section 5). The control for
/// the place kept above: keeping the list's bottom instead would push the
/// rows in view up by the new message's height.
#[test]
fn a_message_arriving_at_the_end_up_in_the_history_moves_nothing() {
    let mut view = view();
    let (from, mut model) = tall(&mut view);
    scroll(&view, 1_600.0);
    apply(&mut view, &mut model);
    let row = first_in_view(&view);
    let before = y_of(&view, &row);
    model.received(received(101, &from, "message 101"));
    view.render(&model);
    apply(&mut view, &mut model);
    settle();
    let after = y_of(&view, &row);
    assert!(
        (after - before).abs() <= 1.0,
        "nothing moves: {before} then {after}"
    );
}

/// Focus on the line when its count reaches zero goes to the row that was
/// just below it, as app-placed focus (section 7's table). Mutation: focus
/// left on the line no longer drawn.
#[test]
fn focus_on_the_line_at_zero_goes_to_the_row_below() {
    let mut view = view();
    let from = peer();
    let mut model = opened(&mut view, &from, 20);
    page(&mut model, &from, 11..=20);
    view.render(&model);
    apply(&mut view, &mut model);
    assert!(
        backtab_to(&view, "earlier-unread:t"),
        "the line takes focus"
    );
    apply(&mut view, &mut model);
    page(&mut model, &from, 1..=10);
    view.render(&model);
    apply(&mut view, &mut model);
    settle();
    let focus = view.window().get_focus_id().to_string();
    assert!(focus.starts_with("item:"), "on the row below: {focus}");
}

/// Focus on a pressed Try again, once its page arrives, goes to the item
/// that was just below the failure row: the line, or with the line gone
/// the row that was oldest (section 7's table). Mutation: focus left on
/// the button no longer drawn.
#[test]
fn focus_on_a_pressed_try_again_goes_below_once_the_page_arrives() {
    let mut view = view();
    let from = peer();
    let mut model = opened(&mut view, &from, 20);
    page(&mut model, &from, 11..=20);
    view.render(&model);
    apply(&mut view, &mut model);
    model.window_input(WindowInput::Reached {
        conversation: direct(&from),
        edge: Edge::Top,
    });
    let ask = model.take_page_asks().remove(0);
    model.page_failed(&ask);
    view.render(&model);
    apply(&mut view, &mut model);
    assert!(backtab_to(&view, "mark-action:t"), "Try again takes focus");
    labelled(&view, text(UiText::TryAgain))[0].invoke_accessible_default_action();
    apply(&mut view, &mut model);
    assert_eq!(
        view.window().get_focus_id(),
        "mark-action:t",
        "kept while in flight"
    );
    // Held by a live control, not only named: the next Tab goes on to the
    // line just after it, not back to the window's first control.
    let tab: slint::SharedString = slint::platform::Key::Tab.into();
    view.window()
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: tab.clone() });
    view.window()
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: tab });
    assert_eq!(
        view.window().get_focus_id(),
        "earlier-unread:t",
        "focus kept on the button"
    );
    page(&mut model, &from, 1..=10);
    view.render(&model);
    apply(&mut view, &mut model);
    settle();
    let focus = view.window().get_focus_id().to_string();
    assert!(focus.starts_with("item:"), "on the row below: {focus}");
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A row's swipe, its buttons and the snackbar, through Slint's testing
//! backend (`design/2026-10-16-message-swipe.md`).
//!
//! A swipe is driven here through the window's own callbacks, with the
//! positions the row's handler would report: what is proved is the view's
//! half -- which positions commit what, what the window then shows, says
//! and focuses. NOT proved here: that Slint's handler reports those
//! positions on a device, that the list does not take the touch first,
//! the system's slop, insets and haptics, or what the screen reader reads. Those
//! are the Android build's, checked on a device.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{
    ClientEvent, Destination, Origin, OutboundStatus, OutboundUpdate, Received,
};
use interweave_human_core::AppMessageId;
use interweave_human_core::RowId;
use interweave_human_ui_model::{
    ConversationKey, Intent, RuntimeHost, UiModel, UiText, placeholder_en,
};
use interweave_human_ui_slint::{TouchSettings, View, ViewEvent};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::{ComponentHandle as _, Model as _};

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer id")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("an endpoint")
}

fn envelope(serial: u128, text: &str) -> HumanChatV2 {
    HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: format!("{serial:032x}"),
        text: text.to_owned(),
        reply_to: None,
        sent_at_ms: None,
        from_endpoint: None,
    }
}

fn direct(peer: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(human()),
    }
}

/// A message from `from`, read: kind A.
fn read(model: &mut UiModel, row: i64, from: &TransportIdentity, text: &str) {
    model.received(Received {
        row: RowId::from_stored(row),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: human(),
        },
        envelope: envelope(u128::try_from(row).expect("positive") + 1_000, text),
        received_at: u64::try_from(row).expect("positive"),
    });
    model.read(RowId::from_stored(row));
}

/// A message to `to`, pending: kind C.
fn pending(model: &mut UiModel, row: i64, to: &TransportIdentity, text: &str) {
    model.sent(
        RowId::from_stored(row),
        &Destination::Direct {
            peer: to.clone(),
            endpoint: Some(human()),
        },
        envelope(u128::try_from(row).expect("positive"), text),
        u64::try_from(row).expect("positive"),
    );
}

fn view() -> View {
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    view
}

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

/// The root's half, as app-core's model side does it: what writes nothing
/// is done on the model, an Undo resolved by it, and the facade's
/// intents returned.
fn turn(view: &mut View, model: &mut UiModel) -> Vec<Intent> {
    let mut out = Vec::new();
    loop {
        let events = view.take_events(model);
        if events.is_empty() {
            break;
        }
        for event in events {
            match event {
                ViewEvent::Intent(Intent::Hide(item)) => model.hide(item),
                ViewEvent::Intent(Intent::HideConversation(key)) => model.hide_conversation(&key),
                ViewEvent::Intent(intent) => out.push(intent),
                ViewEvent::Undo(id) => out.extend(model.undo(id)),
                ViewEvent::SnackbarClosed(id) => model.snackbar_closed(id),
                ViewEvent::DraftChanged { key, draft } => model.draft_changed(key, draft),
                ViewEvent::Trust(_) => {}
            }
        }
    }
    view.render(model);
    // A focus request is carried out once its rows are drawn.
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(0));
    out
}

fn open(view: &mut View, model: &mut UiModel, key: &ConversationKey) {
    view.select(key.clone());
    let _ = turn(view, model);
}

/// The swipe key of the message row at `index` as drawn.
fn row_key(view: &View, index: usize) -> String {
    let row = view.window().get_messages().row_data(index).expect("a row");
    format!("m:{}", row.handle)
}

const WIDTH: f32 = 360.0;
const WINDOW: f32 = 400.0;

/// Drag the row `key` from x 200 by `(dx, dy)` in `steps` reports, then
/// lift: what the row's handler would report.
fn swipe(view: &View, key: &str, dx: f32, dy: f32, steps: u32) {
    let window = view.window();
    for i in 1..=steps {
        #[allow(clippy::cast_precision_loss)]
        let f = i as f32 / steps as f32;
        window.invoke_swipe_moved(
            key.into(),
            200.0,
            300.0,
            200.0 + dx * f,
            300.0 + dy * f,
            WIDTH,
            WINDOW,
        );
    }
    window.invoke_swipe_ended(key.into());
}

fn labelled(view: &View, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label).collect()
}

/// What the window's announcement says now.
fn announced(view: &View) -> String {
    let a = view.window().get_announcement_a().to_string();
    let b = view.window().get_announcement_b().to_string();
    if a.is_empty() { b } else { a }
}

#[test]
fn a_read_rows_buttons_are_keep_and_hide_each_48_by_48() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "hello");
    open(&mut view, &mut model, &direct(&alice));
    for label in [text(UiText::Keep), text(UiText::Dismiss)] {
        let found = labelled(&view, label);
        assert_eq!(found.len(), 1, "one {label} button");
        let size = found[0].size();
        assert!(
            size.width >= 48.0 && size.height >= 48.0,
            "{label} is {size:?}, under 48 x 48"
        );
    }
}

#[test]
fn a_swipe_past_the_threshold_raises_the_rows_action_and_ticks_once() {
    let mut view = view();
    let ticks = Rc::new(Cell::new(0));
    let t = Rc::clone(&ticks);
    view.on_haptic(move || t.set(t.get() + 1));
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "hello");
    open(&mut view, &mut model, &direct(&alice));
    let key = row_key(&view, 0);

    let window = view.window();
    window.invoke_swipe_moved(
        key.as_str().into(),
        200.0,
        300.0,
        250.0,
        302.0,
        WIDTH,
        WINDOW,
    );
    assert!(
        window.get_swipe_tracking(),
        "the list stops scrolling for it"
    );
    assert_eq!(window.get_swipe_key().as_str(), key);
    assert!((window.get_swipe_offset() - 50.0).abs() < 0.01, "1:1");
    swipe(&view, &key, 160.0, 0.0, 8);
    assert_eq!(ticks.get(), 1, "one tick as it armed");
    let intents = turn(&mut view, &mut model);
    assert!(
        matches!(intents.as_slice(), [Intent::Keep { .. }]),
        "{intents:?}"
    );
    assert!(!view.window().get_swipe_tracking());
}

#[test]
fn a_drag_55_degrees_off_horizontal_never_moves_the_row() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "hello");
    open(&mut view, &mut model, &direct(&alice));
    let key = row_key(&view, 0);
    let (dy, dx) = 55.0_f32.to_radians().sin_cos();
    swipe(&view, &key, dx * 300.0, dy * 300.0, 30);
    assert!(!view.window().get_swipe_tracking());
    assert!(view.window().get_swipe_offset().abs() < 0.01);
    assert!(
        turn(&mut view, &mut model).is_empty(),
        "it scrolls, never swipes"
    );
}

#[test]
fn right_to_left_mirrors_the_directions() {
    let mut view = view();
    view.set_touch_settings(TouchSettings {
        rtl: true,
        ..TouchSettings::default()
    });
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "hello");
    open(&mut view, &mut model, &direct(&alice));
    let key = row_key(&view, 0);
    swipe(&view, &key, -170.0, 0.0, 8);
    let intents = turn(&mut view, &mut model);
    assert!(
        matches!(intents.as_slice(), [Intent::Keep { .. }]),
        "a swipe to the left keeps: {intents:?}"
    );
}

#[test]
fn a_hide_is_confirmed_announced_focused_and_undone_in_place() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    let first = row_key(&view, 0);
    let second_handle = view
        .window()
        .get_messages()
        .row_data(1)
        .expect("row")
        .handle;

    swipe(&view, &first, -170.0, 0.0, 8);
    let intents = turn(&mut view, &mut model);
    assert!(intents.is_empty(), "Hide is the model's alone: {intents:?}");
    assert_eq!(model.messages(&direct(&alice)).len(), 1);
    assert!(view.window().get_snack_shown());
    assert_eq!(
        view.window().get_snack_text().as_str(),
        text(UiText::ConfirmDismissed)
    );
    assert_eq!(announced(&view), text(UiText::ConfirmDismissed));
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("action:{second_handle}:0"),
        "focus to the next row's first control"
    );
    let undo = labelled(&view, text(UiText::Undo));
    assert_eq!(undo.len(), 1, "the snackbar's Undo");
    assert!(undo[0].size().width >= 48.0 && undo[0].size().height >= 48.0);

    undo[0].invoke_accessible_default_action();
    assert!(turn(&mut view, &mut model).is_empty());
    assert_eq!(
        model.messages(&direct(&alice)).len(),
        2,
        "back in its place"
    );
    assert!(!view.window().get_snack_shown());
    assert_eq!(announced(&view), text(UiText::ActionUndone));
}

#[test]
fn a_leaving_rows_space_closes_and_then_it_goes() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    let hide = labelled(&view, text(UiText::Dismiss));
    hide[0].invoke_accessible_default_action();
    let _ = turn(&mut view, &mut model);
    let rows = view.window().get_messages();
    assert_eq!(rows.row_count(), 2, "kept while its space closes");
    assert!(rows.row_data(0).expect("row").leaving);
    std::thread::sleep(Duration::from_millis(200));
    view.render(&model);
    assert_eq!(view.window().get_messages().row_count(), 1, "then gone");
}

#[test]
fn a_cancel_swipe_holds_its_row_out_until_the_answer_and_a_refusal_springs_it_back() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    pending(&mut model, 1, &alice, "out");
    open(&mut view, &mut model, &direct(&alice));
    let key = row_key(&view, 0);
    swipe(&view, &key, -170.0, 0.0, 8);
    assert!(
        (view.window().get_swipe_offset() + WIDTH).abs() < 0.01,
        "slid out"
    );
    let intents = turn(&mut view, &mut model);
    let Some(cancel @ Intent::Cancel(_)) = intents.first().cloned() else {
        panic!("Cancel at commit: {intents:?}");
    };
    view.render(&model);
    assert!(
        (view.window().get_swipe_offset() + WIDTH).abs() < 0.01,
        "held out while the facade answers"
    );
    model.intent_refused(&cancel);
    view.render(&model);
    assert!(
        view.window().get_swipe_offset().abs() < 0.01,
        "springs back, still pending"
    );
    assert!(!view.window().get_snack_shown(), "no snackbar");
}

#[test]
fn the_snackbar_closes_after_its_time() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    open(&mut view, &mut model, &direct(&alice));
    labelled(&view, text(UiText::Keep))[0].invoke_accessible_default_action();
    let intents = turn(&mut view, &mut model);
    let Some(Intent::Keep { item, .. }) = intents.first() else {
        panic!("Keep: {intents:?}");
    };
    model.kept(*item, RowId::from_stored(10));
    view.render(&model);
    assert!(view.window().get_snack_shown());
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5_000));
    let _ = turn(&mut view, &mut model);
    assert!(view.window().get_snack_shown(), "up for 6 s");
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1_100));
    let _ = turn(&mut view, &mut model);
    assert!(!view.window().get_snack_shown(), "then closed");
    assert!(model.snackbar().is_none());
}

#[test]
fn hide_conversation_is_offered_on_its_screen_only_when_the_swipe_is() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    open(&mut view, &mut model, &direct(&alice));
    assert!(view.window().get_hide_conversation_shown());
    let control = labelled(&view, text(UiText::HideConversation));
    assert_eq!(control.len(), 1);
    control[0].invoke_accessible_default_action();
    let _ = turn(&mut view, &mut model);
    assert!(model.conversations().is_empty());
    assert!(view.window().get_snack_shown() && view.window().get_snack_on_list());
    assert_eq!(
        view.window().get_snack_text().as_str(),
        text(UiText::ConfirmConversationHidden)
    );

    let bob = peer();
    pending(&mut model, 5, &bob, "out");
    open(&mut view, &mut model, &direct(&bob));
    assert!(
        !view.window().get_hide_conversation_shown(),
        "a pending message: not offered"
    );
}

#[test]
fn a_conversation_whose_rows_are_all_hidden_keeps_its_header() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "only");
    open(&mut view, &mut model, &direct(&alice));
    let title = view.window().get_header_title().to_string();
    assert!(!title.is_empty(), "control");
    labelled(&view, text(UiText::Dismiss))[0].invoke_accessible_default_action();
    let _ = turn(&mut view, &mut model);
    assert_eq!(view.window().get_header_title().as_str(), title);
    assert_ne!(
        view.window().get_focus_id().as_str(),
        "composer",
        "no keyboard asked for"
    );
}

/// A snackbar's closing and leaving for the list are never refused, so
/// they coalesce instead: one closing per snackbar, one Left until a
/// selection follows it (review F4 on #289; the bound `queued_inputs`
/// states).
#[test]
fn closings_and_leavings_wait_once_each() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    open(&mut view, &mut model, &direct(&alice));
    labelled(&view, text(UiText::Dismiss))[0].invoke_accessible_default_action();
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_some(), "control");
    let before = view.queued_inputs();
    for _ in 0..50 {
        view.close_snackbar();
        view.window().invoke_left_conversation();
    }
    assert_eq!(view.queued_inputs(), before + 2, "one closing, one Left");
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none());
    assert_eq!(view.queued_inputs(), 0);
}

/// What a cancel's snackbar says, by whether an attempt may have reached
/// the peer (review N3 on #289).
#[test]
fn a_cancels_snackbar_says_whether_it_may_have_reached_the_peer() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    for (row, may_have_reached, said) in [
        (1, false, UiText::ConfirmSendingCanceled),
        (2, true, UiText::ConfirmSendingCanceledMayHaveReached),
    ] {
        pending(&mut model, row, &alice, "out");
        model.client_event(ClientEvent::Outbound(OutboundUpdate {
            row: RowId::from_stored(row),
            app_message_id: AppMessageId::parse(format!("{row:032x}")).expect("id"),
            status: OutboundStatus::Cancelled { may_have_reached },
            last_code: None,
        }));
        model.cancelled(RowId::from_stored(row), true);
        open(&mut view, &mut model, &direct(&alice));
        assert_eq!(view.window().get_snack_text().as_str(), text(said));
    }
}

/// A conversation row's swipe is labelled "Hide", a message's "Dismiss".
#[test]
fn a_conversation_rows_swipe_says_hide() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "only");
    open(&mut view, &mut model, &direct(&alice));
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row");
    assert_eq!(row.hide_label.as_str(), text(UiText::HideShort));
    assert_ne!(text(UiText::HideShort), text(UiText::Dismiss));
}

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
use interweave_human_ui_slint::{INPUT_CAP, TouchSettings, View, ViewEvent};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::platform::{Key, WindowEvent};
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
                ViewEvent::IdentifierCopied(key) => model.identifier_copied(key),
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

/// Tab until the focused control's `focus-id` starts with `prefix`.
fn tab_to(view: &View, prefix: &str) -> String {
    for _ in 0..60 {
        key(view, Key::Tab);
        let on = view.window().get_focus_id().to_string();
        if on.starts_with(prefix) {
            return on;
        }
    }
    panic!("Tab never reached {prefix}");
}

/// Open row `handle`'s menu from the keyboard, as swipe spec section 7
/// says: Tab to its bubble, Enter; the menu takes focus on its first item
/// once it is in the window.
fn menu_by_keyboard(view: &View, handle: i32) {
    tab_to(view, &format!("item:{handle}"));
    press_focused(view);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(2));
    slint::platform::update_timers_and_animations();
}

fn key(view: &View, key: Key) {
    let text: slint::SharedString = key.into();
    let window = view.window().window();
    window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text });
}

/// Activate the focused control from the keyboard.
fn press_focused(view: &View) {
    key(view, Key::Return);
}

/// The handle of the message row at `index` as drawn.
fn handle(view: &View, index: usize) -> i32 {
    view.window()
        .get_messages()
        .row_data(index)
        .expect("a row")
        .handle
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

/// Trust, by the app's options and their Trust entry (#304 F12).
fn open_trust(view: &View) {
    labelled(view, text(UiText::AppOptions))[0].invoke_accessible_default_action();
    labelled(view, text(UiText::TrustHeading))[0].invoke_accessible_default_action();
}

/// Choose `label` from the menu of the first message row that offers it
/// (Show source and its way back: the first row), opening the menu as a
/// screen reader does, by the row's default action (swipe spec section
/// 7; #304 F2 moved the row's buttons into it).
fn choose(view: &View, label: &str) {
    use slint::Model as _;
    let rows = view.window().get_messages();
    let rows: Vec<_> = (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .collect();
    let row = rows
        .iter()
        .find(|r| r.actions.iter().any(|a| a.text.as_str() == label))
        .or_else(|| rows.first())
        .expect("a message row");
    let item = i_slint_backend_testing::ElementQuery::from_root(view.window())
        .match_accessible_role(i_slint_backend_testing::AccessibleRole::ListItem)
        .find_all()
        .into_iter()
        .find(|e| e.accessible_label().as_deref() == Some(row.label.as_str()))
        .expect("the row's item");
    item.invoke_accessible_default_action();
    let entry = i_slint_backend_testing::ElementQuery::from_root(view.window())
        .find_all()
        .into_iter()
        .find(|e| {
            e.accessible_label().as_deref() == Some(label)
                && e.accessible_description().as_deref() == Some(row.label.as_str())
        })
        .expect("the menu's entry");
    entry.invoke_accessible_default_action();
    // The row hears a menu's Show source through a change callback, which
    // runs on the event loop's turn.
    slint::platform::update_timers_and_animations();
}

/// What the window's announcement says now.
fn announced(view: &View) -> String {
    let a = view.window().get_announcement_a().to_string();
    let b = view.window().get_announcement_b().to_string();
    if a.is_empty() { b } else { a }
}

/// §7, checked by §11 item 13: every item of every row kind's menu --
/// read, kept, pending -- is at least 48 x 48, its touch area with it.
#[test]
fn every_menu_item_is_48_by_48() {
    use slint::Model as _;
    let all =
        |view: &View| i_slint_backend_testing::ElementQuery::from_root(view.window()).find_all();
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "read");
    read(&mut model, 2, &alice, "kept");
    pending(&mut model, 3, &alice, "pending");
    let key = direct(&alice);
    let kept = model.messages(&key)[1].key;
    model.kept(kept, RowId::from_stored(10));
    // Room for every row: the list draws only the rows in view.
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(900.0, 1200.0));
    open(&mut view, &mut model, &key);
    let rows = view.window().get_messages();
    let rows: Vec<_> = (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .collect();
    let mut seen = std::collections::HashSet::new();
    for row in &rows {
        let item = all(&view)
            .into_iter()
            .find(|e| {
                e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::ListItem)
                    && e.accessible_label().as_deref() == Some(row.label.as_str())
            })
            .expect("the row's item");
        item.invoke_accessible_default_action();
        let entries: Vec<_> = all(&view)
            .into_iter()
            .filter(|e| e.accessible_description().as_deref() == Some(row.label.as_str()))
            .filter(|e| {
                e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button)
            })
            .collect();
        assert!(!entries.is_empty(), "{}: its menu", row.label);
        for entry in entries {
            let size = entry.size();
            let label = entry.accessible_label().unwrap_or_default().to_string();
            assert!(
                size.width >= 48.0 && size.height >= 48.0,
                "{label} is {size:?}, under 48 x 48"
            );
            seen.insert(label);
        }
        view.window().invoke_close_row_menu();
    }
    for label in [
        UiText::Keep,
        UiText::Dismiss,
        UiText::Unkeep,
        UiText::Retry,
        UiText::Cancel,
        UiText::ShowSource,
    ] {
        assert!(seen.contains(text(label)), "a {label:?} item");
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
fn a_hide_is_confirmed_announced_and_undone_in_place() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    let first = row_key(&view, 0);
    let first_handle = view
        .window()
        .get_messages()
        .row_data(0)
        .expect("row")
        .handle;
    let focus_before = view.window().get_focus_id().to_string();

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
        focus_before,
        "a touch swipe with focus outside the row leaves focus where it was"
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
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{first_handle}"),
        "an Undo focuses the restored row's first control"
    );
}

#[test]
fn a_leaving_rows_space_closes_and_then_it_goes() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    choose(&view, text(UiText::Dismiss));
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
    choose(&view, text(UiText::Keep));
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
    // In the overflow menu (android-look section 6): there once it opens.
    assert!(labelled(&view, text(UiText::HideConversation)).is_empty());
    labelled(&view, text(UiText::MoreOptions))[0].invoke_accessible_default_action();
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
    // Dismiss is the menu's second item, after Keep.
    let only = handle(&view, 0);
    menu_by_keyboard(&view, only);
    tab_to(&view, &format!("action:{only}:1"));
    press_focused(&view);
    let _ = turn(&mut view, &mut model);
    assert_eq!(view.window().get_header_title().as_str(), title);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "header-title",
        "the header title, never the composer and its keyboard"
    );
}

/// A snackbar's closing and leaving for the list are never refused, so
/// they coalesce instead: one closing per snackbar, one Left until a
/// selection follows it; a pane shown or hidden waits outside the queue
/// (review F4 on #289; j84; the bound `queued_inputs` states).
#[test]
fn closings_and_leavings_wait_once_each() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    open(&mut view, &mut model, &direct(&alice));
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_some(), "control");
    let before = view.queued_inputs();
    for _ in 0..50 {
        view.close_snackbar();
        view.window().invoke_left_conversation();
        view.window().invoke_pane_changed();
    }
    assert_eq!(view.queued_inputs(), before + 2, "one closing, one Left");
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none());
    assert_eq!(view.queued_inputs(), 0);
}

/// A wide window whose trust settings closed, a message dismissed in the
/// conversation it shows: the conversation's snackbar is up, and the
/// list stays "shown" for when the window narrows (j84).
fn dismissed_beside_a_list_kept_shown(view: &mut View, model: &mut UiModel) -> ConversationKey {
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 800.0));
    let alice = peer();
    read(model, 1, &alice, "first");
    open(view, model, &direct(&alice));
    open_trust(view);
    let _ = turn(view, model);
    labelled(view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
    let _ = turn(view, model);
    choose(view, text(UiText::Dismiss));
    let _ = turn(view, model);
    assert!(
        model.snackbar().is_some_and(|s| s.conversation.is_some()),
        "control"
    );
    assert!(
        model.messages(&direct(&alice)).is_empty(),
        "control: dismissed"
    );
    direct(&alice)
}

/// Narrowed into the list, the conversation is left, and its snackbar
/// closes as leaving it by Back does (§5; j84).
#[test]
fn a_resize_to_the_list_closes_the_conversations_snackbar() {
    let mut view = view();
    let mut model = UiModel::new();
    let _ = dismissed_beside_a_list_kept_shown(&mut view, &mut model);
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    slint::platform::update_timers_and_animations();
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none());
}

/// An Undo pressed while the snackbar was up, then the window narrowed
/// into the list before the root took either: the Undo is honoured, not
/// lost to the closing the resize makes after it (j84).
#[test]
fn an_undo_before_a_resize_to_the_list_is_honoured() {
    let mut view = view();
    let mut model = UiModel::new();
    let key = dismissed_beside_a_list_kept_shown(&mut view, &mut model);
    labelled(&view, text(UiText::Undo))[0].invoke_accessible_default_action();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    slint::platform::update_timers_and_animations();
    let _ = turn(&mut view, &mut model);
    assert_eq!(model.messages(&key).len(), 1, "back in its place");
    assert!(model.snackbar().is_none());
}

/// Back, then the same conversation picked again, before the root takes
/// either: the person left it, so its snackbar closes (§5), though the
/// pane is shown again by the take (review R1 on #291).
#[test]
fn leaving_and_coming_back_in_one_take_closes_the_snackbar() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_some(), "control");
    labelled(&view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
    // Picked again as a person picks it: the list hands the pane back.
    view.select(direct(&alice));
    slint::platform::update_timers_and_animations();
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none());
}

/// Back, the same conversation picked again, a Dismiss and a keystroke,
/// all before the root takes: the snackbar that was up when the person
/// left closes, and the one the Dismiss makes after they came back stays
/// up, though the keystroke ends the first take early (review P3-1 on
/// #291, round 2).
#[test]
fn a_snackbar_made_after_coming_back_stays_up() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    read(&mut model, 3, &alice, "third");
    open(&mut view, &mut model, &direct(&alice));
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    let before = model.snackbar().expect("control: one up").id;
    // The dismissed row is held while its space closes; then it goes.
    std::thread::sleep(Duration::from_millis(200));
    view.render(&model);
    labelled(&view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
    view.select(direct(&alice));
    choose(&view, text(UiText::Dismiss));
    view.window().invoke_draft_edited("h".into());
    slint::platform::update_timers_and_animations();
    let _ = turn(&mut view, &mut model);
    assert_eq!(
        model.messages(&direct(&alice)).len(),
        1,
        "control: both dismissed"
    );
    let after = model
        .snackbar()
        .expect("the second Dismiss's snackbar is up");
    assert_ne!(after.id, before);
}

/// Back and the same conversation picked again before the root takes: the
/// snackbar left with the person, so it is not drawn again with an Undo
/// the queue would answer after closing it, and the Dismiss stands
/// (review P2-1 on #291, round 4).
#[test]
fn no_undo_is_offered_after_leaving_and_coming_back() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    assert_eq!(
        labelled(&view, text(UiText::Undo)).len(),
        1,
        "control: offered"
    );
    labelled(&view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
    view.select(direct(&alice));
    assert!(
        labelled(&view, text(UiText::Undo)).is_empty(),
        "no Undo for a snackbar the person left"
    );
    slint::platform::update_timers_and_animations();
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none());
    assert_eq!(
        model.messages(&direct(&alice)).len(),
        1,
        "the Dismiss stands"
    );
}

/// Two conversations side by side in a wide window, a message dismissed
/// in the first: its snackbar is up with an Undo. Returns the two.
fn dismissed_in_a_wide_window(
    view: &mut View,
    model: &mut UiModel,
) -> (ConversationKey, ConversationKey) {
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 800.0));
    let (alice, bob) = (peer(), peer());
    read(model, 1, &alice, "first");
    read(model, 2, &alice, "second");
    read(model, 3, &bob, "hello");
    open(view, model, &direct(&alice));
    choose(view, text(UiText::Dismiss));
    let _ = turn(view, model);
    assert_eq!(
        labelled(view, text(UiText::Undo)).len(),
        1,
        "control: offered"
    );
    (direct(&alice), direct(&bob))
}

/// Another conversation picked, then the first again, before the root
/// takes: the pick closes the snackbar in the queue, so it is gone from
/// the screen at the pick, a render before the take draws it no more, and
/// no Undo is offered that the queue would answer after the close (review
/// 5551078222 on #291, pre-existing item 1).
#[test]
fn no_undo_is_offered_after_picking_another_conversation_and_coming_back() {
    let mut view = view();
    let mut model = UiModel::new();
    let (alice, bob) = dismissed_in_a_wide_window(&mut view, &mut model);
    view.select(bob);
    assert!(
        labelled(&view, text(UiText::Undo)).is_empty(),
        "gone at the pick"
    );
    view.select(alice.clone());
    view.render(&model);
    assert!(
        labelled(&view, text(UiText::Undo)).is_empty(),
        "not drawn again while its closing waits"
    );
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none());
    assert_eq!(model.messages(&alice).len(), 1, "the Dismiss stands");
}

/// A pick of another conversation refused at a full queue closes nothing,
/// so the snackbar and its Undo stay on screen, and the Undo still works.
#[test]
fn a_refused_pick_leaves_the_undo_on_screen() {
    let mut view = view();
    let mut model = UiModel::new();
    let (alice, bob) = dismissed_in_a_wide_window(&mut view, &mut model);
    for _ in 0..INPUT_CAP {
        view.select(alice.clone());
    }
    view.select(bob);
    assert_eq!(view.refused_inputs(), 1, "control: refused");
    assert_eq!(labelled(&view, text(UiText::Undo)).len(), 1);
    let _ = turn(&mut view, &mut model);
    labelled(&view, text(UiText::Undo))[0].invoke_accessible_default_action();
    let _ = turn(&mut view, &mut model);
    assert_eq!(model.messages(&alice).len(), 2, "undone");
}

/// Picking the conversation already shown leaves its snackbar and its
/// Undo where they are: only a pick of ANOTHER closes it.
#[test]
fn picking_the_shown_conversation_keeps_its_undo() {
    let mut view = view();
    let mut model = UiModel::new();
    let (alice, _) = dismissed_in_a_wide_window(&mut view, &mut model);
    view.select(alice.clone());
    assert_eq!(labelled(&view, text(UiText::Undo)).len(), 1);
    labelled(&view, text(UiText::Undo))[0].invoke_accessible_default_action();
    let _ = turn(&mut view, &mut model);
    assert_eq!(model.messages(&alice).len(), 2, "undone");
}

/// The snackbar's time runs out before the root takes the closing: its
/// Undo goes from the screen then, not at the next turn, so it cannot be
/// pressed after the close it would follow in the queue (review
/// 5551078222 on #291, its first risk).
#[test]
fn a_timed_out_snackbar_leaves_the_screen_at_once() {
    closing_leaves_the_screen_at_once(|_| {
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(6_100));
    });
}

/// The app leaves the foreground before the root takes the closing: as
/// for a timeout, the Undo goes from the screen at once.
#[test]
fn a_snackbar_closed_by_the_background_leaves_the_screen_at_once() {
    closing_leaves_the_screen_at_once(View::close_snackbar);
}

fn closing_leaves_the_screen_at_once(close: impl FnOnce(&View)) {
    let mut view = view();
    let mut model = UiModel::new();
    let (alice, _) = dismissed_in_a_wide_window(&mut view, &mut model);
    close(&view);
    assert_eq!(view.queued_inputs(), 1, "control: the closing waits");
    assert!(
        labelled(&view, text(UiText::Undo)).is_empty(),
        "gone at once"
    );
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none());
    assert_eq!(model.messages(&alice).len(), 1, "the Dismiss stands");
}

/// The trust settings opened in the conversation's place: the person left
/// it, so its snackbar closes (§5), and is not drawn again with its Undo
/// when the settings close.
#[test]
fn opening_the_trust_settings_closes_the_conversations_snackbar() {
    let mut view = view();
    let mut model = UiModel::new();
    let (alice, _) = dismissed_in_a_wide_window(&mut view, &mut model);
    open_trust(&view);
    assert!(
        labelled(&view, text(UiText::Undo)).is_empty(),
        "gone at the press"
    );
    let _ = turn(&mut view, &mut model);
    labelled(&view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none());
    assert!(labelled(&view, text(UiText::Undo)).is_empty());
    assert_eq!(model.messages(&alice).len(), 1, "the Dismiss stands");
}

/// A Dismiss, then Back and the same conversation picked again, before
/// the root takes -- with a keystroke between, which ends a take early, or
/// without: in the person's order the snackbar was up when they left, so
/// it closes (review P3-1 on #291, round 3, and its pre-existing case).
#[test]
fn a_dismiss_before_leaving_and_coming_back_leaves_no_snackbar() {
    dismiss_leave_and_come_back(true);
}

/// The same with no keystroke: one take drains it all.
#[test]
fn a_dismiss_before_leaving_and_coming_back_in_one_take_leaves_no_snackbar() {
    dismiss_leave_and_come_back(false);
}

fn dismiss_leave_and_come_back(keystroke: bool) {
    {
        let mut view = view();
        view.window()
            .window()
            .set_size(slint::LogicalSize::new(390.0, 800.0));
        let mut model = UiModel::new();
        let alice = peer();
        read(&mut model, 1, &alice, "first");
        read(&mut model, 2, &alice, "second");
        open(&mut view, &mut model, &direct(&alice));
        assert!(model.snackbar().is_none(), "control: none up");
        choose(&view, text(UiText::Dismiss));
        if keystroke {
            view.window().invoke_draft_edited("h".into());
        }
        labelled(&view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
        view.select(direct(&alice));
        slint::platform::update_timers_and_animations();
        let _ = turn(&mut view, &mut model);
        assert_eq!(
            model.messages(&direct(&alice)).len(),
            1,
            "control: dismissed"
        );
        assert!(model.snackbar().is_none(), "keystroke between: {keystroke}");
    }
}

/// A Dismiss and a resize to the list before the root takes: the
/// snackbar the Dismiss makes is for a conversation no longer on screen,
/// so it closes too (review R2 on #291).
#[test]
fn a_dismiss_then_a_resize_to_the_list_in_one_take_leaves_no_snackbar() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 800.0));
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    open(&mut view, &mut model, &direct(&alice));
    open_trust(&view);
    let _ = turn(&mut view, &mut model);
    labelled(&view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none(), "control: none up yet");
    choose(&view, text(UiText::Dismiss));
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    slint::platform::update_timers_and_animations();
    let _ = turn(&mut view, &mut model);
    assert!(
        model.messages(&direct(&alice)).is_empty(),
        "control: dismissed"
    );
    assert!(model.snackbar().is_none());
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

// --- focus after a row leaves (swipe spec section 7; section 11 item 21) --

/// Three read rows; focus placed on the `at`th row's Dismiss by the
/// keyboard and pressed there.
fn dismiss_by_keyboard(rows: usize, at: usize) -> (View, Vec<i32>) {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    for n in 0..rows {
        read(&mut model, i64::try_from(n).unwrap() + 1, &alice, "text");
    }
    open(&mut view, &mut model, &direct(&alice));
    let handles: Vec<i32> = (0..rows).map(|i| handle(&view, i)).collect();
    // Dismiss is the menu's second item, after Keep.
    menu_by_keyboard(&view, handles[at]);
    tab_to(&view, &format!("action:{}:1", handles[at]));
    press_focused(&view);
    let _ = turn(&mut view, &mut model);
    (view, handles)
}

#[test]
fn a_dismiss_by_keyboard_focuses_the_next_rows_first_control() {
    let (view, handles) = dismiss_by_keyboard(3, 1);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{}", handles[2])
    );
}

#[test]
fn a_dismiss_of_the_last_row_focuses_the_previous_one() {
    let (view, handles) = dismiss_by_keyboard(3, 2);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{}", handles[1])
    );
}

#[test]
fn a_dismiss_of_the_only_row_focuses_the_header_title() {
    let (view, _) = dismiss_by_keyboard(1, 0);
    assert_eq!(view.window().get_focus_id().as_str(), "header-title");
}

#[test]
fn a_touch_swipe_leaves_focus_held_in_another_row() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    for n in 1..=3 {
        read(&mut model, n, &alice, "text");
    }
    open(&mut view, &mut model, &direct(&alice));
    let on = tab_to(&view, &format!("item:{}", handle(&view, 2)));
    swipe(&view, &row_key(&view, 0), -170.0, 0.0, 8);
    let _ = turn(&mut view, &mut model);
    assert!(view.window().get_snack_shown(), "control: it was dismissed");
    assert_eq!(view.window().get_focus_id().as_str(), on, "focus stays");
}

#[test]
fn a_touch_swipe_on_the_row_holding_focus_moves_it_as_a_button_would() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    for n in 1..=2 {
        read(&mut model, n, &alice, "text");
    }
    open(&mut view, &mut model, &direct(&alice));
    let (first, second) = (handle(&view, 0), handle(&view, 1));
    tab_to(&view, &format!("item:{first}"));
    swipe(&view, &row_key(&view, 0), -170.0, 0.0, 8);
    let _ = turn(&mut view, &mut model);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{second}")
    );
}

#[test]
fn hiding_the_only_conversation_from_its_row_focuses_the_lists_heading() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "only");
    let _ = turn(&mut view, &mut model);
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row");
    tab_to(&view, &format!("conversation:{}", row.handle));
    swipe(&view, &format!("c:{}", row.handle), -170.0, 0.0, 8);
    let _ = turn(&mut view, &mut model);
    assert!(model.conversations().is_empty(), "control: hidden");
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "conversations-heading"
    );
}

#[test]
fn hide_conversation_in_its_header_focuses_the_next_conversation_row() {
    let mut view = view();
    let mut model = UiModel::new();
    let (alice, bob) = (peer(), peer());
    read(&mut model, 1, &alice, "a");
    read(&mut model, 2, &bob, "b");
    let _ = turn(&mut view, &mut model);
    let rows = view.window().get_conversations();
    let other = rows.row_data(1).expect("a row").handle;
    let key = model.conversations()[0].key.clone();
    open(&mut view, &mut model, &key);
    // In the overflow menu (android-look section 6), which opens with
    // focus on its first entry.
    tab_to(&view, "more-options");
    press_focused(&view);
    // The entry takes focus once it is in the window, a timer's turn later.
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(10));
    assert_eq!(view.window().get_focus_id().as_str(), "hide-conversation");
    press_focused(&view);
    let _ = turn(&mut view, &mut model);
    assert_eq!(model.conversations().len(), 1, "control: hidden");
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("conversation:{other}")
    );
}

#[test]
fn the_two_headings_take_keyboard_focus_and_do_nothing() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "only");
    open(&mut view, &mut model, &direct(&alice));
    tab_to(&view, "conversations-heading");
    press_focused(&view);
    assert!(turn(&mut view, &mut model).is_empty());
    tab_to(&view, "header-title");
    press_focused(&view);
    assert!(turn(&mut view, &mut model).is_empty());
    assert_eq!(model.messages(&direct(&alice)).len(), 1, "nothing happened");
}

/// §3: the snackbar follows the layout direction. Its Undo is at the
/// line's end, the right left to right and the left right to left, at
/// least 48 x 48, inside the snackbar, and never over its text.
#[test]
fn the_snackbar_mirrors_right_to_left() {
    let mut view = view();
    for rtl in [false, true] {
        view.set_touch_settings(TouchSettings {
            rtl,
            ..TouchSettings::default()
        });
        let mut model = UiModel::new();
        let alice = peer();
        read(&mut model, 1, &alice, "first");
        open(&mut view, &mut model, &direct(&alice));
        choose(&view, text(UiText::Dismiss));
        let _ = turn(&mut view, &mut model);
        assert!(view.window().get_snack_shown());
        let undo = labelled(&view, text(UiText::Undo));
        assert_eq!(undo.len(), 1, "one Undo");
        let u = &undo[0];
        // The announcement says it too: the snackbar's is the one beside
        // its Undo.
        let (uy, uh) = (u.absolute_position().y, u.size().height);
        let said: Vec<ElementHandle> = labelled(&view, text(UiText::ConfirmDismissed))
            .into_iter()
            .filter(|t| {
                let mid = t.absolute_position().y + t.size().height / 2.0;
                t.size().width > 1.0 && mid > uy && mid < uy + uh
            })
            .collect();
        assert_eq!(said.len(), 1, "rtl {rtl}: its text beside the Undo");
        let t = &said[0];
        let (ux, uw) = (u.absolute_position().x, u.size().width);
        let (tx, tw) = (t.absolute_position().x, t.size().width);
        assert!(
            u.size().width >= 48.0 && u.size().height >= 48.0,
            "rtl {rtl}: Undo {:?}",
            u.size()
        );
        if rtl {
            assert!(
                ux + uw <= tx,
                "rtl: Undo {ux}+{uw} left of the text at {tx}"
            );
        } else {
            assert!(tx + tw <= ux, "ltr: text {tx}+{tw} left of Undo at {ux}");
        }
        let window = view.window().window();
        let width = window.size().to_logical(window.scale_factor()).width;
        assert!(
            ux >= 0.0 && ux + uw <= width + 0.5,
            "rtl {rtl}: Undo on screen"
        );
    }
}

// --- focus after Keep, Unkeep and Retry (§7; §11 item 21) ------------------

/// Carry out what `turn` returned that changes a row's buttons: a Keep
/// kept as stored row `kept`, an Unkeep released. Then render, as the
/// root does once the store has answered.
fn answer(view: &mut View, model: &mut UiModel, intents: &[Intent], kept: i64) {
    for intent in intents {
        match intent {
            Intent::Keep { item, .. } => model.kept(*item, RowId::from_stored(kept)),
            Intent::Unkeep(row) => model.unkept(*row),
            _ => {}
        }
    }
    let _ = turn(view, model);
}

/// Two read rows, the second's control focused by the keyboard first, as
/// the place focus is "elsewhere".
fn two_read_rows_focus_on_the_second() -> (View, UiModel, i32, String) {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    let first = handle(&view, 0);
    let elsewhere = tab_to(&view, &format!("item:{}", handle(&view, 1)));
    (view, model, first, elsewhere)
}

#[test]
fn keep_from_the_menu_by_keyboard_focuses_the_rows_first_control() {
    let (mut view, mut model, first, _) = two_read_rows_focus_on_the_second();
    // Keep is the menu's first item, which takes focus as it opens.
    menu_by_keyboard(&view, first);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("action:{first}:0"),
        "control: Keep holds focus"
    );
    press_focused(&view);
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 10);
    assert_eq!(
        view.window()
            .get_messages()
            .row_data(0)
            .expect("row")
            .actions
            .row_count(),
        1,
        "control: the row now offers Unkeep alone"
    );
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{first}"),
        "the menu closed; focus is on the row's bubble"
    );
}

/// Chosen from the menu, Keep and Unkeep leave focus on the row's bubble,
/// wherever it was before the menu opened.
#[test]
fn keep_and_unkeep_from_the_menu_focus_the_rows_first_control() {
    let (mut view, mut model, first, elsewhere) = two_read_rows_focus_on_the_second();
    let _ = elsewhere;
    choose(&view, text(UiText::Keep));
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 10);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{first}")
    );

    let _ = tab_to(&view, &format!("item:{}", handle(&view, 1)));
    choose(&view, text(UiText::Unkeep));
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 0);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{first}")
    );
}

#[test]
fn keep_by_a_touch_swipe_leaves_focus_held_in_another_row() {
    let (mut view, mut model, _, elsewhere) = two_read_rows_focus_on_the_second();
    swipe(&view, &row_key(&view, 0), 170.0, 0.0, 8);
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 10);
    assert!(model.snackbar().is_some(), "control: it was kept");
    assert_eq!(view.window().get_focus_id().as_str(), elsewhere);
}

/// A swipe raised after a button press of the same row ends the button's
/// claim: the swipe's Unkeep moves focus only from inside the row.
#[test]
fn a_swipe_after_a_refused_press_moves_focus_only_from_inside_the_row() {
    let (mut view, mut model, first, _) = two_read_rows_focus_on_the_second();
    choose(&view, text(UiText::Keep));
    let intents = turn(&mut view, &mut model);
    let keep = intents.first().expect("Keep").clone();
    model.intent_refused(&keep);
    let _ = turn(&mut view, &mut model);
    let elsewhere = view.window().get_focus_id().to_string();
    swipe(&view, &format!("m:{first}"), 170.0, 0.0, 8);
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 10);
    assert!(model.snackbar().is_some(), "control: it was kept");
    assert_eq!(view.window().get_focus_id().as_str(), elsewhere);
}

#[test]
fn unkeep_by_a_touch_swipe_on_the_row_holding_focus_focuses_its_first_control() {
    let (mut view, mut model, first, _) = two_read_rows_focus_on_the_second();
    choose(&view, text(UiText::Keep));
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 10);
    // Focus inside the row: its bubble.
    tab_to(&view, &format!("item:{first}"));
    swipe(&view, &format!("m:{first}"), -170.0, 0.0, 8);
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 0);
    assert_eq!(
        view.window()
            .get_messages()
            .row_data(0)
            .expect("row")
            .actions
            .row_count(),
        2,
        "control: Keep and Dismiss again"
    );
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{first}")
    );
}

#[test]
fn retry_from_the_menu_focuses_the_rows_bubble() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    pending(&mut model, 1, &alice, "out");
    open(&mut view, &mut model, &direct(&alice));
    let only = handle(&view, 0);
    // Retry is the menu's first item.
    menu_by_keyboard(&view, only);
    let retry = format!("item:{only}");
    press_focused(&view);
    let intents = turn(&mut view, &mut model);
    assert!(
        matches!(intents.first(), Some(Intent::Retry(_))),
        "{intents:?}"
    );
    assert_eq!(view.window().get_focus_id().as_str(), retry);
}

// --- focus when the snackbar closes under its Undo (§7; §11 item 21) -------

/// Let the snackbar's time run out, and turn.
fn time_out(view: &mut View, model: &mut UiModel) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(6_100));
    let _ = turn(view, model);
}

#[test]
fn a_timed_out_dismiss_with_focus_on_its_undo_focuses_the_next_row() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    for n in 1..=3 {
        read(&mut model, n, &alice, "text");
    }
    open(&mut view, &mut model, &direct(&alice));
    let second = handle(&view, 1);
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    tab_to(&view, "undo");
    time_out(&mut view, &mut model);
    assert!(model.snackbar().is_none(), "control: timed out");
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{second}")
    );
}

#[test]
fn a_timed_out_keep_with_focus_on_its_undo_focuses_the_kept_row() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    let first = handle(&view, 0);
    choose(&view, text(UiText::Keep));
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 10);
    tab_to(&view, "undo");
    time_out(&mut view, &mut model);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{first}")
    );
}

/// A touch swipe's snackbar replaces the one whose Undo held focus: focus
/// goes to the older action's row's place, not left on the newer Undo.
#[test]
fn a_snackbar_replaced_under_its_focused_undo_focuses_the_older_actions_place() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    for n in 1..=3 {
        read(&mut model, n, &alice, "text");
    }
    open(&mut view, &mut model, &direct(&alice));
    let (second, third) = (handle(&view, 1), handle(&view, 2));
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    tab_to(&view, "undo");
    swipe(&view, &format!("m:{third}"), 170.0, 0.0, 8);
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 10);
    assert_eq!(
        model.snackbar().map(|s| s.confirms),
        Some(interweave_human_ui_model::Confirmation::Kept),
        "control: replaced"
    );
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("item:{second}")
    );
}

/// Back, with focus on the Undo: the snackbar closes, and focus goes to the
/// conversation's row on the list.
#[test]
fn leaving_with_focus_on_the_undo_focuses_the_conversations_row() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    tab_to(&view, "undo");
    labelled(&view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none(), "control: closed by leaving");
    assert!(view.window().get_list_shown(), "control: on the list");
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row")
        .handle;
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("conversation:{row}")
    );
}

/// A message brings a hidden conversation back while focus is on the
/// hide's Undo: focus goes to the returned conversation's row (§10).
#[test]
fn a_message_into_a_hidden_conversation_moves_focus_from_the_undo_to_its_row() {
    let mut view = view();
    let mut model = UiModel::new();
    let (alice, bob) = (peer(), peer());
    read(&mut model, 1, &alice, "a");
    read(&mut model, 2, &bob, "b");
    let _ = turn(&mut view, &mut model);
    let first = model.conversations()[0].key.clone();
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row")
        .handle;
    swipe(&view, &format!("c:{row}"), -170.0, 0.0, 8);
    let _ = turn(&mut view, &mut model);
    tab_to(&view, "undo");
    let ConversationKey::Direct { peer: from, .. } = &first else {
        panic!("direct");
    };
    model.received(Received {
        row: RowId::from_stored(3),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: human(),
        },
        envelope: envelope(9_999, "again"),
        received_at: 3,
    });
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none(), "control: the arrival closed it");
    let back = view
        .window()
        .get_conversations()
        .iter()
        .find(|r| r.title == model.title(&first))
        .expect("back on the list")
        .handle;
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("conversation:{back}")
    );
}

/// An Undo pressed from the keyboard whose answer has not come: the
/// snackbar has gone, and a later snackbar's timeout moves nothing -- the
/// gone Undo's name no longer stands for focus. (The later Dismiss is
/// chosen from a menu, which leaves focus on a bubble.)
#[test]
fn a_stale_undo_focus_moves_nothing_later() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    for n in 1..=3 {
        read(&mut model, n, &alice, "text");
    }
    open(&mut view, &mut model, &direct(&alice));
    choose(&view, text(UiText::Keep));
    let intents = turn(&mut view, &mut model);
    answer(&mut view, &mut model, &intents, 10);
    tab_to(&view, "undo");
    press_focused(&view);
    let intents = turn(&mut view, &mut model);
    assert!(
        matches!(intents.first(), Some(Intent::Unkeep(_))),
        "control: {intents:?}"
    );
    assert_eq!(view.window().get_focus_id().as_str(), "", "cleared");
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    slint::platform::update_timers_and_animations();
    let before = view.window().get_focus_id().to_string();
    time_out(&mut view, &mut model);
    assert!(model.snackbar().is_none(), "control: timed out");
    assert_eq!(view.window().get_focus_id().as_str(), before);
}

// --- a Cancel's success after leaving (§6; §11 item 19) --------------------

/// A pending row's Cancel raised by its button and not answered yet, in a
/// conversation with a read row beside it.
fn cancel_waiting(view: &mut View, model: &mut UiModel, alice: &TransportIdentity) -> Intent {
    read(model, 1, alice, "in");
    pending(model, 2, alice, "out");
    open(view, model, &direct(alice));
    choose(view, text(UiText::Cancel));
    let intents = turn(view, model);
    let Some(cancel @ Intent::Cancel(row)) = intents.first().cloned() else {
        panic!("Cancel: {intents:?}");
    };
    model.cancel_requested(row);
    cancel
}

fn cancel_succeeds(model: &mut UiModel, cancel: &Intent) {
    let Intent::Cancel(row) = cancel else {
        panic!("a Cancel");
    };
    model.client_event(ClientEvent::Outbound(OutboundUpdate {
        row: *row,
        app_message_id: AppMessageId::parse(format!("{:032x}", 2)).expect("id"),
        status: OutboundStatus::Cancelled {
            may_have_reached: false,
        },
        last_code: None,
    }));
    model.cancelled(*row, true);
}

/// Left by Back, the success is only announced: no snackbar, no Undo, and
/// coming back shows none.
#[test]
fn a_cancels_success_after_leaving_by_back_is_only_announced() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    let mut model = UiModel::new();
    let alice = peer();
    let cancel = cancel_waiting(&mut view, &mut model, &alice);
    labelled(&view, text(UiText::BackToConversations))[0].invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
    let _ = turn(&mut view, &mut model);
    assert!(view.window().get_list_shown(), "control: left");
    cancel_succeeds(&mut model, &cancel);
    let _ = turn(&mut view, &mut model);
    assert_eq!(announced(&view), text(UiText::ConfirmSendingCanceled));
    assert!(!view.window().get_snack_shown());
    assert!(model.snackbar().is_none(), "no Undo held");
    open(&mut view, &mut model, &direct(&alice));
    assert!(!view.window().get_snack_shown(), "none on coming back");
    assert!(labelled(&view, text(UiText::Undo)).is_empty());
}

/// Left by picking another conversation in a wide window: the same.
#[test]
fn a_cancels_success_after_picking_another_conversation_is_only_announced() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 800.0));
    let mut model = UiModel::new();
    let (alice, bob) = (peer(), peer());
    read(&mut model, 7, &bob, "elsewhere");
    let cancel = cancel_waiting(&mut view, &mut model, &alice);
    open(&mut view, &mut model, &direct(&bob));
    cancel_succeeds(&mut model, &cancel);
    let _ = turn(&mut view, &mut model);
    assert_eq!(announced(&view), text(UiText::ConfirmSendingCanceled));
    assert!(!view.window().get_snack_shown());
    assert!(model.snackbar().is_none(), "no Undo held");
}

/// The control: still in the conversation, the Cancel's snackbar shows,
/// with its Undo.
#[test]
fn a_cancels_success_in_the_conversation_shows_its_snackbar() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    let cancel = cancel_waiting(&mut view, &mut model, &alice);
    cancel_succeeds(&mut model, &cancel);
    let _ = turn(&mut view, &mut model);
    assert!(view.window().get_snack_shown());
    assert!(view.window().get_snack_undo());
}

/// The platform's Back in a narrow window's conversation leaves it as the
/// header's Back does, the snackbar closing with it (§5); a second Back
/// finds the list, with no way back, so the platform leaves the app.
/// Before, Back with no control focused reached nothing and left the app
/// from the conversation (ux-designer-01's review of ffce28df).
#[test]
fn platform_back_leaves_the_conversation_then_the_app() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    read(&mut model, 2, &alice, "second");
    open(&mut view, &mut model, &direct(&alice));
    choose(&view, text(UiText::Dismiss));
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_some(), "control");
    assert!(view.back(), "the conversation takes Back");
    let _ = turn(&mut view, &mut model);
    assert!(model.snackbar().is_none(), "left as by the header's Back");
    assert!(!view.back(), "the list has no way back");
}

/// Back on the trust settings closes them, and only then leaves the app.
#[test]
fn platform_back_closes_the_trust_settings_first() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 800.0));
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "first");
    open(&mut view, &mut model, &direct(&alice));
    assert!(
        !view.back(),
        "control: a wide window's conversation has no way back"
    );
    open_trust(&view);
    let _ = turn(&mut view, &mut model);
    assert!(view.back(), "the trust settings take Back");
    let _ = turn(&mut view, &mut model);
    assert!(
        labelled(&view, text(UiText::BackToConversations)).is_empty(),
        "the trust settings are closed"
    );
    assert!(!view.back());
}

// --- the row's menu (§7, revised for #304 F2) -------------------------------

/// Escape and Back close the menu with nothing done and focus back on
/// its bubble; Shift+F10 opens it from the bubble as Enter does.
#[test]
fn the_menu_closes_to_its_bubble_and_opens_by_shift_f10() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    read(&mut model, 1, &alice, "text");
    open(&mut view, &mut model, &direct(&alice));
    let row = handle(&view, 0);
    menu_by_keyboard(&view, row);
    assert_eq!(view.window().get_row_menu(), row, "control: open");
    key(&view, Key::Escape);
    slint::platform::update_timers_and_animations();
    assert_eq!(view.window().get_row_menu(), -1, "Escape closes it");
    assert_eq!(view.window().get_focus_id().as_str(), format!("item:{row}"));
    assert!(turn(&mut view, &mut model).is_empty(), "nothing done");

    // Shift+F10 on the focused bubble.
    let window = view.window().window();
    let shift: slint::SharedString = Key::Shift.into();
    let f10: slint::SharedString = Key::F10.into();
    window.dispatch_event(WindowEvent::KeyPressed {
        text: shift.clone(),
    });
    window.dispatch_event(WindowEvent::KeyPressed { text: f10.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text: f10 });
    window.dispatch_event(WindowEvent::KeyReleased { text: shift });
    assert_eq!(view.window().get_row_menu(), row, "Shift+F10 opens it");
    assert!(view.back(), "Back is the menu's");
    slint::platform::update_timers_and_animations();
    assert_eq!(view.window().get_row_menu(), -1, "Back closes it");
    assert_eq!(view.window().get_focus_id().as_str(), format!("item:{row}"));
    assert!(turn(&mut view, &mut model).is_empty(), "nothing done");
}

/// A pending row accepted while its menu is open no longer offers what
/// the menu shows: the menu closes and focus goes to its bubble.
#[test]
fn a_row_whose_actions_change_under_its_open_menu_closes_it() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    pending(&mut model, 1, &alice, "out");
    open(&mut view, &mut model, &direct(&alice));
    let row = handle(&view, 0);
    menu_by_keyboard(&view, row);
    assert_eq!(view.window().get_row_menu(), row, "control: open");
    model.client_event(ClientEvent::Outbound(OutboundUpdate {
        row: RowId::from_stored(1),
        app_message_id: AppMessageId::parse(format!("{:032x}", 1)).expect("id"),
        status: OutboundStatus::Accepted { endpoint: human() },
        last_code: None,
    }));
    let _ = turn(&mut view, &mut model);
    slint::platform::update_timers_and_animations();
    assert_eq!(view.window().get_row_menu(), -1, "closed");
    assert_eq!(view.window().get_focus_id().as_str(), format!("item:{row}"));
}

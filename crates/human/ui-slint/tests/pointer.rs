// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A touch on a row as the window delivers it: pointer events dispatched
//! to the window, through Slint's own `SwipeGestureHandler`, list and
//! touch areas, rather than the row's callbacks invoked directly
//! (`design/2026-10-16-message-swipe.md` §3, §4 and §10).
//!
//! What this proves beyond `swipe.rs`: which callbacks the toolkit makes
//! for a drag a row does not offer, and that a drag never reaches a
//! control as a tap. NOT proved: a device's event timing, the system's
//! slop or what the screen reader reads.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::Duration;

use i_slint_backend_testing::ElementHandle;
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::{
    ConversationKey, Intent, RuntimeHost, UiModel, UiText, placeholder_en,
};
use interweave_human_ui_slint::{TouchSettings, View, ViewEvent};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle as _, LogicalPosition, Model as _};

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer id")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("an endpoint")
}

fn direct(peer: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(human()),
    }
}

/// A message from `from`, unread.
fn received(model: &mut UiModel, row: i64, from: &TransportIdentity, text: &str) {
    model.received(Received {
        row: RowId::from_stored(row),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: human(),
        },
        envelope: HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{:032x}", 1_000 + row),
            text: text.to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: u64::try_from(row).expect("positive"),
    });
}

/// A phone-sized window with focus, so a conversation on screen is read.
fn phone() -> View {
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    view.set_window_focused(true);
    view
}

/// The root's loop, as app-core's model side runs it.
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
    tick(0);
    out
}

/// Let `ms` pass, timers included: a press inside the list reaches the
/// row only after the toolkit's forwarding delay.
fn tick(ms: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(ms));
}

/// The centre of the conversation row titled as row `index` is.
fn conversation_centre(view: &View, index: usize) -> (f32, f32) {
    let title = view
        .window()
        .get_conversations()
        .row_data(index)
        .expect("a row")
        .title;
    let found: Vec<ElementHandle> =
        ElementHandle::find_by_accessible_label(view.window(), title.as_str()).collect();
    let row = found.first().expect("the row is drawn");
    let (at, size) = (row.absolute_position(), row.size());
    (at.x + size.width / 2.0, at.y + size.height / 2.0)
}

fn event(view: &View, event: WindowEvent) {
    view.window().window().dispatch_event(event);
}

/// A finger down at `from`, moved by `dx` in 10 dp steps 16 ms apart,
/// then lifted where it stopped.
fn drag(view: &View, from: (f32, f32), dx: f32) {
    drag_via(view, from, &[dx]);
}

/// A finger down at `from` and held there 120 ms, then moved through each
/// horizontal offset from it in turn, in 10 dp steps 16 ms apart, and
/// lifted where it stopped. The hold outlasts the list's forwarding delay
/// (about 100 ms), so the control under the finger sees every move: moves
/// before the press reaches it are never seen.
fn drag_via(view: &View, from: (f32, f32), offsets: &[f32]) {
    let position = |dx: f32| LogicalPosition::new(from.0 + dx, from.1);
    event(
        view,
        WindowEvent::PointerPressed {
            position: position(0.0),
            button: PointerEventButton::Left,
        },
    );
    tick(120);
    let mut was = 0.0;
    for &to in offsets {
        let steps = ((to - was).abs() / 10.0).ceil();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        for i in 1..=steps as u32 {
            tick(16);
            #[allow(clippy::cast_precision_loss)]
            let dx = was + (to - was) * (i as f32 / steps);
            event(
                view,
                WindowEvent::PointerMoved {
                    position: position(dx),
                },
            );
        }
        was = to;
    }
    tick(16);
    event(
        view,
        WindowEvent::PointerReleased {
            position: position(was),
            button: PointerEventButton::Left,
        },
    );
    tick(16);
}

/// A finger down and up at `at`, as a tap.
fn tap(view: &View, at: (f32, f32)) {
    let position = LogicalPosition::new(at.0, at.1);
    event(
        view,
        WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        },
    );
    tick(50);
    event(
        view,
        WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        },
    );
    tick(16);
}

/// Item 17: a drag on a Chats row that offers no swipe is not a tap. It
/// neither opens the conversation nor reads it. The control is a tap on
/// the same row, which does both.
#[test]
fn a_drag_on_a_conversation_row_offering_no_swipe_opens_and_reads_nothing() {
    let mut view = phone();
    let mut model = UiModel::new();
    let alice = peer();
    received(&mut model, 1, &alice, "unread, so the row does not move");
    let _ = turn(&mut view, &mut model);
    assert!(view.window().get_list_shown(), "the list is on screen");
    let centre = conversation_centre(&view, 0);

    for dx in [-150.0, 150.0] {
        drag(&view, centre, dx);
        let raised = turn(&mut view, &mut model);
        assert!(raised.is_empty(), "a {dx} drag raised {raised:?}");
        assert!(view.window().get_list_shown(), "a {dx} drag opened it");
    }

    tap(&view, centre);
    let raised = turn(&mut view, &mut model);
    assert!(!view.window().get_list_shown(), "the tap opens it");
    assert_eq!(raised, vec![Intent::MarkRead(RowId::from_stored(1))]);
    let _ = direct(&alice);
}

/// The point on message row `index` a finger lands on away from its
/// controls: the author's name, at the row's top.
fn message_text_at(view: &View, index: usize) -> (f32, f32) {
    let label = view
        .window()
        .get_messages()
        .row_data(index)
        .expect("a row")
        .label;
    let found: Vec<ElementHandle> =
        ElementHandle::find_by_accessible_label(view.window(), label.as_str()).collect();
    let row = found.first().expect("the row is drawn");
    let at = row.absolute_position();
    (250.0, at.y + 10.0)
}

/// Item 5: on a kept row, a right drag (which it does not offer) leaves
/// the next left drag, from the same point, tracking and committing its
/// Unkeep.
#[test]
fn a_drag_a_row_does_not_offer_leaves_the_next_drag_its_own() {
    let mut view = phone();
    let mut model = UiModel::new();
    let alice = peer();
    received(&mut model, 1, &alice, "kept");
    model.read(RowId::from_stored(1));
    let key = direct(&alice);
    view.select(key.clone());
    let _ = turn(&mut view, &mut model);
    let item = model.messages(&key)[0].key;
    model.kept(item, RowId::from_stored(10));
    let _ = turn(&mut view, &mut model);
    let at = message_text_at(&view, 0);

    drag(&view, at, 120.0);
    let raised = turn(&mut view, &mut model);
    assert!(raised.is_empty(), "right offers nothing: {raised:?}");
    drag(&view, at, -200.0);
    let raised = turn(&mut view, &mut model);
    assert!(
        matches!(raised.as_slice(), [Intent::Unkeep(_)]),
        "the left drag after it: {raised:?}"
    );
}

/// The centre of the control labelled `label`, which must be drawn once.
fn control_centre(view: &View, label: &str) -> (f32, f32) {
    let found: Vec<ElementHandle> =
        ElementHandle::find_by_accessible_label(view.window(), label).collect();
    assert_eq!(found.len(), 1, "one {label}");
    let (at, size) = (found[0].absolute_position(), found[0].size());
    (at.x + size.width / 2.0, at.y + size.height / 2.0)
}

/// Open message row `index`'s menu as a screen reader does, by the row's
/// default action.
fn open_menu(view: &View, index: usize) {
    let label = view
        .window()
        .get_messages()
        .row_data(index)
        .expect("a row")
        .label;
    i_slint_backend_testing::ElementQuery::from_root(view.window())
        .match_accessible_role(i_slint_backend_testing::AccessibleRole::ListItem)
        .find_all()
        .into_iter()
        .find(|e| e.accessible_label().as_deref() == Some(label.as_str()))
        .expect("the row's item")
        .invoke_accessible_default_action();
    tick(2);
}

/// A kept row, read, in a phone-sized view: it offers Unkeep.
fn kept_row() -> (View, UiModel) {
    let mut view = phone();
    let mut model = UiModel::new();
    let alice = peer();
    received(&mut model, 1, &alice, "kept");
    model.read(RowId::from_stored(1));
    let key = direct(&alice);
    view.select(key.clone());
    let _ = turn(&mut view, &mut model);
    let item = model.messages(&key)[0].key;
    model.kept(item, RowId::from_stored(10));
    let _ = turn(&mut view, &mut model);
    (view, model)
}

/// §4 (revised for #304 F2): a press held within the slop for the
/// platform's long-press time opens the row's menu; a press that passed
/// the slop first never does, however long it is held; neither raises an
/// intent. The menu opens with one haptic tick.
#[test]
fn a_press_held_within_the_slop_opens_the_rows_menu_and_one_past_it_never_does() {
    let (mut view, mut model) = kept_row();
    view.set_touch_settings(TouchSettings {
        long_press_ms: 400,
        ..TouchSettings::default()
    });
    let ticks = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = std::rc::Rc::clone(&ticks);
    view.on_haptic(move || counted.set(counted.get() + 1));
    let at = message_text_at(&view, 0);
    let handle = view
        .window()
        .get_messages()
        .row_data(0)
        .expect("a row")
        .handle;
    let press = |view: &View, moved: f32, hold: u64| {
        let position = |dx: f32| LogicalPosition::new(at.0 + dx, at.1);
        event(
            view,
            WindowEvent::PointerPressed {
                position: position(0.0),
                button: PointerEventButton::Left,
            },
        );
        tick(120);
        if moved != 0.0 {
            event(
                view,
                WindowEvent::PointerMoved {
                    position: position(moved),
                },
            );
        }
        tick(hold);
        let opened = view.window().get_row_menu();
        event(
            view,
            WindowEvent::PointerReleased {
                position: position(moved),
                button: PointerEventButton::Left,
            },
        );
        tick(16);
        opened
    };
    assert_eq!(
        press(&view, 0.0, 200),
        -1,
        "control: under the long-press time"
    );
    assert_eq!(
        press(&view, 0.0, 400),
        handle,
        "held within the slop: its menu"
    );
    assert_eq!(ticks.get(), 1, "one haptic tick as it opens");
    view.window().invoke_close_row_menu();
    // To the right, which a kept row does not offer, so no swipe takes it.
    assert_eq!(press(&view, 30.0, 800), -1, "past the slop first: never");
    assert!(turn(&mut view, &mut model).is_empty());
}

/// §7: a secondary click on a bubble opens its menu at once.
#[test]
fn a_secondary_click_opens_the_rows_menu() {
    let (view, _) = kept_row();
    let at = message_text_at(&view, 0);
    let handle = view
        .window()
        .get_messages()
        .row_data(0)
        .expect("a row")
        .handle;
    let position = LogicalPosition::new(at.0, at.1);
    event(
        &view,
        WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Right,
        },
    );
    tick(16);
    event(
        &view,
        WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Right,
        },
    );
    tick(16);
    assert_eq!(view.window().get_row_menu(), handle);
}

/// §4: a drag that starts on an item of an open menu presses nothing,
/// even released over it; a tap on it presses it.
#[test]
fn a_drag_from_a_menu_item_never_presses_it_and_a_tap_does() {
    let (mut view, mut model) = kept_row();
    open_menu(&view, 0);
    let unkeep = placeholder_en::text(UiText::Unkeep);
    let on = control_centre(&view, unkeep);

    // 30 is past the slop and still over the item.
    drag(&view, on, 30.0);
    assert_eq!(control_centre(&view, unkeep), on, "the menu stayed");
    let raised = turn(&mut view, &mut model);
    assert!(raised.is_empty(), "a right drag from {unkeep}: {raised:?}");

    // Out past the slop and back: the press did not stay within it.
    drag_via(&view, on, &[30.0, 0.0]);
    let raised = turn(&mut view, &mut model);
    assert!(raised.is_empty(), "out and back on {unkeep}: {raised:?}");

    tap(&view, on);
    let raised = turn(&mut view, &mut model);
    assert!(
        matches!(raised.as_slice(), [Intent::Unkeep(_)]),
        "a tap on {unkeep}: {raised:?}"
    );
}

/// The tap's slop is the platform's, as the swipe's is: with a slop of 40
/// a 30 dp wander is still the menu item's tap.
#[test]
fn a_taps_slop_is_the_platforms() {
    let (mut view, mut model) = kept_row();
    view.set_touch_settings(TouchSettings {
        slop: 40.0,
        ..TouchSettings::default()
    });
    open_menu(&view, 0);
    let unkeep = placeholder_en::text(UiText::Unkeep);
    drag_via(&view, control_centre(&view, unkeep), &[30.0, 0.0]);
    let raised = turn(&mut view, &mut model);
    assert!(
        matches!(raised.as_slice(), [Intent::Unkeep(_)]),
        "within a 40 slop: {raised:?}"
    );
}

/// #304 F5: the focus ring is for keyboard focus only. A tap that opens
/// the conversation's menu, whose first item then takes focus, draws no
/// ring; a Tab draws one; a tap clears it again.
#[test]
fn the_focus_ring_shows_for_the_keyboard_never_for_a_tap() {
    let (view, _) = kept_row();
    let rings = |view: &View| {
        i_slint_backend_testing::ElementQuery::from_root(view.window())
            .find_all()
            .into_iter()
            .filter(|e| e.type_name().as_deref() == Some("FocusRing"))
            .filter(|r| r.computed_opacity() > 0.0)
            .count()
    };
    tap(
        &view,
        control_centre(&view, placeholder_en::text(UiText::MoreOptions)),
    );
    tick(10);
    tick(10);
    assert!(view.window().get_menu_open(), "control: the tap opened it");
    assert!(
        !view.window().get_focus_id().is_empty(),
        "control: focus moved"
    );
    assert_eq!(rings(&view), 0, "a tap draws no ring");
    let key: slint::SharedString = slint::platform::Key::Tab.into();
    event(&view, WindowEvent::KeyPressed { text: key.clone() });
    event(&view, WindowEvent::KeyReleased { text: key });
    tick(10);
    assert_eq!(rings(&view), 1, "a Tab draws one");
    tap(&view, message_text_at(&view, 0));
    tick(10);
    assert_eq!(rings(&view), 0, "a tap clears it");
}

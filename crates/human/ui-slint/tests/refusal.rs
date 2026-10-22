// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The start-up refusal screen on Slint's testing backend: the tree a
//! screen reader is handed, focus, the keyboard, and what each action
//! calls. What it does NOT prove: a real window, a platform adapter or
//! the app's flow around it -- `apps/human-desktop` and the desktop
//! end-to-end suite's refusal cases.

#![allow(clippy::expect_used, clippy::panic)]

use std::cell::Cell;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use interweave_human_ui_model::{StoreRecovery, StoreScreen, UiText, placeholder_en};
use interweave_human_ui_slint::{RefusalPage, RefusalView};
use slint::ComponentHandle as _;
use slint::platform::{Key, WindowEvent};

const PATH: &str = "/home/someone/.local/state/interweave/profiles/desk/human/human.sqlite";

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

fn store_page() -> RefusalPage {
    RefusalPage {
        reason: UiText::RefusalStoreRecovery,
        path: Some(PATH.to_owned()),
        can_retry: true,
        can_discard: false,
    }
}

fn closed_page() -> RefusalPage {
    RefusalPage {
        reason: UiText::RefusalAlreadyOpen,
        path: None,
        can_retry: false,
        can_discard: false,
    }
}

/// The screen, shown and focused as `run` leaves it.
fn screen(page: &RefusalPage) -> RefusalView {
    thread_local! {
        static INIT: () = i_slint_backend_testing::init_no_event_loop();
    }
    INIT.with(|()| {});
    let view = RefusalView::new(page).expect("a window");
    view.window().show().expect("shown");
    view.window().invoke_focus_primary();
    slint::platform::update_timers_and_animations();
    view
}

fn labelled(view: &RefusalView, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label).collect()
}

fn the(view: &RefusalView, label: &str) -> ElementHandle {
    let mut found = labelled(view, label);
    assert_eq!(found.len(), 1, "one element labelled {label:?}");
    found.remove(0)
}

fn key(view: &RefusalView, key: Key) {
    let key: slint::SharedString = key.into();
    let window = view.window().window();
    window.dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text: key });
    slint::platform::update_timers_and_animations();
}

fn focus(view: &RefusalView) -> String {
    view.window().get_focus_id().to_string()
}

/// What the live region says now: the two slots' texts, the empty one
/// left out.
fn said(view: &RefusalView) -> Vec<String> {
    [view.window().get_status_a(), view.window().get_status_b()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn the_reason_and_the_whole_path_are_read_and_try_again_has_focus() {
    let view = screen(&store_page());
    the(&view, text(UiText::RefusalHeading));
    the(&view, text(UiText::RefusalStoreRecovery));
    let path = the(&view, text(UiText::RefusalPath));
    assert_eq!(
        path.accessible_value().as_deref(),
        Some(PATH),
        "the path is read whole, as shown"
    );
    let retry = the(&view, text(UiText::TryAgain));
    assert_eq!(
        retry.accessible_role(),
        Some(i_slint_backend_testing::AccessibleRole::Button)
    );
    the(&view, text(UiText::Close));
    assert_eq!(focus(&view), "retry", "Try again is where focus starts");
    assert_eq!(view.window().get_window_title(), text(UiText::AppTitle));
}

#[test]
fn without_try_again_it_is_neither_shown_nor_reached_and_close_has_focus() {
    let view = screen(&closed_page());
    assert!(
        labelled(&view, text(UiText::TryAgain)).is_empty(),
        "no Try again in the tree"
    );
    assert!(
        labelled(&view, text(UiText::RefusalPath)).is_empty(),
        "no path field when the refusal names none"
    );
    assert_eq!(focus(&view), "close");
    for _ in 0..3 {
        key(&view, Key::Tab);
        assert_eq!(
            focus(&view),
            "close",
            "Tab never reaches a hidden Try again"
        );
    }
}

#[test]
fn tab_runs_through_the_path_close_and_try_again() {
    let view = screen(&store_page());
    let mut seen = Vec::new();
    for _ in 0..3 {
        key(&view, Key::Tab);
        seen.push(focus(&view));
    }
    assert_eq!(seen, ["path", "close", "retry"]);
}

#[test]
fn try_again_calls_the_root_and_a_refusal_after_it_is_said_each_time() {
    let view = screen(&store_page());
    let pressed = Rc::new(Cell::new(0));
    let count = Rc::clone(&pressed);
    view.on_retry(move || count.set(count.get() + 1));

    the(&view, text(UiText::TryAgain)).invoke_accessible_default_action();
    key(&view, Key::Return);
    assert_eq!(pressed.get(), 2, "a screen reader's action and Enter both");
    assert!(
        said(&view).is_empty(),
        "nothing said before the root answers"
    );

    let still = text(UiText::RefusalStillRefused);
    view.tried_again();
    let first = (view.window().get_status_a(), view.window().get_status_b());
    assert_eq!(said(&view), [still]);
    view.tried_again();
    let second = (view.window().get_status_a(), view.window().get_status_b());
    assert_eq!(said(&view), [still]);
    assert_ne!(
        first, second,
        "the same words a second time move slot, so the live region changes"
    );
}

#[test]
fn a_new_page_replaces_the_reason_and_clears_what_was_said() {
    let view = screen(&store_page());
    view.tried_again();
    view.show_page(&RefusalPage {
        reason: UiText::RefusalStoreUnavailable,
        path: Some(PATH.to_owned()),
        can_retry: true,
        can_discard: false,
    });
    slint::platform::update_timers_and_animations();
    the(&view, text(UiText::RefusalStoreUnavailable));
    assert!(labelled(&view, text(UiText::RefusalStoreRecovery)).is_empty());
    assert!(said(&view).is_empty());
    assert_eq!(focus(&view), "retry");
}

#[test]
fn close_and_escape_each_close_the_window() {
    let view = screen(&store_page());
    the(&view, text(UiText::Close)).invoke_accessible_default_action();
    assert!(!view.window().window().is_visible(), "Close closes");

    let view = screen(&store_page());
    key(&view, Key::Escape);
    assert!(!view.window().window().is_visible(), "Escape closes");

    let view = screen(&closed_page());
    view.close();
    assert!(
        !view.window().window().is_visible(),
        "the root's close closes"
    );
}

#[test]
fn a_long_path_wraps_inside_the_narrowest_window() {
    let view = screen(&store_page());
    let long = format!("/{}", "deep/".repeat(60));
    view.show_page(&RefusalPage {
        path: Some(long.clone()),
        ..store_page()
    });
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(320.0, 800.0));
    slint::platform::update_timers_and_animations();
    let path = the(&view, text(UiText::RefusalPath));
    assert_eq!(path.accessible_value().as_deref(), Some(long.as_str()));
    let right = path.absolute_position().x + path.size().width;
    assert!(right <= 320.0, "the path ends inside the window: {right}");
    for label in [text(UiText::RefusalStoreRecovery), text(UiText::TryAgain)] {
        let at = the(&view, label);
        let right = at.absolute_position().x + at.size().width;
        assert!(right <= 320.0, "{label:?} ends inside the window: {right}");
    }
}

/// Where the element labelled `label` is: top, bottom and right edge.
fn edges(view: &RefusalView, label: &str) -> (f32, f32, f32) {
    let at = the(view, label);
    let (pos, size) = (at.absolute_position(), at.size());
    (pos.y, pos.y + size.height, pos.x + size.width)
}

#[test]
fn the_actions_stay_in_view_and_a_taller_window_keeps_the_path_under_its_reason() {
    let view = screen(&RefusalPage {
        path: Some(format!("/{}", "deep/".repeat(40))),
        ..store_page()
    });
    let window = view.window().window();
    let height = window.size().to_logical(window.scale_factor()).height;
    let (_, bottom, _) = edges(&view, text(UiText::TryAgain));
    assert!(
        bottom <= height,
        "Try again inside the window: {bottom} > {height}"
    );
    // The smallest window: the text scrolls, the actions stay in view.
    window.set_size(slint::LogicalSize::new(320.0, 200.0));
    slint::platform::update_timers_and_animations();
    for label in [text(UiText::TryAgain), text(UiText::Close)] {
        let (_, bottom, right) = edges(&view, label);
        assert!(
            bottom <= 200.0 && right <= 320.0,
            "{label:?} in view at 320 x 200: {bottom}, {right}"
        );
    }

    window.set_size(slint::LogicalSize::new(520.0, 1200.0));
    slint::platform::update_timers_and_animations();
    let path = the(&view, text(UiText::RefusalPath));
    let reason = edges(&view, text(UiText::RefusalStoreRecovery));
    let gap = path.absolute_position().y - reason.1;
    assert!(
        gap < 64.0,
        "the spare height is not put between the reason and the path: {gap}"
    );
}

#[test]
fn close_alone_sits_at_the_edge_try_again_would_take() {
    let with = screen(&store_page());
    let (_, _, retry_right) = edges(&with, text(UiText::TryAgain));
    let without = screen(&closed_page());
    let (_, _, close_right) = edges(&without, text(UiText::Close));
    assert!(
        (close_right - retry_right).abs() < 1.0,
        "Close takes the end of the row when alone: {close_right} vs {retry_right}"
    );
}

fn discard_page() -> RefusalPage {
    RefusalPage {
        can_discard: true,
        ..store_page()
    }
}

/// A flow at the confirmation of `kind`.
fn confirming(kind: interweave_human_ui_model::DiscardKind, identities: usize) -> StoreRecovery {
    let mut flow = StoreRecovery::new(StoreScreen::Recovery, identities);
    flow.press_discard();
    flow.planned(kind);
    flow
}

/// Delete stored data… is on the store's recovery screen, at the start
/// side on a line of its own, and never the initial focus: Try again is
/// (design 2026-10-21 section 7.1). All three actions stay inside the
/// narrowest window.
#[test]
fn the_recovery_screen_offers_delete_stored_data_apart_and_try_again_keeps_focus() {
    let view = screen(&discard_page());
    let discard = the(&view, text(UiText::DiscardStore));
    assert_eq!(
        discard.accessible_role(),
        Some(i_slint_backend_testing::AccessibleRole::Button)
    );
    assert_eq!(focus(&view), "retry");
    let close = the(&view, text(UiText::Close));
    assert!(
        discard.absolute_position().x <= close.absolute_position().x
            && discard.absolute_position().y + discard.size().height <= close.absolute_position().y,
        "apart, at the start side, on a line above Close"
    );
    assert!(labelled(&screen(&store_page()), text(UiText::DiscardStore)).is_empty());
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(320.0, 600.0));
    slint::platform::update_timers_and_animations();
    for label in [
        text(UiText::DiscardStore),
        text(UiText::Close),
        text(UiText::TryAgain),
    ] {
        let at = the(&view, label);
        let right = at.absolute_position().x + at.size().width;
        assert!(
            right <= 320.0 && at.absolute_position().x >= 0.0,
            "{label:?} at {right}"
        );
    }
}

/// The confirmation lists what goes one item per line, opens with focus
/// on Cancel so Enter alone never discards, keeps Tab on its two buttons,
/// and Escape is Cancel (human-client-ui.md §13; design section 7.2).
#[test]
fn the_full_confirmation_lists_every_loss_and_cancel_has_focus() {
    use interweave_human_ui_model::DiscardKind;
    use std::cell::Cell;
    use std::rc::Rc;
    let view = screen(&discard_page());
    let cancelled = Rc::new(Cell::new(0));
    let confirmed = Rc::new(Cell::new(0));
    {
        let c = Rc::clone(&cancelled);
        view.on_dialog_cancel(move || c.set(c.get() + 1));
        let c = Rc::clone(&confirmed);
        view.on_dialog_confirm(move || c.set(c.get() + 1));
    }
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(520.0, 900.0));
    let flow = confirming(DiscardKind::Full, 1);
    view.show_store_recovery(&flow);
    slint::platform::update_timers_and_animations();
    the(&view, text(UiText::DiscardStoreTitle));
    for item in flow.dialog().expect("open").items {
        let row = the(&view, text(item));
        assert_eq!(
            row.accessible_role(),
            Some(i_slint_backend_testing::AccessibleRole::ListItem),
            "{item:?}"
        );
    }
    the(&view, text(UiText::DiscardStoreKeepsOne));
    the(&view, text(UiText::DiscardStoreRemains));
    // A focus change reports itself at the loop's next turn.
    slint::platform::update_timers_and_animations();
    assert_eq!(focus(&view), "dialog-cancel", "Cancel is the initial focus");
    key(&view, Key::Return);
    assert_eq!(
        (cancelled.get(), confirmed.get()),
        (1, 0),
        "Enter alone cancels"
    );
    key(&view, Key::Tab);
    assert_eq!(focus(&view), "dialog-confirm");
    key(&view, Key::Tab);
    assert_eq!(focus(&view), "dialog-cancel", "Tab stays in the dialog");
    key(&view, Key::Escape);
    assert_eq!(cancelled.get(), 2, "Escape is Cancel");
    the(&view, text(UiText::DiscardStoreConfirm)).invoke_accessible_default_action();
    assert_eq!(confirmed.get(), 1);
}

#[test]
fn the_partial_confirmation_names_the_unfinished_file_only() {
    use interweave_human_ui_model::DiscardKind;
    let view = screen(&discard_page());
    view.show_store_recovery(&confirming(DiscardKind::Partial, 2));
    slint::platform::update_timers_and_animations();
    the(&view, text(UiText::DiscardPartialTitle));
    the(&view, text(UiText::DiscardPartialBody));
    the(&view, text(UiText::DiscardPartialConfirm));
    assert!(labelled(&view, text(UiText::DiscardLosesContacts)).is_empty());
    assert!(
        !view.window().get_dialog_danger(),
        "nothing the person has is lost"
    );
}

/// While the plan is built and while the discard runs, every action is
/// inert and Escape does not close the screen; the line under the heading
/// says what is under way, and after a failure, that it failed.
#[test]
fn while_checking_or_deleting_every_action_is_inert() {
    use interweave_human_ui_model::DiscardKind;
    use std::cell::Cell;
    use std::rc::Rc;
    let view = screen(&discard_page());
    let pressed = Rc::new(Cell::new(0));
    {
        let p = Rc::clone(&pressed);
        view.on_discard(move || p.set(p.get() + 1));
        let p = Rc::clone(&pressed);
        view.on_retry(move || p.set(p.get() + 1));
    }
    let mut flow = StoreRecovery::new(StoreScreen::Recovery, 1);
    flow.press_discard();
    view.show_store_recovery(&flow);
    slint::platform::update_timers_and_animations();
    the(&view, text(UiText::StoreChecking));
    the(&view, text(UiText::DiscardStore)).invoke_accessible_default_action();
    the(&view, text(UiText::TryAgain)).invoke_accessible_default_action();
    key(&view, Key::Escape);
    assert_eq!(pressed.get(), 0, "inert");
    assert!(
        view.window().window().is_visible(),
        "Escape did not close it"
    );
    flow.planned(DiscardKind::Full);
    flow.confirm();
    view.show_store_recovery(&flow);
    slint::platform::update_timers_and_animations();
    the(&view, text(UiText::StoreDiscarding));
    flow.failed();
    view.show_store_recovery(&flow);
    slint::platform::update_timers_and_animations();
    the(&view, text(UiText::StoreDiscardFailed));
    the(&view, text(UiText::DiscardStore)).invoke_accessible_default_action();
    assert_eq!(pressed.get(), 1, "live again after the failure");
}

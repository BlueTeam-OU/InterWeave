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
use interweave_human_ui_model::{UiText, placeholder_en};
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
    }
}

fn closed_page() -> RefusalPage {
    RefusalPage {
        reason: UiText::RefusalAlreadyOpen,
        path: None,
        can_retry: false,
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

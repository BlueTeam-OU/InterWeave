// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What more than one of these test binaries reads of a drawn window.

// Each test binary compiles this module whole and uses its own part.
#![allow(dead_code)]

use i_slint_backend_testing::{ElementHandle, ElementQuery, ElementRoot};

/// How far a focus ring reaches outside its control, as app.slint's
/// `FocusMode.ring-reach` says and each `FocusRing` is placed;
/// `ring_reach_is_where_the_ring_is_drawn` in cards.rs holds this, the
/// global and one drawn ring (a `FocusLine`'s) equal.
pub(crate) const REACH: f32 = 5.0;

/// An element's box in the window: left, top, right, bottom.
pub(crate) struct Edges {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) right: f32,
    pub(crate) bottom: f32,
}

impl Edges {
    pub(crate) fn of(e: &ElementHandle) -> Self {
        let (at, size) = (e.absolute_position(), e.size());
        Self {
            left: at.x,
            top: at.y,
            right: at.x + size.width,
            bottom: at.y + size.height,
        }
    }

    fn centre(&self) -> (f32, f32) {
        (
            f32::midpoint(self.left, self.right),
            f32::midpoint(self.top, self.bottom),
        )
    }

    fn near(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
    }
}

/// The one focus ring drawn, if any: it shows only for keyboard focus.
pub(crate) fn shown_ring(window: &impl ElementRoot) -> Option<ElementHandle> {
    ElementQuery::from_root(window)
        .find_all()
        .into_iter()
        .find(|e| e.type_name().as_deref() == Some("FocusRing") && e.computed_opacity() > 0.0)
}

/// The drawn text within the shown focus ring's reach of its control: a
/// ring 5 px outside the control draws over a neighbour laid out nearer
/// than that (the own card's fingerprint under its label at 2 px, A40
/// 2026-11-03). Text inside the control's box is its own. `None` when no
/// ring is shown, so a walk cannot pass a stop it never checked.
pub(crate) fn ring_overdraws(window: &impl ElementRoot) -> Option<Vec<String>> {
    let ring = Edges::of(&shown_ring(window)?);
    let elements = ElementQuery::from_root(window).find_all();
    // A text for a screen reader only is clipped to a 1 px point, which
    // centres it there (app.slint's announcement slots, the Messages
    // heading): it is said and never seen.
    let points: Vec<_> = elements
        .iter()
        .filter(|e| Edges::near((e.size().width, e.size().height), (1.0, 1.0)))
        .map(|e| Edges::of(e).centre())
        .collect();
    let over = elements
        .iter()
        .filter(|e| e.type_name().as_deref() == Some("Text") && e.computed_opacity() > 0.0)
        .filter_map(|e| {
            let text = Edges::of(e);
            let said_only = points.iter().any(|p| Edges::near(*p, text.centre()));
            let drawn = text.right > text.left && text.bottom > text.top;
            let touches = text.left < ring.right
                && text.right > ring.left
                && text.top < ring.bottom
                && text.bottom > ring.top;
            let own = text.left >= ring.left + REACH
                && text.right <= ring.right - REACH
                && text.top >= ring.top + REACH
                && text.bottom <= ring.bottom - REACH;
            (!said_only && drawn && touches && !own).then(|| {
                format!(
                    "{:?} at ({}, {})-({}, {}) under the ring ({}, {})-({}, {})",
                    e.accessible_label().or_else(|| e.id()),
                    text.left,
                    text.top,
                    text.right,
                    text.bottom,
                    ring.left,
                    ring.top,
                    ring.right,
                    ring.bottom,
                )
            })
        })
        .collect();
    Some(over)
}

/// Each scroll area holding the shown ring's control that cuts the ring:
/// a `Flickable` clips, so a control in one keeps a ring's reach of room
/// inside it, or its ring is drawn without the sides that fall outside
/// (the refusal screen's path, seen rendered in j83). A scroll area holds
/// the control when the ring is among its descendants (each `FocusRing`
/// sits beside its control), so an overlay drawn over another view's list
/// is not judged by that list (#547 F2). `None` when no ring is shown, as
/// for [`ring_overdraws`].
pub(crate) fn ring_cut(window: &impl ElementRoot) -> Option<Vec<String>> {
    let ring = Edges::of(&shown_ring(window)?);
    let shown = |e: &ElementHandle| {
        e.type_name().as_deref() == Some("FocusRing") && e.computed_opacity() > 0.0
    };
    let cut = ElementQuery::from_root(window)
        .find_all()
        .into_iter()
        .filter(|e| e.type_name().as_deref() == Some("Flickable"))
        .filter(|e| {
            e.query_descendants()
                .match_predicate(shown)
                .find_first()
                .is_some()
        })
        .filter_map(|e| {
            let area = Edges::of(&e);
            let inside = ring.left >= area.left
                && ring.right <= area.right
                && ring.top >= area.top
                && ring.bottom <= area.bottom;
            (!inside).then(|| {
                format!(
                    "{:?} ({}, {})-({}, {}) cuts the ring ({}, {})-({}, {})",
                    e.id(),
                    area.left,
                    area.top,
                    area.right,
                    area.bottom,
                    ring.left,
                    ring.top,
                    ring.right,
                    ring.bottom,
                )
            })
        })
        .collect();
    Some(cut)
}

/// Ctrl+Tab, pressed and released: a Tab that moves no focus, in Slint's
/// navigation and in every modal's own (`FocusMode.navigates`).
pub(crate) fn ctrl_tab(window: &slint::Window) {
    use slint::platform::{Key, WindowEvent};
    let (ctrl, tab): (slint::SharedString, slint::SharedString) =
        (Key::Control.into(), Key::Tab.into());
    window.dispatch_event(WindowEvent::KeyPressed { text: ctrl.clone() });
    window.dispatch_event(WindowEvent::KeyPressed { text: tab.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text: tab });
    window.dispatch_event(WindowEvent::KeyReleased { text: ctrl });
    slint::platform::update_timers_and_animations();
}

/// In a modal that took focus by a call (no ring yet), Ctrl+Tab moves no
/// focus and draws no ring, and Tab moves focus and draws it: the modal's
/// own Tab handler and the window's `KeyboardScope` ask one rule
/// (`FocusMode.navigates`, #547 F1). Returns where Tab moved focus.
pub(crate) fn modal_tab(
    root: &impl ElementRoot,
    window: &slint::Window,
    focus: impl Fn() -> String,
    settle: impl Fn(),
) -> String {
    let at = focus();
    assert!(shown_ring(root).is_none(), "control: no ring at {at:?} yet");
    ctrl_tab(window);
    settle();
    assert_eq!(focus(), at, "Ctrl+Tab moves no focus");
    assert!(shown_ring(root).is_none(), "Ctrl+Tab draws no ring");
    let tab: slint::SharedString = slint::platform::Key::Tab.into();
    window.dispatch_event(slint::platform::WindowEvent::KeyPressed { text: tab.clone() });
    window.dispatch_event(slint::platform::WindowEvent::KeyReleased { text: tab });
    settle();
    let to = focus();
    assert_ne!(to, at, "Tab moves focus");
    assert!(shown_ring(root).is_some(), "Tab draws the ring at {to:?}");
    to
}

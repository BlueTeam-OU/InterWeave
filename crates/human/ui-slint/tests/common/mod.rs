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

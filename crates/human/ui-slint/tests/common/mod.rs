// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What more than one of these test binaries reads of a drawn window.

use i_slint_backend_testing::{ElementHandle, ElementQuery, ElementRoot};

/// The ring's stroke (`FocusRing`'s `border-width` in app.slint).
const STROKE: f32 = 3.0;

/// An element's box in the window: left, top, right, bottom.
struct Edges {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Edges {
    fn of(e: &ElementHandle) -> Self {
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

/// The drawn text the shown focus ring partly covers: a 3 px stroke 2 to
/// 5 px outside its control lands on a neighbour laid out nearer than
/// that (the own card's fingerprint under its label at 2 px, A40
/// 2026-11-03). Text wholly inside the ring is the control's own; with no
/// ring shown, nothing.
pub(crate) fn ring_overdraws(window: &impl ElementRoot) -> Vec<String> {
    let elements = ElementQuery::from_root(window).find_all();
    let Some(ring) = elements
        .iter()
        .find(|e| e.type_name().as_deref() == Some("FocusRing") && e.computed_opacity() > 0.0)
    else {
        return Vec::new();
    };
    let ring = Edges::of(ring);
    // A text for a screen reader only is clipped to a 1 px point, which
    // centres it there (app.slint's announcement slots, the Messages
    // heading): it is said and never seen.
    let points: Vec<_> = elements
        .iter()
        .filter(|e| Edges::near((e.size().width, e.size().height), (1.0, 1.0)))
        .map(|e| Edges::of(e).centre())
        .collect();
    elements
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
            let inside = text.left >= ring.left + STROKE
                && text.right <= ring.right - STROKE
                && text.top >= ring.top + STROKE
                && text.bottom <= ring.bottom - STROKE;
            (!said_only && drawn && touches && !inside).then(|| {
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
        .collect()
}

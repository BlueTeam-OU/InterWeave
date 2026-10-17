// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android look (design/2026-10-16-android-look.md) through the
//! compiled views: its colour roles, measured from the palette the window
//! draws with in both schemes.
//!
//! What this cannot prove: that a screenshot on the phone measures the
//! same (section 11 item 1 is a person's check there), or that every
//! element takes the role the section gives it -- the views tests hold
//! the places that matter.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use interweave_human_ui_model::RuntimeHost;
use interweave_human_ui_slint::{Palette, View};
use slint::{Color, ComponentHandle as _};

/// WCAG 2.2's relative luminance of an sRGB colour.
fn luminance(c: Color) -> f64 {
    let channel = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(c.red()) + 0.7152 * channel(c.green()) + 0.0722 * channel(c.blue())
}

/// WCAG 2.2's contrast ratio.
fn contrast(a: Color, b: Color) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Every pair section 3's contrast table names, with its need: the
/// foreground's role, the background's, and the least ratio.
fn pairs(p: &Palette<'_>) -> Vec<(&'static str, Color, Color, f64)> {
    let white = Color::from_rgb_u8(0xff, 0xff, 0xff);
    let neutral_ink = if p.get_dark() {
        p.get_swipe_on_neutral()
    } else {
        white
    };
    vec![
        (
            "on-surface on surface",
            p.get_on_surface(),
            p.get_surface(),
            4.5,
        ),
        (
            "on-surface-variant on surface",
            p.get_on_surface_variant(),
            p.get_surface(),
            4.5,
        ),
        (
            "on-surface on surface-container-high",
            p.get_on_surface(),
            p.get_surface_container_high(),
            4.5,
        ),
        (
            "on-surface-variant on surface-container-high",
            p.get_on_surface_variant(),
            p.get_surface_container_high(),
            4.5,
        ),
        (
            "on-surface-variant on surface-container",
            p.get_on_surface_variant(),
            p.get_surface_container(),
            4.5,
        ),
        (
            "on-surface-variant on primary-container",
            p.get_on_surface_variant(),
            p.get_primary_container(),
            4.5,
        ),
        (
            "on-surface on primary-container",
            p.get_on_surface(),
            p.get_primary_container(),
            4.5,
        ),
        (
            "error on primary-container",
            p.get_error(),
            p.get_primary_container(),
            4.5,
        ),
        (
            "on-primary-container on primary-container",
            p.get_on_primary_container(),
            p.get_primary_container(),
            4.5,
        ),
        (
            "on-primary on primary",
            p.get_on_primary(),
            p.get_primary(),
            4.5,
        ),
        ("primary on surface", p.get_primary(), p.get_surface(), 4.5),
        (
            "primary on surface-container-high",
            p.get_primary(),
            p.get_surface_container_high(),
            4.5,
        ),
        (
            "on-surface code on surface",
            p.get_on_surface(),
            p.get_surface(),
            4.5,
        ),
        (
            "primary ring on primary-container",
            p.get_primary(),
            p.get_primary_container(),
            3.0,
        ),
        (
            "primary ring on surface-container",
            p.get_primary(),
            p.get_surface_container(),
            3.0,
        ),
        (
            "focus-ring-on-inverse on inverse-surface",
            p.get_focus_ring_on_inverse(),
            p.get_inverse_surface(),
            3.0,
        ),
        (
            "focus-ring-on-notice on notice-background",
            p.get_focus_ring_on_notice(),
            p.get_notice_background(),
            3.0,
        ),
        (
            "on-tertiary on tertiary",
            p.get_on_tertiary(),
            p.get_tertiary(),
            4.5,
        ),
        (
            "unread-dot on surface-container-high",
            p.get_unread_dot(),
            p.get_surface_container_high(),
            3.0,
        ),
        (
            "unread-dot on primary-container",
            p.get_unread_dot(),
            p.get_primary_container(),
            3.0,
        ),
        (
            "inverse-on-surface on inverse-surface",
            p.get_inverse_on_surface(),
            p.get_inverse_surface(),
            4.5,
        ),
        (
            "inverse-primary on inverse-surface",
            p.get_inverse_primary(),
            p.get_inverse_surface(),
            4.5,
        ),
        ("error on surface", p.get_error(), p.get_surface(), 4.5),
        ("outline on surface", p.get_outline(), p.get_surface(), 3.0),
        (
            "swipe-neutral on surface",
            p.get_swipe_neutral(),
            p.get_surface(),
            3.0,
        ),
        (
            "swipe-neutral on surface-container",
            p.get_swipe_neutral(),
            p.get_surface_container(),
            3.0,
        ),
        (
            "swipe-neutral on surface-container-high",
            p.get_swipe_neutral(),
            p.get_surface_container_high(),
            3.0,
        ),
        (
            "swipe-neutral on primary-container",
            p.get_swipe_neutral(),
            p.get_primary_container(),
            3.0,
        ),
        (
            "neutral ink on swipe-neutral",
            neutral_ink,
            p.get_swipe_neutral(),
            4.5,
        ),
        (
            "swipe-on-fill on swipe-keep",
            p.get_swipe_on_fill(),
            p.get_swipe_keep(),
            4.5,
        ),
        (
            "swipe-on-fill on swipe-remove",
            p.get_swipe_on_fill(),
            p.get_swipe_remove(),
            4.5,
        ),
        (
            "swipe-on-fill on swipe-retry",
            p.get_swipe_on_fill(),
            p.get_swipe_retry(),
            4.5,
        ),
        (
            "notice-text on notice-background",
            p.get_notice_text(),
            p.get_notice_background(),
            4.5,
        ),
        (
            "avatar teal",
            p.get_on_avatar_teal(),
            p.get_avatar_teal(),
            4.5,
        ),
        (
            "avatar amber",
            p.get_on_avatar_amber(),
            p.get_avatar_amber(),
            4.5,
        ),
        (
            "avatar navy",
            p.get_on_avatar_navy(),
            p.get_avatar_navy(),
            4.5,
        ),
    ]
}

#[test]
fn every_contrast_pair_of_section_3_meets_its_need() {
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Daemon).expect("a view");
    let palette = view.window().global::<Palette<'_>>();
    let mut failed = Vec::new();
    for scheme in [
        slint::language::ColorScheme::Dark,
        slint::language::ColorScheme::Light,
    ] {
        palette.set_scheme(scheme);
        assert_eq!(
            palette.get_dark(),
            scheme == slint::language::ColorScheme::Dark
        );
        for (name, fg, bg, need) in pairs(&palette) {
            let ratio = contrast(fg, bg);
            if ratio < need {
                failed.push(format!("{scheme:?} {name}: {ratio:.2} < {need}"));
            }
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

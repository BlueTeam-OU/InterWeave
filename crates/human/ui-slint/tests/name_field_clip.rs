// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A name at the cluster bound in a contact's name field stays inside
//! the field's input, drawn by the desktop's own window and software
//! renderer. The field (`NameField` in `app.slint`) sits in a Rectangle
//! that does not clip, so its ink is bounded by the `TextInput` alone:
//! Slint 1.18.1 clips a `TextInput` to its own box whatever its text
//! (i-slint-core `textlayout/sharedparley.rs`, `draw_text_input`), where
//! an elided `Text` is clipped only under `overflow: clip` and drew 31
//! stacked marks through the row above it (`one_line_clip.rs`). The
//! contact's page is drawn twice, its name field holding a plain letter
//! and then that character, put there as the page prefills it from the
//! contact's name (`drawable`, `set_contact_name_draft`), and every pixel
//! that differs lies within the input's own rows. One test in this
//! binary, since a process holds one window's event loop.
//!
//! What this cannot prove: the phone's renderer (Skia, the same shared
//! text drawing), or a name the person types, which the field's `edited`
//! handler passes through the same `drawable`.

#![cfg(feature = "desktop")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use i_slint_backend_testing::ElementHandle;
use interweave_human_ui_model::{
    ContactView, Intent, RouteView, RuntimeHost, UiModel, UiText, cluster, isolated, placeholder_en,
};
use interweave_human_ui_slint::{View, ViewEvent};
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::ComponentHandle as _;

const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

fn contact(name: &str) -> ContactView {
    ContactView {
        contact_id: "c1".to_owned(),
        name: name.to_owned(),
        name_from_card: true,
        name_at: Some(1_792_843_200_000),
        photo_from_card: None,
        photo_at: None,
        photo: None,
        routes: vec![RouteView {
            peer: TransportIdentity::parse(P).expect("a PeerId"),
            endpoint: EndpointId::parse("human").expect("an endpoint"),
            fingerprint: None,
        }],
    }
}

/// The root's loop, for what opening a page asks: take, apply, render.
fn settle(view: &mut View, model: &mut UiModel) {
    loop {
        let events = view.take_events(model);
        if events.is_empty() {
            view.render(model);
            slint::platform::update_timers_and_animations();
            return;
        }
        for event in events {
            if let ViewEvent::Intent(Intent::MarkRead(row)) = event {
                model.read(row);
            }
        }
        view.render(model);
    }
}

fn press(view: &mut View, model: &mut UiModel, label: &str) {
    let found: Vec<_> = ElementHandle::find_by_accessible_label(view.window(), label).collect();
    assert!(!found.is_empty(), "a control labelled {label:?}");
    found[0].invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
    settle(view, model);
}

fn snapshot(view: &View) -> slint::SharedPixelBuffer<slint::Rgba8Pixel> {
    slint::platform::update_timers_and_animations();
    view.window()
        .window()
        .take_snapshot()
        .expect("the page drawn")
}

#[test]
fn a_name_at_the_cluster_bound_draws_inside_its_field() {
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 700.0));
    view.window().show().expect("shown");
    let mut model = UiModel::new();
    model.contacts_listed(vec![contact("a")]);
    view.render(&model);
    press(
        &mut view,
        &mut model,
        placeholder_en::text(UiText::AppOptions),
    );
    press(
        &mut view,
        &mut model,
        placeholder_en::text(UiText::ContactsTitle),
    );
    // The row as a screen reader reads it.
    let spoken = cluster::spoken(
        &isolated("a"),
        placeholder_en::text(UiText::ReplacedCharacter),
    )
    .into_owned();
    press(&mut view, &mut model, &spoken);
    assert_eq!(view.window().get_contact_name_draft(), "a");
    let label = view.window().get_name_label();
    let input = ElementHandle::find_by_accessible_label(view.window(), &label)
        .find(|e| e.accessible_value().is_some())
        .expect("the name field's input");
    let plain = snapshot(&view);

    // As the page prefills it from a card's name.
    let bound = format!("a{}", "\u{301}".repeat(31));
    view.window()
        .set_contact_name_draft(cluster::drawable(&bound).as_ref().into());
    let at_bound = snapshot(&view);
    assert_eq!(
        (plain.width(), plain.height()),
        (at_bound.width(), at_bound.height())
    );

    let scale = view.window().window().scale_factor();
    let top = input.absolute_position().y * scale;
    let bottom = top + input.size().height * scale;
    let width = usize::try_from(plain.width()).expect("fits");
    let rows: Vec<usize> = plain
        .as_slice()
        .chunks(width)
        .zip(at_bound.as_slice().chunks(width))
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(y, _)| y)
        .collect();
    let (Some(&first), Some(&last)) = (rows.first(), rows.last()) else {
        panic!("the character drew nothing the letter did not");
    };
    let row = |y: usize| f32::from(u16::try_from(y).expect("a window's height"));
    // The input's rows, rounded out by a pixel for antialiasing at its
    // edges. At scale 1 the character changed rows 236..=242 of an input
    // at 235.5..254.5; drawn as well by an unclipped elided Text in the
    // same box, its marks ran from row 146, 90 px above the field (both
    // measured 2026-11-13).
    assert!(
        row(first) >= top.floor() - 1.0 && row(last) <= bottom.ceil() + 1.0,
        "the at-bound name changed pixel rows {first}..={last}, outside the input's {top}..{bottom}"
    );
}

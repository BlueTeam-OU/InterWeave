// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A character at the cluster bound in a one-line label stays inside the
//! label's box, drawn by the desktop's own window and software renderer.
//! Slint 1.18.1 clips a Text to its box only under `overflow: clip`, and
//! an elided line drew a character of one letter and 31 stacked marks
//! about 280 px above a Chats row on the Galaxy A40
//! (tests/android-e2e/runs/2026-11-13-samsung-a40-cluster-bound.md).
//! The Chats list is drawn twice, its one row's preview a plain letter
//! and then that character, and every pixel that differs lies within one
//! line's height: the marks drew nowhere above the preview's line. One
//! test in this binary, since a process holds one window's event loop.
//!
//! What this cannot prove: the phone's renderer (Skia, the same shared
//! text drawing, `cluster_bound.rs` a device case), and the other
//! one-line labels, which `compile.rs` holds to the same clipping box.

#![cfg(feature = "desktop")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::{RuntimeHost, UiModel};
use interweave_human_ui_slint::View;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::ComponentHandle as _;

fn received(row: i64, from: &TransportIdentity, text: &str) -> Received {
    Received {
        row: RowId::from_stored(row),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: EndpointId::parse("human").expect("valid"),
        },
        envelope: HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{:032x}", row + 1_000),
            text: text.to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: u64::try_from(row).expect("positive"),
    }
}

/// The Chats list drawn once message `row`, whose text is `text`, is
/// received: its one row's preview is that text.
fn drawn(
    view: &mut View,
    model: &mut UiModel,
    from: &TransportIdentity,
    row: i64,
    text: &str,
) -> slint::SharedPixelBuffer<slint::Rgba8Pixel> {
    model.received(received(row, from, text));
    view.render(model);
    slint::platform::update_timers_and_animations();
    view.window()
        .window()
        .take_snapshot()
        .expect("the list drawn")
}

#[test]
fn a_character_at_the_cluster_bound_draws_inside_its_row() {
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    view.window().show().expect("shown");
    let from = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let mut model = UiModel::new();
    let plain = drawn(&mut view, &mut model, &from, 1, "a");
    // The same row, its preview now the newer message and its unread
    // count one more: both inside the row.
    let bound = format!("a{}", "\u{301}".repeat(31));
    let at_bound = drawn(&mut view, &mut model, &from, 2, &bound);
    assert_eq!(
        (plain.width(), plain.height()),
        (at_bound.width(), at_bound.height())
    );
    let width = usize::try_from(plain.width()).expect("fits");
    let rows: Vec<usize> = plain
        .as_slice()
        .chunks(width)
        .zip(at_bound.as_slice().chunks(width))
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(y, _)| y)
        .collect();
    let (first, last) = (rows.first().copied(), rows.last().copied());
    let (Some(first), Some(last)) = (first, last) else {
        panic!("the character drew nothing the letter did not");
    };
    // Clipped, the character and the badge's 1 to 2 change only the
    // preview's own line, 12 px measured at scale 1; unclipped, the marks
    // ran 49 px, up through the name above (both measured 2026-11-13).
    // A preview line of body-medium is under 24 px.
    let line = 24.0 * view.window().window().scale_factor();
    let span = last - first + 1;
    let high = f32::from(u16::try_from(span).expect("a window's height"));
    assert!(
        high < line,
        "the at-bound character changed pixel rows {first}..={last}, {span} high, past one line of {line}"
    );
}

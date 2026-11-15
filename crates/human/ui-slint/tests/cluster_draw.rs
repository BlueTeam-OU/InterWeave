// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A character past the cluster bound, drawn by the desktop's own window
//! and software renderer, whose text shaper counts a character's code
//! points in eight bits (HUMAN-CHAT.md, Bounds; j92). The testing backend
//! shapes no text, so only a real window can show that the bound keeps
//! the shaper's counter in range: `take_snapshot` lays out and draws a
//! frame as the event loop does. One test in this binary, since a process
//! holds one window's event loop.
//!
//! What this cannot prove: the phone, whose window and renderer are its
//! own (`tests/android-e2e/tests/cluster_bound.rs`, a device case). The
//! other scripts are `cluster_scripts.rs`'s, Show source `source_draw.rs`'s.

#![cfg(feature = "desktop")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::{ConversationKey, RuntimeHost, UiModel};
use interweave_human_ui_slint::View;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::ComponentHandle as _;

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn received(row: i64, from: &TransportIdentity, text: &str) -> Received {
    Received {
        row: RowId::from_stored(row),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: human(),
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

/// A letter and 255 combining marks, one character of 256 code points,
/// in a body and so in the Chats line: drawn as one U+FFFD, the window
/// draws the conversation and the list beside it.
#[test]
fn a_character_past_the_cluster_bound_draws_in_a_real_window() {
    let view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    view.window().show().expect("shown");
    let from = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let past = format!("a{}", "\u{301}".repeat(255));
    let mut model = UiModel::new();
    model.received(received(
        1,
        &from,
        &format!("first {past} line\n\n```\n{past}\n```"),
    ));
    let mut view = view;
    view.render(&model);
    // The Chats line first, then the conversation beside it.
    let _ = view
        .window()
        .window()
        .take_snapshot()
        .expect("the list drawn");
    view.select(ConversationKey::Direct {
        peer: from,
        endpoint: Some(human()),
    });
    let _ = view.take_events(&model);
    view.render(&model);
    slint::platform::update_timers_and_animations();
    let _ = view
        .window()
        .window()
        .take_snapshot()
        .expect("the conversation drawn");
}

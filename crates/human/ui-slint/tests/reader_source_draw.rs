// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The reader's Show source with a character past the cluster bound,
//! drawn by the desktop's own window and software renderer, whose text
//! shaper counts a character's code points in eight bits (HUMAN-CHAT.md,
//! the raw source's bullet: "Show source draws a replaced cluster as
//! the body does").
//! `long_message.rs` reaches the reader's source on the testing backend,
//! which shapes no text; `source_draw.rs` draws a row's source in a real
//! window. Here a long body opens in the reader, its Show source is
//! pressed, and the source drawn, the test finding the drawn source's
//! text by the label it speaks.
//!
//! The reader's source has its own replacement (`reader::source`), apart
//! from the row's. With it taken out the shaper overflows under the
//! overflow checks this test runs with (parley's `attempt to add with
//! overflow`, measured 2026-11-16) as Show source is pressed: the source
//! form is built then and its pieces measured with the code font, which
//! shapes the character before anything draws it. In the app the form is
//! built after the first frame that draws the formatted reader; a
//! snapshot tells no frame, so here the press builds it.
//! One test in this binary, since a process holds one window's event
//! loop.
//!
//! What this cannot prove: the phone's renderer
//! (`tests/android-e2e/tests/cluster_bound.rs`, which did not open the
//! reader).

#![cfg(feature = "desktop")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementQuery};
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::{
    ConversationKey, Intent, RuntimeHost, UiModel, UiText, placeholder_en,
};
use interweave_human_ui_slint::{View, ViewEvent};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::{ComponentHandle as _, Model as _};
use unicode_segmentation::UnicodeSegmentation as _;

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

/// What the drawn texts speak, the reader's source among them once it is
/// shown.
fn spoken_texts(view: &View) -> Vec<String> {
    ElementQuery::from_root(view.window())
        .match_accessible_role(AccessibleRole::Text)
        .find_all()
        .into_iter()
        .filter_map(|e| e.accessible_label().map(|l| l.to_string()))
        .collect()
}

/// One turn of the root's loop, then a frame: what the view hands back
/// is taken, the model told of reads, the timers run and the window
/// drawn.
fn turn(view: &mut View, model: &mut UiModel, what: &str) {
    loop {
        let events = view.take_events(model);
        if events.is_empty() {
            break;
        }
        for event in events {
            if let ViewEvent::Intent(Intent::MarkRead(row)) = event {
                model.read(row);
            }
        }
    }
    view.render(model);
    slint::platform::update_timers_and_animations();
    let _ = view.window().window().take_snapshot().expect(what);
}

/// Turns until `done` holds, on the real clock the window's timers run
/// on; a panic naming `what` after ten seconds.
fn until(view: &mut View, model: &mut UiModel, what: &str, done: impl Fn(&View) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        turn(view, model, what);
        if done(view) {
            return;
        }
        assert!(Instant::now() < deadline, "{what}: not within ten seconds");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn the(view: &View, label: &str) -> ElementHandle {
    let mut found: Vec<_> = ElementHandle::find_by_accessible_label(view.window(), label).collect();
    assert_eq!(found.len(), 1, "exactly one element labelled {label:?}");
    found.remove(0)
}

/// A letter and 255 combining marks in a body long enough to be cut:
/// the reader opened on it and switched to its source draws the source
/// with the character as one U+FFFD.
#[test]
fn the_readers_show_source_draws_a_character_past_the_cluster_bound_in_a_real_window() {
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    view.window().show().expect("shown");
    let from = ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id");
    let past = format!("a{}", "\u{301}".repeat(255));
    assert_eq!(
        past.graphemes(true)
            .map(|c| c.chars().count())
            .collect::<Vec<_>>(),
        [256],
        "the premise: one character of 256 code points by UAX #29"
    );
    // Sixty paragraphs pass the conversation's 40 lines, so the row is
    // cut and the reader is the way to the whole.
    let paragraphs = (1..=60)
        .map(|i| format!("Paragraph {i}."))
        .collect::<Vec<_>>()
        .join("\n\n");
    let source = format!("x {past} y **bold**\n\n{paragraphs}");
    let mut model = UiModel::new();
    model.received(received(1, &from, &source));
    view.render(&model);
    view.select(ConversationKey::Direct {
        peer: from,
        endpoint: Some(human()),
    });
    turn(&mut view, &mut model, "the conversation drawn");

    the(&view, placeholder_en::text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    until(&mut view, &mut model, "the reader drawn", |view| {
        view.window().get_reader_open() && view.window().get_reader_rows().row_count() > 1
    });
    assert!(
        !view.window().get_reader_as_source(),
        "control: the reader opens formatted"
    );
    let replaced = placeholder_en::text(UiText::ReplacedCharacter);
    // The source's first piece, as the reader's source row would speak it:
    // the markup is what tells it from the formatted line.
    let first_piece = source.replace(&past, "\u{FFFD}");
    let first_piece = first_piece.split("\n\n").next().expect("a first paragraph");
    let first_spoken = interweave_human_ui_model::cluster::spoken(first_piece, replaced);
    let before = spoken_texts(&view);
    assert!(
        !before.iter().any(|s| s.contains(&*first_spoken)),
        "the source is not drawn before Show source is pressed: {before:?}"
    );

    the(&view, placeholder_en::text(UiText::ShowSource)).invoke_accessible_default_action();
    until(&mut view, &mut model, "the source drawn", |view| {
        view.window().get_reader_as_source()
    });

    let rows = view.window().get_reader_rows();
    let texts: Vec<String> = (1..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .map(|r| r.text.to_string())
        .collect();
    // The rows split the source at paragraph breaks, a break's newline
    // standing between two rows rather than in either, so the text is
    // compared without its newlines.
    let flat = |s: &str| s.replace('\n', "");
    assert_eq!(
        flat(&texts.concat()),
        flat(&source.replace(&past, "\u{FFFD}")),
        "the source is shown whole, the character as one U+FFFD"
    );
    assert!(
        !texts.iter().any(|t| t.contains(&past)),
        "no row holds the character itself"
    );
    let after = spoken_texts(&view);
    let shown = texts
        .iter()
        .find(|t| t.contains('\u{FFFD}'))
        .expect("the piece with the character");
    let shown_spoken = interweave_human_ui_model::cluster::spoken(shown, replaced);
    assert!(
        shown_spoken.contains(&*first_spoken),
        "control: the piece drawn holds the first paragraph's words"
    );
    assert!(
        after.iter().any(|s| *s == *shown_spoken),
        "the source drawn, its piece {shown_spoken:?} among {after:?}"
    );
}

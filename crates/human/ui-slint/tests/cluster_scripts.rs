// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Characters at the cluster bound and one past it, in the scripts a
//! text shaper clusters otherwise than a Latin letter and its marks,
//! drawn by the desktop's own window and software renderer (HUMAN-CHAT.md,
//! Bounds, Clusters: the condition every other figure must keep).
//! `cluster_draw.rs` measured that one class; this measures the others
//! the record named: Indic conjunct chains, Khmer and Myanmar stacks and
//! emoji tag sequences, each a character of 32 code points by UAX #29,
//! 33 and 256; and runs UAX #29 cuts into characters of 32 or fewer, 300
//! code points long in all, that a shaper's syllable rules might join.
//!
//! The counter that overflows is parley 0.11.1's (under Slint 1.18.1):
//! an eight-bit count, per grapheme cluster as its own segmenter (ICU4X
//! 2.3) breaks the text, of the code points that contribute to shaping
//! (`shape/mod.rs`, `fill_cluster_in_place`); 256 of them panic it under
//! the overflow checks this test runs with, on in the test profile as in
//! release. So the test also asserts, for every text it draws, that the
//! segmenter breaks where UAX #29 does as the model counts: that is the
//! shaper's counted cluster never spanning more than one of the model's,
//! for these texts. A character of 256 fails with the replacement taken
//! out, in every script here but the tag sequence: a tag character is a
//! format character of no script, which parley does not count, so a tag
//! sequence of any length counts one and no mutation of the replacement
//! reaches it there. The 33 shows that the replaced text draws in each
//! script, not that it is replaced (`views.rs` holds the replacement on
//! the testing backend, which shapes no text). One test in this binary,
//! since a process holds one window's event loop.
//!
//! What this cannot prove: the phone, whose renderer is its own
//! (`tests/android-e2e/tests/cluster_bound.rs`); that the glyphs drawn
//! are the right ones, which no pixel here is compared for; a counter
//! below parley's (harfrust's own clusters), which only the absence of
//! a panic speaks for; and what a font this host lacks would do, since
//! a script with no font is shaped as missing glyphs (this host has Noto
//! fonts for each script here; CI's runner may not).

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

/// The code points of each UAX #29 cluster of `text`.
fn clusters(text: &str) -> Vec<usize> {
    text.graphemes(true).map(|c| c.chars().count()).collect()
}

/// Where the shaper's grapheme segmenter breaks `text` (ICU4X, which
/// parley counts a cluster's code points over) against where UAX #29
/// breaks it as the model counts: the same offsets, or the shaper's
/// counted cluster could hold several of the model's.
fn same_boundaries(text: &str) -> bool {
    let model: Vec<usize> = text
        .grapheme_indices(true)
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    let shaper: Vec<usize> = icu_segmenter::GraphemeClusterSegmenter::new()
        .segment_str(text)
        .collect();
    model == shaper
}

/// One character of `n` code points: `base`, then `mark`'s code points
/// in turn until there are `n`, so a chain of two may end on its virama.
fn stack(base: &str, mark: &str, n: usize) -> String {
    let s: String = base.chars().chain(mark.chars().cycle()).take(n).collect();
    assert_eq!(s.chars().count(), n, "{base:?} {mark:?} reach {n} exactly");
    s
}

/// A script's character at the bound, one past it and far past it,
/// each one UAX #29 cluster.
struct Character {
    script: &'static str,
    base: &'static str,
    mark: &'static str,
}

const CHARACTERS: &[Character] = &[
    // GB9c (Unicode 15.1): consonant, virama, consonant… is one cluster.
    Character {
        script: "Devanagari conjunct chain",
        base: "\u{915}",
        mark: "\u{94D}\u{915}",
    },
    Character {
        script: "Bengali conjunct chain",
        base: "\u{995}",
        mark: "\u{9CD}\u{995}",
    },
    // GB9c as unicode-segmentation 1.13.3 implements it (measured): a
    // Khmer coeng and a Myanmar stacking virama link as a virama does,
    // so a subscript stack is one character.
    Character {
        script: "Khmer coeng stack",
        base: "\u{1780}",
        mark: "\u{17D2}\u{1780}",
    },
    Character {
        script: "Myanmar virama stack",
        base: "\u{1000}",
        mark: "\u{1039}\u{1000}",
    },
    // A Khmer and a Myanmar consonant with signs stacked above it.
    Character {
        script: "Khmer mark stack",
        base: "\u{1780}",
        mark: "\u{17C6}",
    },
    Character {
        script: "Myanmar mark stack",
        base: "\u{1000}",
        mark: "\u{1036}",
    },
    // A black flag and tag characters; a cancel tag closes a real one.
    Character {
        script: "emoji tag sequence",
        base: "\u{1F3F4}",
        mark: "\u{E0067}",
    },
];

/// Runs UAX #29 cuts into clusters of 32 or fewer code points that a
/// shaper's syllable rules may take as one cluster of its own, each
/// past 255 code points in all.
fn joinable_runs() -> Vec<(&'static str, String)> {
    let run = |unit: &str| unit.repeat(300 / unit.chars().count() + 1);
    vec![
        // A Devanagari virama and ZWNJ end a UAX #29 character; a shaper
        // shapes the half form with the consonant after it.
        ("Devanagari half forms", run("\u{915}\u{94D}\u{200C}")),
        // A dependent vowel ends a Khmer stack for UAX #29, which a
        // shaper's syllable may run on past.
        (
            "Khmer stacks and vowels",
            run("\u{1780}\u{17D2}\u{1780}\u{17B6}"),
        ),
        // Tibetan subjoined letters, one character with their base.
        ("Tibetan subjoined letters", run("\u{F40}\u{F90}")),
        // Tamil's virama links nothing for UAX #29.
        ("Tamil virama chain", run("\u{B95}\u{BCD}")),
        // Sinhala conjuncts are made with a virama and ZWJ, which UAX #29
        // does not join.
        ("Sinhala ZWJ conjuncts", run("\u{D9A}\u{DCA}\u{200D}")),
        // Tag sequences, one character each to UAX #29, back to back.
        (
            "emoji tag sequences",
            run("\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}"),
        ),
    ]
}

/// Every text this test draws, named.
fn texts() -> Vec<(String, String)> {
    let mut texts = Vec::new();
    for c in CHARACTERS {
        for n in [32, 33, 256] {
            let s = stack(c.base, c.mark, n);
            assert_eq!(
                clusters(&s),
                [n],
                "{}: one character of {n} code points by UAX #29",
                c.script
            );
            texts.push((format!("{} of {n}", c.script), s));
        }
    }
    for (name, s) in joinable_runs() {
        let counts = clusters(&s);
        assert!(
            counts.len() > 1 && counts.iter().all(|&n| n <= 32),
            "{name}: several characters of 32 or fewer by UAX #29, {counts:?}"
        );
        assert!(s.chars().count() > 255, "{name}: past the shaper's count");
        texts.push((name.to_owned(), s));
    }
    for (name, s) in &texts {
        assert!(
            same_boundaries(s),
            "{name}: the shaper's segmenter breaks where UAX #29 does"
        );
    }
    texts
}

/// Each text in a body, in a code block and so in the Chats line, each
/// from a peer of its own: the window draws the Chats list and then the
/// conversation, for every one in turn.
#[test]
fn characters_the_shaper_clusters_otherwise_draw_in_a_real_window() {
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    view.window().show().expect("shown");
    let mut model = UiModel::new();
    for (row, (name, text)) in (1..).zip(texts()) {
        // Which text a panic in the shaper came from.
        eprintln!("drawing {name}");
        let from = ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id");
        model.received(received(
            row,
            &from,
            &format!("first {text} line\n\n```\n{text}\n```"),
        ));
        view.render(&model);
        slint::platform::update_timers_and_animations();
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
}

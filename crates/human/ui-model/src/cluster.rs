// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What a view may draw of one character, as a person counts characters
//! (an extended grapheme cluster, UAX #29): at most [`CLUSTER_CODE_POINTS`]
//! code points, and a cluster past that drawn as one U+FFFD in its place
//! (HUMAN-CHAT.md, Bounds). A text shaper's counters are sized for the
//! clusters real text has; a cluster far past them is no character anyone
//! writes, and drawing it is the shaper's overflow, not the person's
//! text. Never part of a cluster: drawn in part, the marks dropped would
//! change what the sender wrote with nothing on screen to say so.
//!
//! Only what is drawn: the text as received, stored, copied and forwarded
//! is the bytes as sent.

use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation as _;

/// The most code points a drawn character holds. Conforming text needs
/// at most 31 (UAX #15's Stream-Safe Text Format: a starter and 30
/// non-starters), the longest emoji ZWJ sequence 10.
pub const CLUSTER_CODE_POINTS: usize = 32;

/// What a character past [`CLUSTER_CODE_POINTS`] is drawn as.
pub const REPLACEMENT: char = '\u{FFFD}';

/// Whether `cluster` is past the bound. A cluster of no more bytes than
/// the bound has no more code points, so most are decided by length.
fn too_long(cluster: &str) -> bool {
    cluster.len() > CLUSTER_CODE_POINTS && cluster.chars().nth(CLUSTER_CODE_POINTS).is_some()
}

/// Whether every character of `text` may be drawn as it is.
#[must_use]
pub fn fits(text: &str) -> bool {
    !text.graphemes(true).any(too_long)
}

/// `text` as a view may draw it: each character past the bound replaced
/// by one [`REPLACEMENT`], every other character as it is. Borrowed when
/// nothing is replaced.
#[must_use]
pub fn drawable(text: &str) -> Cow<'_, str> {
    if fits(text) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for cluster in text.graphemes(true) {
        if too_long(cluster) {
            out.push(REPLACEMENT);
        } else {
            out.push_str(cluster);
        }
    }
    Cow::Owned(out)
}

/// `text` as a screen reader is given it: each [`REPLACEMENT`] spoken as
/// `words` (`UiText::ReplacedCharacter`), set apart by a space where it
/// touches a character that is not one -- but none before words that
/// start with punctuation, which attaches to what precedes it
/// (design/2026-11-04-long-message.md section 6). Both kinds of mark: one
/// drawn for a cluster past the bound, and one the text already carried,
/// since no client can tell them apart. Borrowed when there is none.
#[must_use]
pub fn spoken<'a>(text: &'a str, words: &str) -> Cow<'a, str> {
    if !text.contains(REPLACEMENT) {
        return Cow::Borrowed(text);
    }
    let attaches = words.chars().next().is_some_and(punctuation);
    let mut out = String::with_capacity(text.len() + words.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != REPLACEMENT {
            out.push(c);
            continue;
        }
        if !attaches && out.chars().next_back().is_some_and(|p| !p.is_whitespace()) {
            out.push(' ');
        }
        out.push_str(words);
        if chars.peek().is_some_and(|n| !n.is_whitespace()) {
            out.push(' ');
        }
    }
    Cow::Owned(out)
}

/// Whether `c` is Unicode punctuation (general category P), so another
/// script's comma attaches as a Latin one does.
fn punctuation(c: char) -> bool {
    use unicode_general_category::GeneralCategory as G;
    matches!(
        unicode_general_category::get_general_category(c),
        G::ConnectorPunctuation
            | G::DashPunctuation
            | G::OpenPunctuation
            | G::ClosePunctuation
            | G::InitialPunctuation
            | G::FinalPunctuation
            | G::OtherPunctuation
    )
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{CLUSTER_CODE_POINTS, REPLACEMENT, drawable, fits, spoken};

    fn marked(marks: usize) -> String {
        format!("a{}", "\u{301}".repeat(marks))
    }

    #[test]
    fn a_character_at_the_bound_is_drawn_as_it_is() {
        let at = marked(CLUSTER_CODE_POINTS - 1);
        assert_eq!(at.chars().count(), CLUSTER_CODE_POINTS);
        assert!(fits(&at));
        assert!(matches!(drawable(&at), Cow::Borrowed(t) if t == at));
    }

    #[test]
    fn a_character_past_the_bound_is_one_replacement_in_its_place() {
        for marks in [CLUSTER_CODE_POINTS, 255, 90_000] {
            let past = marked(marks);
            assert!(!fits(&past));
            let text = format!("before {past} after");
            assert_eq!(
                drawable(&text),
                format!("before {REPLACEMENT} after"),
                "{marks} marks"
            );
        }
    }

    #[test]
    fn the_longest_emoji_sequences_are_drawn_whole() {
        // Kiss: woman, man, light and medium-light skin tones: ten code
        // points, one character.
        let kiss =
            "\u{1F469}\u{1F3FB}\u{200D}\u{2764}\u{FE0F}\u{200D}\u{1F48B}\u{200D}\u{1F468}\u{1F3FC}";
        assert_eq!(kiss.chars().count(), 10);
        assert_eq!(drawable(kiss), kiss);
        // A chain the segmenter keeps as one character, past the bound.
        let chain = vec!["\u{1F468}"; 20].join("\u{200D}");
        assert_eq!(drawable(&chain), REPLACEMENT.to_string());
    }

    #[test]
    fn each_character_is_judged_alone() {
        // 31 marks on each of many letters: every one fits.
        let text = marked(CLUSTER_CODE_POINTS - 1).repeat(50);
        assert!(fits(&text));
        assert_eq!(drawable(&text), text);
    }

    #[test]
    fn a_mark_is_spoken_as_words_set_apart_from_the_text() {
        let w = "character not shown";
        assert!(matches!(spoken("plain", w), Cow::Borrowed("plain")));
        assert_eq!(spoken("x \u{FFFD} y", w), "x character not shown y");
        assert_eq!(spoken("x\u{FFFD}y", w), "x character not shown y");
        assert_eq!(spoken("\u{FFFD}rest", w), "character not shown rest");
        assert_eq!(spoken("end\u{FFFD}", w), "end character not shown");
        assert_eq!(
            spoken("a\u{FFFD}\u{FFFD}b", w),
            "a character not shown character not shown b"
        );
        // Words that start with punctuation attach to the text before,
        // in any script; the space after stays.
        let p = ", character not shown,";
        assert_eq!(
            spoken("cafe\u{FFFD}open", p),
            "cafe, character not shown, open"
        );
        assert_eq!(
            spoken("cafe \u{FFFD} open", p),
            "cafe , character not shown, open"
        );
        assert_eq!(spoken("\u{FFFD}open", p), ", character not shown, open");
        assert_eq!(
            spoken("x\u{FFFD}y", "\u{060C} w\u{060C}"),
            "x\u{060C} w\u{060C} y"
        );
        assert_eq!(spoken("x\u{FFFD}y", "\u{3001}w"), "x\u{3001}w y");
        // What the bubble draws for a cluster past the bound.
        let past = drawable(&format!("x{}y", marked(CLUSTER_CODE_POINTS))).into_owned();
        assert!(!spoken(&past, w).contains(REPLACEMENT));
    }
}

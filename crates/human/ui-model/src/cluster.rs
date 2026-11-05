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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{CLUSTER_CODE_POINTS, REPLACEMENT, drawable, fits};

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
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A Chats row's preview: the first line of the conversation's newest
//! shown row, cut to a bound (design/2026-10-27-start-window.md section 3).
//! The same cut whether the row is held or read from the store for the
//! start, so a preview does not change when its row loads.

use std::borrow::Cow;

use interweave_human_chat_protocol::Rendered;
use unicode_segmentation::UnicodeSegmentation as _;

/// The most characters a preview holds, as a person counts them: grapheme
/// clusters, not code points.
pub const PREVIEW_GRAPHEMES: usize = 160;

/// The most bytes of UTF-8 a preview holds.
pub const PREVIEW_BYTES: usize = 1024;

/// Ends a cut preview, at its logical end, whatever room the row has.
const ELLIPSIS: char = '\u{2026}';

/// A preview: its text, within both bounds, and whether it was cut.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Preview {
    /// The first line, or as much of it as fits, cut only between whole
    /// characters.
    pub text: String,
    /// Some of the first line did not fit.
    pub cut: bool,
}

impl Preview {
    /// What a row shows and its accessible description reads: the text,
    /// and the ellipsis after it when it was cut. The ellipsis counts in
    /// neither bound.
    #[must_use]
    pub fn shown(&self) -> Cow<'_, str> {
        if self.cut {
            let mut shown = String::with_capacity(self.text.len() + ELLIPSIS.len_utf8());
            shown.push_str(&self.text);
            shown.push(ELLIPSIS);
            Cow::Owned(shown)
        } else {
            Cow::Borrowed(&self.text)
        }
    }
}

/// The preview of `rendered`: its first line as [`crate::body::first_line`]
/// gives it -- markup gone, a link's text kept -- then cut, so a cut never
/// leaves half a mark showing. `image` names an image from its alt text.
#[must_use]
pub fn preview(rendered: &Rendered, image: &dyn Fn(&str) -> String) -> Preview {
    cut(&crate::body::first_line(rendered, image))
}

/// `line` cut to [`PREVIEW_GRAPHEMES`] characters and [`PREVIEW_BYTES`]
/// bytes, whichever comes first, only between whole characters. A first
/// character larger than the byte bound leaves nothing: the preview is
/// the ellipsis alone.
#[must_use]
pub fn cut(line: &str) -> Preview {
    let mut end = 0;
    for (taken, (at, grapheme)) in line.grapheme_indices(true).enumerate() {
        if taken == PREVIEW_GRAPHEMES || at + grapheme.len() > PREVIEW_BYTES {
            return Preview {
                text: line[..end].to_owned(),
                cut: true,
            };
        }
        end = at + grapheme.len();
    }
    Preview {
        text: line.to_owned(),
        cut: false,
    }
}

#[cfg(test)]
mod tests {
    use interweave_human_chat_protocol::render;
    use unicode_segmentation::UnicodeSegmentation as _;

    use super::{PREVIEW_BYTES, PREVIEW_GRAPHEMES, cut, preview};

    fn graphemes(text: &str) -> usize {
        text.graphemes(true).count()
    }

    /// A first line of 300 characters previews its first 160 and the
    /// ellipsis, whatever room a row has. Mutation: the cut counting code
    /// points, so a character with a combining mark counts twice.
    #[test]
    fn a_long_first_line_is_cut_at_160_characters_with_the_ellipsis() {
        let plain = "a".repeat(300);
        let p = cut(&plain);
        assert!(p.cut);
        assert_eq!(p.text, "a".repeat(PREVIEW_GRAPHEMES));
        assert_eq!(p.shown(), format!("{}\u{2026}", "a".repeat(160)));

        // Each character a base and a combining acute: 160 of them are
        // 480 bytes, within the byte bound, so the count decides.
        let marked = "e\u{301}".repeat(300);
        let p = cut(&marked);
        assert!(p.cut);
        assert_eq!(graphemes(&p.text), PREVIEW_GRAPHEMES);
        assert_eq!(p.text, "e\u{301}".repeat(160));
    }

    /// A short line shows whole, with no ellipsis -- one of exactly 160
    /// characters too. Mutation: a line of 160 taken as cut.
    #[test]
    fn a_short_line_is_whole_with_no_ellipsis() {
        for line in ["hello", &"a".repeat(PREVIEW_GRAPHEMES)] {
            let p = cut(line);
            assert!(!p.cut, "{} characters", line.len());
            assert_eq!(p.shown(), line);
        }
    }

    /// 100 emoji sequences of 25 bytes each are cut at 1 KiB, between
    /// whole sequences: 40 of them, 1,000 bytes. Mutation: a byte cut at a
    /// code-point boundary, which ends inside a sequence.
    #[test]
    fn emoji_sequences_are_cut_at_1_kib_between_whole_characters() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        assert_eq!(family.len(), 25, "the fixture: 25 bytes, one character");
        let p = cut(&family.repeat(100));
        assert!(p.cut);
        assert!(p.text.len() <= PREVIEW_BYTES);
        assert_eq!(p.text, family.repeat(40));
    }

    /// A first character larger than 1 KiB on its own -- a base with 600
    /// combining marks -- leaves nothing that fits: the ellipsis alone.
    /// Mutation: the first character always kept.
    #[test]
    fn a_first_character_over_1_kib_previews_as_the_ellipsis_alone() {
        let heavy = format!("a{}rest", "\u{301}".repeat(600));
        assert_eq!(
            graphemes(&heavy),
            5,
            "the fixture: one heavy character, then four"
        );
        let p = cut(&heavy);
        assert!(p.cut);
        assert_eq!(p.text, "");
        assert_eq!(p.shown(), "\u{2026}");
    }

    /// A link across the 160th character shows its text, never its
    /// syntax: the first line is computed, then cut.
    #[test]
    fn a_link_across_the_cut_shows_its_text() {
        let source = format!(
            "{}[{}](https://example.org/x)",
            "a".repeat(150),
            "b".repeat(30)
        );
        let p = preview(&render(&source), &|alt| format!("[image: {alt}]"));
        assert!(p.cut);
        assert_eq!(p.text, format!("{}{}", "a".repeat(150), "b".repeat(10)));
        assert!(!p.shown().contains('['), "no syntax: {}", p.shown());
    }

    /// A Hebrew first line is cut in logical order and the ellipsis goes
    /// at its logical end, which a right-to-left line draws on the left.
    #[test]
    fn a_hebrew_line_is_cut_in_logical_order() {
        let hebrew = "\u{5E9}".repeat(300);
        let p = cut(&hebrew);
        assert_eq!(p.text, "\u{5E9}".repeat(PREVIEW_GRAPHEMES));
        assert!(p.shown().ends_with('\u{2026}'));
    }
}

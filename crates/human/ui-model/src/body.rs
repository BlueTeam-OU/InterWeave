// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A message body as a view draws it: `chat-protocol`'s block tree,
//! flattened into lines of plain text, and the links it carries. Here
//! rather than in a view so that the first line a Chats preview cuts is
//! the same whether its row is held or read from the store for the start
//! (design/2026-10-27-start-window.md section 3): the facade has no view.
//!
//! The tree is the one parse of the remote bytes (ADR-0050 rule 6). What
//! leaves this module is plain text for `Text` elements -- never markup
//! for a second parser to read (architect-cto's B6 ruling: Slint's own
//! `StyledText` is not given remote-derived text). So inline emphasis,
//! strong, strikethrough and code spans are drawn as their text alone; a
//! link's label stays where it stood, and its destination becomes a
//! separate control; an image is a placeholder naming its alt text, or
//! only that it is an image when the alt text is empty or blank, and
//! nothing is fetched.
//!
//! A line leaves here in pieces no larger than a piece's bound
//! ([`PIECE_GRAPHEMES`], [`PIECE_LINES`]), which HUMAN-CHAT.md's Bounds
//! asks of every drawn item. The desktop's software renderer keeps a
//! filled rectangle's geometry in 16 bits and panics on one taller than
//! 32,767 physical pixels, and a line's surfaces -- a code block's fill,
//! a quote's bar -- are as tall as the line (j92). Its text alone would
//! not: drawn with a vector font, as the desktop's are, one text item
//! 80,000 physical pixels tall was measured to draw.

use std::borrow::Cow;

use interweave_human_chat_protocol::{Block, Inline, Rendered};
use unicode_segmentation::UnicodeSegmentation as _;

use crate::labels::{UiText, fill, placeholder_en, visible_destination};

/// How an image is named in text, from its alt text, in the interface's
/// words: what a body or a preview shows in an image's place, the same for
/// a view and the facade.
#[must_use]
pub fn image_text(alt: &str) -> String {
    if alt.trim().is_empty() {
        placeholder_en::text(UiText::ImageNotShownWithoutAlt).to_owned()
    } else {
        fill(placeholder_en::text(UiText::ImageNotShown), &[("alt", alt)])
    }
}

/// What a line is, which decides how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Running text: a paragraph, a list item's text, raw HTML shown
    /// literally.
    Text,
    /// A heading, its level 1 to 6.
    Heading(u8),
    /// A code block, as written.
    Code,
    /// A thematic break.
    Rule,
    /// A table's header row, its cells joined.
    TableHeader,
    /// A table body row, its cells joined.
    TableRow,
}

/// The most characters (grapheme clusters) a body draws whole, and so the
/// most any one drawn item holds (HUMAN-CHAT.md, Bounds, A 2026-11-04).
pub const BODY_GRAPHEMES: usize = 2000;

/// A link's control's label as a view shows it: its words and the
/// destination with every hidden code point spelled out
/// (`visible_destination`), whole, never cut.
#[must_use]
pub fn link_label(destination: &str) -> String {
    fill(
        placeholder_en::text(UiText::OpenLink),
        &[("destination", &visible_destination(destination))],
    )
}

/// Whether a link gets a control: only when its label, as shown, is within
/// [`BODY_GRAPHEMES`] and holds no character past the cluster bound. A
/// label is never cut or replaced in part -- either could show one host
/// and open another -- so a link past either has its text drawn inert, as
/// a link of a scheme outside the allow-list does (HUMAN-CHAT.md, Bounds,
/// A 2026-11-04 (ii)).
fn has_control(destination: &str) -> bool {
    let label = link_label(destination);
    label.graphemes(true).nth(BODY_GRAPHEMES).is_none() && crate::cluster::fits(&label)
}

/// Whether a line is a piece of the line before it, and where it was cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    /// A line of its own.
    Starts,
    /// The rest of the line before, cut at a line break the text has:
    /// drawn against it, it reads as the text did.
    AfterBreak,
    /// The rest of the line before, cut inside a run with no line break,
    /// at a word boundary (or between characters where the run has none):
    /// that piece ends early, a ragged line the text did not have, the
    /// seam design/2026-11-04-long-message.md section 5.2 accepts.
    MidLine,
}

/// The most characters, as a person counts them (grapheme clusters), one
/// drawn piece holds: the most the conversation's preview of a cut body
/// holds (design/2026-11-04-long-message.md section 6), so that a piece
/// is no taller than such a preview, which that note's A4 measures under
/// the renderer's ceiling at 200 % text.
pub const PIECE_GRAPHEMES: usize = 1600;

/// The most lines, as the text breaks them, one drawn piece holds: the
/// preview's line bound, for the same reason.
pub const PIECE_LINES: usize = 32;

/// Whether `grapheme` ends a line wherever it stands: a mandatory break of
/// UAX #14 -- LF, CR or CRLF, VT, FF, NEL, U+2028 and U+2029. The desktop's
/// text layout starts a new line at every one, not at LF alone, so a piece
/// that counted LF alone could hold a line for every character.
#[must_use]
pub fn is_line_break(grapheme: &str) -> bool {
    grapheme.ends_with([
        '\n', '\r', '\u{B}', '\u{C}', '\u{85}', '\u{2028}', '\u{2029}',
    ])
}

/// One drawn line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// What the line is.
    pub kind: Kind,
    /// How many lists and quotes it sits in.
    pub depth: usize,
    /// Whether any enclosing block is a quote.
    pub quoted: bool,
    /// A list item's marker on its first line: `•`, or `3.`.
    pub marker: String,
    /// Its text, plain: no markup is left in it.
    pub text: String,
    /// Whether it is a piece of the line before.
    pub join: Join,
}

/// A body: its lines, and its links' destinations in the order they
/// appear. Every destination is one the tree kept, so an allowlisted
/// scheme, and one whose control's label is within the figures
/// ([`link_label`]); the view checks the scheme again on activation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Body {
    /// The lines, in the order they are drawn.
    pub lines: Vec<Line>,
    /// The links' destinations, in the order they appear.
    pub links: Vec<String>,
}

/// Separates a table row's cells.
const CELL_SEPARATOR: &str = " | ";

/// `rendered`, flattened. `image` names an image placeholder from its alt
/// text, in the view's words.
pub fn flatten(rendered: &Rendered, image: &dyn Fn(&str) -> String) -> Body {
    let mut body = Body::default();
    match rendered {
        // Past a bound: the source as received, one block of plain text.
        Rendered::PlainText { source, .. } => body.lines.push(Line {
            kind: Kind::Text,
            depth: 0,
            quoted: false,
            marker: String::new(),
            text: source.clone(),
            join: Join::Starts,
        }),
        Rendered::Markdown(blocks) => {
            let mut walk = Walk {
                body: &mut body,
                image,
                soft_break: ' ',
            };
            for block in blocks {
                walk.block(block, 0, false, "");
            }
        }
    }
    body.lines = body
        .lines
        .into_iter()
        .map(Line::drawable)
        .flat_map(Line::into_pieces)
        .collect();
    body
}

impl Line {
    /// This line with every character past the cluster bound drawn as one
    /// replacement ([`crate::cluster`]), before it is counted or cut.
    fn drawable(mut self) -> Self {
        if let Cow::Owned(text) = crate::cluster::drawable(&self.text) {
            self.text = text;
        }
        self
    }

    /// This line in pieces within the bounds; the first keeps the marker.
    fn into_pieces(self) -> Vec<Self> {
        let cuts = pieces(&self.text);
        if cuts.len() == 1 {
            return vec![self];
        }
        cuts.iter()
            .enumerate()
            .map(|(i, &(text, join))| Self {
                kind: self.kind,
                depth: self.depth,
                quoted: self.quoted,
                marker: if i == 0 {
                    self.marker.clone()
                } else {
                    String::new()
                },
                text: text.to_owned(),
                join,
            })
            .collect()
    }
}

/// `text` cut into pieces of at most [`PIECE_GRAPHEMES`] characters and
/// [`PIECE_LINES`] lines, each with how it joins the one before: at the
/// last line break inside the bound, whose break the cut takes the place
/// of; else at the last word boundary, the space before it kept with the
/// piece it ends; else between two characters. Never inside a character.
#[must_use]
pub fn pieces(text: &str) -> Vec<(&str, Join)> {
    let mut out = Vec::new();
    let mut rest = text;
    let mut join = Join::Starts;
    loop {
        // Where the most a piece may hold ends, if `rest` holds more, and
        // the last break before that end. Ending at a piece's last break
        // drops the break, so the piece holds a line more than it breaks.
        // `beyond` is where the character after that end ends: a word may
        // begin at the end itself, which only the character after says.
        let mut cap = None;
        let mut beyond = rest.len();
        let mut last_break = None;
        let mut breaks = 0;
        for (taken, (at, grapheme)) in rest.grapheme_indices(true).enumerate() {
            if taken == PIECE_GRAPHEMES {
                cap = Some(at);
                beyond = at + grapheme.len();
                break;
            }
            if is_line_break(grapheme) {
                breaks += 1;
                last_break = Some((at, grapheme.len()));
                if breaks == PIECE_LINES {
                    cap = Some(at);
                    break;
                }
            }
        }
        let Some(cap) = cap else {
            out.push((rest, join));
            return out;
        };
        let (end, next, next_join) = match last_break {
            Some((at, len)) if at > 0 => (at, at + len, Join::AfterBreak),
            _ => {
                let at = word_cut(&rest[..beyond], cap).unwrap_or(cap);
                (at, at, Join::MidLine)
            }
        };
        out.push((&rest[..end], join));
        rest = &rest[next..];
        if rest.is_empty() {
            return out;
        }
        join = next_join;
    }
}

/// The last word boundary in `head`, past its start and at most `cap`,
/// where a word begins, so that the next piece starts with a word and not
/// with the space before it.
fn word_cut(head: &str, cap: usize) -> Option<usize> {
    head.split_word_bound_indices()
        .rev()
        .find(|&(at, word)| at > 0 && at <= cap && !word.starts_with(char::is_whitespace))
        .map(|(at, _)| at)
}

/// The first line of `rendered` as the person wrote it, as plain text:
/// the first non-blank line of the drawn text, with a soft line break
/// ending a line as it did in the source (design/2026-10-16-android-look.md
/// section 5.2: markup gone, a link's text kept). Empty when nothing is.
pub fn first_line(rendered: &Rendered, image: &dyn Fn(&str) -> String) -> String {
    let mut body = Body::default();
    match rendered {
        Rendered::PlainText { source, .. } => {
            return source
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or_default()
                .to_owned();
        }
        Rendered::Markdown(blocks) => {
            let mut walk = Walk {
                body: &mut body,
                image,
                soft_break: '\n',
            };
            for block in blocks {
                walk.block(block, 0, false, "");
            }
        }
    }
    body.lines
        .iter()
        .flat_map(|line| line.text.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_owned()
}

struct Walk<'a> {
    body: &'a mut Body,
    image: &'a dyn Fn(&str) -> String,
    /// What a soft line break becomes: a space where the body is drawn,
    /// and the line's end for a preview, which shows only the first.
    soft_break: char,
}

impl Walk<'_> {
    /// Push `block`'s lines. `marker` is given to its first line only:
    /// a list item's second paragraph is not numbered again.
    fn block(&mut self, block: &Block, depth: usize, quoted: bool, marker: &str) {
        let line = |kind, text| Line {
            kind,
            depth,
            quoted,
            marker: marker.to_owned(),
            text,
            join: Join::Starts,
        };
        match block {
            Block::Paragraph(content) => {
                let text = self.inlines(content);
                self.body.lines.push(line(Kind::Text, text));
            }
            Block::Heading { level, content } => {
                let text = self.inlines(content);
                self.body.lines.push(line(Kind::Heading(*level), text));
            }
            Block::Code { text, .. } => {
                // The final newline a fence leaves is not a line.
                let text = text.strip_suffix('\n').unwrap_or(text).to_owned();
                self.body.lines.push(line(Kind::Code, text));
            }
            Block::Html(html) => self.body.lines.push(line(Kind::Text, html.clone())),
            Block::Rule => self.body.lines.push(line(Kind::Rule, String::new())),
            Block::Table { header, rows, .. } => {
                let header = self.cells(header);
                self.body.lines.push(line(Kind::TableHeader, header));
                for row in rows {
                    let row = self.cells(row);
                    self.body.lines.push(Line {
                        marker: String::new(),
                        ..line(Kind::TableRow, row)
                    });
                }
            }
            Block::Quote(blocks) => {
                for (i, inner) in blocks.iter().enumerate() {
                    let marker = if i == 0 { marker } else { "" };
                    self.block(inner, depth + 1, true, marker);
                }
            }
            Block::List { start, items } => {
                for (n, item) in items.iter().enumerate() {
                    let marker = match start {
                        Some(first) => format!("{}.", first.saturating_add(n as u64)),
                        None => "\u{2022}".to_owned(),
                    };
                    for (i, inner) in item.iter().enumerate() {
                        let marker = if i == 0 { marker.as_str() } else { "" };
                        self.block(inner, depth + 1, quoted, marker);
                    }
                }
            }
        }
    }

    fn cells(&mut self, cells: &[Vec<Inline>]) -> String {
        cells
            .iter()
            .map(|cell| self.inlines(cell))
            .collect::<Vec<_>>()
            .join(CELL_SEPARATOR)
    }

    fn inlines(&mut self, content: &[Inline]) -> String {
        let mut text = String::new();
        self.push_inlines(content, &mut text);
        text
    }

    fn push_inlines(&mut self, content: &[Inline], out: &mut String) {
        for inline in content {
            match inline {
                Inline::Text(text) | Inline::Code(text) | Inline::Html(text) => out.push_str(text),
                Inline::Emphasis(inner) | Inline::Strong(inner) | Inline::Strikethrough(inner) => {
                    self.push_inlines(inner, out);
                }
                Inline::Link {
                    destination,
                    content,
                } => {
                    self.push_inlines(content, out);
                    if has_control(destination) {
                        self.body.links.push(destination.clone());
                    }
                }
                Inline::Image { alt, .. } => {
                    let alt = self.inlines(alt);
                    out.push_str(&(self.image)(&alt));
                }
                Inline::SoftBreak => out.push(self.soft_break),
                Inline::HardBreak => out.push('\n'),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use interweave_human_chat_protocol::render;

    use super::{Body, Kind, Line, flatten};

    #[test]
    fn a_preview_is_the_first_line_as_written_with_its_markup_gone() {
        let first =
            |source: &str| super::first_line(&render(source), &|alt| format!("[image: {alt}]"));
        assert_eq!(
            first("**Bold** and [the site](https://example.org)\nsecond"),
            "Bold and the site"
        );
        assert_eq!(first("\n\n# Title\n\nbody"), "Title");
        assert_eq!(first("---\n\n- item one\n- item two"), "item one");
        assert_eq!(first("```\nlet x = 1;\nlet y = 2;\n```"), "let x = 1;");
        assert_eq!(
            first("![a cat](https://example.org/c.png) hi"),
            "[image: a cat] hi"
        );
        assert_eq!(first(""), "");
    }

    fn body(source: &str) -> Body {
        flatten(&render(source), &|alt| format!("[image: {alt}]"))
    }

    fn line(kind: Kind, depth: usize, quoted: bool, marker: &str, text: &str) -> Line {
        Line {
            kind,
            depth,
            quoted,
            marker: marker.to_owned(),
            text: text.to_owned(),
            join: super::Join::Starts,
        }
    }

    #[test]
    fn blocks_become_lines_of_their_text_and_inline_marks_drop_to_text() {
        let got = body(
            "# Title\n\nSome *soft* and **strong** ~~gone~~ `code`.\n\n---\n\n```rust\nfn x() {}\n```\n",
        );
        assert_eq!(
            got.lines,
            [
                line(Kind::Heading(1), 0, false, "", "Title"),
                line(Kind::Text, 0, false, "", "Some soft and strong gone code."),
                line(Kind::Rule, 0, false, "", ""),
                line(Kind::Code, 0, false, "", "fn x() {}"),
            ]
        );
        assert!(got.links.is_empty());
    }

    #[test]
    fn lists_and_quotes_nest_by_depth_and_number_from_their_start() {
        let got = body("3. three\n4. four\n   - inner\n\n> quoted\n> > deeper\n");
        assert_eq!(
            got.lines,
            [
                line(Kind::Text, 1, false, "3.", "three"),
                line(Kind::Text, 1, false, "4.", "four"),
                line(Kind::Text, 2, false, "\u{2022}", "inner"),
                line(Kind::Text, 1, true, "", "quoted"),
                line(Kind::Text, 2, true, "", "deeper"),
            ]
        );
    }

    #[test]
    fn a_link_keeps_its_label_in_place_and_its_destination_becomes_a_link() {
        let got = body("See [the docs](https://example.org/a) and [mail](mailto:a@example.org).");
        assert_eq!(
            got.lines,
            [line(Kind::Text, 0, false, "", "See the docs and mail.")]
        );
        assert_eq!(got.links, ["https://example.org/a", "mailto:a@example.org"]);
    }

    #[test]
    fn a_link_outside_the_allowlist_is_its_text_alone() {
        let got = body("[click](javascript:alert(1)) and [file](file:///etc/passwd)");
        assert_eq!(
            got.lines,
            [line(Kind::Text, 0, false, "", "click and file")]
        );
        assert!(got.links.is_empty(), "no control for an inert link");
    }

    #[test]
    fn an_image_is_a_placeholder_naming_its_alt_and_never_a_link() {
        let got = body("![a cat](https://example.org/cat.png)");
        assert_eq!(
            got.lines,
            [line(Kind::Text, 0, false, "", "[image: a cat]")]
        );
        assert!(got.links.is_empty(), "the image's address is not offered");
    }

    #[test]
    fn raw_html_is_literal_text() {
        let got = body("<b>bold?</b>\n\ninline <i>x</i> html\n");
        assert_eq!(
            got.lines,
            [
                line(Kind::Text, 0, false, "", "<b>bold?</b>"),
                line(Kind::Text, 0, false, "", "inline <i>x</i> html"),
            ]
        );
    }

    #[test]
    fn a_table_is_a_header_and_rows_of_joined_cells() {
        let got = body("| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n");
        assert_eq!(
            got.lines,
            [
                line(Kind::TableHeader, 0, false, "", "a | b"),
                line(Kind::TableRow, 0, false, "", "1 | 2"),
                line(Kind::TableRow, 0, false, "", "3 | 4"),
            ]
        );
    }

    #[test]
    fn a_source_past_a_bound_is_one_line_of_the_source() {
        let deep = format!("{}x", "> ".repeat(17));
        let got = body(&deep);
        assert_eq!(got.lines, [line(Kind::Text, 0, false, "", &deep)]);
        assert!(got.links.is_empty());
    }

    mod pieces {
        use unicode_segmentation::UnicodeSegmentation as _;

        use super::super::{Join, Kind, PIECE_GRAPHEMES, PIECE_LINES, pieces};
        use super::body;

        fn graphemes(text: &str) -> usize {
            text.graphemes(true).count()
        }

        /// The text again, from its pieces: a break where one was cut.
        fn joined(cut: &[(&str, Join)]) -> String {
            let mut text = String::new();
            for &(piece, join) in cut {
                if join == Join::AfterBreak {
                    text.push('\n');
                }
                text.push_str(piece);
            }
            text
        }

        /// A piece's lines as the layout breaks them.
        fn lines(piece: &str) -> usize {
            1 + piece
                .graphemes(true)
                .filter(|g| super::super::is_line_break(g))
                .count()
        }

        #[test]
        fn every_mandatory_break_counts_as_a_line() {
            for brk in ["\r", "\u{B}", "\u{C}", "\u{85}", "\u{2028}", "\u{2029}"] {
                let text = format!("a{brk}").repeat(PIECE_GRAPHEMES / 2);
                let cut = pieces(&text);
                within_bounds(&cut);
                assert!(cut.len() > 1, "{brk:?} breaks a line");
                assert_eq!(
                    lines(cut[0].0),
                    PIECE_LINES,
                    "{brk:?}: a piece takes all it may"
                );
                assert_eq!(
                    cut[1].1,
                    Join::AfterBreak,
                    "{brk:?} is cut at, not inside a run"
                );
            }
        }

        fn within_bounds(cut: &[(&str, Join)]) {
            for &(piece, _) in cut {
                assert!(
                    graphemes(piece) <= PIECE_GRAPHEMES,
                    "{} characters",
                    graphemes(piece)
                );
                assert!(lines(piece) <= PIECE_LINES, "{} lines", lines(piece));
            }
        }

        #[test]
        fn a_line_within_both_bounds_is_one_piece() {
            let text = "x".repeat(PIECE_GRAPHEMES);
            assert_eq!(pieces(&text), [(text.as_str(), Join::Starts)]);
            let lines = vec!["line"; PIECE_LINES].join("\n");
            assert_eq!(pieces(&lines), [(lines.as_str(), Join::Starts)]);
        }

        #[test]
        fn lines_past_the_bound_are_cut_at_a_break_which_the_cut_replaces() {
            let text = (0..=PIECE_LINES * 3)
                .map(|n| format!("line {n}"))
                .collect::<Vec<_>>()
                .join("\n");
            let cut = pieces(&text);
            within_bounds(&cut);
            assert_eq!(cut.len(), 4);
            assert_eq!(
                cut[0].0.lines().count(),
                PIECE_LINES,
                "a piece takes all the lines it may"
            );
            assert_eq!(cut[0].1, Join::Starts);
            assert!(cut[1..].iter().all(|&(_, join)| join == Join::AfterBreak));
            assert!(
                cut.iter()
                    .all(|&(piece, _)| !piece.starts_with('\n') && !piece.ends_with('\n'))
            );
            assert_eq!(joined(&cut), text);
        }

        #[test]
        fn a_run_with_no_break_is_cut_where_a_word_begins() {
            let text = "a word or two, ".repeat(400);
            let cut = pieces(&text);
            within_bounds(&cut);
            assert!(cut.len() > 1);
            assert!(cut[1..].iter().all(|&(_, join)| join == Join::MidLine));
            for &(piece, _) in &cut[1..] {
                assert!(
                    piece.starts_with(|c: char| c.is_alphanumeric()),
                    "{:?}",
                    &piece[..10]
                );
            }
            assert_eq!(joined(&cut), text, "nothing dropped, nothing repeated");
        }

        #[test]
        fn a_cut_never_leaves_the_next_piece_starting_with_spaces() {
            // The bound falls inside a run of spaces: the cut goes back to
            // where the last word began, the spaces kept with the first.
            let text = format!("a {}{}y", "x".repeat(PIECE_GRAPHEMES - 10), " ".repeat(20));
            let cut = pieces(&text);
            assert_eq!(cut[0], ("a ", Join::Starts));
            assert!(cut[1].0.starts_with('x'));
            assert_eq!(joined(&cut), text);
        }

        #[test]
        fn a_break_inside_the_bound_is_cut_at_before_a_word_boundary() {
            // The break is seamless, a word boundary is not.
            let text = format!("first\n{}", "word ".repeat(PIECE_GRAPHEMES));
            let cut = pieces(&text);
            assert_eq!(cut[0], ("first", Join::Starts));
            assert_eq!(cut[1].1, Join::AfterBreak);
            assert_eq!(joined(&cut), text);
        }

        #[test]
        fn a_run_with_no_word_boundary_is_cut_between_characters_never_inside_one() {
            // One word of combining sequences, then of a family emoji: a
            // grapheme of several code points, which a cut must not part.
            for unit in ["e\u{301}", "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"] {
                let text = unit.repeat(PIECE_GRAPHEMES * 2 + 5);
                let cut = pieces(&text);
                within_bounds(&cut);
                assert_eq!(cut.len(), 3);
                assert_eq!(graphemes(cut[0].0), PIECE_GRAPHEMES);
                assert!(
                    cut.iter().all(|&(piece, _)| piece.starts_with(unit)),
                    "{unit:?}"
                );
                assert_eq!(joined(&cut), text);
            }
        }

        #[test]
        fn a_script_written_without_spaces_is_cut_between_its_characters() {
            let text = "\u{6f22}\u{5b57}".repeat(PIECE_GRAPHEMES);
            let cut = pieces(&text);
            within_bounds(&cut);
            assert_eq!(cut.len(), 2);
            assert_eq!(joined(&cut), text);
        }

        #[test]
        fn a_crlf_break_is_cut_whole() {
            let text = vec!["line"; PIECE_LINES + 1].join("\r\n");
            let cut = pieces(&text);
            assert_eq!(cut.len(), 2);
            assert!(
                !cut[0].0.ends_with('\r'),
                "the break is one character and goes whole"
            );
            assert_eq!(cut[1], ("line", Join::AfterBreak));
        }

        #[test]
        fn a_body_line_past_the_bound_leaves_flatten_in_pieces_its_marker_on_the_first() {
            let code = (0..=PIECE_LINES)
                .map(|n| format!("let x{n} = {n};"))
                .collect::<Vec<_>>();
            let got = body(&format!(
                "- item\n\n  ```\n  {}\n  ```\n",
                code.join("\n  ")
            ));
            let kinds = got
                .lines
                .iter()
                .map(|l| (l.kind, l.join))
                .collect::<Vec<_>>();
            assert_eq!(
                kinds,
                [
                    (Kind::Text, Join::Starts),
                    (Kind::Code, Join::Starts),
                    (Kind::Code, Join::AfterBreak),
                ]
            );
            assert_eq!(got.lines[2].text, code[PIECE_LINES]);
            assert!(
                got.lines[1..]
                    .iter()
                    .all(|l| l.depth == 1 && l.marker.is_empty())
            );

            let long = format!("3. {}", "word ".repeat(PIECE_GRAPHEMES));
            let got = body(&long);
            assert_eq!(
                got.lines.len(),
                pieces(&"word ".repeat(PIECE_GRAPHEMES)).len()
            );
            assert!(got.lines.len() > 1);
            assert_eq!(got.lines[0].marker, "3.");
            assert!(
                got.lines[1..]
                    .iter()
                    .all(|l| l.marker.is_empty() && l.join == Join::MidLine)
            );
        }
    }

    #[test]
    fn a_link_whose_label_would_pass_the_figures_gets_no_control_its_text_kept() {
        use super::{BODY_GRAPHEMES, link_label};

        let words = link_label("").chars().count();
        let at = format!("https://e.org/{}", "a".repeat(BODY_GRAPHEMES - words - 14));
        assert_eq!(
            link_label(&at).chars().count(),
            BODY_GRAPHEMES,
            "the case at the bound"
        );
        let past = format!("{at}a");
        let got = body(&format!("[at]({at}) and [past]({past})"));
        assert_eq!(got.lines, [line(Kind::Text, 0, false, "", "at and past")]);
        assert_eq!(got.links, [at], "a label of 2,000 is drawn, of 2,001 not");

        // Short in the source, long as shown: each hidden code point is
        // spelled out on the label.
        let hidden = format!("https://e.org/a{}", "\u{FE0F}".repeat(BODY_GRAPHEMES / 8));
        assert!(hidden.chars().count() < BODY_GRAPHEMES);
        let got = body(&format!("[padded]({hidden})"));
        assert_eq!(got.lines, [line(Kind::Text, 0, false, "", "padded")]);
        assert!(got.links.is_empty(), "counted as shown, not as sent");
    }

    #[test]
    fn a_character_past_the_cluster_bound_is_drawn_as_one_replacement() {
        use crate::cluster::{CLUSTER_CODE_POINTS, REPLACEMENT};

        let marked = |marks: usize| format!("a{}", "\u{301}".repeat(marks));
        let at = marked(CLUSTER_CODE_POINTS - 1);
        assert_eq!(
            body(&format!("x {at} y")).lines[0].text,
            format!("x {at} y")
        );
        for marks in [CLUSTER_CODE_POINTS, 255] {
            let got = body(&format!(
                "x {} y\n\n```\n{}\n```",
                marked(marks),
                marked(marks)
            ));
            assert_eq!(
                got.lines[0].text,
                format!("x {REPLACEMENT} y"),
                "{marks} marks"
            );
            assert_eq!(got.lines[1].text, REPLACEMENT.to_string(), "in code too");
        }
        // Past a bound, the source as one line: drawn the same way.
        let deep = format!("{}{}", "> ".repeat(17), marked(255));
        assert!(body(&deep).lines[0].text.ends_with(REPLACEMENT));

        // A link whose label would hold one gets no control.
        let got = body(&format!("[see](https://e.org/{})", marked(255)));
        assert_eq!(got.lines[0].text, "see");
        assert!(got.links.is_empty(), "a label is never drawn in part");
    }
}

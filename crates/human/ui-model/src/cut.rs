// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Where a long body stops in the conversation
//! (design/2026-11-04-long-message.md sections 4 and 6; HUMAN-CHAT.md,
//! Bounds, A 2026-11-04): a body that draws more than [`BODY_GRAPHEMES`]
//! characters or [`BODY_LINES`] lines is cut to a preview of at most
//! [`PREVIEW_GRAPHEMES`] and [`PREVIEW_LINES`], and the whole is drawn
//! only in the reader the person opens.
//!
//! What is counted is what is drawn, in the mode drawn: formatted, the
//! drawn text, every link control's label as shown (one line each) and
//! every image placeholder; as source, the source's characters and lines.
//! A line is a drawn line: a block, and one more for each line break the
//! text holds, since the text layout starts a line at every one -- a
//! hard-broken paragraph or a code block of many lines draws them all,
//! and a bubble that tall is one filled rectangle past the renderer's
//! ceiling (`a_hard_broken_paragraph_counts_each_line_it_draws`).

use interweave_human_chat_protocol::Rendered;
use unicode_segmentation::UnicodeSegmentation as _;

use crate::body::{self, Body, Kind, Line, Whole};

/// The most characters (grapheme clusters) a body draws whole.
pub const BODY_GRAPHEMES: usize = 2000;
/// The most drawn lines a body draws whole.
pub const BODY_LINES: usize = 40;
/// The most characters a cut body's preview draws.
pub const PREVIEW_GRAPHEMES: usize = 1600;
/// The most drawn lines a cut body's preview draws.
pub const PREVIEW_LINES: usize = 32;

/// What follows a preview cut inside a block, after its last word.
pub const ELLIPSIS: &str = "\u{2026}";

/// A body as the conversation draws it in one mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawn {
    /// The lines and links drawn: the whole body, or its preview.
    pub body: Body,
    /// Set when the body is cut: the share of its characters the preview
    /// shows, a whole percentage from 1 to 99 (section 4.2).
    pub shown_percent: Option<u8>,
}

/// `rendered` drawn formatted in the conversation: whole, or cut to its
/// preview. `image` names an image placeholder, as for
/// [`body::flatten`].
#[must_use]
pub fn formatted(rendered: &Rendered, image: &dyn Fn(&str) -> String) -> Drawn {
    let whole = body::whole(rendered, image);
    let labels: Vec<String> = whole
        .links
        .iter()
        .map(|(_, destination)| body::link_label(destination))
        .collect();
    // Each line's controls are charged to it, so a line the preview shows
    // whole shows its controls too, and one it cuts inside shows none: a
    // control only with its text and the whole of its label (section
    // 4.1).
    let mut controls = vec![Counts::default(); whole.lines.len()];
    for ((line, _), label) in whole.links.iter().zip(&labels) {
        if let Some(counts) = controls.get_mut(*line) {
            *counts = counts.plus_labels(std::slice::from_ref(label));
        }
    }
    let units = units(&whole, &controls);
    let counts = Counts::of(&units);
    if counts.within(BODY_GRAPHEMES, BODY_LINES) {
        return Drawn {
            body: body::pieced(whole),
            shown_percent: None,
        };
    }
    let preview = preview(&units);
    let lines = preview.lines(&whole);
    let shown_links = whole
        .links
        .iter()
        .filter(|(line, _)| *line < preview.whole_lines)
        .cloned()
        .collect();
    Drawn {
        body: body::pieced(Whole {
            lines,
            links: shown_links,
            plain: whole.plain,
        }),
        shown_percent: Some(percent(preview.counts, counts)),
    }
}

/// `source` drawn as source in the conversation: its pieces, whole or of
/// its preview, and the share shown when cut. Each character past the
/// cluster bound is one replacement.
#[must_use]
pub fn source(source: &str) -> (Vec<String>, Option<u8>) {
    let text = crate::cluster::drawable(source).into_owned();
    let whole = Whole {
        lines: vec![Line {
            kind: Kind::Code,
            depth: 0,
            quoted: false,
            marker: String::new(),
            text,
            join: body::Join::Starts,
        }],
        links: Vec::new(),
        plain: true,
    };
    let units = units(&whole, &[]);
    let counts = Counts::of(&units);
    let (lines, shown) = if counts.within(BODY_GRAPHEMES, BODY_LINES) {
        (whole.lines, None)
    } else {
        let preview = preview(&units);
        let shown = percent(preview.counts, counts);
        (preview.lines(&whole), Some(shown))
    };
    let pieces = lines
        .iter()
        .flat_map(|line| body::pieces(&line.text))
        .map(|(piece, _)| piece.to_owned())
        .collect();
    (pieces, shown)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Counts {
    graphemes: usize,
    lines: usize,
}

impl Counts {
    fn of(units: &[Unit<'_>]) -> Self {
        units
            .iter()
            .fold(Self::default(), |n, unit| n.plus(unit.cost()))
    }

    const fn plus(self, other: Self) -> Self {
        Self {
            graphemes: self.graphemes + other.graphemes,
            lines: self.lines + other.lines,
        }
    }

    /// Whether this is in the last quarter of the figure that cuts.
    const fn late(self, lines_cut: bool) -> bool {
        if lines_cut {
            self.lines * 4 >= PREVIEW_LINES * 3
        } else {
            self.graphemes * 4 >= PREVIEW_GRAPHEMES * 3
        }
    }

    /// With each label drawn: its characters, and a line of its own.
    fn plus_labels(self, labels: &[String]) -> Self {
        labels.iter().fold(self, |n, label| Self {
            graphemes: n.graphemes + label.graphemes(true).count(),
            lines: n.lines + 1,
        })
    }

    const fn within(self, graphemes: usize, lines: usize) -> bool {
        self.graphemes <= graphemes && self.lines <= lines
    }
}

/// The preview's share of the body, rounded down, from 1 to 99: of the
/// characters, or of the lines for a body that draws none (rules alone).
fn percent(shown: Counts, of: Counts) -> u8 {
    let (part, all) = if of.graphemes == 0 {
        (shown.lines, of.lines)
    } else {
        (shown.graphemes, of.graphemes)
    };
    let share = part.saturating_mul(100) / all.max(1);
    u8::try_from(share.clamp(1, 99)).unwrap_or(99)
}

/// A stretch the cut may end after: a line, or one line of a code block
/// or of the plain-text fallback, each of which ends at a block boundary
/// (section 4.1: a code line is one).
#[derive(Debug, Clone, Copy)]
struct Unit<'a> {
    /// The index of the line it is in.
    line: usize,
    /// Where in that line's text it starts, and its text.
    start: usize,
    text: &'a str,
    graphemes: usize,
    /// Drawn lines: one, and one more per line break inside it.
    lines: usize,
    /// On a line's last unit, the line's link controls.
    controls: Counts,
}

impl Unit<'_> {
    /// What it draws, its line's controls included on its last unit.
    const fn cost(&self) -> Counts {
        Counts {
            graphemes: self.graphemes,
            lines: self.lines,
        }
        .plus(self.controls)
    }
}

/// `whole` as units, `controls` giving each line's link controls.
fn units<'a>(whole: &'a Whole, controls: &[Counts]) -> Vec<Unit<'a>> {
    let mut out = Vec::new();
    for (index, line) in whole.lines.iter().enumerate() {
        let blocks = line.kind == Kind::Code || whole.plain;
        let mut start = 0;
        let mut graphemes = 0;
        let mut lines = 1;
        for (at, grapheme) in line.text.grapheme_indices(true) {
            // A break is a line, not a character drawn.
            if !body::is_line_break(grapheme) {
                graphemes += 1;
                continue;
            }
            if blocks {
                // The break ends this unit: the next starts after it.
                let end = at + grapheme.len();
                out.push(Unit {
                    line: index,
                    start,
                    text: &line.text[start..end],
                    graphemes,
                    lines: 1,
                    controls: Counts::default(),
                });
                start = end;
                graphemes = 0;
            } else {
                lines += 1;
            }
        }
        if start < line.text.len() || start == 0 {
            out.push(Unit {
                line: index,
                start,
                text: &line.text[start..],
                graphemes,
                lines,
                controls: Counts::default(),
            });
        }
        if let Some(last) = out.last_mut().filter(|unit| unit.line == index) {
            last.controls = controls.get(index).copied().unwrap_or_default();
        }
    }
    out
}

/// Where the preview stops.
struct Preview {
    /// Lines wholly in the preview, from the first.
    whole_lines: usize,
    /// The line it stops in, and the byte it stops at in that line's text,
    /// with or without an ellipsis; `None` when it stops at a line's end.
    stop: Option<(usize, usize, bool)>,
    /// What it shows, its ellipsis aside.
    counts: Counts,
}

impl Preview {
    /// The preview's lines: the whole ones, and the start of the one it
    /// stops in, ending with the ellipsis where it stops inside a block.
    fn lines(&self, whole: &Whole) -> Vec<Line> {
        let mut lines: Vec<Line> = whole.lines[..self.whole_lines].to_vec();
        if let Some((index, at, ellipsis)) = self.stop {
            let mut line = whole.lines[index].clone();
            line.text.truncate(at);
            if ellipsis {
                let kept = line.text.trim_end().len();
                line.text.truncate(kept);
                line.text.push_str(ELLIPSIS);
            } else {
                // A code line's own break, which nothing follows.
                while line
                    .text
                    .graphemes(true)
                    .next_back()
                    .is_some_and(body::is_line_break)
                {
                    let last = line
                        .text
                        .grapheme_indices(true)
                        .next_back()
                        .map_or(0, |(at, _)| at);
                    line.text.truncate(last);
                }
            }
            lines.push(line);
        }
        lines
    }
}

/// Where `units` stop within the preview's figures (section 4.1): at the
/// last block boundary inside them when it is in the last quarter of the
/// figure that cuts, else at the last word boundary inside them, then an
/// ellipsis; never inside a character. `units` pass the body's figures.
fn preview(units: &[Unit<'_>]) -> Preview {
    let mut taken = Counts::default();
    let mut boundary: Option<(usize, Counts)> = None;
    for (index, unit) in units.iter().enumerate() {
        let with = taken.plus(unit.cost());
        if with.within(PREVIEW_GRAPHEMES, PREVIEW_LINES) {
            taken = with;
            boundary = Some((index, taken));
            continue;
        }
        // The cut is in this unit. The figure that cuts is the one the
        // text reaches first, walking it.
        let (word, cap, lines_cut) = inside(unit, taken);
        if let Some((last, at)) = boundary.filter(|(_, at)| at.late(lines_cut)) {
            return stop_after(units, last, at);
        }
        // A word boundary as late as a block's must be, else between two
        // characters: a run of one long word -- a key, an encoded blob --
        // after an early space would otherwise preview as that space.
        let late_word = word.filter(|(_, counts)| counts.late(lines_cut));
        if let Some((end, counts)) = late_word.or(cap) {
            let whole_lines = unit.line;
            return Preview {
                whole_lines,
                stop: Some((unit.line, unit.start + end, true)),
                counts,
            };
        }
        // Nothing of this unit fits: the ellipsis follows the last unit.
        return match boundary {
            Some((last, at)) => {
                let mut preview = stop_after(units, last, at);
                let prev = &units[last];
                preview.whole_lines = prev.line;
                preview.stop = Some((prev.line, prev.start + prev.text.len(), true));
                preview
            }
            None => Preview {
                whole_lines: 0,
                stop: Some((unit.line, unit.start, true)),
                counts: Counts::default(),
            },
        };
    }
    // Every unit fits: not cut. Callers cut only past the body's figures,
    // which are above the preview's.
    Preview {
        whole_lines: units.last().map_or(0, |u| u.line + 1),
        stop: None,
        counts: taken,
    }
}

/// The preview ending after `units[last]`, at a block boundary.
fn stop_after(units: &[Unit<'_>], last: usize, at: Counts) -> Preview {
    let unit = &units[last];
    let ends_line = units
        .get(last + 1)
        .is_none_or(|next| next.line != unit.line);
    if ends_line {
        Preview {
            whole_lines: unit.line + 1,
            stop: None,
            counts: at,
        }
    } else {
        Preview {
            whole_lines: unit.line,
            stop: Some((unit.line, unit.start + unit.text.len(), false)),
            counts: at,
        }
    }
}

/// A place inside a unit, by its byte, with what is shown up to it.
type Stop = Option<(usize, Counts)>;

/// Inside `unit`, after `taken`: where the last word boundary inside the
/// preview's figures is -- the ellipsis counts in neither figure nor the
/// share, as the Chats preview's does (section 4.1) -- with what is
/// shown up to it; the same for the last character boundary; and whether
/// the figure that cuts is the lines'.
fn inside(unit: &Unit<'_>, taken: Counts) -> (Stop, Stop, bool) {
    let mut graphemes = taken.graphemes;
    let mut lines = taken.lines + 1;
    let mut cap = unit.text.len();
    let mut beyond = unit.text.len();
    let mut lines_cut = false;
    if lines > PREVIEW_LINES {
        // The unit's first line is already past the figure.
        return (None, None, true);
    }
    // The counts at each grapheme boundary, to read back the word cut's.
    let mut at_bound: Vec<(usize, Counts)> = vec![(0, Counts { graphemes, lines })];
    for (at, grapheme) in unit.text.grapheme_indices(true) {
        let breaks = body::is_line_break(grapheme);
        if !breaks && graphemes + 1 > PREVIEW_GRAPHEMES {
            cap = at;
            beyond = at + grapheme.len();
            break;
        }
        if breaks && lines + 1 > PREVIEW_LINES {
            cap = at;
            beyond = at + grapheme.len();
            lines_cut = true;
            break;
        }
        if breaks {
            lines += 1;
        } else {
            graphemes += 1;
        }
        at_bound.push((at + grapheme.len(), Counts { graphemes, lines }));
    }
    let end = unit.text[..beyond]
        .split_word_bound_indices()
        .rev()
        .find(|&(at, word)| at > 0 && at <= cap && !word.starts_with(char::is_whitespace))
        .map(|(at, _)| at);
    let counts_at = |end: usize| {
        at_bound
            .iter()
            .find(|(at, _)| *at == end)
            .map(|&(_, counts)| (end, counts))
    };
    // The white space before the word is neither drawn nor counted: the
    // ellipsis follows the last word directly (section 4.1).
    let word = end.and_then(|end| {
        let last_word = unit.text[..end].trim_end().len();
        counts_at(last_word).map(|(_, counts)| (end, counts))
    });
    let cap = (cap > 0).then(|| counts_at(cap)).flatten();
    (word, cap, lines_cut)
}

#[cfg(test)]
mod tests {
    use interweave_human_chat_protocol::render;
    use unicode_segmentation::UnicodeSegmentation as _;

    use super::{
        BODY_GRAPHEMES, BODY_LINES, Drawn, ELLIPSIS, PREVIEW_GRAPHEMES, PREVIEW_LINES, formatted,
        source,
    };
    use crate::body::{Join, link_label};

    fn drawn(text: &str) -> Drawn {
        formatted(&render(text), &|alt| format!("[image: {alt}]"))
    }

    /// The preview's text as drawn: its pieces rejoined.
    fn text(drawn: &Drawn) -> String {
        let mut out = String::new();
        for line in &drawn.body.lines {
            if !out.is_empty() && line.join != Join::MidLine {
                out.push('\n');
            }
            out.push_str(&line.text);
        }
        out
    }

    fn graphemes(text: &str) -> usize {
        text.graphemes(true).count()
    }

    #[test]
    fn a_body_at_both_figures_is_whole_and_one_more_cuts_it() {
        // A1: 2,000 characters in 40 paragraphs of 50.
        let para = "a".repeat(50);
        let at = vec![para.as_str(); BODY_LINES].join("\n\n");
        let whole = drawn(&at);
        assert_eq!(whole.shown_percent, None);
        assert_eq!(graphemes(&text(&whole).replace('\n', "")), BODY_GRAPHEMES);
        // One more character.
        assert!(drawn(&format!("{at}b")).shown_percent.is_some());
        // One more line, of no more characters than fit.
        let shorter = vec!["a".repeat(40); BODY_LINES].join("\n\n");
        assert_eq!(drawn(&shorter).shown_percent, None);
        assert!(drawn(&format!("{shorter}\n\nb")).shown_percent.is_some());
    }

    #[test]
    fn a_hard_broken_paragraph_counts_each_line_it_draws() {
        // One paragraph, 41 lines by hard breaks: one block, but 41 drawn.
        let lines = vec!["a"; BODY_LINES + 1].join("\\\n");
        assert!(drawn(&lines).shown_percent.is_some());
        let at = vec!["a"; BODY_LINES].join("\\\n");
        assert_eq!(drawn(&at).shown_percent, None);
    }

    #[test]
    fn link_labels_count_as_drawn_and_each_control_is_a_line() {
        // A body whose text is short but whose controls' labels draw past
        // the figure is cut.
        let dest = format!("https://example.org/{}", "p".repeat(100));
        let label = graphemes(&link_label(&dest));
        let n = BODY_GRAPHEMES / label + 1;
        let body = vec![format!("<{dest}>"); n].join(" ");
        // The text alone (each autolink's address) is under the figure ...
        assert!(n * (dest.len() + 1) <= BODY_GRAPHEMES);
        // ... and the labels take it over.
        assert!(drawn(&body).shown_percent.is_some());
        // 41 short links on one line: 41 controls' lines.
        let many = vec!["[a](https://e.org)"; BODY_LINES].join(" ");
        assert!(drawn(&many).shown_percent.is_some());
    }

    #[test]
    fn the_preview_is_within_its_figures_and_ends_at_a_block_in_the_last_quarter() {
        // Paragraphs of 100: the last whole one inside 1,600 is at 1,600.
        let body = vec!["a".repeat(100); 30].join("\n\n");
        let cut = drawn(&body);
        let shown = text(&cut);
        assert_eq!(graphemes(&shown.replace('\n', "")), PREVIEW_GRAPHEMES);
        assert!(!shown.ends_with(ELLIPSIS));
        assert_eq!(cut.shown_percent, Some(53));
    }

    #[test]
    fn a_boundary_before_the_last_quarter_gives_way_to_a_word_and_an_ellipsis() {
        // One short paragraph, then one long one: the only boundary is at
        // 5 characters, far from the last quarter.
        let words = vec!["word"; 600].join(" ");
        let cut = drawn(&format!("short\n\n{words}"));
        let shown = text(&cut);
        assert!(shown.ends_with(&format!("word{ELLIPSIS}")), "{shown:?}");
        // What is counted ends at the last word, not the space after it.
        let share = cut.shown_percent.expect("cut");
        let counted = graphemes(shown.replace('\n', "").trim_end_matches(ELLIPSIS));
        let all = graphemes(&format!("short{words}"));
        assert_eq!(usize::from(share), counted * 100 / all);
        let kept = shown.replace('\n', "");
        assert!(graphemes(kept.trim_end_matches(ELLIPSIS)) <= PREVIEW_GRAPHEMES);
        assert!(cut.shown_percent.is_some_and(|p| (1..=99).contains(&p)));
    }

    #[test]
    fn a_long_run_after_an_early_space_is_cut_between_characters() {
        // A2: "key: " and 3,000 hexadecimal digits, one word.
        let value = "0123456789abcdef".repeat(188)[..3000].to_owned();
        let cut = drawn(&format!("key: {value}"));
        let shown = text(&cut);
        let kept = shown.strip_suffix(ELLIPSIS).expect("an ellipsis ends it");
        assert_eq!(graphemes(kept), PREVIEW_GRAPHEMES, "{shown:.40}");
        assert_eq!(cut.shown_percent, Some(53));
    }

    #[test]
    fn lines_cut_by_the_line_figure_end_at_a_block() {
        let body = (1..=60)
            .map(|n| format!("- {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let cut = drawn(&body);
        assert_eq!(cut.body.lines.len(), PREVIEW_LINES);
        assert_eq!(cut.body.lines.last().map(|l| l.text.as_str()), Some("32"));
    }

    #[test]
    fn a_code_block_is_cut_between_its_lines() {
        let code = (1..=60)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let cut = drawn(&format!("```\n{code}\n```"));
        let shown = text(&cut);
        assert!(shown.ends_with("line 32"), "{shown:?}");
        assert!(!shown.contains(ELLIPSIS));
    }

    #[test]
    fn a_table_cut_between_rows_keeps_its_header() {
        let rows = (1..=60)
            .map(|n| format!("| {n} |"))
            .collect::<Vec<_>>()
            .join("\n");
        let cut = drawn(&format!("| head |\n|---|\n{rows}"));
        assert_eq!(cut.body.lines[0].text, "head");
        assert_eq!(cut.body.lines.len(), PREVIEW_LINES);
    }

    #[test]
    fn a_link_control_is_in_the_preview_only_with_its_text_and_whole_label() {
        let words = vec!["word"; 600].join(" ");
        let cut = drawn(&format!(
            "[early](https://e.org/a)\n\n{words} [late](https://e.org/b)"
        ));
        assert_eq!(cut.body.links, ["https://e.org/a"]);
        // The share counts the shown label.
        let without = drawn(&format!("early\n\n{words} [late](https://e.org/b)"));
        assert!(cut.shown_percent >= without.shown_percent);
    }

    #[test]
    fn a_control_whose_label_would_pass_the_preview_is_left_to_the_reader() {
        // 1,584 characters of text, then a link of one character whose
        // control's label takes the preview past 1,600.
        let dest = "https://example.org/a-path-long-enough";
        let label = graphemes(&link_label(dest));
        assert!(1_585 + label > PREVIEW_GRAPHEMES, "control: past with it");
        let text = vec!["a".repeat(99); 16].join("\n\n");
        let cut = drawn(&format!("{text}\n\n[x]({dest})\n\n{}", "b".repeat(600)));
        assert!(cut.body.links.is_empty(), "no control past the figures");
        let counted: usize = cut
            .body
            .lines
            .iter()
            .map(|l| graphemes(l.text.trim_end_matches(ELLIPSIS)))
            .sum::<usize>()
            + cut
                .body
                .links
                .iter()
                .map(|d| graphemes(&link_label(d)))
                .sum::<usize>();
        assert!(counted <= PREVIEW_GRAPHEMES, "{counted}");
    }

    #[test]
    fn the_cut_never_falls_inside_a_character() {
        // 2,400 characters of a letter with a combining mark, no spaces.
        let body = "e\u{301}".repeat(2400);
        let cut = drawn(&body);
        let shown = text(&cut);
        let kept = shown.trim_end_matches(ELLIPSIS);
        assert!(kept.graphemes(true).all(|g| g == "e\u{301}"));
        assert_eq!(graphemes(kept), PREVIEW_GRAPHEMES);
    }

    #[test]
    fn a_replaced_cluster_counts_as_one_character() {
        // A12 C1 and C2: a 256-code-point cluster opening a paragraph.
        let cluster = format!("a{}", "\u{301}".repeat(255));
        let c1 = format!("{cluster}{}", "b".repeat(BODY_GRAPHEMES - 1));
        let whole = drawn(&c1);
        assert_eq!(whole.shown_percent, None);
        assert!(text(&whole).starts_with('\u{FFFD}'));
        let c2 = format!("{cluster}{}", "b".repeat(2399));
        let cut = drawn(&c2);
        // The mark and 1,599 b's, then the ellipsis: 1,600 / 2,400 -- not
        // the word boundary after the mark, 1 / 2,400.
        assert_eq!(cut.shown_percent, Some(66));
        assert_eq!(
            graphemes(text(&cut).trim_end_matches(ELLIPSIS)),
            PREVIEW_GRAPHEMES
        );
    }

    #[test]
    fn source_is_cut_by_its_own_lines_and_characters() {
        // Whole formatted, long as source: reference definitions draw
        // nothing formatted.
        let defs = (1..=60)
            .map(|n| format!("[r{n}]: https://e.org/{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(drawn(&defs).shown_percent, None);
        let (pieces, shown) = source(&defs);
        assert!(shown.is_some());
        let joined = pieces.concat();
        assert_eq!(joined.lines().count(), PREVIEW_LINES);
        assert!(joined.ends_with("[r32]: https://e.org/32"));
        let (whole, none) = source("short\nsource");
        assert_eq!((whole, none), (vec!["short\nsource".to_owned()], None));
    }

    #[test]
    fn the_plain_text_fallback_is_cut_over_its_source_lines() {
        // 17 nested quotes pass the nesting bound: the fallback.
        let deep = format!("{} x", ">".repeat(17));
        let lines = vec![deep.as_str(); 60].join("\n");
        let cut = drawn(&lines);
        assert!(cut.shown_percent.is_some());
        assert_eq!(text(&cut).lines().count(), PREVIEW_LINES);
    }

    #[test]
    fn rules_alone_take_their_share_from_lines() {
        let rules = vec!["---"; 50].join("\n\n");
        let cut = drawn(&rules);
        assert_eq!(cut.body.lines.len(), PREVIEW_LINES);
        assert_eq!(cut.shown_percent, Some(64));
    }
}

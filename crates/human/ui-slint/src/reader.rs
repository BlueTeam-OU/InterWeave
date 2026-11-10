// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The reader (design/2026-11-04-long-message.md section 5): a cut
//! message's whole body, opened by the person's press, in one scroll whose
//! rows the toolkit lays out as they come near the screen (its `ListView`
//! rule; how near, and at what cost, is A5's measurement). Its rows are the body's lines
//! and links, or its source's pieces, each within the figures a whole
//! body may have (HUMAN-CHAT.md, Bounds: "in pieces each within the
//! figures a whole body may have"), and any the layout measures past
//! the item extent split again by that measure (`lib.rs` `by_measure`;
//! A 2026-11-07), so no row draws an item past it, whatever its font
//! draws (`a4_shapes` in `tests/long_message.rs`).

use interweave_human_ui_model::{
    Day, ItemKey, MessageItem, UiText, body, cluster, fill, placeholder_en,
};
use std::cell::RefCell;

use slint::{ModelRc, SharedString, VecModel};

use crate::clock::{self, Clock};
use crate::generated::{BodyLine, ReaderRow, ReaderRowKind};

/// The message the reader shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Open {
    /// The message.
    pub(crate) key: ItemKey,
    /// Its row's handle, which focus returns to (section 9).
    pub(crate) handle: i32,
}

/// The reader's rows in both forms, and its links' destinations by
/// index. The formatted form is built as the reader opens; the source's
/// when first asked for ([`Forms::source`]) -- Show source, or the frame
/// after the reader's first, whichever comes first -- since a body at the
/// bound costs as much to measure again as source as it did formatted,
/// and the person waits on the opening for both (A5).
pub(crate) struct Forms {
    pub(crate) formatted: ModelRc<ReaderRow>,
    source: Source,
    pub(crate) links: Vec<String>,
    /// Each link's row in the formatted form: right after the line its
    /// text is in (section 5.1), in the order the body gives them.
    pub(crate) link_rows: Vec<usize>,
}

/// The source form's rows, or what builds them.
enum Source {
    Built(ModelRc<ReaderRow>),
    Pending(Box<dyn FnOnce() -> ModelRc<ReaderRow>>),
    /// Taken by [`Forms::source`] while it builds.
    Building,
}

impl Forms {
    /// The formatted form, the source's left to `build`.
    pub(crate) fn new(
        (formatted, links, link_rows): (ModelRc<ReaderRow>, Vec<String>, Vec<usize>),
        build: impl FnOnce() -> ModelRc<ReaderRow> + 'static,
    ) -> Self {
        Self {
            formatted,
            source: Source::Pending(Box::new(build)),
            links,
            link_rows,
        }
    }

    /// The open reader's source rows, built first if they were not:
    /// outside `forms`' borrow, since building measures in the window,
    /// and the window's handlers borrow it too. `None` when no reader is
    /// open, or one that is building.
    pub(crate) fn source(forms: &RefCell<Option<Self>>) -> Option<ModelRc<ReaderRow>> {
        let build = {
            let mut forms = forms.borrow_mut();
            let forms = forms.as_mut()?;
            match std::mem::replace(&mut forms.source, Source::Building) {
                Source::Built(rows) => {
                    forms.source = Source::Built(rows.clone());
                    return Some(rows);
                }
                Source::Pending(build) => build,
                Source::Building => return None,
            }
        };
        let rows = build();
        if let Some(forms) = forms.borrow_mut().as_mut() {
            forms.source = Source::Built(rows.clone());
        }
        Some(rows)
    }
}

/// When `local_at` reached this app, as the reader's first line shows it
/// and as it is spoken (section 8, `ReaderSent`): Today, Yesterday, else
/// the date; then the time as the row shows it.
pub(crate) fn sent(clock: &Clock, local_at: u64) -> (String, String) {
    let local = clock.local(local_at);
    let day = match local.day {
        Day::Today | Day::Yesterday => clock::day_text(local.day),
        // Never a weekday's name: the date, so it reads the same tomorrow
        // (tests::the_sent_line_names_a_date_where_the_list_names_a_weekday).
        Day::Weekday(_) | Day::Date { .. } => clock.date_text(local_at),
    };
    let time = clock::clock_text(local.hour, local.minute, clock.hour24);
    let shown = fill(
        placeholder_en::text(UiText::ReaderSent),
        &[("day", &day), ("time", &time)],
    );
    // The full date's own form is the look spec's `DateFull`, which the
    // build does not have yet: spoken as shown until it does.
    (shown.clone(), shown)
}

/// `item`'s formatted rows after the line `sent`, its links'
/// destinations, and each link's row.
pub(crate) fn formatted(
    item: &MessageItem,
    sent: &(String, String),
    image: &dyn Fn(&str) -> String,
    lines: impl Fn(&[body::Line]) -> Vec<BodyLine>,
    label: impl Fn(&str) -> ModelRc<SharedString>,
) -> (ModelRc<ReaderRow>, Vec<String>, Vec<usize>) {
    let (drawn, places) = body::flatten_placed(&item.body, image);
    let mut formatted = vec![sent_row(sent)];
    let as_rows = |lines: Vec<BodyLine>| {
        lines.into_iter().map(|line| ReaderRow {
            kind: ReaderRowKind::Line,
            line,
            text: SharedString::new(),
            spoken: SharedString::new(),
            index: 0,
            pieces: ModelRc::default(),
        })
    };
    // Each link's control after the line that holds it: the lines are
    // measured a run at a time, each run ending where a control comes,
    // which breaks the run there as the control does on screen.
    let mut next = 0;
    let mut link_rows = Vec::with_capacity(drawn.links.len());
    for (index, (destination, &place)) in drawn.links.iter().zip(&places).enumerate() {
        if place >= next && place < drawn.lines.len() {
            formatted.extend(as_rows(lines(&drawn.lines[next..=place])));
            next = place + 1;
        }
        let text = body::link_label(destination);
        link_rows.push(formatted.len());
        formatted.push(ReaderRow {
            kind: ReaderRowKind::Link,
            line: BodyLine::default(),
            pieces: label(&text),
            text: text.into(),
            spoken: SharedString::new(),
            index: i32::try_from(index).unwrap_or(i32::MAX),
        });
    }
    formatted.extend(as_rows(lines(&drawn.lines[next..])));
    (
        ModelRc::new(VecModel::from(formatted)),
        drawn.links,
        link_rows,
    )
}

/// `source`'s rows after the line `sent`.
pub(crate) fn source(
    source: &str,
    sent: &(String, String),
    pieces: impl Fn(Vec<String>) -> Vec<String>,
) -> ModelRc<ReaderRow> {
    let text = cluster::drawable(source);
    let mut rows = vec![sent_row(sent)];
    let source_pieces = pieces(
        body::pieces(&text)
            .into_iter()
            .map(|(piece, _)| piece.to_owned())
            .collect(),
    );
    rows.extend(source_pieces.into_iter().map(|piece| ReaderRow {
        kind: ReaderRowKind::Source,
        line: BodyLine::default(),
        text: piece.into(),
        spoken: SharedString::new(),
        index: 0,
        pieces: ModelRc::default(),
    }));
    ModelRc::new(VecModel::from(rows))
}

/// The reader's first row: when the message came, as [`sent`] gives it.
fn sent_row(sent: &(String, String)) -> ReaderRow {
    ReaderRow {
        kind: ReaderRowKind::Sent,
        line: BodyLine::default(),
        text: sent.0.as_str().into(),
        spoken: sent.1.as_str().into(),
        index: 0,
        pieces: ModelRc::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::sent;
    use crate::clock::Clock;

    #[test]
    fn the_sent_line_names_a_date_where_the_list_names_a_weekday() {
        const DAY: u64 = 86_400_000;
        // 2026-10-17 14:30 UTC.
        let now = 20_743 * DAY + 14 * 3_600_000 + 30 * 60_000;
        let clock = Clock::new(move || now, |_| 0);
        assert_eq!(clock.list_time(now - 2 * DAY), "Thursday", "control");
        assert_eq!(sent(&clock, now - 2 * DAY).0, "Oct 15, 14:30");
        assert_eq!(sent(&clock, now).0, "Today, 14:30");
        assert_eq!(sent(&clock, now - DAY).0, "Yesterday, 14:30");
    }
}

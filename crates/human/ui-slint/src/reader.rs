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

/// The text, in bytes, one [`Formatting::step`] measures at most, a line
/// or a link control counted at least [`ITEM_BYTES`]: a body at the
/// bound measured in one go holds the window for a second or more (A5),
/// so an opening builds by time in whole steps, and the window draws
/// between its turns, the opening's bar among what it draws (section 3,
/// `ReaderOpening`; the budgets are `OPENING_FIRST_GO` and
/// `OPENING_TURN` in `lib.rs`).
pub(crate) const STEP_BYTES: usize = 32 * 1024;
const ITEM_BYTES: usize = 64;

/// `item`'s formatted rows after the line `sent`, built a step at a time:
/// each link's control after the line that holds it, the lines measured
/// a run at a time, each run ending where a control comes, which breaks
/// the run there as the control does on screen. A run is also cut
/// between steps, but only before a line that starts afresh, so the rows
/// are those one step would give.
pub(crate) struct Formatting {
    drawn: body::Body,
    places: Vec<usize>,
    rows: Vec<ReaderRow>,
    link_rows: Vec<usize>,
    /// The next link to place.
    link: usize,
    /// The first line not yet in `rows`.
    next: usize,
}

impl Formatting {
    pub(crate) fn new(
        item: &MessageItem,
        sent: &(String, String),
        image: &dyn Fn(&str) -> String,
    ) -> Self {
        let (drawn, places) = body::flatten_placed(&item.body, image);
        Self {
            link_rows: Vec::with_capacity(drawn.links.len()),
            drawn,
            places,
            rows: vec![sent_row(sent)],
            link: 0,
            next: 0,
        }
    }

    /// Build on for about `room` bytes of text ([`STEP_BYTES`] a turn,
    /// `usize::MAX` for all of it); whether every line and control is in.
    pub(crate) fn step(
        &mut self,
        mut room: usize,
        lines: impl Fn(&[body::Line]) -> Vec<BodyLine>,
        label: impl Fn(&str) -> ModelRc<SharedString>,
    ) -> bool {
        loop {
            if room == 0 {
                return false;
            }
            let count = self.drawn.lines.len();
            let run_end = match self.places.get(self.link) {
                Some(&place) if place >= self.next && place < count => place + 1,
                Some(_) => self.next,
                None => count,
            };
            if self.next < run_end {
                let end = self.cut(run_end, &mut room);
                self.rows
                    .extend(as_rows(lines(&self.drawn.lines[self.next..end])));
                self.next = end;
                continue;
            }
            let Some(destination) = self.drawn.links.get(self.link) else {
                return true;
            };
            let text = body::link_label(destination);
            room = room.saturating_sub(text.len().max(ITEM_BYTES));
            self.link_rows.push(self.rows.len());
            self.rows.push(ReaderRow {
                kind: ReaderRowKind::Link,
                line: BodyLine::default(),
                pieces: label(&text),
                text: text.into(),
                spoken: SharedString::new(),
                index: i32::try_from(self.link).unwrap_or(i32::MAX),
            });
            self.link += 1;
        }
    }

    /// Where this step's part of the run from `next` to `end` ends: as
    /// many lines as `room` holds, one at least, and then on to a line
    /// that starts afresh, since a piece is measured with its line.
    fn cut(&self, end: usize, room: &mut usize) -> usize {
        let lines = &self.drawn.lines;
        let mut at = self.next;
        while at < end && (*room > 0 || at == self.next || lines[at].join != body::Join::Starts) {
            *room = room.saturating_sub(lines[at].text.len().max(ITEM_BYTES));
            at += 1;
        }
        at
    }

    /// The rows, the links' destinations, and each link's row.
    pub(crate) fn done(self) -> (ModelRc<ReaderRow>, Vec<String>, Vec<usize>) {
        (
            ModelRc::new(VecModel::from(self.rows)),
            self.drawn.links,
            self.link_rows,
        )
    }
}

fn as_rows(lines: Vec<BodyLine>) -> impl Iterator<Item = ReaderRow> {
    lines.into_iter().map(|line| ReaderRow {
        kind: ReaderRowKind::Line,
        line,
        text: SharedString::new(),
        spoken: SharedString::new(),
        index: 0,
        pieces: ModelRc::default(),
    })
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

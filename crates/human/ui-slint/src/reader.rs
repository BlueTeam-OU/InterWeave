// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The reader (design/2026-11-04-long-message.md section 5): a cut
//! message's whole body, opened by the person's press, in one scroll whose
//! rows the toolkit lays out as they come near the screen (its `ListView`
//! rule; how near, and at what cost, is A5's measurement). Its rows are the body's lines
//! and links, or its source's pieces, each within the figures a whole
//! body may have (HUMAN-CHAT.md, Bounds: "in pieces each within the
//! figures a whole body may have"), so no row is taller than a whole
//! body at the bound.

use interweave_human_ui_model::{
    Day, ItemKey, MessageItem, UiText, body, cluster, fill, placeholder_en,
};
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

/// The reader's rows in both forms, built once as it opens so that Show
/// source and Show formatted swap them at once, and its links'
/// destinations by index.
pub(crate) struct Forms {
    pub(crate) formatted: ModelRc<ReaderRow>,
    pub(crate) source: ModelRc<ReaderRow>,
    pub(crate) links: Vec<String>,
}

/// When `local_at` reached this app, as the reader's first line shows it
/// and as it is spoken (section 8, `ReaderSent`): Today, Yesterday, else
/// the date; then the time as the row shows it.
pub(crate) fn sent(clock: &Clock, local_at: u64) -> (String, String) {
    let local = clock.local(local_at);
    let day = match local.day {
        Day::Today | Day::Yesterday => clock::day_text(local.day),
        // Never a weekday's name: the date, so it reads the same tomorrow.
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

/// `item`'s rows, formatted and as source, after the line `sent`.
pub(crate) fn forms(
    item: &MessageItem,
    sent: &(String, String),
    image: &dyn Fn(&str) -> String,
    lines: impl Fn(&[body::Line]) -> Vec<BodyLine>,
) -> Forms {
    let first = ReaderRow {
        kind: ReaderRowKind::Sent,
        line: BodyLine::default(),
        text: sent.0.as_str().into(),
        spoken: sent.1.as_str().into(),
        index: 0,
    };
    let drawn = body::flatten(&item.body, image);
    let mut formatted = vec![first.clone()];
    formatted.extend(lines(&drawn.lines).into_iter().map(|line| ReaderRow {
        kind: ReaderRowKind::Line,
        line,
        text: SharedString::new(),
        spoken: SharedString::new(),
        index: 0,
    }));
    formatted.extend(
        drawn
            .links
            .iter()
            .enumerate()
            .map(|(index, destination)| ReaderRow {
                kind: ReaderRowKind::Link,
                line: BodyLine::default(),
                text: body::link_label(destination).into(),
                spoken: SharedString::new(),
                index: i32::try_from(index).unwrap_or(i32::MAX),
            }),
    );
    let text = cluster::drawable(&item.source);
    let mut source = vec![first];
    source.extend(body::pieces(&text).into_iter().map(|(piece, _)| ReaderRow {
        kind: ReaderRowKind::Source,
        line: BodyLine::default(),
        text: piece.into(),
        spoken: SharedString::new(),
        index: 0,
    }));
    Forms {
        formatted: ModelRc::new(VecModel::from(formatted)),
        source: ModelRc::new(VecModel::from(source)),
        links: drawn.links,
    }
}

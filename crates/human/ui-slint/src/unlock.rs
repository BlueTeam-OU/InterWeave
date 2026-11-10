// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Trust page's unlock lines and what their changes say
//! (design/2026-11-05-unlock-policy-pending.md sections 5 and 6): the
//! words of each line, its focus key, and -- from the lines the last
//! render drew -- the announcement and the focus a change asks.

use interweave_human_ui_model::{
    KeyUnlock, PendingWay, UiModel, UiText, UnlockEnded, UnlockLine, UnlockLineKind, UnlockReason,
    fill, placeholder_en, unlock_change,
};

use crate::generated::UnlockRow;

/// A line's focus key: its kind, so a line whose words change keeps
/// focus by the same key (F9).
pub(crate) const fn key(kind: UnlockLineKind) -> &'static str {
    match kind {
        UnlockLineKind::RestartNeedsUnlock => "unlock-d",
        UnlockLineKind::Pending => "unlock-p",
        UnlockLineKind::OldKeyRemains => "unlock-k",
    }
}

/// Where focus goes when the line it was on clears and none takes its
/// place (F7).
const PEER_ID: &str = "own-peer-id";

/// A line's words: a P line is its first sentence, its reason and that
/// InterWeave tries again, each a sentence of its own (section 7).
pub(crate) fn text(line: UnlockLine) -> String {
    let t = placeholder_en::text;
    match line {
        UnlockLine::RestartNeedsUnlock { locked: false } => {
            t(UiText::UnlockRestartNeedsUnlock).to_owned()
        }
        UnlockLine::RestartNeedsUnlock { locked: true } => {
            t(UiText::UnlockRestartNeedsUnlockLocked).to_owned()
        }
        UnlockLine::Pending { way, reason } => {
            let head = t(match way {
                PendingWay::NeedsLock => UiText::UnlockPendingNeedsLock,
                PendingWay::NoLock => UiText::UnlockPendingNoLock,
                PendingWay::Unknown => UiText::UnlockPendingUnknown,
            });
            let reason = t(match reason {
                UnlockReason::Storage => UiText::UnlockReasonStorage,
                UnlockReason::Keystore => UiText::UnlockReasonKeystore,
                UnlockReason::Other => UiText::UnlockReasonOther,
            });
            [head, reason, t(UiText::UnlockTriesAgain)].join(" ")
        }
        UnlockLine::OldKeyRemains => t(UiText::UnlockOldKeyRemains).to_owned(),
    }
}

/// What opened the Trust page: the app's options, or the P-up notice's
/// Details.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum OpenedBy {
    #[default]
    Menu,
    Details,
}

/// A Trust page Details opened: its P-up line's focus still to ask, or
/// asked; `No` for a page opened otherwise, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Details {
    No,
    Wanted,
    Asked,
}

/// What a render of the lines asks of the window.
#[derive(Debug, Default)]
pub(crate) struct Drawn {
    /// The rows to show, when they changed.
    pub(crate) rows: Option<Vec<UnlockRow>>,
    /// What to say, politely, once.
    pub(crate) said: Option<String>,
    /// Where focus goes, when a line it was on was drawn again or went.
    pub(crate) focus: Option<String>,
}

/// The lines as the last render drew them, and the policy in force then.
#[derive(Debug, Default)]
pub(crate) struct Lines {
    drawn: Vec<UnlockLine>,
    in_force: Option<KeyUnlock>,
}

impl Lines {
    /// Draw `model`'s lines. A change is said only `on_trust`, the page
    /// open (F6, F7, F9); off it the lines are taken silently. `focus_id`
    /// is the control focus is on: on a line whose rows are made again,
    /// focus goes back to it by its kind, else to the line now in its
    /// place, else to the `PeerId` field (F7, F9).
    pub(crate) fn draw(&mut self, model: &UiModel, on_trust: bool, focus_id: &str) -> Drawn {
        let now = model.unlock_lines();
        let in_force = model.unlock().map(|facts| facts.in_force);
        if now == self.drawn {
            self.in_force = in_force;
            return Drawn::default();
        }
        let change = unlock_change((&self.drawn, self.in_force), (&now, in_force));
        let said = on_trust
            .then(|| {
                let ended = change.ended.map(|ended| {
                    placeholder_en::text(match ended {
                        UnlockEnded::Applied => UiText::UnlockApplied,
                        UnlockEnded::OldKeyRemoved => UiText::UnlockOldKeyRemoved,
                    })
                    .to_owned()
                });
                ended
                    .into_iter()
                    .chain(change.said.iter().map(|line| text(*line)))
                    .reduce(|first, rest| {
                        fill(
                            placeholder_en::text(UiText::AnnounceBoth),
                            &[("first", &first), ("rest", &rest)],
                        )
                    })
            })
            .flatten();
        let at = self
            .drawn
            .iter()
            .position(|line| key(line.kind()) == focus_id);
        let focus = at.map(|at| {
            let kind = self.drawn[at].kind();
            now.iter()
                .find(|line| line.kind() == kind)
                .or_else(|| now.get(at))
                .or_else(|| now.last())
                .map_or(PEER_ID, |line| key(line.kind()))
                .to_owned()
        });
        let rows = now
            .iter()
            .map(|line| UnlockRow {
                key: key(line.kind()).into(),
                text: text(*line).into(),
            })
            .collect();
        self.drawn = now;
        self.in_force = in_force;
        Drawn {
            rows: Some(rows),
            said,
            focus,
        }
    }
}

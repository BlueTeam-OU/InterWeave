// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Whether InterWeave needs the screen lock after a restart, and a change
//! of that setting not yet in force (`design/2026-11-05-unlock-policy-pending.md`;
//! ADR-0041 A 2026-11-05, ADR-0042 A 2026-11-05). Pure: the platform says
//! the four facts of the note's section 3 ([`UnlockFacts`]), and this
//! module says which state they are in, which lines the Trust page shows,
//! and what a change between two sets of lines announces. Android only:
//! the desktop has no unlock policy, and its platform never says any.

/// The `key_unlock_policy` a record is under, or the one configured: the
/// profile's names, kept here so the model depends on no configuration
/// crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyUnlock {
    /// The record opens after a restart without the person.
    BackgroundCompatible,
    /// The record opens only with the person's screen lock.
    UserPresence,
}

/// Why a reseal toward the configured policy did not finish: the note's
/// reasons r (section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnlockReason {
    /// The phone could not save the change.
    Storage,
    /// The phone's secure key storage did not accept it.
    Keystore,
    /// Anything else: no cause is offered.
    Other,
}

/// What the last start's reseal left, if it did not finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResealLeft {
    /// The reseal failed for this reason.
    Failed(UnlockReason),
    /// The record is under the configured policy, and the other policy's
    /// key remains (an unsynced write included, which the platform
    /// reports so).
    OldKeyRemains,
}

/// The four facts of the note's section 3, as the platform says them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnlockFacts {
    /// The policy of the record the last start left on disk; while
    /// locked, the record header's.
    pub in_force: KeyUnlock,
    /// The configured policy; `None` when the profile could not be read.
    pub configured: Option<KeyUnlock>,
    /// What the reseal left, if anything.
    pub pending: Option<ResealLeft>,
    /// `background_restart_requires_user_authentication` under the
    /// effective availability mode and the policy in force; `None` when
    /// the mode cannot be read, and then D cannot be claimed.
    pub restart_needs_unlock: Option<bool>,
}

/// Which way a pending change goes, for its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PendingWay {
    /// P-up: configured user-presence, a background-compatible record in
    /// force. The only state that reaches Chats.
    NeedsLock,
    /// P-down: configured background-compatible, a user-presence record
    /// in force.
    NoLock,
    /// P-unknown: no direction can be named.
    Unknown,
}

/// One line of the Trust page's identity section (section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnlockLine {
    /// D: a restart needs the screen lock; `locked` while InterWeave is
    /// locked, where "while it keeps running" is not true.
    RestartNeedsUnlock {
        /// Said while the locked notice shows.
        locked: bool,
    },
    /// A P line: what is not in force, the reason, and that InterWeave
    /// tries again.
    Pending {
        /// Which P line.
        way: PendingWay,
        /// Its reason sentence.
        reason: UnlockReason,
    },
    /// K: the setting is in force, and the old key remains.
    OldKeyRemains,
}

/// A line's place by kind: a line whose words change keeps its kind, so
/// focus on it stays (section 6, F9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnlockLineKind {
    /// D's line.
    RestartNeedsUnlock,
    /// A P line, whichever way.
    Pending,
    /// K's line.
    OldKeyRemains,
}

impl UnlockLine {
    /// The line's kind.
    #[must_use]
    pub const fn kind(self) -> UnlockLineKind {
        match self {
            Self::RestartNeedsUnlock { .. } => UnlockLineKind::RestartNeedsUnlock,
            Self::Pending { .. } => UnlockLineKind::Pending,
            Self::OldKeyRemains => UnlockLineKind::OldKeyRemains,
        }
    }
}

/// What a line's ending announces before anything else is said (F7, F9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnlockEnded {
    /// A P line ended, or D ended because the policy in force changed:
    /// the setting is now in force.
    Applied,
    /// The K line ended: the old key was removed.
    OldKeyRemoved,
}

/// What changed between two renders' lines: the announcement, in order
/// -- the ending first, then each line new or changed (F6, F7, F9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlockChange {
    /// What a line's ending announces, if one ended.
    pub ended: Option<UnlockEnded>,
    /// Each line now shown that was not shown so before, in order.
    pub said: Vec<UnlockLine>,
}

impl UnlockFacts {
    /// Whether a change is pending: in force and configured differ.
    fn differs(self) -> bool {
        self.configured.is_some_and(|c| c != self.in_force)
    }

    /// D's condition: user-presence in force under stay-reachable, as the
    /// platform computed it. Unknown is never D.
    const fn d(self) -> bool {
        matches!(self.restart_needs_unlock, Some(true))
    }

    /// The lines of section 3's table, in its order: the P or K line
    /// first, D's under it where the state shows both.
    #[must_use]
    pub fn lines(&self, locked: bool) -> Vec<UnlockLine> {
        let d = UnlockLine::RestartNeedsUnlock { locked };
        let reason = match self.pending {
            Some(ResealLeft::Failed(reason)) => Some(reason),
            Some(ResealLeft::OldKeyRemains) | None => None,
        };
        if self.differs() {
            // A mismatch the reseal gave no class for (`Held`) is shown
            // with reason Other; OldKeyRemains cannot come with a
            // mismatch, since it is said only of a record written under
            // the configured policy, and is shown so too. D's line is
            // never under a P-up or P-down line: the P line says what a
            // restart does now (section 3, Q1).
            let way = if self.configured == Some(KeyUnlock::UserPresence) {
                PendingWay::NeedsLock
            } else {
                PendingWay::NoLock
            };
            return vec![UnlockLine::Pending {
                way,
                reason: reason.unwrap_or(UnlockReason::Other),
            }];
        }
        let mut lines = Vec::with_capacity(2);
        match self.pending {
            Some(ResealLeft::OldKeyRemains) => lines.push(UnlockLine::OldKeyRemains),
            Some(ResealLeft::Failed(reason)) => lines.push(UnlockLine::Pending {
                way: PendingWay::Unknown,
                reason,
            }),
            None => {}
        }
        if self.d() {
            lines.push(d);
        }
        lines
    }

    /// Whether this is P-up, the one state whose notice reaches Chats
    /// (section 4).
    #[must_use]
    pub fn needs_lock_pending(&self) -> bool {
        self.differs() && self.configured == Some(KeyUnlock::UserPresence)
    }
}

/// What changed from `before` to `now`, each the lines a render showed
/// with the policy in force then (`None` with no facts): a P line's
/// ending announces [`UnlockEnded::Applied`], K's
/// [`UnlockEnded::OldKeyRemoved`], D's `Applied` only when the policy in
/// force changed -- never when the mode did -- and then each line new or
/// with changed words is said (F6, F7, F9).
#[must_use]
pub fn change(
    before: (&[UnlockLine], Option<KeyUnlock>),
    now: (&[UnlockLine], Option<KeyUnlock>),
) -> UnlockChange {
    let had = |lines: &[UnlockLine], kind| lines.iter().any(|l| l.kind() == kind);
    let ended_kind = |kind| had(before.0, kind) && !had(now.0, kind);
    let ended = if ended_kind(UnlockLineKind::Pending) {
        Some(UnlockEnded::Applied)
    } else if ended_kind(UnlockLineKind::OldKeyRemains) {
        Some(UnlockEnded::OldKeyRemoved)
    } else if ended_kind(UnlockLineKind::RestartNeedsUnlock)
        && before.1.is_some()
        && now.1.is_some()
        && before.1 != now.1
    {
        Some(UnlockEnded::Applied)
    } else {
        None
    };
    let said = now
        .0
        .iter()
        .filter(|line| !before.0.contains(line))
        .copied()
        .collect();
    UnlockChange { ended, said }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BG: KeyUnlock = KeyUnlock::BackgroundCompatible;
    const UP: KeyUnlock = KeyUnlock::UserPresence;

    fn facts(
        in_force: KeyUnlock,
        configured: Option<KeyUnlock>,
        pending: Option<ResealLeft>,
        d: Option<bool>,
    ) -> UnlockFacts {
        UnlockFacts {
            in_force,
            configured,
            pending,
            restart_needs_unlock: d,
        }
    }

    const fn d(locked: bool) -> UnlockLine {
        UnlockLine::RestartNeedsUnlock { locked }
    }

    const fn p(way: PendingWay, reason: UnlockReason) -> UnlockLine {
        UnlockLine::Pending { way, reason }
    }

    #[test]
    fn each_row_of_section_3s_table_has_its_lines_in_order() {
        let storage = Some(ResealLeft::Failed(UnlockReason::Storage));
        // N, and D.
        assert!(
            facts(BG, Some(BG), None, Some(false))
                .lines(false)
                .is_empty()
        );
        assert_eq!(
            facts(UP, Some(UP), None, Some(true)).lines(false),
            [d(false)]
        );
        assert_eq!(facts(UP, Some(UP), None, Some(true)).lines(true), [d(true)]);
        // P-up and P-down: no D, whatever the diagnostic says.
        assert_eq!(
            facts(BG, Some(UP), storage, Some(true)).lines(false),
            [p(PendingWay::NeedsLock, UnlockReason::Storage)]
        );
        assert_eq!(
            facts(UP, Some(BG), storage, Some(true)).lines(false),
            [p(PendingWay::NoLock, UnlockReason::Storage)]
        );
        // A mismatch with no class (Held) is Other.
        assert_eq!(
            facts(BG, Some(UP), None, Some(false)).lines(false),
            [p(PendingWay::NeedsLock, UnlockReason::Other)]
        );
        // P-unknown, configured unknown or equal, D under it exactly when
        // D's condition holds.
        assert_eq!(
            facts(UP, None, storage, Some(true)).lines(false),
            [p(PendingWay::Unknown, UnlockReason::Storage), d(false)]
        );
        assert_eq!(
            facts(UP, Some(UP), storage, Some(false)).lines(false),
            [p(PendingWay::Unknown, UnlockReason::Storage)]
        );
        // K, with D under it when D holds.
        let k = Some(ResealLeft::OldKeyRemains);
        assert_eq!(
            facts(BG, Some(BG), k, Some(false)).lines(false),
            [UnlockLine::OldKeyRemains]
        );
        assert_eq!(
            facts(UP, Some(UP), k, Some(true)).lines(false),
            [UnlockLine::OldKeyRemains, d(false)]
        );
        // The mode unknown: never D.
        assert!(facts(UP, Some(UP), None, None).lines(true).is_empty());
    }

    #[test]
    fn only_p_up_needs_the_notice() {
        assert!(facts(BG, Some(UP), None, None).needs_lock_pending());
        assert!(!facts(UP, Some(BG), None, None).needs_lock_pending());
        assert!(
            !facts(
                UP,
                None,
                Some(ResealLeft::Failed(UnlockReason::Other)),
                None
            )
            .needs_lock_pending()
        );
        assert!(!facts(UP, Some(UP), Some(ResealLeft::OldKeyRemains), None).needs_lock_pending());
    }

    #[test]
    fn an_ending_is_said_first_then_each_new_or_changed_line() {
        let up = p(PendingWay::NeedsLock, UnlockReason::Storage);
        // P-up to D.
        assert_eq!(
            change((&[up], Some(BG)), (&[d(false)], Some(UP))),
            UnlockChange {
                ended: Some(UnlockEnded::Applied),
                said: vec![d(false)],
            }
        );
        // K with D to D: the ending, and nothing new.
        assert_eq!(
            change(
                (&[UnlockLine::OldKeyRemains, d(false)], Some(UP)),
                (&[d(false)], Some(UP))
            ),
            UnlockChange {
                ended: Some(UnlockEnded::OldKeyRemoved),
                said: vec![],
            }
        );
        // A reason changed: its line said again, nothing ended.
        let keystore = p(PendingWay::NeedsLock, UnlockReason::Keystore);
        assert_eq!(
            change((&[up], Some(BG)), (&[keystore], Some(BG))),
            UnlockChange {
                ended: None,
                said: vec![keystore],
            }
        );
        // D cleared by the policy in force, and by the mode.
        assert_eq!(
            change((&[d(false)], Some(UP)), (&[], Some(BG))).ended,
            Some(UnlockEnded::Applied)
        );
        assert_eq!(change((&[d(false)], Some(UP)), (&[], Some(UP))).ended, None);
        // The locked text to the running one.
        assert_eq!(
            change((&[d(true)], Some(UP)), (&[d(false)], Some(UP))),
            UnlockChange {
                ended: None,
                said: vec![d(false)],
            }
        );
    }
}

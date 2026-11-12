// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A K1 kept for a window (onboard.rs, `recovery_refused`): what
//! `Native.identityState` answers for it, in a process that found the
//! identity ready before, ahead of reading the record (`ready_state`),
//! and what a flow's end does with it, by where the flow was opened
//! (`render`); and whether `FLAG_SECURE` may go off when its off is
//! asked (`secure_off`). On every target, so the host tests it.

/// The answer before the record is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Before {
    /// The client runs and holds its identity: ready, nothing asked.
    Ready,
    /// A start in this process answered `RecoveryRequired` and its K1 is
    /// still owed, its window gone before it ended or none there to take
    /// it: recovery, so the Activity sets `FLAG_SECURE` before the window
    /// draws and binds nothing until K1's end restarts the service.
    Recovery,
    /// Read the record: only a key that needs the person changes the
    /// answer.
    ReadRecord,
}

/// The answer for a check with the client `running` or not, and with a
/// K1 `kept` for a window or not.
pub(crate) const fn before_reading(running: bool, kept: bool) -> Before {
    if running {
        Before::Ready
    } else if kept {
        Before::Recovery
    } else {
        Before::ReadRecord
    }
}

/// Where an identity flow was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Over {
    /// At a launch, before the service: its end binds it.
    Launch,
    /// Show my phrase, from Settings: the service is bound already, and
    /// is taken offline and back rather than bound (design section 3.5).
    Settings,
    /// K1 raised over a window whose service is bound and whose start was
    /// refused (`raise_recovery`): its end starts the service again, as
    /// R's Try again does, rather than binding it.
    Running,
}

impl Over {
    /// Opened from the start the window was given, a launch's or a kept
    /// K1's, rather than by Settings.
    const fn opened_from_start(self) -> bool {
        !matches!(self, Self::Settings)
    }
}

/// What a flow's end does with the start the window was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtFlowEnd {
    /// No start is owed any more: the flow that showed it ended, or none
    /// was kept.
    Drop,
    /// The K1 stays owed and the start the flow asked answers it: refused
    /// again, K1 rises; run, it is dropped. A start held for the person's
    /// Unlock answers once it is granted; the locked notice shows meanwhile.
    Keep,
    /// The K1 stays owed and nothing will start the service: K1 rises now.
    Raise,
}

/// The end of a flow opened `over` a launch, Settings (Show my phrase) or
/// a running window (K1), with a K1 `kept` or not, and the service asked
/// to start at its end or not (Show my phrase left before it stopped it).
pub(crate) const fn at_flow_end(over: Over, kept: bool, start_asked: bool) -> AtFlowEnd {
    if over.opened_from_start() || !kept {
        AtFlowEnd::Drop
    } else if start_asked {
        AtFlowEnd::Keep
    } else {
        AtFlowEnd::Raise
    }
}

/// Whether `FLAG_SECURE` may go off now, with an identity flow showing
/// or not and Show my address open or not: only with neither, since each
/// draws what the flag secures and asks the off again at its own end.
pub(crate) const fn secure_may_clear(flow_shows: bool, show_open: bool) -> bool {
    !flow_shows && !show_open
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A K1 kept past its window opens as recovery at the next check,
    /// never as ready (#611 review F1); a running client is ready
    /// whatever was kept.
    #[test]
    fn a_kept_recovery_answers_recovery_until_the_client_runs() {
        assert_eq!(before_reading(false, true), Before::Recovery);
        assert_eq!(before_reading(false, false), Before::ReadRecord);
        assert_eq!(before_reading(true, true), Before::Ready);
        assert_eq!(before_reading(true, false), Before::Ready);
    }

    /// Show my phrase ending keeps a K1 a refused start left while it
    /// showed (#611 review 4, pre-existing 2): raised at once when its
    /// end starts nothing, left to the start's answer when it does. K1's
    /// own end, or a launch's, drops it.
    #[test]
    fn show_my_phrase_ending_keeps_a_kept_recovery() {
        use Over::{Launch, Running, Settings};
        assert_eq!(at_flow_end(Settings, true, false), AtFlowEnd::Raise);
        assert_eq!(at_flow_end(Settings, true, true), AtFlowEnd::Keep);
        assert_eq!(at_flow_end(Settings, false, true), AtFlowEnd::Drop);
        assert_eq!(at_flow_end(Settings, false, false), AtFlowEnd::Drop);
        for over in [Launch, Running] {
            assert_eq!(at_flow_end(over, true, true), AtFlowEnd::Drop);
            assert_eq!(at_flow_end(over, true, false), AtFlowEnd::Drop);
            assert_eq!(at_flow_end(over, false, true), AtFlowEnd::Drop);
        }
    }

    /// The flag stays on while a flow shows (K1 raised in the turn Show
    /// my phrase ended, #615 F1) or Show my address is open (under a K1
    /// a restore ended), and goes off only once neither draws.
    #[test]
    fn the_flag_goes_off_only_once_nothing_it_secures_is_drawn() {
        assert!(secure_may_clear(false, false));
        assert!(!secure_may_clear(true, false), "a flow shows");
        assert!(!secure_may_clear(false, true), "Show is open");
        assert!(!secure_may_clear(true, true));
    }
}

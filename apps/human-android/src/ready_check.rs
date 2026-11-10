// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A K1 kept for a window (onboard.rs, `recovery_refused`): what
//! `Native.identityState` answers for it, in a process that found the
//! identity ready before, ahead of reading the record (`ready_state`),
//! and what a flow's end does with it (`render`). On every target, so the
//! host tests it.

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

/// What a flow's end does with the start the window was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtFlowEnd {
    /// No start is owed any more: the flow that showed it ended, or none
    /// was kept.
    Drop,
    /// The K1 stays owed and the start the flow asked answers it: refused
    /// again, K1 rises; run, it is dropped.
    Keep,
    /// The K1 stays owed and nothing will start the service: K1 rises now.
    Raise,
}

/// The end of a flow opened `from_start` (a launch's flow or K1) or not
/// (Show my phrase), with a K1 `kept` or not, and the service asked to
/// start at its end or not (Show my phrase left before it stopped it).
pub(crate) const fn at_flow_end(from_start: bool, kept: bool, start_asked: bool) -> AtFlowEnd {
    if from_start || !kept {
        AtFlowEnd::Drop
    } else if start_asked {
        AtFlowEnd::Keep
    } else {
        AtFlowEnd::Raise
    }
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
        assert_eq!(at_flow_end(false, true, false), AtFlowEnd::Raise);
        assert_eq!(at_flow_end(false, true, true), AtFlowEnd::Keep);
        assert_eq!(at_flow_end(false, false, true), AtFlowEnd::Drop);
        assert_eq!(at_flow_end(false, false, false), AtFlowEnd::Drop);
        assert_eq!(at_flow_end(true, true, true), AtFlowEnd::Drop);
        assert_eq!(at_flow_end(true, true, false), AtFlowEnd::Drop);
        assert_eq!(at_flow_end(true, false, true), AtFlowEnd::Drop);
    }
}

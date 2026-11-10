// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What `Native.identityState` answers, in a process that found the
//! identity ready before, ahead of reading the record (onboard.rs,
//! `ready_state`): on every target, so the host tests it.

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
}

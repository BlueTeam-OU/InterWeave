// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! TEST BUILDS ONLY: the window's trace (ui-slint's `trace.rs`, kinds
//! only, never text, an address or a `PeerId`) on stderr, each line
//! stamped with the host's monotonic clock. The desktop end-to-end case
//! of A5 (`tests/desktop-e2e/tests/human_app/opening.rs`) sends its keys
//! from another process and stamps each send on that clock too, so the
//! two hold together the way the phone's uptime clock holds a
//! `KeySender` press against a monotonic logcat. `Instant` is the same
//! clock on Linux but cannot be read as a number, hence `clock_gettime`.
//! Compiled under this app's `ui-trace` feature alone, which no shipped
//! build turns on.

use rustix::time::{ClockId, clock_gettime};

/// What starts a stamped line on stderr, before the trace line itself.
pub(crate) const STAMP: &str = "human-desktop: mono-us=";

/// `line` as it goes to stderr, stamped `at_us` microseconds on the
/// monotonic clock.
pub(crate) fn stamped(at_us: u128, line: &str) -> String {
    format!("{STAMP}{at_us} {line}")
}

/// Now on `CLOCK_MONOTONIC`, in microseconds.
pub(crate) fn now_us() -> u128 {
    let t = clock_gettime(ClockId::Monotonic);
    // Both fields are non-negative for this clock (clock_gettime(2)).
    u128::from(t.tv_sec.unsigned_abs()) * 1_000_000 + u128::from(t.tv_nsec.unsigned_abs()) / 1_000
}

/// Write `line` to stderr, stamped now.
pub(crate) fn say(line: &str) {
    eprintln!("{}", stamped(now_us(), line));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_its_stamp_then_the_trace_line() {
        assert_eq!(
            stamped(1_235_004_017, "ui-trace page=reader"),
            "human-desktop: mono-us=1235004017 ui-trace page=reader"
        );
    }

    #[test]
    fn the_stamp_advances_in_microseconds() {
        let before = now_us();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let after = now_us();
        assert!(
            (20_000..2_000_000).contains(&(after - before)),
            "{before}..{after}"
        );
    }
}

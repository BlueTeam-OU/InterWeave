// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The debug build's start hook (`DebugHooks` launch extra
//! `holdStartMs`, jni.rs): the next service start waits that long before
//! it runs, once -- add-a-peer §8 item 12's case of a start that has not
//! answered within the bound. On every target, so the host tests that it
//! is taken once.

use std::time::Duration;

/// The hold armed by [`arm`], in milliseconds; zero, nothing is held.
/// Built for tests too, so the workspace's own run, which turns no
/// feature of this crate on, runs `the_start_hook_is_taken_once`; a
/// release library has neither (`verifyReleaseNative`).
#[cfg(any(test, feature = "dev-stand-ins"))]
static HOLD_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Arm the hook: the next [`take`] answers `hold`.
#[cfg(any(test, feature = "dev-stand-ins"))]
pub(crate) fn arm(hold: Duration) {
    HOLD_MS.store(
        u64::try_from(hold.as_millis()).unwrap_or(u64::MAX),
        std::sync::atomic::Ordering::SeqCst,
    );
}

/// How long the next start waits, taken once
/// (`the_start_hook_is_taken_once`); zero, and always in the release
/// build, it runs at once.
pub(crate) fn take() -> Duration {
    #[cfg(any(test, feature = "dev-stand-ins"))]
    return Duration::from_millis(HOLD_MS.swap(0, std::sync::atomic::Ordering::SeqCst));
    #[cfg(not(any(test, feature = "dev-stand-ins")))]
    Duration::ZERO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_hook_is_taken_once() {
        assert_eq!(take(), Duration::ZERO, "unarmed");
        arm(Duration::from_secs(12));
        assert_eq!(take(), Duration::from_secs(12));
        assert_eq!(take(), Duration::ZERO, "once");
    }
}

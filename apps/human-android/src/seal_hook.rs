// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The debug build's seal hook (`DebugHooks` launch extra
//! `sealAlreadyProvisioned`, jni.rs): C4's next seal is answered
//! `AlreadyProvisioned` without sealing, once, so `IdAlreadyCreated` shows
//! and the clipboard removal can be checked on a phone (identity note
//! section 10 item 18, the Pixel 9a check). On every target, so the host
//! tests that it is taken once.

/// Armed by [`arm`]: C4's next seal answers `AlreadyProvisioned`. Built for
/// tests too, so the workspace's own test run, which turns no feature of
/// this crate on, runs `the_seal_hook_is_taken_once`; a release library has
/// neither (`verifyReleaseNative`).
#[cfg(any(test, feature = "dev-stand-ins"))]
static ALREADY_PROVISIONED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Arm the hook: the next [`take`] answers true.
#[cfg(any(test, feature = "dev-stand-ins"))]
pub(crate) fn arm() {
    ALREADY_PROVISIONED.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Whether this seal is to be answered `AlreadyProvisioned` unsealed,
/// taken once (`the_seal_hook_is_taken_once`); false, and always in the
/// release build, the seal runs.
pub(crate) fn take() -> bool {
    #[cfg(any(test, feature = "dev-stand-ins"))]
    return ALREADY_PROVISIONED.swap(false, std::sync::atomic::Ordering::SeqCst);
    #[cfg(not(any(test, feature = "dev-stand-ins")))]
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mutation: `take` reading without clearing answers true twice.
    #[test]
    fn the_seal_hook_is_taken_once() {
        assert!(!take(), "unarmed");
        arm();
        assert!(take());
        assert!(!take(), "once");
    }
}

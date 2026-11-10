// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The debug build's identity hook (`DebugHooks.refuseNextIdentity`, jni.rs):
//! the next start answers `RecoveryRequired`, once. On every target, so the
//! host tests that it is taken once.

use interweave_human_android_platform::IdentityRefused;

/// Armed by [`arm`]: the next start's identity answers `RecoveryRequired`.
/// Built for tests too, so the workspace's own test run, which turns no
/// feature of this crate on, runs `the_identity_hook_is_taken_once`; a
/// release library has neither (`verifyReleaseNative`).
#[cfg(any(test, feature = "dev-stand-ins"))]
static REFUSE_IDENTITY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Arm the hook: the next [`take`] answers the refusal.
#[cfg(any(test, feature = "dev-stand-ins"))]
pub(crate) fn arm() {
    REFUSE_IDENTITY.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// The refusal the hook stands in for, taken once
/// (`the_identity_hook_is_taken_once`); `None`, and always in the release
/// build, the record is read.
pub(crate) fn take() -> Option<IdentityRefused> {
    #[cfg(any(test, feature = "dev-stand-ins"))]
    if REFUSE_IDENTITY.swap(false, std::sync::atomic::Ordering::SeqCst) {
        return Some(IdentityRefused::RecoveryRequired(
            interweave_human_android_platform::custody::RecoveryCause::KeyInvalidated,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use interweave_human_android_platform::custody::RecoveryCause;

    #[test]
    fn the_identity_hook_is_taken_once() {
        assert_eq!(take(), None, "unarmed");
        arm();
        assert_eq!(
            take(),
            Some(IdentityRefused::RecoveryRequired(
                RecoveryCause::KeyInvalidated
            ))
        );
        assert_eq!(take(), None, "once");
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! C4's `IdStartUnchecked` (design/2026-10-19-android-identity.md section
//! 6, A start from C4 after a failed read-back) crosses threads: the flow
//! asks its start on the window's thread, and the Service runs it on its
//! own (jni.rs, `start`), which must compare the identity it unlocks with
//! C4's `PeerId` before anything runs as it. What that start is told to
//! expect, and how its refusal answers the flow, are decided here. On
//! every target, so the host tests it.

use interweave_human_android_platform::IdentityRefused;
use interweave_human_android_platform::custody::RecoveryCause;
use interweave_human_ui_model::{Start, UncheckedRefused};
use interweave_transport_api::TransportIdentity;

/// The `PeerId` every start of this process must open the record to once
/// C4's `IdStartUnchecked` was pressed (design section 6, The comparison
/// outlasts the bound): said by the flow at the press and at each render
/// while C4 holds for that start ([`Self::said`]), and taken away by
/// nothing but a start that compared with it, opened the record to it and
/// ran ([`Self::answered`]). The flow's end, a later screen, a recreation,
/// a launch in this process and every refusal leave it standing, so Try
/// again, a return's retry, Unlock's start and a new launch's start all
/// compare. It is a value of its own, held in a static apart from the
/// parked flow (`flow_park.rs`) and from the Activity's saved state, so a
/// process the system ended and restored compares nothing.
/// `the_comparison_outlasts_the_flow_until_a_start_opens_to_it` holds it.
#[derive(Debug, Default)]
pub(crate) struct ExpectedStart {
    peer: Option<TransportIdentity>,
}

impl ExpectedStart {
    /// An empty slot, for a static.
    pub(crate) const fn new() -> Self {
        Self { peer: None }
    }

    /// The flow, shown, says what its start expects: a `PeerId` is held
    /// from then on (the press's, which a later press would replace);
    /// `None` -- any other screen, the flow's end -- changes nothing.
    pub(crate) fn said(&mut self, expects: Option<&TransportIdentity>) {
        if let Some(peer) = expects {
            self.peer = Some(peer.clone());
        }
    }

    /// A start that was told to expect `compared` answered, `ran` when
    /// the service runs: the record opened to that `PeerId`, so nothing is
    /// expected of the next start. Any other answer -- a mismatch, a
    /// refusal, a start that compared nothing -- leaves it.
    pub(crate) fn answered(&mut self, compared: Option<&TransportIdentity>, ran: bool) {
        if ran && compared.is_some() && compared == self.peer.as_ref() {
            self.peer = None;
        }
    }

    /// What a start beginning now must open the record to.
    pub(crate) fn for_start(&self) -> Option<TransportIdentity> {
        self.peer.clone()
    }
}

/// How a start told to expect a `PeerId` answers the flow when its
/// identity is refused (design section 6, Its results): another `PeerId`
/// than C4's, and any refusal that says nothing of the record, are C4's
/// `IdStorageUnknown` again -- the first with Try again only -- and a
/// record that does not open is K1, built by `recovery` as at a launch.
/// A refusal for the person is not here (`None`): the screen's prompt
/// answers it, as for any held start. With no C4 held for the start --
/// the bound passed, or any later start of the process -- the first two
/// show as Chats' `Refused` instead (jni.rs, `refused_compared`; section
/// 6, An answer after the bound).
pub(crate) fn unchecked_answer(
    refused: &IdentityRefused,
    recovery: impl FnOnce(&RecoveryCause) -> Start,
) -> Option<UncheckedRefused> {
    match refused {
        IdentityRefused::NotExpected(_) => Some(UncheckedRefused::OtherIdentity),
        IdentityRefused::RecoveryRequired(cause) => match recovery(cause) {
            Start::Recovery {
                reason,
                expected,
                new_keys,
            } => Some(UncheckedRefused::Recovery {
                reason,
                expected,
                new_keys,
            }),
            Start::FirstRun { .. } | Start::ShowPhrase => Some(UncheckedRefused::Unavailable),
        },
        IdentityRefused::UserNotAuthenticated => None,
        // No record where the read-back found one, the paths, the
        // cipher, the storage: "try again", never K1 (section 6).
        IdentityRefused::Unprovisioned
        | IdentityRefused::Paths(_)
        | IdentityRefused::Unavailable(_)
        | IdentityRefused::Provisioning(_) => Some(UncheckedRefused::Unavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interweave_human_ui_model::RecoveryReason;

    fn peer(id: &str) -> TransportIdentity {
        TransportIdentity::parse(id).expect("a PeerId")
    }

    fn mine() -> TransportIdentity {
        peer("12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e")
    }

    fn other() -> TransportIdentity {
        peer("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN")
    }

    /// Nothing is expected before a press; from it, every start compares
    /// with C4's `PeerId` whatever the flow says next -- its end, another
    /// screen -- and whatever answers: a mismatch, a refusal, the profile
    /// in use, a start that ran without comparing, a start that compared
    /// with another `PeerId` and ran. Only a start that compared with it
    /// and ran takes it away; a later press replaces it (design section
    /// 6, The comparison outlasts the bound). Mutations: `said(None)`
    /// clearing it, `answered` clearing on any `ran`, on a refusal, or
    /// for a start that compared with another `PeerId`, fail here.
    #[test]
    fn the_comparison_outlasts_the_flow_until_a_start_opens_to_it() {
        let mut slot = ExpectedStart::new();
        assert_eq!(slot.for_start(), None, "nothing at first");
        slot.answered(None, true);
        assert_eq!(slot.for_start(), None, "a start before any press");
        slot.said(None);
        assert_eq!(slot.for_start(), None, "a screen that expects nothing");

        slot.said(Some(&mine()));
        assert_eq!(slot.for_start(), Some(mine()), "the press");
        slot.said(None);
        assert_eq!(slot.for_start(), Some(mine()), "the flow ended");
        slot.answered(Some(&mine()), false);
        assert_eq!(
            slot.for_start(),
            Some(mine()),
            "refused: Try again compares"
        );
        slot.answered(None, true);
        assert_eq!(
            slot.for_start(),
            Some(mine()),
            "a start that compared nothing"
        );
        slot.answered(Some(&other()), true);
        assert_eq!(slot.for_start(), Some(mine()), "compared with another");
        slot.answered(Some(&mine()), true);
        assert_eq!(slot.for_start(), None, "opened to it and ran");
        slot.said(None);
        assert_eq!(slot.for_start(), None, "and stays clear");

        slot.said(Some(&mine()));
        slot.said(Some(&other()));
        assert_eq!(slot.for_start(), Some(other()), "a later press");
    }

    /// A start told to expect C4's `PeerId` answers the flow as section 6
    /// says: another `PeerId` fails closed on C4, a record that does not
    /// open is K1 with the record's reason and `PeerId`, a refusal that
    /// says nothing of the record is C4's notice again, and a refusal for
    /// the person is the prompt's, not the flow's. Mutation: any arm
    /// mapped to another answer fails here.
    #[test]
    fn a_refused_unchecked_start_answers_as_a_launch_and_fails_closed() {
        let never = |_: &RecoveryCause| -> Start { panic!("not a recovery") };
        assert_eq!(
            unchecked_answer(&IdentityRefused::NotExpected(mine()), never),
            Some(UncheckedRefused::OtherIdentity)
        );
        assert_eq!(
            unchecked_answer(&IdentityRefused::UserNotAuthenticated, never),
            None
        );
        for unavailable in [
            IdentityRefused::Unprovisioned,
            IdentityRefused::Paths("x".to_owned()),
            IdentityRefused::Unavailable("x".to_owned()),
            IdentityRefused::Provisioning("x".to_owned()),
        ] {
            assert_eq!(
                unchecked_answer(&unavailable, never),
                Some(UncheckedRefused::Unavailable),
                "{unavailable:?}"
            );
        }
        let recovery = unchecked_answer(
            &IdentityRefused::RecoveryRequired(RecoveryCause::KeyInvalidated),
            |cause| {
                assert_eq!(*cause, RecoveryCause::KeyInvalidated, "its cause");
                Start::Recovery {
                    reason: RecoveryReason::Key,
                    expected: Some(mine()),
                    new_keys: true,
                }
            },
        );
        assert_eq!(
            recovery,
            Some(UncheckedRefused::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: true,
            })
        );
    }
}

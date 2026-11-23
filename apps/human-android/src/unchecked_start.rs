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

/// The `PeerId` the Service's next start must open the record to, as the
/// flow last said it ([`interweave_human_ui_model::IdentityFlow::start_expects`]).
/// Said at the ask and at every render, so a start a showing retries
/// under `IdStartBusy` reads it too, and kept past the flow's end while
/// the flow's start has not answered: a start the bound left running
/// behind Chats still compares (`a_start_in_flight_keeps_its_peer_id_past_the_flows_end`).
#[derive(Debug, Default)]
pub(crate) struct ExpectedStart {
    peer: Option<TransportIdentity>,
}

impl ExpectedStart {
    /// An empty slot, for a static.
    pub(crate) const fn new() -> Self {
        Self { peer: None }
    }

    /// The flow, shown, says what its start expects now: `None` for a
    /// start that compares nothing.
    pub(crate) fn shown(&mut self, expects: Option<&TransportIdentity>) {
        self.peer = expects.cloned();
    }

    /// The flow ended. A start it asked that has not answered yet keeps
    /// what it expects, which its answer then clears ([`Self::answered`]);
    /// with none in flight nothing is expected any more.
    pub(crate) fn flow_ended(&mut self, start_in_flight: bool) {
        if !start_in_flight {
            self.peer = None;
        }
    }

    /// A start answered. With no flow shown any more nothing is expected
    /// of the next start: a later one, an Unlock's or a launch's, opens
    /// the record to whatever it holds, as any launch does (design section
    /// 6, Its results). While a flow shows, it says what it expects.
    pub(crate) fn answered(&mut self, flow_shows: bool) {
        if !flow_shows {
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
/// answers it, as for any held start.
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

    /// The slot says what the flow last said, and the flow's end clears
    /// it only once no start of the flow is in flight; that start's
    /// answer then clears it, so no later start -- an Unlock's, a
    /// launch's -- is compared. A start answered while a flow shows
    /// leaves the flow's word standing. Mutations: `flow_ended` clearing
    /// whatever is in flight, or `answered` keeping the slot with no flow
    /// shown, fail here.
    #[test]
    fn a_start_in_flight_keeps_its_peer_id_past_the_flows_end() {
        let mut slot = ExpectedStart::new();
        assert_eq!(slot.for_start(), None, "nothing at first");
        slot.shown(Some(&mine()));
        assert_eq!(slot.for_start(), Some(mine()));
        slot.answered(true);
        assert_eq!(slot.for_start(), Some(mine()), "the flow still shows");
        slot.flow_ended(true);
        assert_eq!(slot.for_start(), Some(mine()), "its start still runs");
        slot.answered(false);
        assert_eq!(slot.for_start(), None, "answered: no later start compares");

        slot.shown(Some(&mine()));
        slot.flow_ended(false);
        assert_eq!(slot.for_start(), None, "nothing in flight");
        slot.shown(Some(&mine()));
        slot.shown(None);
        assert_eq!(slot.for_start(), None, "the flow's own word");
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

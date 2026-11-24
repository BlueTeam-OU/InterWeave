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
/// while C4 holds for that start ([`Self::said`]), and taken away by a
/// start that compared with it, opened the record to it and ran
/// ([`Self::answered`]) -- for good: a render of the same C4 between that
/// start's run and the root's decision says it again, and is not heard
/// (#750 review r3 F2) -- or by R4's Continue after K1, a restore the
/// person committed ([`Self::restored`]; architect-cto's ruling on Q17).
/// The flow's end, a later screen, a recreation, a launch in this process
/// and every refusal leave it standing, so Try again, a return's retry,
/// Unlock's start and a new launch's start all compare. It is a value of
/// its own, held in a static apart from the parked flow (`flow_park.rs`)
/// and from the Activity's saved state, so a process the system ended and
/// restored compares nothing.
/// `the_comparison_outlasts_the_flow_until_a_start_opens_to_it`,
/// `a_compared_start_that_ran_is_not_expected_again` and
/// `a_restore_committed_after_k1_ends_the_comparison` hold it.
#[derive(Debug, Default)]
pub(crate) struct ExpectedStart {
    peer: Option<TransportIdentity>,
    /// The `PeerId` a start that compared with it opened the record to
    /// and ran as: never expected again in this process
    /// (`a_compared_start_that_ran_is_not_expected_again`).
    ran: Option<TransportIdentity>,
}

impl ExpectedStart {
    /// An empty slot, for a static.
    pub(crate) const fn new() -> Self {
        Self {
            peer: None,
            ran: None,
        }
    }

    /// The flow, shown, says what its start expects: a `PeerId` is held
    /// from then on (the press's, which a later press would replace);
    /// `None` -- any other screen, the flow's end -- changes nothing, and
    /// neither does the `PeerId` a compared start already ran as.
    pub(crate) fn said(&mut self, expects: Option<&TransportIdentity>) {
        if let Some(peer) = expects
            && self.ran.as_ref() != Some(peer)
        {
            self.peer = Some(peer.clone());
        }
    }

    /// A start that was told to expect `compared` answered, `ran` when
    /// the service runs: the record opened to that `PeerId`, so nothing is
    /// expected of the next start, nor ever again of that `PeerId`. Any
    /// other answer -- a mismatch, a refusal, a start that compared
    /// nothing -- leaves it.
    pub(crate) fn answered(&mut self, compared: Option<&TransportIdentity>, ran: bool) {
        if ran && compared.is_some() && compared == self.peer.as_ref() {
            self.ran = self.peer.take();
        }
    }

    /// R4's Continue after K1 was pressed (`IdentityFlow::start_ends_comparison`):
    /// its start, and every later one of the process, compares nothing,
    /// and nothing is written of it. Said before that start reads
    /// [`Self::for_start`].
    pub(crate) fn restored(&mut self) {
        self.peer = None;
    }

    /// What a start beginning now must open the record to.
    pub(crate) fn for_start(&self) -> Option<TransportIdentity> {
        self.peer.clone()
    }
}

/// How a start told to expect a `PeerId` was refused for its identity,
/// with the hub's count of start answers read as it was said -- before
/// its own answer is counted. A held screen takes it only for the start
/// it holds for: one whose answer is counted past the screen's `asked_at`
/// ([`Self::for_hold`]), the rule `ViewSide::start_decided` reads the
/// hub's answer by. No later screen step clears it, so an answer that
/// lands before a return's or a recreation's hold is taken up -- the
/// hold's `asked_at` was read before that start was made -- and one an
/// earlier start left is not (#750 review r1 F1).
/// `an_answer_is_taken_by_the_hold_of_its_own_start` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UncheckedAnswer {
    refused: UncheckedRefused,
    before: u64,
}

impl UncheckedAnswer {
    /// `refused`, said with `before` the hub's count of answers then.
    pub(crate) const fn new(refused: UncheckedRefused, before: u64) -> Self {
        Self { refused, before }
    }

    /// The answer, for a hold whose start was asked with the hub's count
    /// at `asked_at`: this start's, or a later one's, when it was said
    /// with at least that count; an earlier start's otherwise.
    pub(crate) fn for_hold(self, asked_at: u64) -> Option<UncheckedRefused> {
        (self.before >= asked_at).then_some(self.refused)
    }
}

/// How a start told to `expect` a `PeerId` answers the flow when its
/// identity is refused (design section 6, Its results): another `PeerId`
/// than C4's, and any refusal that says nothing of the record, are C4's
/// `IdStorageUnknown` again -- the first with Try again only (ui-model's
/// `every_compared_start_held_on_c4_answers_on_c4`) -- and a record that
/// does not open is K1, built by `recovery` as at a launch. A refusal
/// for the person is not here (`None`): the screen's prompt answers it,
/// as for any held start. A start told to expect nothing answers no
/// flow (`None`): its refusal hands a held screen over as before
/// (`a_start_that_compared_nothing_answers_no_flow`). A C4 held for the
/// start answers it whichever C4 start it was
/// (`IdentityFlow::answers_compared`); with none -- the bound passed, or
/// a later start of the process made from no C4 -- the first two show as
/// Chats' `Refused` instead (jni.rs, `refused_compared`; section 6, An
/// answer after the bound).
pub(crate) fn unchecked_answer(
    expect: Option<&TransportIdentity>,
    refused: &IdentityRefused,
    recovery: impl FnOnce(&RecoveryCause) -> Start,
) -> Option<UncheckedRefused> {
    expect?;
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
        // cipher, the storage: "try again", never K1 (section 6;
        // `a_refused_unchecked_start_answers_as_a_launch_and_fails_closed`
        // maps each to `Unavailable` with a recovery that panics).
        IdentityRefused::Unprovisioned
        | IdentityRefused::Paths(_)
        | IdentityRefused::Unavailable(_)
        | IdentityRefused::Provisioning(_) => Some(UncheckedRefused::Unavailable),
    }
}

/// How a start told to `expect` a `PeerId` answers the flow when the
/// profile could not be provisioned before its identity was read: a
/// refusal that says nothing of the record, `Unavailable` (section 6, Its
/// results); nothing for a start told to expect nothing
/// (`a_start_that_compared_nothing_answers_no_flow`).
pub(crate) fn unprovisioned_answer(expect: Option<&TransportIdentity>) -> Option<UncheckedRefused> {
    expect.map(|_| UncheckedRefused::Unavailable)
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

    /// Once a start that compared with a `PeerId` ran, nothing from the
    /// same press arms it again: a render of the held C4 between that
    /// start's run and the root's decision says it once more (#750 review
    /// r3 F2). Another `PeerId` is still expected when said. Mutation:
    /// `answered` forgetting the `PeerId` it ran as, or `said` not reading
    /// it, fails here.
    #[test]
    fn a_compared_start_that_ran_is_not_expected_again() {
        let mut slot = ExpectedStart::new();
        slot.said(Some(&mine()));
        slot.answered(Some(&mine()), true);
        slot.said(Some(&mine()));
        assert_eq!(slot.for_start(), None, "the render after the run");
        slot.said(Some(&other()));
        assert_eq!(slot.for_start(), Some(other()), "another PeerId");
    }

    /// R4's Continue after K1 ends the comparison at its press: that
    /// start and the next compare nothing, even after a refusal; before
    /// it, everything leaves it standing (the flow's side of which press
    /// that is: ui-model's
    /// `only_r4s_continue_after_k1_ends_the_comparison`). Mutation:
    /// `restored` leaving the `PeerId` fails here.
    #[test]
    fn a_restore_committed_after_k1_ends_the_comparison() {
        let mut slot = ExpectedStart::new();
        slot.said(Some(&mine()));
        // K1 shows, R2 opens, a prompt is cancelled: the flow says nothing.
        slot.said(None);
        assert_eq!(slot.for_start(), Some(mine()), "before R4's press");
        slot.restored();
        assert_eq!(slot.for_start(), None, "R4's Continue after K1");
        slot.answered(None, false);
        slot.said(None);
        assert_eq!(slot.for_start(), None, "refused, then Try again");
    }

    /// A refusal said for the start a hold waits on -- counted past the
    /// hold's `asked_at`, even when said before the hold began, as at a
    /// return or a recreation whose start answers before the window's
    /// thread holds C4 (#750 review r1 F1) -- is the hold's; one an
    /// earlier start said is not. Mutation: `for_hold` comparing with
    /// `>`, or answering whatever the count, fails here.
    #[test]
    fn an_answer_is_taken_by_the_hold_of_its_own_start() {
        // `showing_starts` read the count 3, then the start answered: its
        // refusal was said with the count still 3, before the hub counted
        // it as 4; `held_by_showing` held C4 after that.
        let early = UncheckedAnswer::new(UncheckedRefused::OtherIdentity, 3);
        assert_eq!(early.for_hold(3), Some(UncheckedRefused::OtherIdentity));
        let later = UncheckedAnswer::new(UncheckedRefused::Unavailable, 5);
        assert_eq!(later.for_hold(3), Some(UncheckedRefused::Unavailable));
        // An earlier start's, said with the count at 2 and counted 3.
        let earlier = UncheckedAnswer::new(UncheckedRefused::OtherIdentity, 2);
        assert_eq!(earlier.for_hold(3), None);
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
        let expect = Some(mine());
        let expect = expect.as_ref();
        assert_eq!(
            unchecked_answer(expect, &IdentityRefused::NotExpected(mine()), never),
            Some(UncheckedRefused::OtherIdentity)
        );
        assert_eq!(
            unchecked_answer(expect, &IdentityRefused::UserNotAuthenticated, never),
            None
        );
        assert_eq!(
            unprovisioned_answer(expect),
            Some(UncheckedRefused::Unavailable)
        );
        for unavailable in [
            IdentityRefused::Unprovisioned,
            IdentityRefused::Paths("x".to_owned()),
            IdentityRefused::Unavailable("x".to_owned()),
            IdentityRefused::Provisioning("x".to_owned()),
        ] {
            assert_eq!(
                unchecked_answer(expect, &unavailable, never),
                Some(UncheckedRefused::Unavailable),
                "{unavailable:?}"
            );
        }
        let recovery = unchecked_answer(
            expect,
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

    /// Only a start told to expect a `PeerId` answers the flow: one that
    /// compared nothing -- Continue's start on C4 before any
    /// `IdStartUnchecked`, every start of a process that never pressed it
    /// -- answers no refusal on C4, whatever it is, so a held screen
    /// hands over as before (ui-model's `IdentityFlow::answers_compared`
    /// says which screen answers; this, which start). Mutations:
    /// `unchecked_answer` or `unprovisioned_answer` answering a start that
    /// expects nothing fail here.
    #[test]
    fn a_start_that_compared_nothing_answers_no_flow() {
        let never = |_: &RecoveryCause| -> Start { panic!("not read") };
        for refused in [
            IdentityRefused::NotExpected(mine()),
            IdentityRefused::RecoveryRequired(RecoveryCause::KeyInvalidated),
            IdentityRefused::UserNotAuthenticated,
            IdentityRefused::Unprovisioned,
            IdentityRefused::Paths("x".to_owned()),
            IdentityRefused::Unavailable("x".to_owned()),
            IdentityRefused::Provisioning("x".to_owned()),
        ] {
            assert_eq!(unchecked_answer(None, &refused, never), None, "{refused:?}");
        }
        assert_eq!(unprovisioned_answer(None), None);
    }
}

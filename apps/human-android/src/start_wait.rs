// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What a held C4, R4, R3 or R2 waits on (add-a-peer §5.1, Between
//! Continue and the decision; identity note section 6): the hold's clock,
//! and the hub's count of start answers read before the start that hold
//! asked. The root decides a held screen by answers counted past that
//! count (`ViewSide::start_decided`), so the count is the hold's own: a
//! hold begins with none, and only a start asked under it gives it one.
//! A screen given back and held again -- C4's `IdStorageUnknown` after a
//! compared refusal and its Try again, K1 and R4's Continue after it,
//! `IdStartBusy` and its Try again -- therefore never reads an earlier
//! start's answer while its own step (a read-back, a seal, a restore, or
//! the prompt over one) runs before its start is asked (#750 review r3
//! F1). onboard.rs holds one per flow and says each step to it; on every
//! target, so the host tests it.

use interweave_human_android_platform::StoreGeneration;
use interweave_human_ui_model::{Effect, StartHold};

/// A held screen's clock and the count its start was asked at, `G` being
/// the hub's [`StoreGeneration`] (a plain count in the tests).
/// `a_hold_waits_only_on_a_start_asked_under_it` holds the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartWait<G = StoreGeneration> {
    /// The clock of a C4 or R4 held after Continue, from the press until
    /// the start decides; `None` while no screen is held.
    clock: Option<StartHold>,
    /// The hub's count of start answers before the start asked under this
    /// hold; `None` until one is.
    asked_at: Option<G>,
}

impl<G: Copy> StartWait<G> {
    /// Nothing held.
    pub(crate) const fn new() -> Self {
        Self {
            clock: None,
            asked_at: None,
        }
    }

    /// A press held the screen at `now` with `effects` to run first: a
    /// hold of its own, which waits on no start until it asks one
    /// ([`Self::asked`]). The bound counts from the press unless a seal
    /// or a read-back must commit the identity first (§5.1, the bound;
    /// C4's Try again under `IdStorageUnknown` commits as its seal would,
    /// once the read-back finds the record).
    pub(crate) fn pressed(&mut self, now: u64, effects: &[Effect]) {
        let committed = !effects
            .iter()
            .any(|e| matches!(e, Effect::Seal | Effect::ReadBack(_)));
        self.clock = Some(StartHold::pressed(now, committed));
        self.asked_at = None;
    }

    /// A showing's start held the screen at `now` (onboard's
    /// `held_by_showing`): asked by the Activity with the hub's count at
    /// `asked_at`, read before that start, the identity on record.
    pub(crate) fn showing(&mut self, now: u64, asked_at: G) {
        self.clock = Some(StartHold::pressed(now, true));
        self.asked_at = Some(asked_at);
    }

    /// A start was asked with the hub's count at `at`: the hold waits on
    /// it from now. One asked with no screen held -- the flow's end -- is
    /// no later hold's, since a hold begins with none ([`Self::pressed`]).
    pub(crate) fn asked(&mut self, at: G) {
        self.asked_at = Some(at);
    }

    /// The flow, rendered, holds a screen (`start_held`) or gave it back
    /// -- a decided start, a refusal C4 answers, `IdStartBusy`, a failed
    /// seal, a cancelled prompt: given back, the hold and the start it
    /// waited on end with it.
    pub(crate) fn follow(&mut self, start_held: bool) {
        if !start_held {
            self.clock = None;
            self.asked_at = None;
        }
    }

    /// The count of the start the held screen waits on, while the flow
    /// holds one (`start_held`) and a start was asked under that hold;
    /// `None` otherwise -- for a hold whose read-back, seal or restore,
    /// or the prompt over it, runs before its start is asked.
    pub(crate) fn waited_on(&self, start_held: bool) -> Option<G> {
        if start_held && self.clock.is_some() {
            self.asked_at
        } else {
            None
        }
    }

    /// The hold's clock, while a screen is held.
    pub(crate) const fn clock(&self) -> Option<&StartHold> {
        self.clock.as_ref()
    }

    /// The hold's clock, to stop or restart for a prompt or a commit.
    pub(crate) const fn clock_mut(&mut self) -> Option<&mut StartHold> {
        self.clock.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interweave_human_ui_model::{
        IdentityFlow, PHRASE_WORDS, ScreenId, Start, StepFailed, UncheckedRefused, Words,
    };
    use interweave_transport_api::TransportIdentity;

    const LIST: &[&str] = &[
        "acid", "bean", "cabin", "dawn", "eagle", "fabric", "galaxy", "habit", "icon", "jacket",
        "kale", "label", "machine", "napkin", "oak", "paddle", "quarter", "rabbit", "saddle",
        "table", "ugly", "vacant", "wagon", "yard", "zebra", "zone",
    ];

    fn words() -> Words {
        let mut words = [""; PHRASE_WORDS];
        words.copy_from_slice(&LIST[..PHRASE_WORDS]);
        words
    }

    fn mine() -> TransportIdentity {
        TransportIdentity::parse("12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e")
            .expect("a PeerId")
    }

    /// The root's part of a held start, as onboard.rs runs it: the flow,
    /// its wait, and the hub's count of start answers.
    struct Root {
        flow: IdentityFlow,
        wait: StartWait<u64>,
        answers: u64,
    }

    impl Root {
        /// C4, its seal asked.
        fn at_c4() -> Self {
            let mut flow = IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::FirstRun { void_paper: false },
            );
            flow.primary();
            flow.created(words(), mine(), [3, 9]);
            for _ in 0..3 {
                flow.primary();
            }
            for _ in 0..2 {
                let (position, _) = flow.check().expect("C3 checks a word");
                let word = words()[position];
                for letter in word.chars() {
                    if flow.picker().and_then(|p| p.chip(word)).is_some() {
                        break;
                    }
                    assert!(flow.press(letter), "{letter} is enabled");
                }
                flow.choose(word);
            }
            assert_eq!(flow.screen(), ScreenId::C4, "control: at C4");
            let mut root = Self {
                flow,
                wait: StartWait::new(),
                answers: 3,
            };
            root.input(IdentityFlow::primary);
            assert_eq!(root.waited_on(), None, "control: the seal runs first");
            root
        }

        /// A person's input, as `input_later` takes it: a screen that
        /// becomes held holds from the press; then its effects run.
        fn input(&mut self, input: impl FnOnce(&mut IdentityFlow) -> Vec<Effect>) {
            let was_held = self.flow.start_held();
            let effects = input(&mut self.flow);
            if self.flow.start_held() && !was_held {
                self.wait.pressed(0, &effects);
            }
            self.run(&effects);
        }

        /// A step's effects: a service start is asked with the hub's count
        /// then (`ask_service`); then the render.
        fn run(&mut self, effects: &[Effect]) {
            if effects.contains(&Effect::StartService) {
                self.wait.asked(self.answers);
            }
            self.wait.follow(self.flow.start_held());
        }

        /// The hub counts a start's answer.
        fn answered(&mut self) {
            self.answers += 1;
        }

        /// What the root's `held_rule` reads: the count whose answers
        /// past it decide the held screen.
        fn waited_on(&self) -> Option<u64> {
            self.wait.waited_on(self.flow.start_held())
        }
    }

    /// A held screen waits only on a start asked under its own hold
    /// (#750 review r3 F1). After C4 answers a compared refusal, Try
    /// again's read-back -- and the prompt over it, while the refused
    /// start's answer is counted past that start's count -- waits on no
    /// start: `held_rule` decides nothing until the read-back's start is
    /// asked, and that start's count is the hold's. The same for each
    /// other way a held C4 is given back after its start was asked: K1
    /// and `IdStartBusy`; and a start asked with no screen held is no
    /// later hold's. Mutations: `pressed` leaving the last `asked_at`,
    /// `follow` keeping the hold, or `waited_on` ignoring `start_held`,
    /// fail here.
    #[test]
    fn a_hold_waits_only_on_a_start_asked_under_it() {
        // C4's read-back failed: IdStartUnchecked, then its start refused
        // for another PeerId, which C4 answers.
        let mut root = Root::at_c4();
        root.flow.sealed(Err(StepFailed::StorageUnknown));
        root.run(&[]);
        assert_eq!(root.waited_on(), None, "C4 given back");
        root.input(IdentityFlow::start_unchecked);
        assert_eq!(root.waited_on(), Some(3), "IdStartUnchecked's start");
        root.answered();
        let effects = root.flow.unchecked_refused(UncheckedRefused::OtherIdentity);
        root.run(&effects);
        assert_eq!(root.flow.screen(), ScreenId::C4, "control: answered on C4");
        assert_eq!(root.waited_on(), None, "given back");
        assert_eq!(
            root.wait.waited_on(true),
            None,
            "nothing of the refused start is kept for a screen held again"
        );

        // Try again: the read-back, refused for the person, its prompt up.
        root.input(IdentityFlow::primary);
        assert!(root.flow.start_held(), "control: held for the read-back");
        assert_eq!(
            root.waited_on(),
            None,
            "the read-back's hold waits on no start: the refused one's answer is counted past 3"
        );
        // The read-back found the record: its start is the hold's.
        let effects = root.flow.sealed(Ok(()));
        root.run(&effects);
        assert_eq!(root.waited_on(), Some(4), "the read-back's start");

        // K1: C4 given back for a record that does not open.
        let mut root = Root::at_c4();
        root.flow.sealed(Err(StepFailed::StorageUnknown));
        root.run(&[]);
        root.input(IdentityFlow::start_unchecked);
        root.answered();
        let effects = root.flow.unchecked_refused(UncheckedRefused::Recovery {
            reason: interweave_human_ui_model::RecoveryReason::Key,
            expected: Some(mine()),
            new_keys: false,
        });
        root.run(&effects);
        assert_eq!(root.flow.screen(), ScreenId::K1, "control: K1");
        assert_eq!(root.waited_on(), None, "K1 holds nothing");
        // The restore's screen held by its Continue before its start
        // (R4's seal of the restore, R2's restore and their prompts).
        root.wait.pressed(0, &[Effect::Seal]);
        assert_eq!(root.wait.waited_on(true), None, "a later hold, unasked");

        // IdStartBusy: Continue's start refused with the profile in use.
        let mut root = Root::at_c4();
        let effects = root.flow.sealed(Ok(()));
        root.run(&effects);
        assert_eq!(root.waited_on(), Some(3), "Continue's start");
        root.answered();
        root.flow.start_busy();
        root.run(&[]);
        assert_eq!(root.waited_on(), None, "IdStartBusy given back");
        root.input(IdentityFlow::primary);
        assert_eq!(root.waited_on(), Some(4), "Try again is its own start");
        // A return's retry under IdStartBusy holds with its own count.
        root.answered();
        root.flow.start_busy();
        root.run(&[]);
        assert!(root.flow.showing_started(), "control: a showing's start");
        root.wait.showing(0, 5);
        assert_eq!(root.waited_on(), Some(5), "the showing's start");

        // A start asked at the flow's end, no screen held, is no later
        // hold's.
        let mut wait = StartWait::<u64>::new();
        wait.asked(7);
        assert_eq!(wait.waited_on(true), None, "no hold asked it");
        wait.pressed(0, &[]);
        assert_eq!(wait.waited_on(true), None, "a press after it");
        wait.asked(8);
        assert_eq!(wait.waited_on(false), None, "a screen not held");
    }
}

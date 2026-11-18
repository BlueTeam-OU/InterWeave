// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The user-presence window of the identity screens
//! (design/2026-10-19-android-identity.md section 6, "Authentication
//! needed"; acceptance 10): on a profile whose key needs the person, a key
//! operation outside the window since the last authentication waits for
//! the system prompt, raised by the screen before the operation, never by
//! the service. A cancelled prompt runs nothing the gate held: a
//! cancelled service start leaves the person offline, and the window says
//! why, with an Unlock that raises the prompt again (the root's; design
//! section 6, acceptance 10; android-key-custody.md's failure list). A
//! prompt asked and dropped before anything raised it releases what it
//! held the same way, so no screen is held by a prompt nobody will raise
//! ([`PresenceGate::unasked`]).
//!
//! It stands between [`crate::IdentityFlow`]'s effects and the platform
//! that carries them out, so every key operation passes it whichever
//! screen asked: C4's seal, the restores, and the service start after S2's
//! Done and after C4's or R4's continue.

use crate::identity_flow::Effect;

/// How long an authentication lets key operations run without another
/// prompt: the Keystore key's own validity window on a user-presence
/// profile (`android-key-custody.md`, Unlock policies).
pub const WINDOW_MS: u64 = 60_000;

/// What became of an effect offered to [`PresenceGate::admit`].
#[derive(Debug, PartialEq, Eq)]
pub enum Admit {
    /// Carry it out now.
    Run(Effect),
    /// Raise the prompt; the effect is held until it answers.
    Prompt,
}

/// What a prompt's answer released.
#[derive(Debug, PartialEq, Eq)]
pub enum Prompted {
    /// Granted: carry out the held effect now.
    Resume(Effect),
    /// Cancelled or denied: the held effect is dropped, nothing ran.
    Dropped(Effect),
    /// No effect was held: the prompt was another's, S1's.
    NotHeld,
}

/// The window, and the one effect waiting on its prompt.
#[derive(Debug, Default)]
pub struct PresenceGate {
    required: bool,
    last_authenticated: Option<u64>,
    held: Option<Effect>,
    /// The root's number for the ask of the held effect's prompt
    /// ([`Self::asked`]); 0 before it said one.
    ask: u64,
}

impl PresenceGate {
    /// A gate for a profile whose key needs the person (`required`), or
    /// one that lets everything through.
    #[must_use]
    pub fn new(required: bool) -> Self {
        Self {
            required,
            last_authenticated: None,
            held: None,
            ask: 0,
        }
    }

    /// `effect` at monotonic time `now` (ms): run it, or hold it for the
    /// prompt when it needs the key and the window has passed.
    pub fn admit(&mut self, effect: Effect, now: u64) -> Admit {
        if !self.required || !needs_the_key(&effect) || self.fresh(now) {
            return Admit::Run(effect);
        }
        self.held = Some(effect);
        self.ask = 0;
        Admit::Prompt
    }

    /// The prompt [`Self::admit`] asked for went to the platform as ask
    /// number `ask`, counted up from 1 by the root, so a dropped ask names
    /// the hold it was for ([`Self::unasked`]).
    pub const fn asked(&mut self, ask: u64) {
        self.ask = ask;
    }

    /// The platform dropped the root's asks for a prompt up to number
    /// `through` before anything raised them (a new showing of the window,
    /// a new Activity): when the held effect's ask was among them, no
    /// answer will ever come for it, and it is released unrun. Not a
    /// cancel -- the person decided nothing -- so the window opens to
    /// nothing and the caller does what a cancel does to the screen: gives
    /// it back as it was, retrying nothing (design section 6,
    /// Authentication needed). A held effect whose ask is still up, or
    /// was numbered after `through`, stays held.
    pub fn unasked(&mut self, through: u64) -> Option<Effect> {
        if self.ask == 0 || self.ask > through {
            return None;
        }
        self.ask = 0;
        self.held.take()
    }

    /// The prompt's answer at `now`. A grant opens the window, whoever's
    /// prompt it was.
    pub fn prompted(&mut self, granted: bool, now: u64) -> Prompted {
        if granted {
            self.last_authenticated = Some(now);
        }
        self.ask = 0;
        match self.held.take() {
            Some(effect) if granted => Prompted::Resume(effect),
            Some(effect) => Prompted::Dropped(effect),
            None => Prompted::NotHeld,
        }
    }

    /// Whether an effect waits on the prompt: the flow may not end while
    /// one does, or the prompt's answer would find nothing to resume.
    #[must_use]
    pub const fn holding(&self) -> bool {
        self.held.is_some()
    }

    fn fresh(&self, now: u64) -> bool {
        self.last_authenticated
            .is_some_and(|at| now.saturating_sub(at) <= WINDOW_MS)
    }
}

/// An operation that opens or writes the Keystore-sealed identity.
fn needs_the_key(effect: &Effect) -> bool {
    matches!(
        effect,
        Effect::Seal
            | Effect::RestoreRecorded(_)
            | Effect::RestoreConfirmed(..)
            | Effect::StartService
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_without_user_presence_runs_everything() {
        let mut gate = PresenceGate::new(false);
        assert_eq!(gate.admit(Effect::Seal, 0), Admit::Run(Effect::Seal));
        assert_eq!(
            gate.admit(Effect::StartService, 1_000_000),
            Admit::Run(Effect::StartService)
        );
    }

    #[test]
    fn a_key_operation_outside_the_window_waits_for_the_prompt() {
        let mut gate = PresenceGate::new(true);
        assert_eq!(
            gate.admit(Effect::Seal, 5),
            Admit::Prompt,
            "never authenticated"
        );
        assert_eq!(gate.prompted(true, 10), Prompted::Resume(Effect::Seal));
        assert_eq!(
            gate.admit(Effect::StartService, 10 + WINDOW_MS),
            Admit::Run(Effect::StartService),
            "inside the window"
        );
        assert_eq!(
            gate.admit(Effect::StartService, 11 + WINDOW_MS),
            Admit::Prompt,
            "past it"
        );
    }

    #[test]
    fn a_cancelled_prompt_drops_the_held_operation_and_opens_nothing() {
        let mut gate = PresenceGate::new(true);
        assert_eq!(gate.admit(Effect::StartService, 0), Admit::Prompt);
        assert_eq!(
            gate.prompted(false, 1),
            Prompted::Dropped(Effect::StartService)
        );
        assert_eq!(gate.admit(Effect::Seal, 2), Admit::Prompt, "still closed");
    }

    /// A held effect whose prompt's ask was dropped before anything raised
    /// it is released, and nothing it held runs on a later answer: the
    /// next prompt's grant -- a locked start's, S1's -- finds nothing to
    /// resume, and the window stays closed. Mutation: `unasked` keeping
    /// the hold (returning `None` always) fails the first assertion; the
    /// grant then resumes the seal, the retry section 6 forbids.
    #[test]
    fn a_dropped_ask_releases_its_hold_unrun_and_opens_nothing() {
        let mut gate = PresenceGate::new(true);
        assert_eq!(gate.admit(Effect::Seal, 0), Admit::Prompt);
        gate.asked(7);
        assert_eq!(gate.unasked(7), Some(Effect::Seal));
        assert!(!gate.holding(), "nothing holds the screen any more");
        assert_eq!(
            gate.prompted(true, 1),
            Prompted::NotHeld,
            "a later grant retries nothing"
        );

        let mut gate = PresenceGate::new(true);
        assert_eq!(gate.admit(Effect::StartService, 0), Admit::Prompt);
        gate.asked(3);
        assert_eq!(gate.unasked(9), Some(Effect::StartService));
        assert_eq!(
            gate.admit(Effect::Seal, 1),
            Admit::Prompt,
            "a dropped ask authenticated nobody"
        );
    }

    /// Only the hold whose own ask was dropped is released: one asked
    /// after the dropped ones, or one whose ask was raised and waits on
    /// its answer, stays held for that answer. Mutation: `unasked` ignoring
    /// `through` fails the first assertion.
    #[test]
    fn a_dropped_ask_leaves_a_hold_asked_after_it() {
        let mut gate = PresenceGate::new(true);
        assert_eq!(
            gate.admit(Effect::RestoreRecorded("w".into()), 0),
            Admit::Prompt
        );
        gate.asked(5);
        assert_eq!(gate.unasked(4), None, "asked after the dropped asks");
        assert!(gate.holding());
        assert_eq!(gate.unasked(0), None, "no ask of the gate's was dropped");
        assert_eq!(
            gate.prompted(false, 1),
            Prompted::Dropped(Effect::RestoreRecorded("w".into()))
        );
        assert_eq!(gate.unasked(5), None, "answered: nothing left to release");
    }

    #[test]
    fn s1s_own_prompt_opens_the_window_and_what_needs_no_key_passes() {
        let mut gate = PresenceGate::new(true);
        assert_eq!(gate.prompted(true, 100), Prompted::NotHeld);
        assert_eq!(gate.admit(Effect::Seal, 200), Admit::Run(Effect::Seal));
        assert_eq!(
            gate.admit(Effect::OpenSettings, 10 * WINDOW_MS),
            Admit::Run(Effect::OpenSettings)
        );
    }
}

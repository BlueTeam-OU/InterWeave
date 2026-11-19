// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The key operation of the identity screens that waits on the person
//! (design/2026-10-19-android-identity.md section 6, "Authentication
//! needed"; acceptance 10). Every key operation runs at once: the app
//! keeps no clock of the Keystore's window, whose opening by a device
//! unlock it never sees, and a refusal for the person
//! (`UserNotAuthenticated`) is its only reading of it (ADR-0042,
//! A 2026-10-20). The refused operation is held here while the screen
//! raises the system prompt -- the screen, never the service -- and a
//! grant runs it again. A cancelled prompt runs nothing the gate held: a
//! cancelled service start leaves the person offline, and the window says
//! why, with an Unlock that raises the prompt again (design section 6,
//! acceptance 10; android-key-custody.md's failure list). A prompt asked
//! and dropped before anything raised it releases what it held the same
//! way, so no screen is held by a prompt nobody will raise
//! ([`PresenceGate::unasked`]).
//!
//! It stands between [`crate::IdentityFlow`]'s effects and the platform
//! that carries them out, so whichever screen asked, every refused key
//! operation passes it: C4's seal, the restores, the read-back of
//! `IdStorageUnknown`'s Try again, and a held C4's or R4's service start.

use crate::identity_flow::Effect;

/// What a prompt's answer released.
#[derive(Debug, PartialEq, Eq)]
pub enum Prompted {
    /// Granted: carry out the held effect again now.
    Resume(Effect),
    /// Cancelled or denied: the held effect is dropped, nothing ran.
    Dropped(Effect),
    /// No effect was held: the prompt was another's, S1's.
    NotHeld,
}

/// The one key operation waiting on its prompt.
#[derive(Debug, Default)]
pub struct PresenceGate {
    held: Option<Effect>,
    /// The root's number for the ask of the held effect's prompt
    /// ([`Self::asked`]); 0 before it said one.
    ask: u64,
}

impl PresenceGate {
    /// `effect`, a key operation, was refused because the key needs the
    /// person (`UserNotAuthenticated`): it waits for the prompt the screen
    /// raises now, and a grant runs it again ([`Self::prompted`]).
    /// [`Effect::AuthenticateReadBack`], the flow's own word for a refused
    /// read-back, is held as the [`Effect::ReadBack`] it asks again.
    pub fn refused(&mut self, effect: Effect) {
        self.held = Some(match effect {
            Effect::AuthenticateReadBack(peer) => Effect::ReadBack(peer),
            effect => effect,
        });
        self.ask = 0;
    }

    /// The prompt [`Self::refused`] asked for went to the platform as ask
    /// number `ask`, counted up from 1 by the root, so a dropped ask names
    /// the hold it was for ([`Self::unasked`]).
    pub const fn asked(&mut self, ask: u64) {
        self.ask = ask;
    }

    /// The platform dropped the root's asks for a prompt up to number
    /// `through` before anything raised them (a new showing of the window,
    /// a new Activity): when the held effect's ask was among them, no
    /// answer will ever come for it, and it is released unrun. Not a
    /// cancel -- the person decided nothing -- and the caller does what a
    /// cancel does to the screen: gives it back as it was, retrying
    /// nothing (design section 6, Authentication needed). A held effect
    /// whose ask is still up, or was numbered after `through`, stays held.
    pub fn unasked(&mut self, through: u64) -> Option<Effect> {
        if self.ask == 0 || self.ask > through {
            return None;
        }
        self.ask = 0;
        self.held.take()
    }

    /// The prompt's answer.
    pub fn prompted(&mut self, granted: bool) -> Prompted {
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

    /// Whether the effect waiting on the prompt is the service start: a
    /// held screen whose start was refused is not decided until the
    /// prompt answers.
    #[must_use]
    pub const fn holding_start(&self) -> bool {
        matches!(self.held, Some(Effect::StartService))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer() -> interweave_transport_api::TransportIdentity {
        interweave_transport_api::TransportIdentity::parse(
            "12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e",
        )
        .expect("a PeerId")
    }

    /// A refused key operation waits for its prompt; a grant runs it
    /// again, once, a cancel drops it unrun, and a refused read-back is
    /// held as the read-back it asks again. Mutations: `refused` holding
    /// nothing, or holding `AuthenticateReadBack` itself, or `prompted`
    /// resuming on a cancel, fail here.
    #[test]
    fn a_refused_key_operation_waits_for_the_prompt() {
        let mut gate = PresenceGate::default();
        gate.refused(Effect::Seal);
        assert!(gate.holding());
        assert_eq!(gate.prompted(true), Prompted::Resume(Effect::Seal));
        assert!(!gate.holding(), "resumed once, then nothing waits");
        assert_eq!(gate.prompted(true), Prompted::NotHeld, "a later grant");

        assert!(!gate.holding_start(), "control: nothing held");
        gate.refused(Effect::StartService);
        assert!(gate.holding_start());
        assert_eq!(
            gate.prompted(false),
            Prompted::Dropped(Effect::StartService)
        );
        assert!(!gate.holding_start());

        gate.refused(Effect::RestoreConfirmed("w".into(), peer()));
        assert!(!gate.holding_start(), "a restore is not the start");
        assert_eq!(
            gate.prompted(false),
            Prompted::Dropped(Effect::RestoreConfirmed("w".into(), peer()))
        );

        gate.refused(Effect::AuthenticateReadBack(peer()));
        assert_eq!(
            gate.prompted(true),
            Prompted::Resume(Effect::ReadBack(peer()))
        );
    }

    /// Nothing reaches the gate but a refusal: a grant with nothing held
    /// -- S1's own prompt, or an Unlock that unlocked the Keystore for a
    /// later seal -- holds nothing open, so the next operation is not
    /// answered by it but runs and is refused or not by the Keystore. The
    /// gate has no clock to consult (design section 6: "The app keeps no
    /// clock of the window"). Mutation: `prompted` keeping the held effect
    /// on a grant fails the second assertion.
    #[test]
    fn a_grant_with_nothing_held_opens_nothing() {
        let mut gate = PresenceGate::default();
        assert_eq!(gate.prompted(true), Prompted::NotHeld);
        assert!(!gate.holding());
        gate.refused(Effect::Seal);
        assert_eq!(gate.prompted(true), Prompted::Resume(Effect::Seal));
        assert!(!gate.holding(), "the retry is the Keystore's to answer");
    }

    /// A held effect whose prompt's ask was dropped before anything raised
    /// it is released, and nothing it held runs on a later answer: the
    /// next prompt's grant -- a locked start's, S1's -- finds nothing to
    /// resume. Mutation: `unasked` keeping the hold (returning `None`
    /// always) fails the first assertion; the grant then resumes the
    /// seal, the retry section 6 forbids.
    #[test]
    fn a_dropped_ask_releases_its_hold_unrun() {
        let mut gate = PresenceGate::default();
        gate.refused(Effect::Seal);
        gate.asked(7);
        assert_eq!(gate.unasked(7), Some(Effect::Seal));
        assert!(!gate.holding(), "nothing holds the screen any more");
        assert_eq!(
            gate.prompted(true),
            Prompted::NotHeld,
            "a later grant retries nothing"
        );

        gate.refused(Effect::StartService);
        gate.asked(3);
        assert_eq!(gate.unasked(9), Some(Effect::StartService));
    }

    /// Only the hold whose own ask was dropped is released: one asked
    /// after the dropped ones, or one whose ask was raised and waits on
    /// its answer, stays held for that answer. Mutation: `unasked` ignoring
    /// `through` fails the first assertion.
    #[test]
    fn a_dropped_ask_leaves_a_hold_asked_after_it() {
        let mut gate = PresenceGate::default();
        gate.refused(Effect::RestoreRecorded("w".into()));
        gate.asked(5);
        assert_eq!(gate.unasked(4), None, "asked after the dropped asks");
        assert!(gate.holding());
        assert_eq!(gate.unasked(0), None, "no ask of the gate's was dropped");
        assert_eq!(
            gate.prompted(false),
            Prompted::Dropped(Effect::RestoreRecorded("w".into()))
        );
        assert_eq!(gate.unasked(5), None, "answered: nothing left to release");
    }

    /// A refused read-back prompts again each time it is refused, and a
    /// cancel drops it: what the screen then shows is the flow's
    /// (`IdStorageUnknown`, design section 6).
    #[test]
    fn a_read_back_refused_again_prompts_again() {
        let mut gate = PresenceGate::default();
        gate.refused(Effect::AuthenticateReadBack(peer()));
        assert_eq!(
            gate.prompted(true),
            Prompted::Resume(Effect::ReadBack(peer()))
        );
        gate.refused(Effect::AuthenticateReadBack(peer()));
        assert_eq!(
            gate.prompted(false),
            Prompted::Dropped(Effect::ReadBack(peer()))
        );
    }
}

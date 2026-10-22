// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The store's recovery screen (R), the key-lost screen (K) and Discard
//! and recreate (ADR-0060 decision 6; `design/2026-10-21-contact-cards.md`
//! §7; `human-client-ui.md` §13): one model for both clients, which the
//! views render and the platform drives.
//!
//! Nothing here deletes. The model asks the platform to build the plan
//! ([`Effect::Plan`]) when the person presses Delete stored data, shows
//! the confirmation the plan's scope calls for, and asks the platform to
//! discard ([`Effect::Discard`]) only from that confirmation's primary
//! action. Its initial focus is Cancel, so Enter alone never discards, and
//! there is no second confirmation: the loss is listed once.

use crate::labels::UiText;

/// Which screen the store's refusal shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreScreen {
    /// R: recovery mode -- the key missing or not opening the file, a
    /// damaged file, a newer build's, a failed migration, names decision 6
    /// does not list. Try again, Delete stored data, Close.
    Recovery,
    /// K (Android): the platform invalidated the store key. Delete stored
    /// data is the primary action, then Close; no Try again, since the
    /// platform has said the key is gone.
    KeyLost,
}

/// Which confirmation the plan calls for (`at_rest::DiscardScope`, told
/// apart here without the store's types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardKind {
    /// Everything the store held, and its key.
    Full,
    /// The unfinished plaintext alone; the complete copy opens and stays.
    Partial,
}

/// Where the flow is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The screen, its actions live.
    Shown,
    /// The plan is being built; every action inert.
    Checking,
    /// The confirmation is open.
    Confirming(DiscardKind),
    /// The confirmed discard runs; every action inert, Back ignored.
    Discarding,
    /// Done: the app opens as on any start, with this notice in Chats.
    Done(DiscardKind),
}

/// What the platform must do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Build the discard's plan (the copy opened read-only under the key)
    /// and answer [`StoreRecovery::planned`] or [`StoreRecovery::failed`].
    Plan,
    /// Run the confirmed discard and answer [`StoreRecovery::discarded`]
    /// or [`StoreRecovery::failed`].
    Discard,
}

/// The store recovery flow's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreRecovery {
    screen: StoreScreen,
    phase: Phase,
    identities: usize,
    failed: bool,
}

impl StoreRecovery {
    /// The flow on `screen`, for a client holding `identities` identities
    /// (the lists name every identity's data from the second on).
    #[must_use]
    pub const fn new(screen: StoreScreen, identities: usize) -> Self {
        Self {
            screen,
            phase: Phase::Shown,
            identities,
            failed: false,
        }
    }

    /// The screen.
    #[must_use]
    pub const fn screen(&self) -> StoreScreen {
        self.screen
    }

    /// Where the flow is.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Whether the screen's actions take a press: not while the plan is
    /// built or the discard runs.
    #[must_use]
    pub const fn actions_live(&self) -> bool {
        matches!(self.phase, Phase::Shown)
    }

    /// Whether Try again is offered: on R only.
    #[must_use]
    pub const fn offers_try_again(&self) -> bool {
        matches!(self.screen, StoreScreen::Recovery)
    }

    /// The person pressed Delete stored data.
    pub const fn press_discard(&mut self) -> Option<Effect> {
        if !self.actions_live() {
            return None;
        }
        self.phase = Phase::Checking;
        Some(Effect::Plan)
    }

    /// The plan is built: open its confirmation.
    pub const fn planned(&mut self, kind: DiscardKind) {
        if matches!(self.phase, Phase::Checking) {
            self.phase = Phase::Confirming(kind);
        }
    }

    /// Cancel, Escape, Back or a tap outside on the confirmation.
    pub const fn cancel(&mut self) {
        if matches!(self.phase, Phase::Confirming(_)) {
            self.phase = Phase::Shown;
        }
    }

    /// The confirmation's primary action: the person's word.
    pub const fn confirm(&mut self) -> Option<Effect> {
        if let Phase::Confirming(_) = self.phase {
            self.phase = Phase::Discarding;
            return Some(Effect::Discard);
        }
        None
    }

    /// The discard ran; `kind` is what it removed.
    pub const fn discarded(&mut self, kind: DiscardKind) {
        if matches!(self.phase, Phase::Discarding) {
            self.phase = Phase::Done(kind);
        }
    }

    /// The plan could not be built, or the discard failed or found the
    /// store changed: back on the screen, saying so; the next press builds
    /// the plan again from the files as they then are.
    pub const fn failed(&mut self) {
        if matches!(self.phase, Phase::Checking | Phase::Discarding) {
            self.phase = Phase::Shown;
            self.failed = true;
        }
    }

    /// Back (Android) or Escape (desktop): cancels a confirmation, is
    /// ignored while the discard runs, and otherwise leaves it to the
    /// screen (Close). Whether the press was taken here.
    pub const fn back(&mut self) -> bool {
        match self.phase {
            Phase::Confirming(_) => {
                self.cancel();
                true
            }
            Phase::Checking | Phase::Discarding => true,
            Phase::Shown | Phase::Done(_) => false,
        }
    }

    /// The screen's heading.
    #[must_use]
    pub const fn heading(&self) -> UiText {
        match self.screen {
            StoreScreen::Recovery => UiText::StoreRecoveryTitle,
            StoreScreen::KeyLost => UiText::StoreKeyLostTitle,
        }
    }

    /// The screen's body on Android; the desktop's R shows its refusal
    /// text, [`UiText::RefusalStoreRecovery`], which keeps the newer-build
    /// advice.
    #[must_use]
    pub const fn body(&self) -> UiText {
        match self.screen {
            StoreScreen::Recovery => UiText::StoreRecoveryBody,
            StoreScreen::KeyLost => UiText::StoreKeyLostBody,
        }
    }

    /// The line under the heading: the failure, or what is under way.
    #[must_use]
    pub const fn status(&self) -> Option<UiText> {
        match self.phase {
            Phase::Checking => Some(UiText::StoreChecking),
            Phase::Discarding => Some(UiText::StoreDiscarding),
            Phase::Shown if self.failed => Some(UiText::StoreDiscardFailed),
            _ => None,
        }
    }

    /// The open confirmation, if any.
    #[must_use]
    pub fn dialog(&self) -> Option<DiscardConfirmation> {
        let Phase::Confirming(kind) = self.phase else {
            return None;
        };
        let many = self.identities > 1;
        Some(match kind {
            DiscardKind::Full => DiscardConfirmation {
                kind,
                title: UiText::DiscardStoreTitle,
                lead: Some(if many {
                    UiText::DiscardStoreLosesMany
                } else {
                    UiText::DiscardStoreLosesOne
                }),
                items: vec![
                    UiText::DiscardLosesContacts,
                    UiText::DiscardLosesPending,
                    UiText::DiscardLosesMessages,
                    UiText::DiscardLosesConversations,
                    if many {
                        UiText::DiscardLosesCardMany
                    } else {
                        UiText::DiscardLosesCardOne
                    },
                    UiText::DiscardLosesSettings,
                    UiText::DiscardLosesShownAgain,
                ],
                notes: vec![
                    if many {
                        UiText::DiscardStoreKeepsMany
                    } else {
                        UiText::DiscardStoreKeepsOne
                    },
                    UiText::DiscardStoreRemains,
                ],
                confirm: UiText::DiscardStoreConfirm,
                destructive: true,
            },
            DiscardKind::Partial => DiscardConfirmation {
                kind,
                title: UiText::DiscardPartialTitle,
                lead: None,
                items: Vec::new(),
                notes: vec![UiText::DiscardPartialBody],
                confirm: UiText::DiscardPartialConfirm,
                destructive: false,
            },
        })
    }

    /// The notice Chats shows once after the discard.
    #[must_use]
    pub const fn notice(&self) -> Option<UiText> {
        match self.phase {
            Phase::Done(DiscardKind::Full) => Some(UiText::StoreDiscarded),
            Phase::Done(DiscardKind::Partial) => Some(UiText::StoreRestored),
            _ => None,
        }
    }
}

/// A discard's confirmation, as a view renders it: the title, the list
/// of what goes (one item per line), the notes, then Cancel -- the
/// initial focus -- and the primary action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardConfirmation {
    /// Which discard.
    pub kind: DiscardKind,
    /// The title.
    pub title: UiText,
    /// The list's lead-in, when there is a list.
    pub lead: Option<UiText>,
    /// What goes, one per line.
    pub items: Vec<UiText>,
    /// What stays, and what the device may still hold.
    pub notes: Vec<UiText>,
    /// The primary action's label.
    pub confirm: UiText,
    /// Whether the primary action is shown in the error colour: the full
    /// discard loses data, the partial loses nothing the person has.
    pub destructive: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_discarded_without_the_plan_then_the_confirmation() {
        let mut flow = StoreRecovery::new(StoreScreen::Recovery, 1);
        assert_eq!(flow.confirm(), None, "no confirmation is open");
        assert_eq!(flow.press_discard(), Some(Effect::Plan));
        assert_eq!(flow.status(), Some(UiText::StoreChecking));
        assert_eq!(flow.press_discard(), None, "inert while checking");
        assert_eq!(flow.confirm(), None, "still no confirmation");
        flow.planned(DiscardKind::Full);
        assert_eq!(flow.confirm(), Some(Effect::Discard));
        assert_eq!(flow.phase(), Phase::Discarding);
        assert_eq!(flow.confirm(), None, "one discard per confirmation");
        flow.discarded(DiscardKind::Full);
        assert_eq!(flow.notice(), Some(UiText::StoreDiscarded));
    }

    #[test]
    fn cancel_and_back_close_the_confirmation_and_back_is_ignored_while_discarding() {
        let mut flow = StoreRecovery::new(StoreScreen::KeyLost, 1);
        flow.press_discard();
        flow.planned(DiscardKind::Full);
        assert!(flow.back());
        assert_eq!(flow.phase(), Phase::Shown);
        flow.press_discard();
        flow.planned(DiscardKind::Full);
        flow.confirm();
        assert!(flow.back(), "taken, and ignored");
        assert_eq!(flow.phase(), Phase::Discarding);
        assert!(!flow.offers_try_again(), "K has no Try again");
    }

    #[test]
    fn a_failure_returns_to_the_screen_saying_so_and_the_next_press_plans_again() {
        let mut flow = StoreRecovery::new(StoreScreen::Recovery, 1);
        flow.press_discard();
        flow.planned(DiscardKind::Full);
        flow.confirm();
        flow.failed();
        assert_eq!(flow.phase(), Phase::Shown);
        assert_eq!(flow.status(), Some(UiText::StoreDiscardFailed));
        assert_eq!(flow.press_discard(), Some(Effect::Plan));
    }

    #[test]
    fn the_full_list_names_every_identity_from_the_second_on() {
        let mut one = StoreRecovery::new(StoreScreen::Recovery, 1);
        one.press_discard();
        one.planned(DiscardKind::Full);
        let d = one.dialog().expect("open");
        assert_eq!(d.lead, Some(UiText::DiscardStoreLosesOne));
        assert!(d.items.contains(&UiText::DiscardLosesCardOne));
        assert_eq!(d.notes[0], UiText::DiscardStoreKeepsOne);
        assert!(d.destructive);
        let mut two = StoreRecovery::new(StoreScreen::Recovery, 2);
        two.press_discard();
        two.planned(DiscardKind::Full);
        let d = two.dialog().expect("open");
        assert_eq!(d.lead, Some(UiText::DiscardStoreLosesMany));
        assert!(d.items.contains(&UiText::DiscardLosesCardMany));
        assert_eq!(d.notes[0], UiText::DiscardStoreKeepsMany);
        assert_eq!(d.items.len(), 7);
    }

    #[test]
    fn the_partial_confirmation_lists_no_loss_and_is_not_destructive() {
        let mut flow = StoreRecovery::new(StoreScreen::Recovery, 3);
        flow.press_discard();
        flow.planned(DiscardKind::Partial);
        let d = flow.dialog().expect("open");
        assert!(d.items.is_empty() && d.lead.is_none());
        assert!(!d.destructive);
        assert_eq!(d.confirm, UiText::DiscardPartialConfirm);
        flow.confirm();
        flow.discarded(DiscardKind::Partial);
        assert_eq!(flow.notice(), Some(UiText::StoreRestored));
    }
}

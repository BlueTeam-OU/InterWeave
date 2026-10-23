// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android identity screens' flow (design/2026-10-19-android-identity.md
//! §3, §6, §7; plan §20 steps 7-8): create and check a new identity, show
//! its phrase, restore one, and recover from a key the phone lost. Pure:
//! each method is a person's action or the platform's answer, and returns
//! the [`Effect`]s the root carries out -- sealing, restoring, stopping
//! and starting the service. Nothing here seals, writes or reaches the
//! network, and nothing here decides which `PeerId` a restore may give
//! back: the root's custody path does (`restore_identity`).
//!
//! Words are held as references into the caller's word list, never as
//! owned strings, and only while their screen needs them: leaving drops
//! them (§7). No `Debug` here prints a word.

use std::fmt;

use interweave_transport_api::TransportIdentity;

use crate::picker::{PHRASE_WORDS, PhraseEntry, WordPicker};

/// A phrase's words, as references into the word list.
pub type Words = [&'static str; PHRASE_WORDS];

/// Why the record a start found does not open (§3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryReason {
    /// The phone's key is gone or invalidated (`IdQReasonKey`).
    Key,
    /// The record cannot be read on this phone (`IdQReasonRecord`).
    Record,
}

/// How a flow starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// No record: Welcome, with the void-paper notice if a creation
    /// showed its words last time and never completed (§3.1).
    FirstRun {
        /// The non-secret marker is set.
        void_paper: bool,
    },
    /// A record that does not open (§3.4).
    Recovery {
        /// Why.
        reason: RecoveryReason,
        /// The `PeerId` the record names, when it still names one.
        expected: Option<TransportIdentity>,
    },
    /// Settings asked to show the phrase (§3.5).
    ShowPhrase,
}

/// A failure the platform reports for a step that stops the service or
/// writes (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepFailed {
    /// The profile stayed held past the wait (`IdBusy`).
    Busy,
    /// Storage or the cipher failed; nothing was changed
    /// (`IdStorageFailed`).
    Storage,
    /// C4's seal found the profile already provisioned: another first run
    /// sealed first, so the words shown were never used (`IdVoidPaper`,
    /// #303 review F5).
    AlreadyProvisioned,
}

/// What the root must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Leave the app (Back on a first screen).
    Finish,
    /// Make a new identity in memory, unsealed, and answer
    /// [`IdentityFlow::created`], or [`IdentityFlow::not_created`].
    Generate,
    /// Take the profile's lock for the flow again, after it was refused,
    /// and answer [`IdentityFlow::held`].
    Hold,
    /// Drop the unsealed identity.
    Discard,
    /// Set or clear the void-paper marker.
    VoidPaper(bool),
    /// Seal the created identity and answer [`IdentityFlow::sealed`].
    Seal,
    /// Parse `phrase` and derive its `PeerId`; answer
    /// [`IdentityFlow::checked`]. The root drops the string once parsed.
    Check(String),
    /// Restore `phrase` to the record's `PeerId` (`Expected::Recorded`);
    /// answer [`IdentityFlow::restored`].
    RestoreRecorded(String),
    /// Restore `phrase` to the `PeerId` the person confirmed
    /// (`Expected::Confirmed`); answer [`IdentityFlow::restored`].
    RestoreConfirmed(String, TransportIdentity),
    /// Raise the device-credential or strong-biometric prompt; answer
    /// [`IdentityFlow::prompted`].
    Prompt,
    /// Take the service offline and re-derive the words; answer
    /// [`IdentityFlow::offline`].
    GoOffline,
    /// Start the service.
    StartService,
    /// Open the Chats screen: the flow is over.
    OpenChats,
    /// Return to Settings: the flow is over.
    OpenSettings,
}

/// A notice shown inline on the screen it belongs to (§6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// `IdChecksumFailed`.
    ChecksumFailed,
    /// `IdOtherIdentity`, with the derived and the expected `PeerId`.
    OtherIdentity {
        /// What the words derive to.
        derived: TransportIdentity,
        /// What the record names.
        expected: TransportIdentity,
    },
    /// `IdBusy`.
    Busy,
    /// `IdStorageFailed`.
    StorageFailed,
    /// `IdVoidPaper` on C4: the phone holds another identity, and
    /// Continue opens it.
    VoidPaper,
}

impl From<StepFailed> for Notice {
    fn from(failed: StepFailed) -> Self {
        match failed {
            StepFailed::Busy => Self::Busy,
            StepFailed::Storage => Self::StorageFailed,
            StepFailed::AlreadyProvisioned => Self::VoidPaper,
        }
    }
}

/// Where R2 was entered from: what it restores against, and where Back
/// returns.
#[derive(Clone, PartialEq, Eq)]
enum Origin {
    /// R1, a fresh install: R3 asks the person.
    Fresh,
    /// K1, whose record names `expected`: R2 restores against it and
    /// there is no R3.
    Recovery {
        reason: RecoveryReason,
        expected: TransportIdentity,
    },
}

impl Origin {
    fn back(&self) -> Screen {
        match self {
            Self::Fresh => Screen::R1,
            Self::Recovery { reason, expected } => Screen::K1 {
                reason: *reason,
                expected: Some(expected.clone()),
            },
        }
    }
}

/// C3's check of two words.
struct Check {
    positions: [usize; 2],
    done: usize,
    picker: WordPicker,
    wrong: Option<usize>,
}

/// The screen showing.
enum Screen {
    Welcome {
        void_paper: bool,
    },
    C1,
    C2,
    C3(Check),
    C4 {
        notice: Option<Notice>,
    },
    R1,
    R2 {
        origin: Origin,
        entry: PhraseEntry,
        notice: Option<Notice>,
        checking: bool,
        discard_asked: bool,
    },
    R3 {
        origin: Origin,
        entry: PhraseEntry,
        derived: TransportIdentity,
        notice: Option<Notice>,
    },
    R4 {
        peer: Option<TransportIdentity>,
    },
    K1 {
        reason: RecoveryReason,
        expected: Option<TransportIdentity>,
    },
    S1 {
        going_offline: bool,
        notice: Option<Notice>,
    },
    S2,
    Done,
}

/// Which screen shows, for the view (§3's names).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    /// Welcome (§3.1).
    W,
    /// Before you see your words.
    C1,
    /// Your 24 words.
    C2,
    /// Check two words.
    C3,
    /// Identity created.
    C4,
    /// Restore your identity.
    R1,
    /// Enter your 24 words.
    R2,
    /// Is this your identity?
    R3,
    /// Restored.
    R4,
    /// Recovery required (§3.4).
    K1,
    /// Show your recovery phrase (§3.5).
    S1,
    /// Your 24 words, on demand.
    S2,
    /// The flow handed over to Chats or Settings.
    Done,
}

/// The identity screens' state.
pub struct IdentityFlow {
    list: &'static [&'static str],
    screen: Screen,
    /// The unsealed identity's words and `PeerId`, from `created` until
    /// the seal or the discard; S2's words while they show.
    words: Option<Words>,
    peer: Option<TransportIdentity>,
    check_positions: [usize; 2],
    /// C2 was shown for the unsealed identity: the marker is set.
    shown: bool,
    /// The root could not take the profile's lock: no word is shown or
    /// taken until it can, since the lock is taken before the first word
    /// (android-key-custody.md, Recovery). The first screens' primary
    /// asks for it again.
    unheld: bool,
    /// Why Welcome, R1 or K1 shows a notice: the lock refused, or a new
    /// identity that could not be made.
    start_notice: Option<Notice>,
}

impl IdentityFlow {
    /// A flow over `list` (BIP-39's English list on the device), starting
    /// as `start` says.
    #[must_use]
    pub fn new(list: &'static [&'static str], start: Start) -> Self {
        let screen = match start {
            Start::FirstRun { void_paper } => Screen::Welcome { void_paper },
            Start::Recovery { reason, expected } => Screen::K1 { reason, expected },
            Start::ShowPhrase => Screen::S1 {
                going_offline: false,
                notice: None,
            },
        };
        Self {
            list,
            screen,
            words: None,
            peer: None,
            check_positions: [0, 1],
            shown: false,
            unheld: false,
            start_notice: None,
        }
    }

    /// Which screen shows.
    #[must_use]
    pub fn screen(&self) -> ScreenId {
        match &self.screen {
            Screen::Welcome { .. } => ScreenId::W,
            Screen::C1 => ScreenId::C1,
            Screen::C2 => ScreenId::C2,
            Screen::C3(_) => ScreenId::C3,
            Screen::C4 { .. } => ScreenId::C4,
            Screen::R1 => ScreenId::R1,
            Screen::R2 { .. } => ScreenId::R2,
            Screen::R3 { .. } => ScreenId::R3,
            Screen::R4 { .. } => ScreenId::R4,
            Screen::K1 { .. } => ScreenId::K1,
            Screen::S1 { .. } => ScreenId::S1,
            Screen::S2 => ScreenId::S2,
            Screen::Done => ScreenId::Done,
        }
    }

    /// W shows the void-paper notice.
    #[must_use]
    pub fn void_paper(&self) -> bool {
        matches!(self.screen, Screen::Welcome { void_paper: true })
    }

    /// K1 offers a restore: its record names a `PeerId`.
    #[must_use]
    pub fn can_restore(&self) -> bool {
        matches!(
            self.screen,
            Screen::K1 {
                expected: Some(_),
                ..
            }
        )
    }

    /// The screen warns that the phrase may still be live on another
    /// device (`human-client-ui.md` section 9; IDENTITY-RECOVERY.md
    /// section Restore semantics item 11; ADR-0043): R1, and K1 when it
    /// offers a restore -- the screen each path into R2 passes through,
    /// so the warning is read before R2 takes a word, and R2 keeps its
    /// height for the keypad and the grid. A statement, never a gate:
    /// nothing here or in [`Self::primary`] reads it. Nothing on the
    /// phone can tell whether the identity runs elsewhere, so it is
    /// unconditional. `every_path_into_r2_passes_a_screen_that_warns`
    /// holds it.
    #[must_use]
    pub fn warns_active_elsewhere(&self) -> bool {
        matches!(self.screen, Screen::R1) || self.can_restore()
    }

    /// K1's reason and the `PeerId` the record names.
    #[must_use]
    pub fn recovery(&self) -> Option<(RecoveryReason, Option<&TransportIdentity>)> {
        match &self.screen {
            Screen::K1 { reason, expected } => Some((*reason, expected.as_ref())),
            _ => None,
        }
    }

    /// The words C2 or S2 shows.
    #[must_use]
    pub fn words(&self) -> Option<&Words> {
        match self.screen {
            Screen::C2 | Screen::S2 => self.words.as_ref(),
            _ => None,
        }
    }

    /// The `PeerId` C2, C4, R3, R4 or S2 shows.
    #[must_use]
    pub fn peer(&self) -> Option<&TransportIdentity> {
        match &self.screen {
            Screen::C2 | Screen::C4 { .. } | Screen::S2 => self.peer.as_ref(),
            Screen::R3 { derived, .. } => Some(derived),
            Screen::R4 { peer } => peer.as_ref(),
            _ => None,
        }
    }

    /// The notice the screen shows, if any.
    #[must_use]
    pub fn notice(&self) -> Option<&Notice> {
        match &self.screen {
            Screen::C4 { notice }
            | Screen::R2 { notice, .. }
            | Screen::R3 { notice, .. }
            | Screen::S1 { notice, .. } => notice.as_ref(),
            Screen::Welcome { .. } | Screen::R1 | Screen::K1 { .. } => self.start_notice.as_ref(),
            _ => None,
        }
    }

    /// S1 is taking the service offline (`IdGoingOffline`), or R2 is
    /// checking: the primary action waits.
    #[must_use]
    pub fn waiting(&self) -> bool {
        matches!(
            self.screen,
            Screen::S1 {
                going_offline: true,
                ..
            } | Screen::R2 { checking: true, .. }
        )
    }

    /// R2's Back with words entered asks first (`IdDiscardTitle`).
    #[must_use]
    pub fn discard_asked(&self) -> bool {
        matches!(
            self.screen,
            Screen::R2 {
                discard_asked: true,
                ..
            }
        )
    }

    /// C3: the position being checked (from 0), and the position a wrong
    /// word was entered for, if the last try was wrong.
    #[must_use]
    pub fn check(&self) -> Option<(usize, Option<usize>)> {
        match &self.screen {
            Screen::C3(check) => Some((check.positions[check.done.min(1)], check.wrong)),
            _ => None,
        }
    }

    /// The picker of C3, or R2's phrase entry's.
    #[must_use]
    pub fn picker(&self) -> Option<&WordPicker> {
        match &self.screen {
            Screen::C3(check) => Some(&check.picker),
            Screen::R2 { entry, .. } => Some(entry.picker()),
            _ => None,
        }
    }

    /// R2's phrase entry, for its grid.
    #[must_use]
    pub fn entry(&self) -> Option<&PhraseEntry> {
        match &self.screen {
            Screen::R2 { entry, .. } => Some(entry),
            _ => None,
        }
    }

    // --- the person's actions ------------------------------------------

    /// The screen's primary action.
    pub fn primary(&mut self) -> Vec<Effect> {
        // The way to the first word waits for the lock (Try again).
        if self.unheld
            && matches!(
                self.screen,
                Screen::Welcome { .. } | Screen::R1 | Screen::K1 { .. }
            )
        {
            return vec![Effect::Hold];
        }
        self.start_notice = None;
        match &mut self.screen {
            Screen::Welcome { .. } => {
                self.screen = Screen::C1;
                vec![Effect::VoidPaper(false), Effect::Generate]
            }
            Screen::C1 => {
                self.screen = Screen::C2;
                if self.shown {
                    Vec::new()
                } else {
                    self.shown = true;
                    vec![Effect::VoidPaper(true)]
                }
            }
            Screen::C2 => {
                self.screen = Screen::C3(Check {
                    positions: self.check_positions,
                    done: 0,
                    picker: WordPicker::new(self.list),
                    wrong: None,
                });
                Vec::new()
            }
            Screen::C4 {
                notice: Some(Notice::VoidPaper),
            } => {
                self.drop_created();
                self.screen = Screen::Done;
                vec![
                    Effect::VoidPaper(false),
                    Effect::StartService,
                    Effect::OpenChats,
                ]
            }
            Screen::C4 { .. } => vec![Effect::Seal],
            Screen::R1 => {
                self.screen = Self::r2(self.list, Origin::Fresh);
                Vec::new()
            }
            Screen::R2 {
                entry, checking, ..
            } => match entry.phrase() {
                Some(phrase) if !*checking => {
                    *checking = true;
                    vec![Effect::Check(phrase)]
                }
                _ => Vec::new(),
            },
            Screen::R3 { entry, derived, .. } => match entry.phrase() {
                Some(phrase) => vec![Effect::RestoreConfirmed(phrase, derived.clone())],
                None => Vec::new(),
            },
            Screen::R4 { .. } => {
                self.screen = Screen::Done;
                vec![
                    Effect::VoidPaper(false),
                    Effect::StartService,
                    Effect::OpenChats,
                ]
            }
            // A record that names no readable PeerId offers no restore:
            // the phrase-only path never runs over a record that exists
            // (design §3.4, #303 review F4).
            Screen::K1 {
                reason,
                expected: Some(expected),
            } => {
                let origin = Origin::Recovery {
                    reason: *reason,
                    expected: expected.clone(),
                };
                self.screen = Self::r2(self.list, origin);
                Vec::new()
            }
            Screen::S1 { going_offline, .. } if !*going_offline => vec![Effect::Prompt],
            Screen::S2 => self.leave_s2(),
            Screen::C3(_) | Screen::K1 { .. } | Screen::S1 { .. } | Screen::Done => Vec::new(),
        }
    }

    /// W's secondary action: restore from a phrase.
    pub fn restore(&mut self) -> Vec<Effect> {
        if matches!(self.screen, Screen::Welcome { .. }) {
            self.screen = Screen::R1;
        }
        Vec::new()
    }

    /// Back, the platform's or the app bar's (§3, §7).
    pub fn back(&mut self) -> Vec<Effect> {
        match &mut self.screen {
            Screen::Welcome { .. } | Screen::K1 { .. } => vec![Effect::Finish],
            Screen::C1 => {
                let void_paper = self.shown;
                self.drop_created();
                self.screen = Screen::Welcome { void_paper };
                vec![Effect::Discard]
            }
            Screen::C2 => {
                self.screen = Screen::C1;
                Vec::new()
            }
            Screen::C3(_) => {
                self.screen = Screen::C2;
                Vec::new()
            }
            // The identity is about to be sealed: there is no way back.
            Screen::C4 { .. } | Screen::R4 { .. } | Screen::Done => Vec::new(),
            Screen::R1 => {
                self.screen = Screen::Welcome { void_paper: false };
                Vec::new()
            }
            Screen::R2 {
                entry,
                discard_asked,
                checking,
                ..
            } => {
                if *checking {
                    return Vec::new();
                }
                let entered = (0..PHRASE_WORDS).any(|p| entry.word(p).is_some())
                    || !entry.picker().typed().is_empty();
                if entered && !*discard_asked {
                    *discard_asked = true;
                    Vec::new()
                } else {
                    self.leave_r2();
                    Vec::new()
                }
            }
            Screen::R3 { .. } => {
                self.back_to_r2();
                Vec::new()
            }
            Screen::S1 { going_offline, .. } => {
                if *going_offline {
                    Vec::new()
                } else {
                    self.screen = Screen::Done;
                    vec![Effect::OpenSettings]
                }
            }
            Screen::S2 => self.leave_s2(),
        }
    }

    /// R2's discard dialog: delete the words (`true`) or keep entering.
    pub fn discard(&mut self, delete: bool) {
        if let Screen::R2 { discard_asked, .. } = &mut self.screen {
            *discard_asked = false;
            if delete {
                self.leave_r2();
            }
        }
    }

    /// R3's secondary action, "no, check my words": back to R2 with every
    /// word kept.
    pub fn recheck(&mut self) {
        if matches!(self.screen, Screen::R3 { .. }) {
            self.back_to_r2();
        }
    }

    /// C3, after a wrong word: "Try again" takes the error away; the entry
    /// is already clear.
    pub fn try_again(&mut self) {
        if let Screen::C3(check) = &mut self.screen {
            check.wrong = None;
        }
    }

    /// C3, after a wrong word: show the words again (→ C2); the same two
    /// positions are asked on return.
    pub fn show_words_again(&mut self) {
        if matches!(self.screen, Screen::C3(_)) {
            self.screen = Screen::C2;
        }
    }

    /// A keypad letter on C3 or R2.
    pub fn press(&mut self, letter: char) -> bool {
        self.picker_mut().is_some_and(|picker| picker.press(letter))
    }

    /// Delete a letter on C3 or R2.
    pub fn delete(&mut self) {
        if let Some(picker) = self.picker_mut() {
            picker.delete();
        }
    }

    /// Clear the position's letters on C3 or R2.
    pub fn clear_letters(&mut self) {
        if let Some(picker) = self.picker_mut() {
            picker.clear();
        }
    }

    /// R2: edit a grid position.
    pub fn edit(&mut self, position: usize) {
        if let Screen::R2 { entry, notice, .. } = &mut self.screen {
            entry.edit(position);
            *notice = None;
        }
    }

    /// A chip tapped on C3 or R2. On C3 a right word moves to the next
    /// position or to C4; a wrong one shows which position and clears.
    pub fn choose(&mut self, chip: &str) -> Vec<Effect> {
        match &mut self.screen {
            Screen::R2 { entry, notice, .. } => {
                if entry.choose(chip) {
                    *notice = None;
                }
                Vec::new()
            }
            Screen::C3(check) => {
                let Some(word) = check.picker.chip(chip) else {
                    return Vec::new();
                };
                let position = check.positions[check.done];
                check.picker.clear();
                let right = self.words.is_some_and(|words| words[position] == word);
                if !right {
                    check.wrong = Some(position);
                    return Vec::new();
                }
                check.wrong = None;
                check.done += 1;
                if check.done == check.positions.len() {
                    self.screen = Screen::C4 { notice: None };
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    // --- the platform's answers ----------------------------------------

    /// The new identity, unsealed: its words, its `PeerId`, and the two
    /// distinct positions C3 checks (drawn by the root).
    pub fn created(&mut self, words: Words, peer: TransportIdentity, check: [usize; 2]) {
        debug_assert!(check[0] != check[1] && check.iter().all(|&p| p < PHRASE_WORDS));
        self.words = Some(words);
        self.peer = Some(peer);
        self.check_positions = check;
        self.shown = false;
    }

    /// The profile's lock for the flow: taken, or refused. Refused, the
    /// first screens say why and nothing goes on to a word until a later
    /// [`Effect::Hold`] takes it.
    pub fn held(&mut self, outcome: Result<(), StepFailed>) {
        match outcome {
            Ok(()) => {
                self.unheld = false;
                self.start_notice = None;
            }
            Err(failed) => {
                self.unheld = true;
                self.start_notice = Some(failed.into());
            }
        }
    }

    /// [`Effect::Generate`] made nothing: back to Welcome, saying so,
    /// where Create tries again.
    pub fn not_created(&mut self, failed: StepFailed) {
        if matches!(self.screen, Screen::C1) {
            self.screen = Screen::Welcome { void_paper: false };
            self.start_notice = Some(failed.into());
        }
    }

    /// The seal of C4: done, or why not (nothing was changed).
    pub fn sealed(&mut self, outcome: Result<(), StepFailed>) -> Vec<Effect> {
        let Screen::C4 { notice } = &mut self.screen else {
            return Vec::new();
        };
        match outcome {
            Ok(()) => {
                self.drop_created();
                self.screen = Screen::Done;
                vec![
                    Effect::VoidPaper(false),
                    Effect::StartService,
                    Effect::OpenChats,
                ]
            }
            Err(failed) => {
                *notice = Some(failed.into());
                Vec::new()
            }
        }
    }

    /// R2's check: the `PeerId` the phrase derives to, or `None` for a
    /// failed checksum.
    pub fn checked(&mut self, derived: Option<TransportIdentity>) -> Vec<Effect> {
        let Screen::R2 {
            origin,
            entry,
            notice,
            checking,
            ..
        } = &mut self.screen
        else {
            return Vec::new();
        };
        *checking = false;
        let Some(derived) = derived else {
            *notice = Some(Notice::ChecksumFailed);
            return Vec::new();
        };
        let expected = match origin {
            Origin::Recovery { expected, .. } => Some(expected.clone()),
            Origin::Fresh => None,
        };
        match expected {
            None => {
                let origin = origin.clone();
                let entry = std::mem::replace(entry, PhraseEntry::new(self.list));
                self.screen = Screen::R3 {
                    origin,
                    entry,
                    derived,
                    notice: None,
                };
                Vec::new()
            }
            Some(expected) if expected != derived => {
                *notice = Some(Notice::OtherIdentity { derived, expected });
                Vec::new()
            }
            Some(_) => match entry.phrase() {
                Some(phrase) => {
                    *checking = true;
                    vec![Effect::RestoreRecorded(phrase)]
                }
                None => Vec::new(),
            },
        }
    }

    /// A restore's outcome, from R2 (against the record) or R3.
    pub fn restored(&mut self, outcome: Result<(), StepFailed>) -> Vec<Effect> {
        match (&mut self.screen, outcome) {
            (Screen::R2 { origin, .. }, Ok(())) => {
                let peer = match origin {
                    Origin::Recovery { expected, .. } => Some(expected.clone()),
                    Origin::Fresh => None,
                };
                self.screen = Screen::R4 { peer };
                Vec::new()
            }
            (Screen::R3 { derived, .. }, Ok(())) => {
                let peer = Some(derived.clone());
                self.screen = Screen::R4 { peer };
                Vec::new()
            }
            (
                Screen::R2 {
                    notice, checking, ..
                },
                Err(failed),
            ) => {
                *checking = false;
                *notice = Some(failed.into());
                Vec::new()
            }
            (Screen::R3 { notice, .. }, Err(failed)) => {
                *notice = Some(failed.into());
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// A key operation this flow asked for did not run: its prompt was
    /// cancelled or denied (the presence window, `PresenceGate`). Nothing
    /// was committed and the screen stays, with its words, and no notice:
    /// the person chose not to. R2's check gives the primary action back.
    pub fn cancelled_step(&mut self) {
        if let Screen::R2 { checking, .. } = &mut self.screen {
            *checking = false;
        }
    }

    /// S1's prompt: granted, or cancelled/denied (back on S1, nothing
    /// shown, no error).
    pub fn prompted(&mut self, granted: bool) -> Vec<Effect> {
        let Screen::S1 {
            going_offline,
            notice,
        } = &mut self.screen
        else {
            return Vec::new();
        };
        if !granted {
            return Vec::new();
        }
        *going_offline = true;
        *notice = None;
        vec![Effect::GoOffline]
    }

    /// The service is offline and the words and `PeerId` re-derived
    /// (→ S2), or it could not be taken offline.
    pub fn offline(
        &mut self,
        outcome: Result<(Words, TransportIdentity), StepFailed>,
    ) -> Vec<Effect> {
        let Screen::S1 {
            going_offline,
            notice,
        } = &mut self.screen
        else {
            return Vec::new();
        };
        *going_offline = false;
        match outcome {
            Ok((words, peer)) => {
                self.words = Some(words);
                self.peer = Some(peer);
                self.screen = Screen::S2;
                Vec::new()
            }
            Err(failed) => {
                *notice = Some(failed.into());
                Vec::new()
            }
        }
    }

    // --- helpers ---------------------------------------------------------

    fn r2(list: &'static [&'static str], origin: Origin) -> Screen {
        Screen::R2 {
            origin,
            entry: PhraseEntry::new(list),
            notice: None,
            checking: false,
            discard_asked: false,
        }
    }

    fn picker_mut(&mut self) -> Option<&mut WordPicker> {
        match &mut self.screen {
            Screen::C3(check) => Some(&mut check.picker),
            Screen::R2 { entry, .. } => Some(entry.picker_mut()),
            _ => None,
        }
    }

    fn leave_r2(&mut self) {
        if let Screen::R2 { origin, .. } = &self.screen {
            self.screen = origin.back();
        }
    }

    fn back_to_r2(&mut self) {
        let Screen::R3 { origin, entry, .. } = &mut self.screen else {
            return;
        };
        let origin = origin.clone();
        let entry = std::mem::replace(entry, PhraseEntry::new(self.list));
        self.screen = Screen::R2 {
            origin,
            entry,
            notice: None,
            checking: false,
            discard_asked: false,
        };
    }

    fn leave_s2(&mut self) -> Vec<Effect> {
        self.words = None;
        self.peer = None;
        self.screen = Screen::Done;
        vec![Effect::StartService, Effect::OpenSettings]
    }

    fn drop_created(&mut self) {
        self.words = None;
        self.peer = None;
        self.shown = false;
    }
}

impl fmt::Debug for IdentityFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IdentityFlow({:?}, <words redacted>)", self.screen())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sorted, and unique by the first four letters, as BIP-39's list is.
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

    fn peer(id: &str) -> TransportIdentity {
        TransportIdentity::parse(id).expect("a PeerId")
    }

    fn mine() -> TransportIdentity {
        peer("12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e")
    }

    fn other() -> TransportIdentity {
        peer("12D3KooW9xCm2jWjNVrwh51SWCQBMYdMyeU3NpT85QhLVkF6PcNM")
    }

    /// Type `word`'s letters until it is a chip, then tap it.
    fn enter(flow: &mut IdentityFlow, word: &str) -> Vec<Effect> {
        for letter in word.chars() {
            if flow.picker().and_then(|p| p.chip(word)).is_some() {
                break;
            }
            assert!(flow.press(letter), "{letter} is enabled");
        }
        flow.choose(word)
    }

    fn created_at_c2() -> IdentityFlow {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        assert_eq!(flow.primary(), [Effect::VoidPaper(false), Effect::Generate]);
        flow.created(words(), mine(), [6, 18]);
        assert_eq!(
            flow.primary(),
            [Effect::VoidPaper(true)],
            "C2's first display"
        );
        flow
    }

    #[test]
    fn create_shows_words_checks_two_and_seals_only_at_c4() {
        let mut flow = created_at_c2();
        assert_eq!(flow.screen(), ScreenId::C2);
        assert_eq!(flow.words(), Some(&words()));
        assert_eq!(flow.peer(), Some(&mine()));
        flow.primary();
        assert_eq!(flow.check(), Some((6, None)));
        enter(&mut flow, "galaxy");
        assert_eq!(flow.check(), Some((18, None)));
        enter(&mut flow, "saddle");
        assert_eq!(flow.screen(), ScreenId::C4);
        assert_eq!(flow.back(), [], "C4 has no Back");
        assert_eq!(flow.screen(), ScreenId::C4);
        assert_eq!(flow.primary(), [Effect::Seal]);
        assert_eq!(
            flow.sealed(Ok(())),
            [
                Effect::VoidPaper(false),
                Effect::StartService,
                Effect::OpenChats
            ]
        );
        assert_eq!(flow.screen(), ScreenId::Done);
    }

    #[test]
    fn a_wrong_check_word_names_its_position_and_there_is_no_way_past() {
        let mut flow = created_at_c2();
        flow.primary();
        enter(&mut flow, "zebra");
        assert_eq!(flow.check(), Some((6, Some(6))));
        assert_eq!(flow.picker().map(WordPicker::typed), Some(""), "cleared");
        flow.try_again();
        assert_eq!(flow.check(), Some((6, None)));
        enter(&mut flow, "zebra");
        flow.show_words_again();
        assert_eq!(flow.screen(), ScreenId::C2);
        assert_eq!(flow.primary(), [], "the marker is already set");
        assert_eq!(flow.check(), Some((6, None)), "the same positions again");
    }

    #[test]
    fn back_from_c1_discards_and_warns_of_a_void_paper_only_once_words_showed() {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        flow.primary();
        flow.created(words(), mine(), [0, 1]);
        assert_eq!(flow.back(), [Effect::Discard]);
        assert!(!flow.void_paper(), "no words were shown");

        let mut flow = created_at_c2();
        flow.back();
        assert_eq!(flow.back(), [Effect::Discard]);
        assert_eq!(flow.screen(), ScreenId::W);
        assert!(flow.void_paper());
        assert_eq!(flow.words(), None);
    }

    #[test]
    fn a_failed_seal_is_shown_on_c4_and_can_be_tried_again() {
        let mut flow = created_at_c2();
        flow.primary();
        enter(&mut flow, "galaxy");
        enter(&mut flow, "saddle");
        flow.primary();
        assert_eq!(flow.sealed(Err(StepFailed::Storage)), []);
        assert_eq!(flow.notice(), Some(&Notice::StorageFailed));
        assert_eq!(flow.primary(), [Effect::Seal]);
    }

    fn fill(flow: &mut IdentityFlow) {
        for word in words() {
            enter(flow, word);
        }
        assert!(flow.entry().is_some_and(PhraseEntry::complete));
    }

    /// No word is shown or taken before the root holds the profile's
    /// lock: refused, the first screens say why and wait, and Try again
    /// asks for it again (android-key-custody.md, Recovery; #309 F4).
    #[test]
    fn no_word_is_shown_or_taken_until_the_lock_is_held() {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        flow.held(Err(StepFailed::Busy));
        assert_eq!(flow.notice(), Some(&Notice::Busy));
        assert_eq!(flow.primary(), [Effect::Hold], "Create waits for the lock");
        assert_eq!(flow.screen(), ScreenId::W);
        flow.restore();
        assert_eq!(flow.screen(), ScreenId::R1, "R1 shows no word");
        assert_eq!(flow.primary(), [Effect::Hold], "R2 waits for the lock");
        assert_eq!(flow.screen(), ScreenId::R1);
        flow.held(Ok(()));
        assert_eq!(flow.notice(), None);
        assert!(flow.primary().is_empty());
        assert_eq!(flow.screen(), ScreenId::R2);

        let mut flow = IdentityFlow::new(
            LIST,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
            },
        );
        flow.held(Err(StepFailed::Storage));
        assert_eq!(flow.primary(), [Effect::Hold]);
        assert_eq!(flow.screen(), ScreenId::K1);
        assert_eq!(flow.notice(), Some(&Notice::StorageFailed));
    }

    /// A new identity that could not be made goes back to Welcome and
    /// says so, never on to an empty grid (#309 F7).
    #[test]
    fn an_identity_not_made_is_said_on_welcome_and_create_tries_again() {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        assert_eq!(flow.primary(), [Effect::VoidPaper(false), Effect::Generate]);
        flow.not_created(StepFailed::Storage);
        assert_eq!(flow.screen(), ScreenId::W);
        assert_eq!(flow.notice(), Some(&Notice::StorageFailed));
        assert_eq!(
            flow.primary(),
            [Effect::VoidPaper(false), Effect::Generate],
            "Create tries again"
        );
        assert_eq!(flow.notice(), None);
    }

    #[test]
    fn a_fresh_install_restores_only_the_peer_the_person_confirmed() {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: true });
        flow.restore();
        flow.primary();
        assert_eq!(flow.screen(), ScreenId::R2);
        fill(&mut flow);
        let phrase = words().join(" ");
        assert_eq!(flow.primary(), [Effect::Check(phrase.clone())]);
        assert_eq!(flow.primary(), [], "one check at a time");

        flow.checked(None);
        assert_eq!(flow.notice(), Some(&Notice::ChecksumFailed));
        assert!(
            flow.entry().is_some_and(PhraseEntry::complete),
            "every word stays"
        );

        flow.primary();
        flow.checked(Some(mine()));
        assert_eq!(flow.screen(), ScreenId::R3);
        assert_eq!(flow.peer(), Some(&mine()));
        flow.recheck();
        assert!(
            flow.entry().is_some_and(PhraseEntry::complete),
            "words kept"
        );
        flow.primary();
        flow.checked(Some(mine()));

        assert_eq!(flow.primary(), [Effect::RestoreConfirmed(phrase, mine())]);
        assert_eq!(flow.restored(Err(StepFailed::Busy)), []);
        assert_eq!(flow.notice(), Some(&Notice::Busy));
        assert_eq!(flow.restored(Ok(())), []);
        assert_eq!(flow.screen(), ScreenId::R4);
        assert_eq!(flow.peer(), Some(&mine()), "R4 shows what was restored");
        assert_eq!(
            flow.primary(),
            [
                Effect::VoidPaper(false),
                Effect::StartService,
                Effect::OpenChats
            ]
        );
    }

    #[test]
    fn recovery_restores_against_the_record_with_no_r3_and_refuses_another_identity() {
        let mut flow = IdentityFlow::new(
            LIST,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
            },
        );
        assert_eq!(flow.recovery(), Some((RecoveryReason::Key, Some(&mine()))));
        assert_eq!(flow.back(), [Effect::Finish], "Back leaves the app");
        flow.primary();
        fill(&mut flow);
        flow.primary();
        assert_eq!(flow.checked(Some(other())), []);
        assert_eq!(
            flow.notice(),
            Some(&Notice::OtherIdentity {
                derived: other(),
                expected: mine()
            })
        );
        flow.primary();
        assert_eq!(
            flow.checked(Some(mine())),
            [Effect::RestoreRecorded(words().join(" "))]
        );
        assert_eq!(flow.screen(), ScreenId::R2, "no R3 against a record");
        flow.restored(Ok(()));
        assert_eq!(flow.screen(), ScreenId::R4);
        assert_eq!(flow.peer(), Some(&mine()));
    }

    /// Acceptance 10: a restore whose presence prompt was cancelled
    /// commits nothing, shows no notice, keeps the words and gives the
    /// primary action back, so the person can try again.
    #[test]
    fn a_cancelled_restore_keeps_the_words_and_gives_the_action_back() {
        let mut flow = IdentityFlow::new(
            LIST,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
            },
        );
        flow.primary();
        fill(&mut flow);
        flow.primary();
        assert_eq!(
            flow.checked(Some(mine())),
            [Effect::RestoreRecorded(words().join(" "))],
            "control: the restore asked"
        );
        assert_eq!(flow.primary(), [], "control: busy while it runs");
        flow.cancelled_step();
        assert_eq!(flow.screen(), ScreenId::R2);
        assert_eq!(flow.notice(), None, "the person chose not to");
        assert_eq!(
            flow.primary(),
            [Effect::Check(words().join(" "))],
            "the words are kept and the action is back"
        );
    }

    /// ADR-0043's warning (item 11) on both ways into R2, a fresh
    /// install's and a lost key's: the screen R2 was entered from warned,
    /// R2 was reached all the same, and R2 does not carry it.
    #[test]
    fn every_path_into_r2_passes_a_screen_that_warns() {
        let fresh = {
            let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
            assert!(!flow.warns_active_elsewhere(), "Welcome restores nothing");
            flow.restore();
            flow
        };
        let recovery = |reason| {
            IdentityFlow::new(
                LIST,
                Start::Recovery {
                    reason,
                    expected: Some(mine()),
                },
            )
        };
        for mut flow in [
            fresh,
            recovery(RecoveryReason::Key),
            recovery(RecoveryReason::Record),
        ] {
            let from = flow.screen();
            assert!(flow.warns_active_elsewhere(), "{from:?} warns");
            assert_eq!(flow.primary(), [], "{from:?}: a statement, not a gate");
            assert_eq!(flow.screen(), ScreenId::R2, "from {from:?}");
            assert!(!flow.warns_active_elsewhere(), "R2 keeps its room");
            flow.back();
            assert_eq!(flow.screen(), from, "Back returns to it");
            assert!(flow.warns_active_elsewhere(), "{from:?} warns again");
        }

        let mut no_restore = IdentityFlow::new(
            LIST,
            Start::Recovery {
                reason: RecoveryReason::Record,
                expected: None,
            },
        );
        assert!(
            !no_restore.warns_active_elsewhere(),
            "K1 with no restore takes no phrase"
        );
        assert_eq!(no_restore.primary(), []);
    }

    /// The warning is R1's and K1's only: no screen that shows, checks or
    /// takes a word carries it, nor the screens after a restore, so their
    /// layouts are as they were (#392 review F1).
    #[test]
    fn no_screen_that_shows_or_takes_a_word_warns() {
        use ScreenId::{C1, C2, C3, C4, R2, R3, R4, S1, S2};

        let mut seen = Vec::new();
        let mut look = |flow: &IdentityFlow| {
            assert!(!flow.warns_active_elsewhere(), "{:?} warns", flow.screen());
            seen.push(flow.screen());
        };

        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        flow.primary();
        look(&flow);
        flow.created(words(), mine(), [6, 18]);
        flow.primary();
        look(&flow);
        flow.primary();
        look(&flow);
        enter(&mut flow, "galaxy");
        enter(&mut flow, "saddle");
        look(&flow);

        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        flow.restore();
        flow.primary();
        fill(&mut flow);
        look(&flow);
        flow.primary();
        flow.checked(Some(mine()));
        look(&flow);
        flow.primary();
        flow.restored(Ok(()));
        look(&flow);

        let mut flow = IdentityFlow::new(LIST, Start::ShowPhrase);
        look(&flow);
        flow.primary();
        flow.prompted(true);
        flow.offline(Ok((words(), mine())));
        look(&flow);

        assert_eq!(seen, [C1, C2, C3, C4, R2, R3, R4, S1, S2], "control");
    }

    #[test]
    fn a_record_naming_no_peer_offers_no_restore_at_all() {
        let mut flow = IdentityFlow::new(
            LIST,
            Start::Recovery {
                reason: RecoveryReason::Record,
                expected: None,
            },
        );
        assert!(!flow.can_restore());
        assert_eq!(flow.primary(), []);
        assert_eq!(flow.screen(), ScreenId::K1, "no R2 over a record");
        assert_eq!(flow.back(), [Effect::Finish]);
    }

    #[test]
    fn back_from_r2_after_k1_returns_to_k1_with_its_reason() {
        let mut flow = IdentityFlow::new(
            LIST,
            Start::Recovery {
                reason: RecoveryReason::Record,
                expected: Some(mine()),
            },
        );
        flow.primary();
        enter(&mut flow, "acid");
        flow.back();
        flow.discard(true);
        assert_eq!(
            flow.recovery(),
            Some((RecoveryReason::Record, Some(&mine())))
        );
    }

    #[test]
    fn a_seal_that_finds_another_identity_warns_of_a_void_paper_and_opens_it() {
        let mut flow = created_at_c2();
        flow.primary();
        enter(&mut flow, "galaxy");
        enter(&mut flow, "saddle");
        flow.primary();
        flow.sealed(Err(StepFailed::AlreadyProvisioned));
        assert_eq!(flow.notice(), Some(&Notice::VoidPaper));
        assert_eq!(
            flow.primary(),
            [
                Effect::VoidPaper(false),
                Effect::StartService,
                Effect::OpenChats
            ],
            "Continue opens what the phone holds, and seals nothing"
        );
    }

    #[test]
    fn back_from_r2_with_words_asks_first_and_keeping_stays() {
        let mut flow = IdentityFlow::new(LIST, Start::FirstRun { void_paper: false });
        flow.restore();
        flow.primary();
        assert_eq!(flow.back(), []);
        assert_eq!(
            flow.screen(),
            ScreenId::R1,
            "nothing entered: straight back"
        );
        flow.primary();
        enter(&mut flow, "acid");
        flow.back();
        assert!(flow.discard_asked());
        flow.discard(false);
        assert_eq!(flow.screen(), ScreenId::R2);
        assert_eq!(flow.entry().and_then(|e| e.word(0)), Some("acid"));
        flow.back();
        flow.discard(true);
        assert_eq!(flow.screen(), ScreenId::R1);
        flow.primary();
        assert_eq!(flow.entry().and_then(|e| e.word(0)), None, "words dropped");
    }

    #[test]
    fn show_phrase_prompts_goes_offline_and_restarts_the_service_after() {
        let mut flow = IdentityFlow::new(LIST, Start::ShowPhrase);
        assert_eq!(flow.primary(), [Effect::Prompt]);
        assert_eq!(flow.prompted(false), []);
        assert_eq!(flow.screen(), ScreenId::S1);
        assert_eq!(flow.notice(), None, "a cancelled prompt is no error");
        flow.primary();
        assert_eq!(flow.prompted(true), [Effect::GoOffline]);
        assert!(flow.waiting());
        assert_eq!(flow.back(), [], "not while going offline");
        flow.offline(Err(StepFailed::Busy));
        assert_eq!(flow.notice(), Some(&Notice::Busy));
        flow.primary();
        flow.prompted(true);
        flow.offline(Ok((words(), mine())));
        assert_eq!(flow.screen(), ScreenId::S2);
        assert_eq!(flow.words(), Some(&words()));
        assert_eq!(flow.peer(), Some(&mine()));
        assert_eq!(flow.back(), [Effect::StartService, Effect::OpenSettings]);
        assert_eq!(flow.words(), None);
    }

    #[test]
    fn debug_output_carries_no_word() {
        let flow = created_at_c2();
        let printed = format!("{flow:?}");
        assert!(LIST.iter().all(|word| !printed.contains(word)), "{printed}");
    }
}

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
        /// The state root holds no `identity-keys/` directory, so a
        /// restore gives every identity a new key (IDENTITY-RECOVERY.md
        /// section Restore semantics item 12; ADR-0063 decision 1).
        new_keys: bool,
    },
    /// Settings asked to show the phrase (§3.5).
    ShowPhrase,
}

/// A failure the platform reports for a step that stops the service or
/// writes (§6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepFailed {
    /// The profile stayed held past the wait (`IdBusy`).
    Busy,
    /// Storage or the cipher failed; nothing was changed
    /// (`IdStorageFailed`).
    Storage,
    /// A refusal that may follow the write, whose read-back did not find
    /// the record opening to the `PeerId` shown or confirmed: the app
    /// cannot tell whether the identity was saved (`IdStorageUnknown`,
    /// design section 6).
    StorageUnknown,
    /// C4's seal found the profile already provisioned: another first run
    /// sealed first, so the words shown were never used
    /// (`IdAlreadyCreated`, #303 review F5).
    AlreadyProvisioned,
    /// A restore found a record that names another `PeerId`, the one
    /// carried here (`IdRestoreOther`, design section 6).
    RestoreOther(TransportIdentity),
    /// The read-back after a refused write, or `IdStorageUnknown`'s Try
    /// again of it, was refused because authentication is needed: the
    /// flow asks for the prompt ([`Effect::AuthenticateReadBack`]) rather
    /// than show a notice (design section 6, Authentication needed).
    ReadBackNeedsAuthentication,
}

/// Why the service start of C4's `IdStartUnchecked` did not run (design
/// section 6, A start from C4 after a failed read-back), as the root
/// learnt it from the start's refusal. A refusal for the person and one
/// with the profile in use are not here: they answer as any held start's
/// (the gate's prompt, [`IdentityFlow::start_busy`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UncheckedRefused {
    /// The record opened to another `PeerId` than the one C4 shows (the
    /// app's comparison): nothing started.
    OtherIdentity,
    /// A platform error said nothing of the record: try again.
    Unavailable,
    /// The record does not open (`RecoveryRequired`, whatever its
    /// cause): K1, as at a launch.
    Recovery {
        /// Why, by cause.
        reason: RecoveryReason,
        /// The `PeerId` the record names, when it still names one.
        expected: Option<TransportIdentity>,
        /// As [`Start::Recovery`]'s.
        new_keys: bool,
    },
}

/// Whether C4 offers `IdStartUnchecked` (design section 6, A start from
/// C4 after a failed read-back).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Offer {
    /// No read-back on this C4 has answered as failing yet.
    No,
    /// One has: offered while `IdStorageUnknown` shows.
    Yes,
    /// A start from it met another `PeerId` than C4's: not offered again
    /// while C4 is open.
    Withdrawn,
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
    /// Read the record back, writing nothing: does it open, under the
    /// cipher, to this `PeerId`? `IdStorageUnknown`'s Try again (design
    /// section 6). Answered as the step it reads back for:
    /// [`IdentityFlow::sealed`] on C4, [`IdentityFlow::restored`] on R2
    /// and R3, with [`StepFailed::StorageUnknown`] when it does not.
    ReadBack(TransportIdentity),
    /// A read-back was refused because authentication is needed (design
    /// section 6, Authentication needed): raise the prompt -- the refusal
    /// is the app's only reading of the Keystore's window -- and, granted, carry out [`Effect::ReadBack`] of
    /// this `PeerId`. Cancelled, denied or dropped, answer
    /// [`IdentityFlow::step_not_run`] with that read-back.
    AuthenticateReadBack(TransportIdentity),
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
    /// `IdStorageUnknown`: Try again reads the record back.
    StorageUnknown,
    /// `IdAlreadyCreated` on C4: the phone holds another identity, and
    /// Continue opens it.
    AlreadyCreated,
    /// `IdRestoreOther` on R3 or R2: the phone's record names
    /// `recorded`, shown under the notice for the eye alone, and the
    /// notice's Continue opens it.
    RestoreOther {
        /// The `PeerId` the record names.
        recorded: TransportIdentity,
    },
    /// `IdStartBusy`: the service start a held Continue made was refused
    /// with the profile in use (the runtime's `LockHeld`) before
    /// add-a-peer §5.1's bound (design section 6). An identity is on
    /// record, so Try again in the primary's place -- on R2 and R3 in
    /// `IdRestoreOther`'s Continue's -- makes one service start and
    /// nothing else ([`IdentityFlow::start_busy`]).
    StartBusy {
        /// On R2 and R3, the `PeerId` `IdRestoreOther` showed, kept under
        /// this notice.
        recorded: Option<TransportIdentity>,
        /// On C4, the start was `IdAlreadyCreated`'s Continue's: the title
        /// stays `IdAlreadyTitle` and the `PeerId` the unused words made
        /// stays hidden.
        already: bool,
    },
}

impl Notice {
    /// The notices whose action is Try again, which retries the step that
    /// raised them (design section 6).
    #[must_use]
    pub const fn retries(&self) -> bool {
        matches!(
            self,
            Self::Busy | Self::StorageFailed | Self::StorageUnknown
        )
    }
}

impl From<StepFailed> for Notice {
    fn from(failed: StepFailed) -> Self {
        match failed {
            StepFailed::Busy => Self::Busy,
            StepFailed::Storage => Self::StorageFailed,
            // A read-back the flow could not ask the prompt for (no
            // `PeerId` to read back for) is still one whose answer is
            // unknown.
            StepFailed::StorageUnknown | StepFailed::ReadBackNeedsAuthentication => {
                Self::StorageUnknown
            }
            StepFailed::AlreadyProvisioned => Self::AlreadyCreated,
            StepFailed::RestoreOther(recorded) => Self::RestoreOther { recorded },
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
    discard_asked: bool,
}

impl Check {
    /// A right word was taken, or letters are typed: Back asks first.
    fn entered(&self) -> bool {
        self.done > 0 || !self.picker.typed().is_empty()
    }
}

/// The screen showing.
enum Screen {
    Welcome {
        void_paper: bool,
    },
    C1,
    /// Your 24 words, then, on `peer_page`, C2's `PeerId` page: the words
    /// and the `PeerId` are never on one page (section 3.2;
    /// `the_words_and_the_peer_id_are_never_on_one_page`). One screen,
    /// so every rule of C2 -- the words held until C4, the void-paper
    /// marker, Back to C1 from the words -- holds on both pages.
    C2 {
        peer_page: bool,
    },
    C3(Check),
    C4 {
        notice: Option<Notice>,
        /// Continue was pressed: the screen holds, Continue disabled,
        /// until the root says the start decided (add-a-peer §5.1).
        held: bool,
        /// The root's clock says `ConnectStarting` shows under the held
        /// Continue ([`crate::StartHold::starting`]); cleared at each
        /// press.
        starting: bool,
        /// `IdStartUnchecked` is offered, or withdrawn for C4's life.
        offer: Offer,
        /// The start held, or the one `IdStartBusy` shows refused, is
        /// `IdStartUnchecked`'s: it must open the record to C4's `PeerId`
        /// ([`IdentityFlow::start_expects`]), and `IdStartBusy`'s Try
        /// again repeats it, comparison included.
        unchecked: bool,
    },
    R1,
    R2 {
        origin: Origin,
        entry: PhraseEntry,
        notice: Option<Notice>,
        checking: bool,
        discard_asked: bool,
        /// "Show all my words" opened the grid as a full list (section
        /// 4.4, over 130 % font): flow state, so a rotation or a return
        /// from Home finds it open.
        reviewing: bool,
        /// `IdRestoreOther`'s Continue was pressed: the service starts
        /// with the identity on record, and R2 holds, `checking` set,
        /// until the root says the start decided (section 8).
        started: bool,
        /// As C4's.
        starting: bool,
        /// A restore from here was answered `IdStorageUnknown`: whether
        /// the record now opens to the `PeerId` the restore confirmed is
        /// unknown, and stays so after an edit takes the notice, until a
        /// read-back opens the record to that `PeerId` or R2 closes. A check whose words derive that
        /// `PeerId` reads back rather than restoring again (design section
        /// 6, the read-back row;
        /// `an_edit_after_storage_unknown_keeps_the_read_back`).
        unknown: bool,
    },
    R3 {
        origin: Origin,
        entry: PhraseEntry,
        derived: TransportIdentity,
        notice: Option<Notice>,
        /// The restore runs: from the press that starts it (Yes, or a
        /// notice's Try again) until its result, a cancelled prompt, or a
        /// prompt dropped before it was raised (`PresenceGate::unasked`).
        /// R3 holds meanwhile -- no second restore, no Back, no "no,
        /// check my words" -- and the notice it showed stays (design
        /// section 8, While R3's restore runs).
        restoring: bool,
        /// As R2's: `IdRestoreOther`'s Continue, `restoring` set.
        started: bool,
        /// As C4's.
        starting: bool,
    },
    R4 {
        peer: Option<TransportIdentity>,
        /// `IdStartBusy`, in C4's notice place (R4 has C4's layout,
        /// section 3.3); the only notice R4 shows.
        notice: Option<Notice>,
        /// As C4's.
        held: bool,
        /// As C4's.
        starting: bool,
        /// The restore R4 follows was R2's after K1 (`Origin::Recovery`):
        /// its Continue ends the comparison with C4's `PeerId`
        /// ([`IdentityFlow::start_ends_comparison`]).
        after_k1: bool,
    },
    K1 {
        reason: RecoveryReason,
        expected: Option<TransportIdentity>,
    },
    S1 {
        going_offline: bool,
        notice: Option<Notice>,
    },
    /// As C2: the words, then on `peer_page` the `PeerId` (section 3.5);
    /// the time on S2 counts both pages.
    S2 {
        peer_page: bool,
    },
    Done,
}

/// Who asked a held screen's service start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeldBy {
    /// The person's press: Continue, or a notice's Try again or Continue.
    Press,
    /// A showing of the window that started the service again.
    Showing,
}

/// Which screen shows, for the view (§3's names).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    /// Welcome (§3.1).
    W,
    /// Before you see your words.
    C1,
    /// Your 24 words: C2's words page.
    C2,
    /// C2's `PeerId` page, after the words.
    C2PeerId,
    /// Check two words.
    C3,
    /// `PeerId` created.
    C4,
    /// Restore your `PeerId`.
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
    /// Your 24 words, on demand: S2's words page.
    S2,
    /// S2's `PeerId` page, after the words.
    S2PeerId,
    /// The flow handed over to Chats or Settings.
    Done,
}

/// The identity screens' state.
pub struct IdentityFlow {
    list: &'static [&'static str],
    /// The phrase's format identifier, the recovery record's own, which
    /// the words pages show (section 3.2): the root hands it over with
    /// the word list, so the client holds no copy of it.
    format: &'static str,
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
    /// K1's restore gives every identity a new key: the root found no
    /// `identity-keys/` (item 12). Kept for the flow, since K1 is rebuilt
    /// when Back leaves R2.
    new_keys: bool,
    /// How many chips C3 and R2 have taken, a tap's or Enter's: the page
    /// carries it, so the window sees a chip taken in the page that
    /// shows it rather than guessing at the press (j261).
    chips_taken: u32,
    /// How many grid positions R2 has opened to edit, from the grid or
    /// the full list: as with `chips_taken`, the page carries it, so the
    /// window hands focus to the keypad on the page that shows the
    /// position chosen (design section 8).
    positions_chosen: u32,
    /// How many notices a step's result has raised on C4, R2, R3 or S1:
    /// the page carries it, so the window gives focus to a notice raised
    /// again with the text it already showed, such as a second failed
    /// checksum (design section 8, #697 review P-2).
    notices_raised: u32,
    /// Who asked the held start: a showing's ([`Self::showing_started`])
    /// answers no press, so the notice its `LockHeld` shows again is not
    /// raised for focus. `Showing` only while the screen holds for the
    /// showing's own start: taken back to `Press` by its `LockHeld`, or by
    /// [`Self::person_started`] when a granted prompt starts it again.
    held_by: HeldBy,
}

impl IdentityFlow {
    /// A flow over `list` (BIP-39's English list on the device), whose
    /// phrases are in `format` (the recovery record's identifier),
    /// starting as `start` says.
    #[must_use]
    pub fn new(list: &'static [&'static str], format: &'static str, start: Start) -> Self {
        let mut new_keys = false;
        let screen = match start {
            Start::FirstRun { void_paper } => Screen::Welcome { void_paper },
            Start::Recovery {
                reason,
                expected,
                new_keys: none_kept,
            } => {
                new_keys = none_kept;
                Screen::K1 { reason, expected }
            }
            Start::ShowPhrase => Screen::S1 {
                going_offline: false,
                notice: None,
            },
        };
        Self {
            list,
            format,
            screen,
            words: None,
            peer: None,
            check_positions: [0, 1],
            shown: false,
            unheld: false,
            start_notice: None,
            new_keys,
            chips_taken: 0,
            positions_chosen: 0,
            notices_raised: 0,
            held_by: HeldBy::Press,
        }
    }

    /// Which screen shows.
    #[must_use]
    pub fn screen(&self) -> ScreenId {
        match &self.screen {
            Screen::Welcome { .. } => ScreenId::W,
            Screen::C1 => ScreenId::C1,
            Screen::C2 { peer_page: false } => ScreenId::C2,
            Screen::C2 { peer_page: true } => ScreenId::C2PeerId,
            Screen::C3(_) => ScreenId::C3,
            Screen::C4 { .. } => ScreenId::C4,
            Screen::R1 => ScreenId::R1,
            Screen::R2 { .. } => ScreenId::R2,
            Screen::R3 { .. } => ScreenId::R3,
            Screen::R4 { .. } => ScreenId::R4,
            Screen::K1 { .. } => ScreenId::K1,
            Screen::S1 { .. } => ScreenId::S1,
            Screen::S2 { peer_page: false } => ScreenId::S2,
            Screen::S2 { peer_page: true } => ScreenId::S2PeerId,
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

    /// The screen says that the phrase restores the node and not the
    /// identities' keys, so every identity gets a new key and contacts see
    /// it change (`human-client-ui.md` section 9; IDENTITY-RECOVERY.md
    /// section Restore semantics item 12; ADR-0063 decision 1). R1
    /// always: it follows Welcome, which shows only where no identity was
    /// ever sealed here. K1 only when it offers a restore and the root
    /// found no `identity-keys/`: a lost seed key leaves the identities'
    /// keys where they were, and a restore keeps them. On the screens
    /// that warn of [`Self::warns_active_elsewhere`], and never a gate.
    /// `r1_always_says_new_keys` and
    /// `k1_says_new_keys_only_with_no_identity_keys` hold it.
    #[must_use]
    pub fn says_new_keys(&self) -> bool {
        matches!(self.screen, Screen::R1) || (self.new_keys && self.can_restore())
    }

    /// K1's reason and the `PeerId` the record names.
    #[must_use]
    pub fn recovery(&self) -> Option<(RecoveryReason, Option<&TransportIdentity>)> {
        match &self.screen {
            Screen::K1 { reason, expected } => Some((*reason, expected.as_ref())),
            _ => None,
        }
    }

    /// The words C2's or S2's words page shows. None on their `PeerId`
    /// page, where the words are held and not shown (section 3.2;
    /// `the_words_and_the_peer_id_are_never_on_one_page`).
    #[must_use]
    pub fn words(&self) -> Option<&Words> {
        match self.screen {
            Screen::C2 { peer_page: false } | Screen::S2 { peer_page: false } => {
                self.words.as_ref()
            }
            _ => None,
        }
    }

    /// The format identifier the words pages show under the words' line
    /// (section 3.2): wherever [`Self::words`] shows words, and nowhere
    /// else.
    #[must_use]
    pub fn format(&self) -> Option<&'static str> {
        self.words().map(|_| self.format)
    }

    /// The `PeerId` C2's or S2's `PeerId` page, C4, R3 or R4 shows. None
    /// on C2's and S2's words pages (section 3.2).
    #[must_use]
    pub fn peer(&self) -> Option<&TransportIdentity> {
        match &self.screen {
            Screen::C2 { peer_page: true } | Screen::C4 { .. } | Screen::S2 { peer_page: true } => {
                self.peer.as_ref()
            }
            Screen::R3 { derived, .. } => Some(derived),
            Screen::R4 { peer, .. } => peer.as_ref(),
            _ => None,
        }
    }

    /// The notice the screen shows, if any.
    #[must_use]
    pub fn notice(&self) -> Option<&Notice> {
        match &self.screen {
            Screen::C4 { notice, .. }
            | Screen::R2 { notice, .. }
            | Screen::R3 { notice, .. }
            | Screen::R4 { notice, .. }
            | Screen::S1 { notice, .. } => notice.as_ref(),
            Screen::Welcome { .. } | Screen::R1 | Screen::K1 { .. } => self.start_notice.as_ref(),
            _ => None,
        }
    }

    /// S1 is taking the service offline (`IdGoingOffline`), R2 is
    /// checking, or R3 is restoring: the primary action waits, and Back
    /// with it. On R3 the secondary ("no, check my words") and a notice's
    /// Try again wait too (design section 6, Restore running).
    #[must_use]
    pub fn waiting(&self) -> bool {
        matches!(
            self.screen,
            Screen::S1 {
                going_offline: true,
                ..
            } | Screen::R2 { checking: true, .. }
                | Screen::R3 {
                    restoring: true,
                    ..
                }
        )
    }

    /// The app bar shows a back arrow (design section 3): on every screen
    /// whose Back stays in the flow or returns to Settings. W and K1 are
    /// first screens, whose Back leaves the app, which an arrow would
    /// not say; C4 and R4 have no Back. The arrow is [`Self::back`], as
    /// the platform's Back is. `the_back_arrow_shows_where_back_stays_in_the_app`
    /// holds the list.
    #[must_use]
    pub fn back_arrow(&self) -> bool {
        matches!(
            self.screen,
            Screen::C1
                | Screen::C2 { .. }
                | Screen::C3(_)
                | Screen::R1
                | Screen::R2 { .. }
                | Screen::R3 { .. }
                | Screen::S1 { .. }
                | Screen::S2 { .. }
        )
    }

    /// R2's or C3's Back with words entered asks first
    /// (`IdDiscardTitle`).
    #[must_use]
    pub fn discard_asked(&self) -> bool {
        matches!(
            self.screen,
            Screen::R2 {
                discard_asked: true,
                ..
            } | Screen::C3(Check {
                discard_asked: true,
                ..
            })
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

    /// R2's words show as a full list (`IdReviewWords` opened it).
    #[must_use]
    pub fn reviewing(&self) -> bool {
        matches!(
            self.screen,
            Screen::R2 {
                reviewing: true,
                ..
            }
        )
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

    /// The screen's primary action. Behind the discard dialog it does
    /// nothing: Back and the dialog's buttons are what act there, and a
    /// press queued in one batch behind the Back that opened it reaches
    /// the flow after it (`no_screen_action_runs_behind_the_dialog`;
    /// trace.rs `check_queued_behind_the_discard_dialog_checks_nothing`).
    pub fn primary(&mut self) -> Vec<Effect> {
        if self.discard_asked() {
            return Vec::new();
        }
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
        // While `IdStorageUnknown` shows, R2's `IdCheckWords` and R3's
        // `IdR3Yes` do what its Try again does: read back, and never check
        // or restore again, since the record is there and a second restore
        // would write over what may be the person's identity (design
        // section 6, the read-back row;
        // `storage_unknowns_primary_on_r2_and_r3_reads_back_and_writes_nothing`).
        if matches!(
            self.screen,
            Screen::R2 {
                notice: Some(Notice::StorageUnknown),
                ..
            } | Screen::R3 {
                notice: Some(Notice::StorageUnknown),
                ..
            }
        ) {
            return self.notice_action();
        }
        let shown_peer = self.peer.clone();
        match &mut self.screen {
            Screen::Welcome { .. } => {
                self.screen = Screen::C1;
                vec![Effect::VoidPaper(false), Effect::Generate]
            }
            Screen::C1 => {
                self.screen = Screen::C2 { peer_page: false };
                if self.shown {
                    Vec::new()
                } else {
                    self.shown = true;
                    vec![Effect::VoidPaper(true)]
                }
            }
            // The words written (C2), or Next (S2): the PeerId on its own
            // page. S2's Done there ends the showing.
            Screen::C2 { peer_page } | Screen::S2 { peer_page } if !*peer_page => {
                *peer_page = true;
                Vec::new()
            }
            Screen::C2 { .. } => {
                self.screen = Screen::C3(Check {
                    positions: self.check_positions,
                    done: 0,
                    picker: WordPicker::new(self.list),
                    wrong: None,
                    discard_asked: false,
                });
                Vec::new()
            }
            // Held: a second press does nothing (§5.1).
            Screen::C4 { held: true, .. } | Screen::R4 { held: true, .. } => Vec::new(),
            // Try again in Continue's place under `IdStartBusy`: one
            // service start, no seal -- the identity is on record, and a
            // second seal would be refused as another identity on record
            // (section 6; `start_busys_try_again_starts_the_service_and_nothing_else`).
            // The notice stays until the start's result, as any notice
            // does while C4's primary runs.
            Screen::C4 {
                notice: Some(Notice::StartBusy { .. }),
                held,
                starting,
                ..
            }
            | Screen::R4 {
                notice: Some(Notice::StartBusy { .. }),
                held,
                starting,
                ..
            } => {
                *held = true;
                *starting = false;
                vec![Effect::StartService]
            }
            Screen::C4 {
                notice: Some(Notice::AlreadyCreated),
                held,
                starting,
                ..
            } => {
                *held = true;
                *starting = false;
                self.drop_created();
                vec![Effect::VoidPaper(false), Effect::StartService]
            }
            // Try again in Continue's place under `IdStorageUnknown` reads
            // the record back and writes nothing: both refusals that raise
            // it came after the publish, so a second seal would find the
            // record and be refused as another identity on record
            // (section 6; `storage_unknown_try_again_reads_back_and_never_seals`).
            Screen::C4 {
                notice: Some(Notice::StorageUnknown),
                held,
                starting,
                ..
            } => match shown_peer {
                Some(peer) => {
                    *held = true;
                    *starting = false;
                    vec![Effect::ReadBack(peer)]
                }
                None => Vec::new(),
            },
            Screen::C4 { held, starting, .. } => {
                *held = true;
                *starting = false;
                vec![Effect::Seal]
            }
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
            // Yes, or a notice's Try again: one restore at a time, and the
            // notice stays while it runs (section 8).
            Screen::R3 {
                entry,
                derived,
                restoring,
                ..
            } => match entry.phrase() {
                Some(phrase) if !*restoring => {
                    *restoring = true;
                    vec![Effect::RestoreConfirmed(phrase, derived.clone())]
                }
                _ => Vec::new(),
            },
            Screen::R4 { held, starting, .. } => {
                *held = true;
                *starting = false;
                vec![Effect::VoidPaper(false), Effect::StartService]
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
            Screen::S2 { .. } => self.leave_s2(),
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
            // From the PeerId page back to the same words; from the words
            // to C1, the words kept until C4 (section 3.2).
            Screen::C2 { peer_page } => {
                if *peer_page {
                    *peer_page = false;
                } else {
                    self.screen = Screen::C1;
                }
                Vec::new()
            }
            Screen::C3(check) => {
                // As on R2 below: the open dialog's Back is Continue
                // entering, and with nothing entered Back goes straight
                // back (section 7).
                if check.discard_asked {
                    check.discard_asked = false;
                } else if check.entered() {
                    check.discard_asked = true;
                } else {
                    self.screen = Screen::C2 { peer_page: true };
                }
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
                reviewing,
                ..
            } => {
                // The list's Back closes it, nothing changed: the
                // discard dialog is for Back on R2 itself.
                if *reviewing {
                    *reviewing = false;
                    return Vec::new();
                }
                if *checking {
                    return Vec::new();
                }
                let entered = (0..PHRASE_WORDS).any(|p| entry.word(p).is_some())
                    || !entry.picker().typed().is_empty();
                if *discard_asked {
                    // Back on the open dialog is its default, Continue
                    // entering: a second Back never drops the words
                    // unasked (section 7).
                    *discard_asked = false;
                    Vec::new()
                } else if entered {
                    *discard_asked = true;
                    Vec::new()
                } else {
                    self.leave_r2();
                    Vec::new()
                }
            }
            // While the restore runs Back does nothing: it writes the
            // record from the words it read (section 7).
            Screen::R3 { restoring, .. } => {
                if !*restoring {
                    self.back_to_r2();
                }
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
            // Back from the PeerId page returns to the words; from the
            // words it is Done (section 3.5).
            Screen::S2 { peer_page } => {
                if *peer_page {
                    *peer_page = false;
                    Vec::new()
                } else {
                    self.leave_s2()
                }
            }
        }
    }

    /// R2's or C3's discard dialog: delete the words (`true`) or keep
    /// entering. C3's delete goes back as C3's Back does, to C2's `PeerId`
    /// page, and C3 asks both positions again on return.
    pub fn discard(&mut self, delete: bool) {
        match &mut self.screen {
            Screen::R2 { discard_asked, .. } if *discard_asked => {
                *discard_asked = false;
                if delete {
                    self.leave_r2();
                }
            }
            Screen::C3(check) if check.discard_asked => {
                check.discard_asked = false;
                if delete {
                    self.screen = Screen::C2 { peer_page: true };
                }
            }
            _ => {}
        }
    }

    /// R3's secondary action, "no, check my words": back to R2 with every
    /// word kept. While the restore runs it does nothing, since R2 would
    /// let the words change under the restore reading them (section 7).
    pub fn recheck(&mut self) {
        if matches!(
            self.screen,
            Screen::R3 {
                restoring: false,
                ..
            }
        ) {
            self.back_to_r2();
        }
    }

    /// The action under the text of R2's or R3's notice (section 6):
    /// `IdBusy`'s and `IdStorageFailed`'s Try again runs the step again
    /// -- R2's check, R3's restore --, `IdStorageUnknown`'s reads the
    /// record back, writing nothing, and `IdRestoreOther`'s Continue
    /// starts the service with the identity on record, holding the screen
    /// until the start decides ([`Self::decided`]), as C4's Continue does
    /// under `IdAlreadyCreated`. While the check, the restore or that
    /// start runs, and behind the discard dialog, it does nothing.
    /// `notice_actions_retry_read_back_or_open_the_record` holds it.
    pub fn notice_action(&mut self) -> Vec<Effect> {
        if self.discard_asked() {
            return Vec::new();
        }
        match &mut self.screen {
            Screen::R2 {
                origin,
                entry,
                notice: Some(notice),
                checking: checking @ false,
                started,
                starting,
                ..
            } => match notice {
                Notice::Busy | Notice::StorageFailed => match entry.phrase() {
                    Some(phrase) => {
                        *checking = true;
                        vec![Effect::Check(phrase)]
                    }
                    None => Vec::new(),
                },
                Notice::StorageUnknown => match origin {
                    Origin::Recovery { expected, .. } => {
                        *checking = true;
                        vec![Effect::ReadBack(expected.clone())]
                    }
                    Origin::Fresh => Vec::new(),
                },
                Notice::RestoreOther { .. } => {
                    *checking = true;
                    *started = true;
                    *starting = false;
                    vec![Effect::VoidPaper(false), Effect::StartService]
                }
                // In that Continue's place: the start again, and nothing
                // else (section 6).
                Notice::StartBusy { .. } => {
                    *checking = true;
                    *started = true;
                    *starting = false;
                    vec![Effect::StartService]
                }
                _ => Vec::new(),
            },
            Screen::R3 {
                entry,
                derived,
                notice: Some(notice),
                restoring: restoring @ false,
                started,
                starting,
                ..
            } => match notice {
                Notice::Busy | Notice::StorageFailed => match entry.phrase() {
                    Some(phrase) => {
                        *restoring = true;
                        vec![Effect::RestoreConfirmed(phrase, derived.clone())]
                    }
                    None => Vec::new(),
                },
                Notice::StorageUnknown => {
                    *restoring = true;
                    vec![Effect::ReadBack(derived.clone())]
                }
                Notice::RestoreOther { .. } => {
                    *restoring = true;
                    *started = true;
                    *starting = false;
                    vec![Effect::VoidPaper(false), Effect::StartService]
                }
                Notice::StartBusy { .. } => {
                    *restoring = true;
                    *started = true;
                    *starting = false;
                    vec![Effect::StartService]
                }
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// C4 offers `IdStartUnchecked` under its Try again, with
    /// `IdStorageUnknownRestore` under it (design section 6, A start from
    /// C4 after a failed read-back): while `IdStorageUnknown` shows, once a
    /// read-back on this C4 has answered as failing -- not after a
    /// cancelled or denied read-back prompt, where none answered -- and
    /// until a start from it meets another `PeerId` than C4's, after which
    /// it is not offered again while C4 is open. While C4 is held (Try
    /// again's read-back, or this control's own start) it still shows,
    /// disabled, as the primary does ([`Self::start_held`]).
    /// `start_unchecked_is_offered_only_after_a_read_back_failed` and
    /// `a_mismatch_withdraws_start_unchecked_for_c4s_life` hold it.
    #[must_use]
    pub const fn offers_unchecked(&self) -> bool {
        matches!(
            self.screen,
            Screen::C4 {
                notice: Some(Notice::StorageUnknown),
                offer: Offer::Yes,
                ..
            }
        )
    }

    /// `IdStartUnchecked`'s press (design section 6, A start from C4
    /// after a failed read-back): the creation ends -- its unsealed
    /// identity dropped ([`Effect::Discard`]), its words gone -- and the
    /// service starts, sealing and writing nothing: no void-paper marker,
    /// no connect-owed marker, so the start takes add-a-peer §5.1's
    /// not-owed path. C4 is held from the press as after Continue, the
    /// notice staying until the result, and the start is the one
    /// [`Self::start_expects`] names, which the root must find opening to
    /// C4's `PeerId` before anything runs. Nothing where the control is
    /// not offered, or while C4 is held, so a second press starts nothing.
    /// `start_unchecked_holds_c4_and_writes_nothing` holds it.
    pub fn start_unchecked(&mut self) -> Vec<Effect> {
        if !self.offers_unchecked() || self.peer.is_none() {
            return Vec::new();
        }
        let Screen::C4 {
            held: held @ false,
            starting,
            unchecked,
            ..
        } = &mut self.screen
        else {
            return Vec::new();
        };
        *held = true;
        *starting = false;
        *unchecked = true;
        self.held_by = HeldBy::Press;
        self.words = None;
        self.shown = false;
        vec![Effect::Discard, Effect::StartService]
    }

    /// C3, after a wrong word: "Try again" takes the error away; the entry
    /// is already clear. Behind the discard dialog it does nothing, as
    /// [`Self::primary`].
    pub fn try_again(&mut self) {
        if let Screen::C3(check) = &mut self.screen
            && !check.discard_asked
        {
            check.wrong = None;
        }
    }

    /// C3, after a wrong word: show the words again (→ C2's words page);
    /// the same two positions are asked on return. Behind the discard dialog it does
    /// nothing, as [`Self::primary`].
    pub fn show_words_again(&mut self) {
        if matches!(
            self.screen,
            Screen::C3(Check {
                discard_asked: false,
                ..
            })
        ) {
            self.screen = Screen::C2 { peer_page: false };
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

    /// R2: edit a grid position; chosen from the full list, the list
    /// closes onto the picker. Behind the discard dialog, and while the
    /// check runs, it does nothing.
    pub fn edit(&mut self, position: usize) {
        if self.discard_asked() || self.checking() {
            return;
        }
        if let Screen::R2 {
            entry,
            notice,
            reviewing,
            ..
        } = &mut self.screen
        {
            entry.edit(position);
            *notice = None;
            *reviewing = false;
            self.positions_chosen += 1;
        }
    }

    /// R2: open (`true`) or close the words as a full list. It opens only
    /// while neither the discard dialog nor a check is pending: either
    /// holds R2's words where they are.
    pub fn review_words(&mut self, open: bool) {
        if let Screen::R2 {
            reviewing,
            discard_asked,
            checking,
            ..
        } = &mut self.screen
        {
            *reviewing = open && !*discard_asked && !*checking;
        }
    }

    /// A chip taken on C3 or R2, by a tap or by [`Self::enter`]. On C3 a right word moves to the next
    /// position or to C4; a wrong one shows which position and clears.
    pub fn choose(&mut self, chip: &str) -> Vec<Effect> {
        if self.words_held() {
            return Vec::new();
        }
        match &mut self.screen {
            Screen::R2 { entry, notice, .. } => {
                if entry.choose(chip) {
                    *notice = None;
                    self.chips_taken += 1;
                }
                Vec::new()
            }
            Screen::C3(check) => {
                let Some(word) = check.picker.chip(chip) else {
                    return Vec::new();
                };
                self.chips_taken += 1;
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
                    self.screen = Screen::C4 {
                        notice: None,
                        held: false,
                        starting: false,
                        offer: Offer::No,
                        unchecked: false,
                    };
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// Enter while typing on C3 or R2 (design section 8): it takes the
    /// chip [`WordPicker::enter_chip`] names, as a tap on it would, and
    /// otherwise does nothing. Applied in order behind the letters before
    /// it, so the letters it judges are the ones typed (j261).
    pub fn enter(&mut self) -> Vec<Effect> {
        match self.picker().and_then(WordPicker::enter_chip) {
            Some(word) => self.choose(word),
            None => Vec::new(),
        }
    }

    /// How many chips have been taken: a page whose count moved shows one
    /// just taken.
    #[must_use]
    pub fn chips_taken(&self) -> u32 {
        self.chips_taken
    }

    /// How many grid positions R2 has opened to edit: a page whose count
    /// moved shows one just chosen.
    #[must_use]
    pub fn positions_chosen(&self) -> u32 {
        self.positions_chosen
    }

    /// How many notices a step's result has raised: a page whose count
    /// moved shows one just raised, even with the text it showed before.
    #[must_use]
    pub fn notices_raised(&self) -> u32 {
        self.notices_raised
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

    /// The seal of C4, or the read-back after it: done, or why not. A
    /// read-back refused because authentication is needed keeps C4 held
    /// and asks for the prompt ([`Effect::AuthenticateReadBack`]).
    pub fn sealed(&mut self, outcome: Result<(), StepFailed>) -> Vec<Effect> {
        let shown_peer = self.peer.clone();
        let Screen::C4 {
            notice,
            held,
            starting,
            offer,
            ..
        } = &mut self.screen
        else {
            return Vec::new();
        };
        let outcome = match (outcome, shown_peer) {
            // Held: the prompt shows over C4, and the notice already
            // showing, if any, stays until the result (design section 6,
            // Seal running).
            (Err(StepFailed::ReadBackNeedsAuthentication), Some(peer)) => {
                return vec![Effect::AuthenticateReadBack(peer)];
            }
            (outcome, _) => outcome,
        };
        match outcome {
            // Still held: the start goes on under it, and the root ends
            // the hold once the start decided ([`Self::decided`]). A
            // failure an earlier seal left is no longer true (#651 F3).
            // The words go; the PeerId stays on C4, which shows it until
            // the next screen opens, `IdStartBusy` included (section 6;
            // `a_start_refused_busy_shows_start_busy_on_each_held_screen`).
            Ok(()) => {
                *notice = None;
                self.words = None;
                self.shown = false;
                vec![Effect::VoidPaper(false), Effect::StartService]
            }
            // C4 stays, Continue (Try again) enabled again (§5.1). A
            // read-back answered as failing -- the seal's own, or Try
            // again's -- offers `IdStartUnchecked` beside Try again, unless
            // a start from it met another `PeerId` (section 6, A start
            // from C4 after a failed read-back;
            // `start_unchecked_is_offered_only_after_a_read_back_failed`).
            Err(failed) => {
                if failed == StepFailed::StorageUnknown && *offer == Offer::No {
                    *offer = Offer::Yes;
                }
                *notice = Some(failed.into());
                *held = false;
                *starting = false;
                self.notices_raised += 1;
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
            unknown,
            ..
        } = &mut self.screen
        else {
            return Vec::new();
        };
        *checking = false;
        let Some(derived) = derived else {
            *notice = Some(Notice::ChecksumFailed);
            self.notices_raised += 1;
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
                    restoring: false,
                    started: false,
                    starting: false,
                };
                Vec::new()
            }
            Some(expected) if expected != derived => {
                *notice = Some(Notice::OtherIdentity { derived, expected });
                self.notices_raised += 1;
                Vec::new()
            }
            Some(expected) if *unknown => {
                *checking = true;
                vec![Effect::ReadBack(expected)]
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

    /// A restore's outcome, from R2 (against the record) or R3, or the
    /// read-back's after it. A read-back refused because authentication is
    /// needed keeps the screen held and asks for the prompt
    /// ([`Effect::AuthenticateReadBack`]) for the `PeerId` the restore was
    /// to give back.
    pub fn restored(&mut self, outcome: Result<(), StepFailed>) -> Vec<Effect> {
        if outcome == Err(StepFailed::ReadBackNeedsAuthentication) {
            match &self.screen {
                Screen::R2 {
                    origin: Origin::Recovery { expected, .. },
                    checking: true,
                    ..
                } => return vec![Effect::AuthenticateReadBack(expected.clone())],
                Screen::R3 {
                    derived,
                    restoring: true,
                    ..
                } => return vec![Effect::AuthenticateReadBack(derived.clone())],
                _ => {}
            }
        }
        match (&mut self.screen, outcome) {
            (Screen::R2 { origin, .. }, Ok(())) => {
                let peer = match origin {
                    Origin::Recovery { expected, .. } => Some(expected.clone()),
                    Origin::Fresh => None,
                };
                let after_k1 = matches!(origin, Origin::Recovery { .. });
                self.screen = Screen::R4 {
                    after_k1,
                    peer,
                    notice: None,
                    held: false,
                    starting: false,
                };
                Vec::new()
            }
            (Screen::R3 { derived, .. }, Ok(())) => {
                let peer = Some(derived.clone());
                self.screen = Screen::R4 {
                    peer,
                    notice: None,
                    held: false,
                    starting: false,
                    after_k1: false,
                };
                Vec::new()
            }
            (
                Screen::R2 {
                    notice,
                    checking,
                    unknown,
                    ..
                },
                Err(failed),
            ) => {
                *checking = false;
                let failed = Notice::from(failed);
                *unknown |= failed == Notice::StorageUnknown;
                *notice = Some(failed);
                self.notices_raised += 1;
                Vec::new()
            }
            (
                Screen::R3 {
                    notice, restoring, ..
                },
                Err(failed),
            ) => {
                *restoring = false;
                *notice = Some(failed.into());
                self.notices_raised += 1;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// A key operation this flow asked for did not run: its prompt was
    /// cancelled or denied after a refusal for the person (`PresenceGate`),
    /// or dropped before anything raised it (`PresenceGate::unasked`), where
    /// the person decided nothing. Nothing was committed and the screen
    /// stays, with its words, and no notice; nothing here records a
    /// decline. R2's check gives the primary action back,
    /// and R3's restore gives Yes and No back, with the notice it showed
    /// if any; nothing restores again until Yes or Try again is pressed.
    pub fn cancelled_step(&mut self) {
        match &mut self.screen {
            Screen::R2 { checking, .. } => *checking = false,
            Screen::R3 { restoring, .. } => *restoring = false,
            // The seal's prompt, cancelled: C4 as it was, Continue enabled
            // again (§5.1).
            Screen::C4 { held, starting, .. } => {
                *held = false;
                *starting = false;
            }
            _ => {}
        }
    }

    /// A key operation the presence gate held, `effect`, did not run: its
    /// prompt was cancelled or denied, or dropped before anything raised
    /// it. A seal or a restore is [`Self::cancelled_step`]: the screen as
    /// it was. A read-back is not (design section 6, Authentication
    /// needed): the write before it may have committed the identity, so
    /// `IdStorageUnknown` shows, raised so it takes focus, and the
    /// screen's primary reads back rather than writing again
    /// (`a_read_backs_prompt_not_answered_shows_storage_unknown`). Any
    /// other effect does nothing here.
    pub fn step_not_run(&mut self, effect: &Effect) {
        match effect {
            Effect::Seal | Effect::RestoreRecorded(_) | Effect::RestoreConfirmed(..) => {
                self.cancelled_step();
            }
            Effect::ReadBack(_) => {
                let notice = match &mut self.screen {
                    Screen::C4 {
                        notice,
                        held,
                        starting,
                        ..
                    } => {
                        *held = false;
                        *starting = false;
                        notice
                    }
                    Screen::R2 {
                        notice,
                        checking,
                        unknown,
                        ..
                    } => {
                        *checking = false;
                        *unknown = true;
                        notice
                    }
                    Screen::R3 {
                        notice, restoring, ..
                    } => {
                        *restoring = false;
                        notice
                    }
                    _ => return,
                };
                *notice = Some(Notice::StorageUnknown);
                self.notices_raised += 1;
            }
            _ => {}
        }
    }

    /// C4 or R4 holds after Continue, Continue disabled, until the start
    /// decides (add-a-peer §5.1); and R2 or R3 after `IdRestoreOther`'s
    /// Continue (section 8).
    #[must_use]
    pub const fn start_held(&self) -> bool {
        matches!(
            self.screen,
            Screen::C4 { held: true, .. }
                | Screen::R4 { held: true, .. }
                | Screen::R2 { started: true, .. }
                | Screen::R3 { started: true, .. }
        )
    }

    /// The screen now shows a service start the flow asked: held for it
    /// ([`Self::start_held`]), or showing `IdStartBusy` on C4, R4, R3 or
    /// R2 -- the screens a showing's start holds
    /// ([`Self::showing_started`]). A reading of the current screen, not
    /// of the flow's past: R3's Back off `IdStartBusy`, to R2 and on to
    /// R1, shows no start. `a_start_shows_on_its_screen_and_goes_with_it`
    /// holds it.
    #[must_use]
    pub const fn start_shows(&self) -> bool {
        self.start_held()
            || matches!(
                self.screen,
                Screen::C4 {
                    notice: Some(Notice::StartBusy { .. }),
                    ..
                } | Screen::R4 {
                    notice: Some(Notice::StartBusy { .. }),
                    ..
                } | Screen::R3 {
                    notice: Some(Notice::StartBusy { .. }),
                    ..
                } | Screen::R2 {
                    notice: Some(Notice::StartBusy { .. }),
                    ..
                }
            )
    }

    /// Whether `ConnectStarting` shows under the held Continue, as the
    /// root's clock says: 1 s after the press with no decision, and not
    /// while a system prompt is up (§5.1).
    pub fn set_starting(&mut self, shown: bool) {
        if let Screen::C4 {
            held: true,
            starting,
            ..
        }
        | Screen::R4 {
            held: true,
            starting,
            ..
        }
        | Screen::R2 {
            started: true,
            starting,
            ..
        }
        | Screen::R3 {
            started: true,
            starting,
            ..
        } = &mut self.screen
        {
            *starting = shown;
        }
    }

    /// `ConnectStarting` and its progress mark show under Continue: held,
    /// and the root said so. Never on a screen that is not held, so a
    /// failed seal's Try again shows nothing under it (§8 item 12).
    #[must_use]
    pub const fn starting(&self) -> bool {
        matches!(
            self.screen,
            Screen::C4 {
                held: true,
                starting: true,
                ..
            } | Screen::R4 {
                held: true,
                starting: true,
                ..
            } | Screen::R2 {
                started: true,
                starting: true,
                ..
            } | Screen::R3 {
                started: true,
                starting: true,
                ..
            }
        )
    }

    /// The start under a held C4 or R4, or under R2 or R3 after
    /// `IdRestoreOther`'s Continue, decided which screen opens -- Connect
    /// or Chats, which the root shows: the flow is over. Nothing on a
    /// screen that is not held.
    pub fn decided(&mut self) -> Vec<Effect> {
        if !self.start_held() {
            return Vec::new();
        }
        self.screen = Screen::Done;
        vec![Effect::OpenChats]
    }

    /// The `PeerId` the service start this flow now holds a screen for, or
    /// shows refused under `IdStartBusy`, must open the record to: C4's,
    /// while that start is `IdStartUnchecked`'s (design section 6, A start
    /// from C4 after a failed read-back) -- its press, `IdStartBusy`'s Try
    /// again after it, and a start a showing retries under that notice
    /// alike. The root compares the identity the start unlocks with it
    /// before the identity reaches the service, and answers a mismatch
    /// with [`Self::unchecked_refused`]. `None` for every other start of
    /// the flow: what the flow says. Once said, the root holds the
    /// comparison for every later start of its process until one opens
    /// the record to that `PeerId`, the flow's end included (section 6,
    /// The comparison outlasts the bound; human-android's
    /// `ExpectedStart`). `start_expects_names_c4s_peer_id_for_its_starts_alone`
    /// holds the flow's word.
    #[must_use]
    pub fn start_expects(&self) -> Option<&TransportIdentity> {
        match &self.screen {
            Screen::C4 {
                unchecked: true, ..
            } => self.peer.as_ref(),
            _ => None,
        }
    }

    /// The start under this screen is one C4 answers when it compared the
    /// identity it unlocks with an expected `PeerId` and was refused
    /// (design section 6, An answer after the bound: "A start made from C4
    /// that answers before the bound answers on C4, as Its results says,
    /// wherever it comes from"): C4 held for a start while it shows a
    /// `PeerId` -- `IdStartUnchecked`'s press, Continue's start after a
    /// seal or after Try again's read-back found the record,
    /// `IdStartBusy`'s Try again and a showing's retry under it alike. Not
    /// C4 under `IdAlreadyCreated`, whose created identity is dropped at
    /// Continue, so `IdStorageUnknown`'s Try again would have no `PeerId`
    /// to read back, and not R4, R3 or R2: their compared refusal hands
    /// over as before. The root asks it only of a start that compared
    /// (human-android's `unchecked_answer`, which answers no flow for a
    /// start told to expect nothing:
    /// `a_start_that_compared_nothing_answers_no_flow`); this says which
    /// screen answers. `every_compared_start_held_on_c4_answers_on_c4`
    /// holds it.
    #[must_use]
    pub const fn answers_compared(&self) -> bool {
        matches!(self.screen, Screen::C4 { held: true, .. }) && self.peer.is_some()
    }

    /// A start under C4 that compared with an expected `PeerId` did not
    /// run, for `refused` (design section 6, Its results), whichever C4
    /// start it was ([`Self::answers_compared`]): another `PeerId` than
    /// the expected one shows `IdStorageUnknown` with Try again only, the
    /// control withdrawn while C4 is open; a platform error shows it with
    /// both controls, the second only while it is offered; either raised,
    /// so the notice takes focus. A record that does not open leads to
    /// K1, with its reason and the `PeerId` the record names, and asks for
    /// the flow's profile lock again ([`Effect::Hold`]): K1 takes it
    /// before any word, as at a launch. Nothing on any other screen.
    /// `start_uncheckeds_results_are_a_launchs_and_a_mismatch_fails_closed`
    /// and `every_compared_start_held_on_c4_answers_on_c4` hold it.
    pub fn unchecked_refused(&mut self, refused: UncheckedRefused) -> Vec<Effect> {
        if !self.answers_compared() {
            return Vec::new();
        }
        let Screen::C4 {
            notice,
            held,
            starting,
            offer,
            unchecked,
        } = &mut self.screen
        else {
            return Vec::new();
        };
        self.held_by = HeldBy::Press;
        match refused {
            UncheckedRefused::OtherIdentity | UncheckedRefused::Unavailable => {
                if refused == UncheckedRefused::OtherIdentity {
                    *offer = Offer::Withdrawn;
                }
                *notice = Some(Notice::StorageUnknown);
                *held = false;
                *starting = false;
                *unchecked = false;
                self.notices_raised += 1;
                Vec::new()
            }
            UncheckedRefused::Recovery {
                reason,
                expected,
                new_keys,
            } => {
                self.drop_created();
                self.new_keys = new_keys;
                self.screen = Screen::K1 { reason, expected };
                vec![Effect::Hold]
            }
        }
    }

    /// The start this flow now holds a screen for ends the comparison with
    /// C4's `PeerId` (design section 6, The comparison outlasts the bound;
    /// architect-cto's ruling on Q17): R4 held for its Continue after a
    /// restore from R2 after K1 -- the person committed a restore from the
    /// phrase they just entered, so that phrase, not the one written down
    /// on C4, is the paper for the identity that runs. The root clears the
    /// expected `PeerId` before that start reads it, so it and every later
    /// start of the process compare nothing (human-android's
    /// `ExpectedStart::restored`). False everywhere before that press --
    /// K1 showing, R2 open, R2's words entered then Back, a cancelled
    /// restore prompt, K1 left with no restore -- and on R2 after K1 held
    /// for `IdRestoreOther`'s Continue, whose start stays compared, and on
    /// R4 after a fresh install's restore (R1, R3).
    /// `only_r4s_continue_after_k1_ends_the_comparison` holds it.
    #[must_use]
    pub const fn start_ends_comparison(&self) -> bool {
        matches!(
            self.screen,
            Screen::R4 {
                held: true,
                after_k1: true,
                ..
            }
        )
    }

    /// The start under a held screen was refused with the profile in use
    /// (the runtime's `LockHeld`) before add-a-peer §5.1's bound: the
    /// hold ends and `IdStartBusy` shows in the screen's notice place,
    /// raised so it takes focus (design section 6, section 8). On C4
    /// under `IdAlreadyCreated`, and on R2 and R3 under `IdRestoreOther`,
    /// the rest of the screen stays as that notice left it. Nothing on a
    /// screen that is not held, and nothing is retried: only the person's
    /// Try again, or a showing's start ([`Self::showing_started`]), starts
    /// again. A showing's answers no press: its notice shows again with
    /// focus left where it is, not raised
    /// (`a_showings_start_refused_busy_again_raises_nothing`).
    pub fn start_busy(&mut self) {
        if !self.start_held() {
            return;
        }
        let pressed = std::mem::replace(&mut self.held_by, HeldBy::Press) == HeldBy::Press;
        let kept = |notice: &Option<Notice>| match notice {
            Some(Notice::RestoreOther { recorded }) => Some(recorded.clone()),
            Some(Notice::StartBusy { recorded, .. }) => recorded.clone(),
            _ => None,
        };
        match &mut self.screen {
            Screen::C4 {
                notice,
                held,
                starting,
                ..
            } => {
                let already = matches!(
                    notice,
                    Some(Notice::AlreadyCreated | Notice::StartBusy { already: true, .. })
                );
                *notice = Some(Notice::StartBusy {
                    recorded: None,
                    already,
                });
                *held = false;
                *starting = false;
            }
            Screen::R4 {
                notice,
                held,
                starting,
                ..
            } => {
                *notice = Some(Notice::StartBusy {
                    recorded: None,
                    already: false,
                });
                *held = false;
                *starting = false;
            }
            Screen::R2 {
                notice,
                checking,
                started,
                starting,
                ..
            } => {
                *notice = Some(Notice::StartBusy {
                    recorded: kept(notice),
                    already: false,
                });
                *checking = false;
                *started = false;
                *starting = false;
            }
            Screen::R3 {
                notice,
                restoring,
                started,
                starting,
                ..
            } => {
                *notice = Some(Notice::StartBusy {
                    recorded: kept(notice),
                    already: false,
                });
                *restoring = false;
                *started = false;
                *starting = false;
            }
            _ => return,
        }
        if pressed {
            self.notices_raised += 1;
        }
    }

    /// The person asked the held screen's start again -- a granted prompt
    /// over it resuming the start a showing made: that start answers the
    /// person, so its `LockHeld` raises `IdStartBusy` for focus as Try
    /// again's would (design section 6, the row's "answers the person's
    /// press"). Nothing on a screen a press holds, or one not held.
    /// `a_granted_prompt_over_a_showings_start_answers_the_person` holds
    /// it.
    pub fn person_started(&mut self) {
        self.held_by = HeldBy::Press;
    }

    /// A showing of the window -- a return -- started the service again
    /// while `IdStartBusy` shows on C4, R4, R3 or R2 (unlock note section
    /// 11, L3; identity note section 6, A return or a recreation): the
    /// screen holds as the Try again it replaces would hold it, with no
    /// effect, since the Activity's own start is the one start. Its
    /// answers are Try again's, by [`Self::decided`] and
    /// [`Self::start_busy`], except that it answers no press. R2's discard
    /// dialog closes as its Continue entering would, all words kept: a
    /// held screen offers no way off it. True when the screen now holds;
    /// on any other screen, or one already held, nothing changes.
    /// `a_showings_start_holds_as_try_again_and_asks_nothing` holds it.
    pub fn showing_started(&mut self) -> bool {
        let held = match &mut self.screen {
            Screen::C4 {
                notice: Some(Notice::StartBusy { .. }),
                held: held @ false,
                starting,
                ..
            }
            | Screen::R4 {
                notice: Some(Notice::StartBusy { .. }),
                held: held @ false,
                starting,
                ..
            } => {
                *held = true;
                *starting = false;
                true
            }
            Screen::R2 {
                notice: Some(Notice::StartBusy { .. }),
                checking: checking @ false,
                started,
                starting,
                discard_asked,
                ..
            } => {
                *checking = true;
                *started = true;
                *starting = false;
                *discard_asked = false;
                true
            }
            Screen::R3 {
                notice: Some(Notice::StartBusy { .. }),
                restoring: restoring @ false,
                started,
                starting,
                ..
            } => {
                *restoring = true;
                *started = true;
                *starting = false;
                true
            }
            _ => false,
        };
        if held {
            self.held_by = HeldBy::Showing;
        }
        held
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
                self.screen = Screen::S2 { peer_page: false };
                Vec::new()
            }
            Err(failed) => {
                *notice = Some(failed.into());
                self.notices_raised += 1;
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
            reviewing: false,
            started: false,
            starting: false,
            unknown: false,
        }
    }

    /// While the discard dialog or R2's full list shows, or R2's check
    /// runs, no letter, chip or Enter reaches the words. The view refuses
    /// the first two too, but it judges the page it last showed: inputs
    /// queued in one batch with the Back or the list's opening reach the
    /// flow after it, and only the flow, applying them in order, knows
    /// (j261, #634 F1). A check reports on the words it read, and after
    /// K1 [`Self::checked`] restores the phrase the entry holds at its
    /// result, so a word changed while it ran would be restored unchecked
    /// (design section 8, `no_input_changes_the_words_while_the_check_runs`).
    fn words_held(&self) -> bool {
        self.discard_asked() || self.reviewing() || self.checking()
    }

    /// R2's check is running: from the press that starts it (Check my
    /// words, or a notice's Try again) until its result, a key operation
    /// after K1 included.
    fn checking(&self) -> bool {
        matches!(self.screen, Screen::R2 { checking: true, .. })
    }

    fn picker_mut(&mut self) -> Option<&mut WordPicker> {
        if self.words_held() {
            return None;
        }
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
            reviewing: false,
            started: false,
            starting: false,
            unknown: false,
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
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        assert_eq!(flow.primary(), [Effect::VoidPaper(false), Effect::Generate]);
        flow.created(words(), mine(), [6, 18]);
        assert_eq!(
            flow.primary(),
            [Effect::VoidPaper(true)],
            "C2's first display"
        );
        flow
    }

    /// C2's words, its `PeerId` page, then C3.
    fn created_at_c3() -> IdentityFlow {
        let mut flow = created_at_c2();
        assert_eq!(flow.primary(), [], "control: to the PeerId page");
        assert_eq!(flow.screen(), ScreenId::C2PeerId, "control");
        assert_eq!(flow.primary(), [], "control: to C3");
        assert_eq!(flow.screen(), ScreenId::C3, "control");
        flow
    }

    #[test]
    fn create_shows_words_checks_two_and_seals_only_at_c4() {
        let mut flow = created_at_c2();
        assert_eq!(flow.screen(), ScreenId::C2);
        assert_eq!(flow.words(), Some(&words()));
        assert_eq!(flow.peer(), None, "no PeerId beside the words");
        assert_eq!(flow.primary(), [], "I wrote down the words");
        assert_eq!(flow.screen(), ScreenId::C2PeerId);
        assert_eq!(flow.words(), None, "no word beside the PeerId");
        assert_eq!(flow.peer(), Some(&mine()));
        assert_eq!(flow.primary(), [], "I wrote down the PeerId");
        assert_eq!(flow.check(), Some((6, None)));
        enter(&mut flow, "galaxy");
        assert_eq!(flow.check(), Some((18, None)));
        enter(&mut flow, "saddle");
        assert_eq!(flow.screen(), ScreenId::C4);
        assert_eq!(flow.back(), [], "C4 has no Back");
        assert_eq!(flow.screen(), ScreenId::C4);
        assert_eq!(flow.primary(), [Effect::Seal]);
        assert!(
            flow.start_held(),
            "Continue disabled at once, before the seal answers"
        );
        assert_eq!(flow.primary(), [], "a second press does nothing");
        assert_eq!(
            flow.sealed(Ok(())),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        assert_eq!(flow.screen(), ScreenId::C4, "held until the start decides");
        assert!(flow.start_held());
        assert_eq!(flow.decided(), [Effect::OpenChats]);
        assert_eq!(flow.screen(), ScreenId::Done);
        assert_eq!(flow.decided(), [], "said once");
    }

    /// A seal that fails, or whose prompt was cancelled, gives C4 back
    /// with Continue enabled and nothing under it (§5.1; §8 item 12's
    /// failed seal). Mutation: leaving `held` set in either arm fails
    /// here.
    #[test]
    fn a_failed_or_cancelled_seal_gives_continue_back_with_nothing_under_it() {
        let mut flow = created_at_c3();
        enter(&mut flow, "galaxy");
        enter(&mut flow, "saddle");
        assert_eq!(flow.primary(), [Effect::Seal]);
        flow.set_starting(true);
        assert!(flow.starting(), "control: shown while held");
        assert_eq!(flow.sealed(Err(StepFailed::Storage)), []);
        assert!(!flow.start_held());
        assert!(!flow.starting(), "nothing under Try again");
        assert_eq!(flow.decided(), [], "not held: nothing to decide");
        assert_eq!(flow.screen(), ScreenId::C4);

        assert_eq!(flow.primary(), [Effect::Seal], "Try again");
        assert!(!flow.starting(), "a press clears the line");
        flow.cancelled_step();
        assert!(!flow.start_held(), "a cancelled seal prompt");
        assert_eq!(flow.primary(), [Effect::Seal]);
        assert!(flow.notice().is_some(), "control: the failure still said");
        assert_eq!(
            flow.sealed(Ok(())),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        assert!(flow.start_held());
        assert_eq!(flow.notice(), None, "a seal that worked says no failure");
    }

    #[test]
    fn a_wrong_check_word_names_its_position_and_there_is_no_way_past() {
        let mut flow = created_at_c3();
        enter(&mut flow, "zebra");
        assert_eq!(flow.check(), Some((6, Some(6))));
        assert_eq!(flow.picker().map(WordPicker::typed), Some(""), "cleared");
        flow.try_again();
        assert_eq!(flow.check(), Some((6, None)));
        enter(&mut flow, "zebra");
        flow.show_words_again();
        assert_eq!(flow.screen(), ScreenId::C2, "the words page");
        assert_eq!(flow.words(), Some(&words()), "the same words");
        assert_eq!(flow.primary(), [], "the marker is already set");
        assert_eq!(flow.screen(), ScreenId::C2PeerId);
        assert_eq!(flow.primary(), [], "the marker is already set");
        assert_eq!(flow.check(), Some((6, None)), "the same positions again");
    }

    #[test]
    fn back_from_c1_discards_and_warns_of_a_void_paper_only_once_words_showed() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
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
        let mut flow = created_at_c3();
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
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
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
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
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
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
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
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: true },
        );
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
            [Effect::VoidPaper(false), Effect::StartService]
        );
        assert!(flow.start_held());
        assert_eq!(flow.primary(), [], "a second press does nothing");
        assert_eq!(flow.screen(), ScreenId::R4);
        assert_eq!(flow.decided(), [Effect::OpenChats]);
        assert_eq!(flow.screen(), ScreenId::Done);
    }

    #[test]
    fn recovery_restores_against_the_record_with_no_r3_and_refuses_another_identity() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
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
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
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
            let mut flow = IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::FirstRun { void_paper: false },
            );
            assert!(!flow.warns_active_elsewhere(), "Welcome restores nothing");
            flow.restore();
            flow
        };
        let recovery = |reason| {
            IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::Recovery {
                    reason,
                    expected: Some(mine()),
                    new_keys: false,
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
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Record,
                expected: None,
                new_keys: false,
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
        use ScreenId::{C1, C2, C2PeerId, C3, C4, R2, R3, R4, S1, S2, S2PeerId};

        let mut seen = Vec::new();
        let mut look = |flow: &IdentityFlow| {
            assert!(!flow.warns_active_elsewhere(), "{:?} warns", flow.screen());
            assert!(!flow.says_new_keys(), "{:?} says new keys", flow.screen());
            seen.push(flow.screen());
        };

        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.primary();
        look(&flow);
        flow.created(words(), mine(), [6, 18]);
        flow.primary();
        look(&flow);
        flow.primary();
        look(&flow);
        flow.primary();
        look(&flow);
        enter(&mut flow, "galaxy");
        enter(&mut flow, "saddle");
        look(&flow);

        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
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

        let mut flow =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        look(&flow);
        flow.primary();
        flow.prompted(true);
        flow.offline(Ok((words(), mine())));
        look(&flow);
        flow.primary();
        look(&flow);

        assert_eq!(
            seen,
            [C1, C2, C2PeerId, C3, C4, R2, R3, R4, S1, S2, S2PeerId],
            "control"
        );
    }

    /// Section 3's app bar: the back arrow shows on every screen whose
    /// Back goes somewhere in the app, and on no other. Each screen is
    /// built fresh and its Back pressed, so the arrow's answer is checked
    /// against what Back does there: never `Finish` behind an arrow, and
    /// on a screen without one, `Finish` (W, K1) or nothing (C4, R4).
    /// Mutation: dropping any arm of `back_arrow`, or adding W, K1, C4 or
    /// R4, fails here.
    #[test]
    fn the_back_arrow_shows_where_back_stays_in_the_app() {
        use ScreenId::{C1, C2, C2PeerId, C3, C4, K1, R1, R2, R3, R4, S1, S2, S2PeerId, W};

        let create = |steps: usize| {
            let mut flow = IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::FirstRun { void_paper: false },
            );
            if steps > 0 {
                flow.primary();
            }
            if steps > 1 {
                flow.created(words(), mine(), [6, 18]);
                flow.primary();
            }
            for _ in 2..steps.min(4) {
                flow.primary();
            }
            if steps > 4 {
                enter(&mut flow, "galaxy");
                enter(&mut flow, "saddle");
            }
            flow
        };
        let restore = |steps: usize| {
            let mut flow = IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::FirstRun { void_paper: false },
            );
            flow.restore();
            if steps > 0 {
                flow.primary();
            }
            if steps > 1 {
                fill(&mut flow);
                flow.primary();
                flow.checked(Some(mine()));
            }
            if steps > 2 {
                flow.primary();
                flow.restored(Ok(()));
            }
            flow
        };
        let show = |steps: usize| {
            let mut flow =
                IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
            if steps > 0 {
                flow.primary();
                flow.prompted(true);
                flow.offline(Ok((words(), mine())));
            }
            if steps > 1 {
                flow.primary();
            }
            flow
        };
        let k1 = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
            },
        );

        let mut seen = Vec::new();
        for mut flow in [
            create(0),
            create(1),
            create(2),
            create(3),
            create(4),
            create(5),
            restore(0),
            restore(1),
            restore(2),
            restore(3),
            k1,
            show(0),
            show(1),
            show(2),
        ] {
            let screen = flow.screen();
            let arrow = flow.back_arrow();
            let effects = flow.back();
            let moved = flow.screen() != screen || flow.discard_asked();
            if arrow {
                assert!(
                    !effects.contains(&Effect::Finish),
                    "{screen:?}: {effects:?}"
                );
                assert!(moved, "{screen:?}: the arrow goes somewhere");
            } else {
                assert!(
                    effects == [Effect::Finish] || (effects.is_empty() && !moved),
                    "{screen:?}: {effects:?}"
                );
            }
            seen.push((screen, arrow));
        }
        assert_eq!(
            seen,
            [
                (W, false),
                (C1, true),
                (C2, true),
                (C2PeerId, true),
                (C3, true),
                (C4, false),
                (R1, true),
                (R2, true),
                (R3, true),
                (R4, false),
                (K1, false),
                (S1, true),
                (S2, true),
                (S2PeerId, true),
            ]
        );
    }

    /// Item 12 on R1: every fresh-install path into R2 is told that the
    /// identities get new keys, after the ADR-0043 warning and before R2,
    /// as a statement. Mutation: without `matches!(self.screen,
    /// Screen::R1)` in `says_new_keys`, R1 is silent and this fails.
    #[test]
    fn r1_always_says_new_keys() {
        for void_paper in [false, true] {
            let mut flow = IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::FirstRun { void_paper },
            );
            assert!(!flow.says_new_keys(), "Welcome restores nothing");
            flow.restore();
            assert_eq!(flow.screen(), ScreenId::R1);
            assert!(flow.says_new_keys(), "R1, void paper {void_paper}");
            assert!(flow.warns_active_elsewhere(), "under the ADR-0043 warning");
            assert_eq!(flow.primary(), [], "a statement, not a gate");
            assert_eq!(flow.screen(), ScreenId::R2);
            assert!(!flow.says_new_keys(), "R2 keeps its room");
            flow.back();
            assert!(flow.says_new_keys(), "R1 again after Back");
        }
    }

    /// Item 12 on K1: said only when K1 offers a restore and the root
    /// found no `identity-keys/`; a lost seed key with the identities'
    /// keys still in place restores them, and K1 with no restore takes no
    /// phrase. Back from R2 rebuilds K1 and keeps the answer. Mutation:
    /// without `self.new_keys &&` in `says_new_keys`, K1 over kept keys
    /// says it and this fails; with `can_restore()` widened to any K1, the
    /// record naming no `PeerId` says it and this fails.
    #[test]
    fn k1_says_new_keys_only_with_no_identity_keys() {
        let k1 = |reason, expected, new_keys| {
            IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::Recovery {
                    reason,
                    expected,
                    new_keys,
                },
            )
        };
        for reason in [RecoveryReason::Key, RecoveryReason::Record] {
            for new_keys in [false, true] {
                let mut flow = k1(reason, Some(mine()), new_keys);
                let case = format!("{reason:?}, no identity-keys/ {new_keys}");
                assert_eq!(flow.says_new_keys(), new_keys, "{case}");
                assert!(flow.warns_active_elsewhere(), "{case}");
                assert_eq!(flow.primary(), [], "{case}: a statement, not a gate");
                assert_eq!(flow.screen(), ScreenId::R2, "{case}");
                assert!(!flow.says_new_keys(), "{case}: R2 keeps its room");
                flow.back();
                assert_eq!(flow.screen(), ScreenId::K1, "{case}");
                assert_eq!(flow.says_new_keys(), new_keys, "{case}: after Back");
            }
            let no_restore = k1(reason, None, true);
            assert!(
                !no_restore.says_new_keys(),
                "{reason:?}: K1 with no restore takes no phrase"
            );
        }
    }

    #[test]
    fn a_record_naming_no_peer_offers_no_restore_at_all() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Record,
                expected: None,
                new_keys: false,
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
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Record,
                expected: Some(mine()),
                new_keys: false,
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
    fn a_seal_that_finds_another_identity_shows_already_created_and_opens_it() {
        let mut flow = created_at_c3();
        enter(&mut flow, "galaxy");
        enter(&mut flow, "saddle");
        flow.primary();
        flow.sealed(Err(StepFailed::AlreadyProvisioned));
        assert_eq!(flow.notice(), Some(&Notice::AlreadyCreated));
        assert_eq!(
            flow.primary(),
            [Effect::VoidPaper(false), Effect::StartService],
            "Continue opens what the phone holds, and seals nothing"
        );
        assert!(
            flow.start_held(),
            "held until the start decides, as any Continue"
        );
        assert_eq!(flow.decided(), [Effect::OpenChats]);
    }

    #[test]
    fn back_from_r2_with_words_asks_first_and_keeping_stays() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
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
        flow.back();
        assert!(
            !flow.discard_asked(),
            "Back on the dialog continues entering"
        );
        assert_eq!(flow.screen(), ScreenId::R2);
        assert_eq!(flow.entry().and_then(|e| e.word(0)), Some("acid"));
        flow.back();
        flow.discard(true);
        assert_eq!(flow.screen(), ScreenId::R1);
        flow.primary();
        assert_eq!(flow.entry().and_then(|e| e.word(0)), None, "words dropped");
    }

    /// Section 4.4's full list over 130 % font: Back closes it with every
    /// word kept and no dialog; a position chosen there closes it onto
    /// that position; it never opens over the dialog or a check.
    #[test]
    fn r2s_full_list_closes_on_back_or_a_position_and_waits_for_the_dialog() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        enter(&mut flow, "acid");
        flow.review_words(true);
        assert!(flow.reviewing());
        assert_eq!(flow.back(), []);
        assert!(!flow.reviewing(), "Back closes the list");
        assert!(!flow.discard_asked(), "the list's Back asks nothing");
        assert_eq!(flow.entry().and_then(|e| e.word(0)), Some("acid"));
        flow.review_words(true);
        flow.edit(0);
        assert!(!flow.reviewing(), "a position chosen closes the list");
        assert_eq!(flow.picker().map(WordPicker::typed), Some("acid"));
        flow.back();
        assert!(flow.discard_asked());
        flow.review_words(true);
        assert!(!flow.reviewing(), "never over the discard dialog");
        flow.discard(false);
        flow.review_words(true);
        flow.review_words(false);
        assert!(!flow.reviewing());

        // A check pending: the list waits for it.
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        fill(&mut flow);
        flow.primary();
        assert!(flow.waiting(), "control: the check is pending");
        flow.review_words(true);
        assert!(!flow.reviewing(), "never over a check");
    }

    /// Behind the discard dialog and over R2's full list nothing the
    /// keypad sends reaches the words: a letter, Delete, Clear, a chip,
    /// Enter, a grid position (#634 F1). Each is an input queued behind
    /// the Back in one batch, which the view cannot refuse.
    #[test]
    fn no_letter_chip_or_enter_reaches_the_words_behind_the_dialog_or_the_list() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        assert!(flow.press('z'));
        assert!(flow.press('e'));
        flow.back();
        assert!(flow.discard_asked(), "control: the dialog is up");
        assert_eq!(flow.enter(), []);
        assert_eq!(flow.choose("zebra"), []);
        assert!(!flow.press('b'));
        flow.delete();
        flow.clear_letters();
        flow.edit(3);
        assert_eq!(
            flow.entry().and_then(|e| e.word(0)),
            None,
            "no word behind it"
        );
        assert_eq!(flow.picker().map(WordPicker::typed), Some("ze"));
        assert_eq!(flow.chips_taken(), 0);
        flow.discard(false);
        flow.enter();
        assert_eq!(
            flow.entry().and_then(|e| e.word(0)),
            Some("zebra"),
            "control: with the dialog closed Enter takes it"
        );

        flow.review_words(true);
        assert!(flow.reviewing(), "control: the list is open");
        assert!(!flow.press('b'));
        assert_eq!(flow.picker().map(WordPicker::typed), Some(""));
        flow.edit(0);
        assert!(!flow.reviewing(), "a position chosen still closes the list");
        flow.clear_letters();
        assert!(flow.press('b'), "control: the list closed, b is a letter");

        // C3 behind its dialog.
        let mut flow = created_at_c3();
        assert!(flow.press('g'));
        flow.back();
        assert!(flow.discard_asked());
        assert_eq!(flow.enter(), []);
        assert_eq!(flow.check(), Some((6, None)), "galaxy not taken");
        flow.discard(false);
        flow.enter();
        assert_eq!(flow.check(), Some((18, None)), "control: taken once closed");
    }

    /// While R2's check runs the words are held (design section 8, Check
    /// running): a letter, Delete, Clear, a chip, Enter, a grid position,
    /// the full list and Back change nothing, through the check and,
    /// after K1, through the restore it starts, so the restore asks for
    /// the words that were checked.
    #[test]
    fn no_input_changes_the_words_while_the_check_runs() {
        fn hold(flow: &mut IdentityFlow) {
            let entry = format!("{:?}", flow.entry());
            let words: Vec<_> = (0..PHRASE_WORDS)
                .map(|p| flow.entry().and_then(|e| e.word(p)))
                .collect();
            let typed = flow.picker().map(|p| p.typed().to_owned());
            let counts = (flow.chips_taken(), flow.positions_chosen());
            let current = flow.entry().map(PhraseEntry::current);
            for letter in 'a'..='z' {
                assert!(!flow.press(letter), "{letter} held");
            }
            flow.delete();
            flow.clear_letters();
            for word in LIST {
                assert_eq!(flow.choose(word), [], "{word} held");
            }
            assert_eq!(flow.enter(), []);
            flow.edit(4);
            flow.review_words(true);
            assert_eq!(flow.back(), []);
            assert_eq!(flow.primary(), [], "one check at a time");
            assert_eq!(flow.screen(), ScreenId::R2);
            assert!(!flow.reviewing() && !flow.discard_asked());
            let after: Vec<_> = (0..PHRASE_WORDS)
                .map(|p| flow.entry().and_then(|e| e.word(p)))
                .collect();
            assert_eq!(after, words, "no word changed");
            assert_eq!(flow.picker().map(|p| p.typed().to_owned()), typed);
            assert_eq!((flow.chips_taken(), flow.positions_chosen()), counts);
            assert_eq!(flow.entry().map(PhraseEntry::current), current);
            assert_eq!(format!("{:?}", flow.entry()), entry);
        }
        // A fresh restore: position 6 reopened with "fabri" typed, so
        // Delete, Clear and a letter each have something to change.
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        fill(&mut flow);
        flow.edit(5);
        flow.delete();
        assert_eq!(flow.picker().map(WordPicker::typed), Some("fabri"));
        assert_eq!(flow.primary(), [Effect::Check(words().join(" "))]);
        hold(&mut flow);
        assert_eq!(flow.checked(None), []);
        assert_eq!(flow.notice(), Some(&Notice::ChecksumFailed));
        assert!(flow.press('c'), "control: the result frees the words");

        // After K1: the check, then the restore it starts, then a Try
        // again of the refused restore, each holding the words.
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
            },
        );
        flow.primary();
        fill(&mut flow);
        assert_eq!(flow.primary(), [Effect::Check(words().join(" "))]);
        hold(&mut flow);
        assert_eq!(
            flow.checked(Some(mine())),
            [Effect::RestoreRecorded(words().join(" "))]
        );
        hold(&mut flow);
        flow.restored(Err(StepFailed::Busy));
        assert_eq!(flow.notice(), Some(&Notice::Busy));
        assert_eq!(flow.primary(), [Effect::Check(words().join(" "))]);
        hold(&mut flow);
        assert_eq!(flow.notice(), Some(&Notice::Busy), "the notice stays");
        flow.cancelled_step();
        flow.edit(4);
        assert_eq!(flow.positions_chosen(), 1, "control: a cancel frees them");
    }

    /// The screen's actions behind the discard dialog: R2's Check my
    /// words, C3's Try again and Show words again do nothing until the
    /// dialog closes, then act (#634 review, pre-existing 1).
    #[test]
    fn no_screen_action_runs_behind_the_dialog() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        fill(&mut flow);
        flow.back();
        assert!(flow.discard_asked(), "control: the dialog is up");
        assert_eq!(flow.primary(), [], "no check behind the dialog");
        assert!(flow.discard_asked(), "the dialog stays");
        flow.discard(false);
        assert_eq!(
            flow.primary(),
            [Effect::Check(words().join(" "))],
            "control: with the dialog closed Check runs"
        );

        let mut flow = created_at_c3();
        enter(&mut flow, "habit");
        assert_eq!(flow.check(), Some((6, Some(6))), "control: a wrong word");
        assert!(flow.press('g'));
        flow.back();
        assert!(flow.discard_asked(), "control: letters typed ask first");
        flow.try_again();
        assert_eq!(flow.check(), Some((6, Some(6))), "Try again held");
        flow.show_words_again();
        assert_eq!(flow.screen(), ScreenId::C3, "Show words again held");
        assert!(flow.discard_asked(), "the dialog stays");
        flow.discard(false);
        flow.try_again();
        assert_eq!(flow.check(), Some((6, None)), "control: Try again runs");
        flow.back();
        assert!(flow.discard_asked(), "control: g still typed");
        flow.discard(false);
        flow.show_words_again();
        assert_eq!(
            flow.screen(),
            ScreenId::C2,
            "control: Show words again runs"
        );
    }

    /// The count a page carries for section 8's focus rule: R2 counts a
    /// chip the entry took, never one it refused; C3 counts every chip
    /// of its picker, right or wrong; Enter counts as the chip it takes,
    /// and an Enter that takes nothing counts nothing (#634 F3).
    #[test]
    fn chips_taken_counts_each_chip_a_page_took() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        assert_eq!(flow.choose("zebra"), []);
        assert_eq!(flow.chips_taken(), 0, "R2: a chip not on the picker");
        assert!(flow.press('z'));
        flow.enter();
        assert_eq!(
            flow.chips_taken(),
            0,
            "R2: zebra and zone, Enter takes nothing"
        );
        assert_eq!(flow.entry().and_then(|e| e.word(0)), None);
        assert!(flow.press('o'));
        flow.enter();
        assert_eq!(flow.chips_taken(), 1, "R2: Enter took the only chip");
        assert_eq!(flow.entry().and_then(|e| e.word(0)), Some("zone"));
        enter(&mut flow, "acid");
        assert_eq!(flow.chips_taken(), 2, "R2: a chip tapped");

        let mut flow = created_at_c3();
        enter(&mut flow, "habit");
        assert_eq!(flow.check(), Some((6, Some(6))), "control: a wrong word");
        assert_eq!(flow.chips_taken(), 1, "C3: a wrong word counts");
        flow.try_again();
        assert!(flow.press('g'));
        flow.enter();
        assert_eq!(
            flow.check(),
            Some((18, None)),
            "C3: Enter took the right word"
        );
        assert_eq!(flow.chips_taken(), 2);
    }

    /// The count a page carries for a position chosen to edit (section
    /// 8): every edit R2 makes counts, the same position again included,
    /// and one behind the discard dialog, which edits nothing, does not.
    #[test]
    fn positions_chosen_counts_each_edit_r2_made() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        assert_eq!(flow.positions_chosen(), 0, "R2 opens on none chosen");
        enter(&mut flow, "acid");
        flow.edit(0);
        assert_eq!(flow.positions_chosen(), 1);
        flow.edit(0);
        assert_eq!(flow.positions_chosen(), 2, "the same position again");
        flow.review_words(true);
        flow.edit(3);
        assert_eq!(flow.positions_chosen(), 3, "a row of the full list");
        assert_eq!(flow.back(), []);
        assert!(flow.discard_asked(), "control: the dialog is up");
        flow.edit(1);
        assert_eq!(flow.positions_chosen(), 3, "nothing edited behind it");
    }

    /// R3, a valid check of every word on a fresh install.
    fn at_r3() -> IdentityFlow {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        fill(&mut flow);
        flow.primary();
        flow.checked(Some(mine()));
        assert_eq!(flow.screen(), ScreenId::R3, "control: at R3");
        flow
    }

    /// While R3's restore runs (design section 8, section 6 Restore
    /// running) Yes, No, Back and a notice's Try again (the primary
    /// again, as `apply` routes it) start nothing and leave R3 as it
    /// was; the screen waits, so the back arrow is inactive. Its result
    /// gives them back. Mutation: dropping the `!*restoring` guard in
    /// `primary`, or the one in `back` or `recheck`, fails here.
    #[test]
    fn r3_holds_while_its_restore_runs() {
        let mut flow = at_r3();
        let phrase = words().join(" ");
        assert!(!flow.waiting(), "control: nothing runs");
        assert_eq!(
            flow.primary(),
            [Effect::RestoreConfirmed(phrase.clone(), mine())]
        );
        assert!(flow.waiting(), "the restore runs");
        let held = |flow: &mut IdentityFlow| {
            assert_eq!(flow.primary(), [], "no second restore");
            assert_eq!(flow.back(), [], "Back does nothing");
            assert_eq!(flow.screen(), ScreenId::R3, "Back leaves R3 open");
            flow.recheck();
            assert_eq!(flow.screen(), ScreenId::R3, "No leaves R3 open");
            assert!(flow.waiting(), "still held");
        };
        held(&mut flow);

        flow.restored(Err(StepFailed::Storage));
        assert!(!flow.waiting(), "the result gives R3 back");
        assert_eq!(flow.notice(), Some(&Notice::StorageFailed));
        // Try again: the restore again, and R3 held the same way, with
        // the notice still showing.
        assert_eq!(
            flow.primary(),
            [Effect::RestoreConfirmed(phrase.clone(), mine())]
        );
        held(&mut flow);
        assert_eq!(
            flow.notice(),
            Some(&Notice::StorageFailed),
            "the notice stays while the restore runs"
        );
        assert_eq!(flow.restored(Ok(())), []);
        assert_eq!(flow.screen(), ScreenId::R4);

        // Not held, No and Back go back to R2, words kept.
        let mut flow = at_r3();
        flow.recheck();
        assert_eq!(flow.screen(), ScreenId::R2, "control: No acts");
        let mut flow = at_r3();
        assert_eq!(flow.back(), []);
        assert_eq!(flow.screen(), ScreenId::R2, "control: Back acts");
    }

    /// A restore whose prompt was cancelled or denied leaves R3 as it
    /// was: no notice, not held, and no further restore asked until Yes
    /// is pressed again (acceptance 15's sixth run). Mutation: dropping
    /// the R3 arm of `cancelled_step` leaves R3 held, and fails here.
    #[test]
    fn a_cancelled_restore_gives_r3_back_with_no_notice() {
        let mut flow = at_r3();
        let phrase = words().join(" ");
        assert_eq!(
            flow.primary(),
            [Effect::RestoreConfirmed(phrase.clone(), mine())]
        );
        let raised = flow.notices_raised();
        flow.cancelled_step();
        assert_eq!(flow.screen(), ScreenId::R3);
        assert!(!flow.waiting(), "Yes and No enabled again");
        assert_eq!(flow.notice(), None, "no notice");
        assert_eq!(flow.notices_raised(), raised, "none raised");
        assert_eq!(
            flow.primary(),
            [Effect::RestoreConfirmed(phrase, mine())],
            "Yes again restores again"
        );
    }

    /// Each result that raises a notice counts, the same notice again
    /// included, so the window can give focus to a repeated one (#697
    /// review P-2); a cancelled prompt raises none.
    #[test]
    fn notices_raised_counts_each_result_that_raises_one() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::FirstRun { void_paper: false },
        );
        flow.restore();
        flow.primary();
        fill(&mut flow);
        assert_eq!(flow.notices_raised(), 0, "R2 opens on none");
        flow.primary();
        flow.checked(None);
        assert_eq!(flow.notices_raised(), 1);
        flow.primary();
        flow.checked(None);
        assert_eq!(flow.notice(), Some(&Notice::ChecksumFailed), "the same");
        assert_eq!(flow.notices_raised(), 2, "raised again");
        flow.primary();
        flow.cancelled_step();
        assert_eq!(flow.notices_raised(), 2, "a cancel raises none");
        flow.primary();
        flow.checked(Some(mine()));
        assert_eq!(flow.screen(), ScreenId::R3);
        flow.primary();
        flow.restored(Err(StepFailed::Busy));
        assert_eq!(flow.notices_raised(), 3, "R3's restore");
        flow.primary();
        flow.restored(Err(StepFailed::Busy));
        assert_eq!(flow.notices_raised(), 4, "R3's restore again");
    }

    /// After K1, R2's other two results count each raise, the same one
    /// again included: another identity's `PeerId`, and a restore against
    /// the record that failed (#698 review F3).
    #[test]
    fn notices_raised_counts_r2s_results_after_k1() {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
            },
        );
        flow.primary();
        fill(&mut flow);
        for raised in 1..=2 {
            flow.primary();
            flow.checked(Some(other()));
            assert!(matches!(flow.notice(), Some(Notice::OtherIdentity { .. })));
            assert_eq!(flow.notices_raised(), raised, "another identity");
        }
        for raised in 3..=4 {
            flow.primary();
            flow.checked(Some(mine()));
            flow.restored(Err(StepFailed::Busy));
            assert_eq!(flow.notice(), Some(&Notice::Busy));
            assert_eq!(flow.notices_raised(), raised, "the restore on R2");
        }
    }

    /// C4's seal and S1's going offline count each failure, the same
    /// one again included (#698 review F3).
    #[test]
    fn notices_raised_counts_c4s_seal_and_s1s_going_offline() {
        let mut flow = created_at_c3();
        enter(&mut flow, "galaxy");
        enter(&mut flow, "saddle");
        for raised in 1..=2 {
            flow.primary();
            flow.sealed(Err(StepFailed::Storage));
            assert_eq!(flow.notices_raised(), raised, "the seal");
        }
        let mut flow =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        for raised in 1..=2 {
            flow.primary();
            flow.prompted(true);
            flow.offline(Err(StepFailed::Busy));
            assert_eq!(flow.notices_raised(), raised, "going offline");
        }
    }

    #[test]
    fn back_from_c3_with_words_asks_first_and_delete_asks_both_again() {
        let mut flow = created_at_c3();
        assert_eq!(flow.back(), []);
        assert_eq!(
            flow.screen(),
            ScreenId::C2PeerId,
            "nothing entered: straight back, to C2's PeerId page"
        );
        flow.primary();
        assert!(flow.press('g'));
        flow.back();
        assert!(flow.discard_asked(), "letters typed are words entered");
        flow.discard(false);
        assert_eq!(flow.screen(), ScreenId::C3);
        assert_eq!(flow.picker().map(WordPicker::typed), Some("g"));
        flow.clear_letters();
        enter(&mut flow, "galaxy");
        flow.back();
        assert!(flow.discard_asked(), "a right word taken is a word entered");
        flow.back();
        assert!(
            !flow.discard_asked(),
            "Back on the dialog continues entering"
        );
        assert_eq!(flow.check(), Some((18, None)), "the right word kept");
        flow.back();
        flow.discard(true);
        assert_eq!(flow.screen(), ScreenId::C2PeerId, "back as C3's Back goes");
        assert_eq!(flow.peer(), Some(&mine()), "the identity is kept");
        flow.back();
        assert_eq!(flow.words(), Some(&words()), "the same words");
        flow.primary();
        flow.primary();
        assert_eq!(flow.check(), Some((6, None)), "both positions asked again");
        flow.discard(true);
        assert_eq!(flow.screen(), ScreenId::C3, "no dialog, no discard");
    }

    #[test]
    fn show_phrase_prompts_goes_offline_and_restarts_the_service_after() {
        let mut flow =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
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
        assert_eq!(flow.peer(), None, "no PeerId beside the words");
        assert_eq!(flow.back(), [Effect::StartService, Effect::OpenSettings]);
        assert_eq!(flow.words(), None);
    }

    /// S2's two pages (section 3.5): Next to the `PeerId` page, Back there
    /// to the same words, and Done -- the `PeerId` page's primary, or Back
    /// on the words page -- drops both and restarts the service. Mutation:
    /// sending S2's Next to `leave_s2`, or its `PeerId` page's Back,
    /// fails here.
    #[test]
    fn s2_shows_the_words_then_the_peer_id_and_done_ends_on_either() {
        let mut flow =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        flow.primary();
        flow.prompted(true);
        flow.offline(Ok((words(), mine())));
        assert_eq!(flow.primary(), [], "Next restarts nothing");
        assert_eq!(flow.screen(), ScreenId::S2PeerId);
        assert_eq!(flow.words(), None, "no word beside the PeerId");
        assert_eq!(flow.format(), None, "nor the words' format");
        assert_eq!(flow.peer(), Some(&mine()));
        assert!(flow.back_arrow());
        assert_eq!(flow.back(), [], "back to the words, still offline");
        assert_eq!(flow.screen(), ScreenId::S2);
        assert_eq!(flow.words(), Some(&words()), "the same words");
        flow.primary();
        assert_eq!(
            flow.primary(),
            [Effect::StartService, Effect::OpenSettings],
            "Done"
        );
        assert_eq!(flow.screen(), ScreenId::Done);
        assert_eq!(flow.words(), None);
        assert_eq!(flow.peer(), None);
    }

    /// C2's two pages (section 3.2): the words page's primary opens the
    /// `PeerId` page, whose Back returns to the same words and whose
    /// primary opens C3; C3's "Show my words again" returns to the words
    /// page and its Back to the `PeerId` page. Nothing is sealed or
    /// discarded on either page. Mutation: sending the words page's
    /// primary to C3, or the `PeerId` page's Back to C1, fails here.
    #[test]
    fn c2s_words_page_and_peer_id_page_lead_to_each_other() {
        let mut flow = created_at_c2();
        assert_eq!(flow.primary(), []);
        assert_eq!(flow.screen(), ScreenId::C2PeerId);
        assert_eq!(flow.check(), None, "not C3 yet");
        assert!(flow.back_arrow());
        assert_eq!(flow.back(), [], "nothing discarded");
        assert_eq!(flow.screen(), ScreenId::C2);
        assert_eq!(flow.words(), Some(&words()), "the same words");
        assert_eq!(flow.back(), [], "the words page's Back");
        assert_eq!(flow.screen(), ScreenId::C1, "C1, nothing discarded yet");
        assert_eq!(flow.primary(), [], "the marker is already set");
        assert_eq!(flow.screen(), ScreenId::C2, "the words page first");
        assert_eq!(flow.words(), Some(&words()), "the same words on return");
        flow.primary();
        flow.primary();
        assert_eq!(flow.screen(), ScreenId::C3);
        enter(&mut flow, "zebra");
        flow.show_words_again();
        assert_eq!(flow.screen(), ScreenId::C2);
        flow.primary();
        flow.primary();
        assert_eq!(flow.back(), []);
        assert_eq!(flow.screen(), ScreenId::C2PeerId, "C3's Back");
    }

    /// The secret words and the public `PeerId` are never on one page, and
    /// the format identifier shows with the words alone (section 3.2).
    /// Every page of C2 and S2 is visited and each accessor read there.
    /// Mutation: letting `words` answer on a `PeerId` page, `peer` on a
    /// words page, or `format` anywhere the words are not, fails here.
    #[test]
    fn the_words_and_the_peer_id_are_never_on_one_page() {
        let mut c2 = created_at_c2();
        let mut s2 =
            IdentityFlow::new(LIST, interweave_profile_identity::FORMAT, Start::ShowPhrase);
        s2.primary();
        s2.prompted(true);
        s2.offline(Ok((words(), mine())));
        for flow in [&mut c2, &mut s2] {
            for page in 0..2 {
                let screen = flow.screen();
                let words = flow.words().is_some();
                let peer = flow.peer().is_some();
                assert!(words != peer, "{screen:?}: one of the two, never both");
                assert_eq!(words, page == 0, "{screen:?}: the words first");
                assert_eq!(
                    flow.format(),
                    words.then_some(interweave_profile_identity::FORMAT),
                    "{screen:?}: the format with the words alone"
                );
                flow.primary();
            }
        }
        assert_eq!(c2.screen(), ScreenId::C3, "control: past both pages");
        assert_eq!(c2.format(), None, "C3 shows no format");
        assert_eq!(s2.screen(), ScreenId::Done, "control: past both pages");
    }

    /// C4 after its seal, R2 after K1 after its check, and R3: each at
    /// the point where the step's result arrives.
    fn c4_sealing() -> IdentityFlow {
        let mut flow = created_at_c3();
        enter(&mut flow, "galaxy");
        enter(&mut flow, "saddle");
        assert_eq!(flow.primary(), [Effect::Seal], "control: the seal");
        flow
    }

    fn r2_after_k1_restoring() -> IdentityFlow {
        let mut flow = IdentityFlow::new(
            LIST,
            interweave_profile_identity::FORMAT,
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
            },
        );
        flow.primary();
        fill(&mut flow);
        flow.primary();
        assert_eq!(
            flow.checked(Some(mine())),
            [Effect::RestoreRecorded(words().join(" "))],
            "control: the restore"
        );
        flow
    }

    /// `IdStorageUnknown` (design section 6): on C4 its Try again, the
    /// primary, reads the record back for the `PeerId` C4 shows and seals
    /// nothing; on R2 after K1 and on R3 the notice's action reads it
    /// back for the record's `PeerId` or the one confirmed. Each holds its
    /// screen while it runs, and a read-back that finds the record goes
    /// on as the step's success would; one that does not shows the notice
    /// again. Mutation: the C4 `StorageUnknown` arm of `primary` removed
    /// (the seal again), or `notice_action`'s `ReadBack` replaced by the
    /// step's retry, fails here.
    #[test]
    fn storage_unknown_try_again_reads_back_and_never_seals() {
        let mut c4 = c4_sealing();
        c4.sealed(Err(StepFailed::StorageUnknown));
        assert_eq!(c4.notice(), Some(&Notice::StorageUnknown));
        assert!(!c4.start_held(), "control: C4 given back");
        assert_eq!(c4.primary(), [Effect::ReadBack(mine())]);
        assert!(c4.start_held(), "held while it reads back");
        assert_eq!(c4.primary(), [], "one read-back at a time");
        assert_eq!(c4.sealed(Err(StepFailed::StorageUnknown)), []);
        assert_eq!(c4.notices_raised(), 2, "raised again, focus again");
        assert_eq!(c4.primary(), [Effect::ReadBack(mine())]);
        assert_eq!(
            c4.sealed(Ok(())),
            [Effect::VoidPaper(false), Effect::StartService],
            "found: C4 goes on as after a seal"
        );

        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::StorageUnknown));
        assert_eq!(r2.notice(), Some(&Notice::StorageUnknown));
        assert_eq!(r2.notice_action(), [Effect::ReadBack(mine())]);
        assert!(r2.waiting(), "R2 held while it reads back");
        assert_eq!(r2.notice_action(), [], "one at a time");
        assert_eq!(r2.back(), [], "Back waits");
        assert_eq!(r2.screen(), ScreenId::R2);
        r2.restored(Ok(()));
        assert_eq!(r2.screen(), ScreenId::R4);

        let mut r3 = at_r3();
        r3.primary();
        r3.restored(Err(StepFailed::StorageUnknown));
        assert_eq!(r3.notice_action(), [Effect::ReadBack(mine())]);
        assert!(r3.waiting(), "R3 held while it reads back");
        r3.restored(Err(StepFailed::StorageUnknown));
        assert!(!r3.waiting());
        assert_eq!(r3.notice(), Some(&Notice::StorageUnknown), "shown again");
    }

    /// C4 under `IdStorageUnknown` after a read-back answered as failing,
    /// `IdStartUnchecked` offered (design section 6, A start from C4 after
    /// a failed read-back).
    fn c4_unchecked_offered() -> IdentityFlow {
        let mut c4 = c4_sealing();
        c4.sealed(Err(StepFailed::StorageUnknown));
        assert!(c4.offers_unchecked(), "control: offered");
        c4
    }

    /// `IdStartUnchecked` is offered only once a read-back on C4 has
    /// answered as failing -- the seal's own read-back, or Try again's --
    /// and only while `IdStorageUnknown` shows: never on a C4 before its
    /// seal, nor under another notice, nor after a cancelled or denied
    /// read-back prompt, where no read-back answered (design section 6,
    /// A start from C4 after a failed read-back; acceptance 12). Under a
    /// Try again that runs it stays, held (the view shows it disabled).
    /// Mutations: `sealed` offering it for any failure, or `step_not_run`
    /// offering it, or `offers_unchecked` reading no notice, fail here.
    #[test]
    fn start_unchecked_is_offered_only_after_a_read_back_failed() {
        let mut fresh = created_at_c3();
        enter(&mut fresh, "galaxy");
        enter(&mut fresh, "saddle");
        assert!(!fresh.offers_unchecked(), "not before a seal");
        assert_eq!(fresh.start_unchecked(), [], "and starts nothing");

        for failed in [StepFailed::Busy, StepFailed::Storage] {
            let mut c4 = c4_sealing();
            c4.sealed(Err(failed.clone()));
            assert!(!c4.offers_unchecked(), "not under {failed:?}");
            // Nor once `IdStorageUnknown` follows with no read-back
            // answered: that failure offered nothing for later.
            assert_eq!(c4.primary(), [Effect::Seal], "{failed:?}: seals again");
            c4.sealed(Err(StepFailed::ReadBackNeedsAuthentication));
            c4.step_not_run(&Effect::ReadBack(mine()));
            assert_eq!(c4.notice(), Some(&Notice::StorageUnknown));
            assert!(
                !c4.offers_unchecked(),
                "{failed:?}, then a cancelled prompt"
            );
        }
        let mut already = c4_sealing();
        already.sealed(Err(StepFailed::AlreadyProvisioned));
        assert!(!already.offers_unchecked(), "not under IdAlreadyCreated");

        // The read-back's prompt cancelled: the notice, and no control.
        let mut cancelled = c4_sealing();
        assert_eq!(
            cancelled.sealed(Err(StepFailed::ReadBackNeedsAuthentication)),
            [Effect::AuthenticateReadBack(mine())]
        );
        cancelled.step_not_run(&Effect::ReadBack(mine()));
        assert_eq!(cancelled.notice(), Some(&Notice::StorageUnknown));
        assert!(!cancelled.offers_unchecked(), "no read-back answered");
        assert_eq!(cancelled.start_unchecked(), [], "starts nothing");
        // Try again's read-back answered as failing: offered from then on.
        assert_eq!(cancelled.primary(), [Effect::ReadBack(mine())]);
        cancelled.sealed(Err(StepFailed::StorageUnknown));
        assert!(cancelled.offers_unchecked(), "a read-back failed");

        // The seal's own read-back answered as failing.
        let mut c4 = c4_unchecked_offered();
        assert_eq!(c4.primary(), [Effect::ReadBack(mine())], "Try again");
        assert!(c4.offers_unchecked() && c4.start_held(), "shown, held");
        assert_eq!(c4.start_unchecked(), [], "disabled while Try again runs");
        // A later prompt for the read-back, cancelled: still offered, a
        // read-back has failed before on this C4.
        c4.step_not_run(&Effect::ReadBack(mine()));
        assert!(
            c4.offers_unchecked(),
            "stays offered while the notice shows"
        );
        // Under another notice with the offer still standing: its start
        // refused with the profile in use, `IdStartBusy`'s Try again
        // replaces both controls.
        c4.start_unchecked();
        c4.start_busy();
        assert!(!c4.offers_unchecked(), "not under IdStartBusy");
    }

    /// Its press holds C4 as Continue's does -- held at once, a second
    /// press of it or of Try again starting nothing, Back doing nothing,
    /// the notice and the `PeerId` staying until the result -- ends the
    /// creation (the unsealed identity dropped, its words gone) and asks
    /// one service start and nothing else: no seal, no read-back, no
    /// void-paper marker, so nothing is written (design section 6, What
    /// it does). Mutations: `start_unchecked` returning a seal, a
    /// `VoidPaper`, or no `Discard`, or not holding C4, fail here.
    #[test]
    fn start_unchecked_holds_c4_and_writes_nothing() {
        let mut c4 = c4_unchecked_offered();
        assert_eq!(
            c4.start_unchecked(),
            [Effect::Discard, Effect::StartService]
        );
        assert!(c4.start_held(), "held from the press");
        assert_eq!(c4.start_unchecked(), [], "a second press starts nothing");
        assert_eq!(c4.primary(), [], "Try again starts nothing either");
        assert_eq!(c4.back(), [], "C4 has no Back");
        assert_eq!(c4.notice(), Some(&Notice::StorageUnknown), "stays");
        assert_eq!(c4.peer(), Some(&mine()), "the PeerId stays on C4");
        assert!(c4.offers_unchecked(), "shown, disabled, while it runs");
        c4.set_starting(true);
        assert!(c4.starting(), "ConnectStarting counts from the press");
        assert_eq!(c4.decided(), [Effect::OpenChats], "opens: Chats");
        assert_eq!(c4.screen(), ScreenId::Done);
    }

    /// The start that must open the record to C4's `PeerId` is
    /// `IdStartUnchecked`'s, from its press, and `IdStartBusy`'s Try again
    /// and a showing's retry after it: no other start of the flow names
    /// one -- Continue's after a seal, Try again's after a read-back that
    /// found the record, `IdAlreadyCreated`'s, R4's (design section 6, the
    /// profile-in-use row and The hold). Mutations: `start_expects`
    /// answering C4's `PeerId` on any held C4, or `start_busy` clearing
    /// `unchecked`, or `unchecked_refused` leaving it set, fail here.
    #[test]
    fn start_expects_names_c4s_peer_id_for_its_starts_alone() {
        for (place, flow) in held_starts() {
            assert_eq!(flow.start_expects(), None, "{place}");
        }
        let mut c4 = c4_unchecked_offered();
        assert_eq!(c4.start_expects(), None, "nothing asked yet");
        assert_eq!(c4.primary(), [Effect::ReadBack(mine())]);
        assert_eq!(c4.start_expects(), None, "a read-back compares nothing");
        c4.sealed(Err(StepFailed::StorageUnknown));
        c4.start_unchecked();
        assert_eq!(c4.start_expects(), Some(&mine()), "its press");
        c4.start_busy();
        assert_eq!(
            c4.notice(),
            Some(&Notice::StartBusy {
                recorded: None,
                already: false
            })
        );
        assert!(!c4.offers_unchecked(), "Try again replaces both controls");
        assert_eq!(c4.start_expects(), Some(&mine()), "under IdStartBusy");
        assert_eq!(c4.primary(), [Effect::StartService], "Try again: a start");
        assert_eq!(c4.start_expects(), Some(&mine()), "comparison included");
        c4.start_busy();
        assert!(c4.showing_started(), "a return's retry holds C4");
        assert_eq!(c4.start_expects(), Some(&mine()), "and compares too");

        // A mismatch ends it, and Try again's read-back that finds the
        // record starts the service as after a seal, comparing nothing:
        // the read-back answered the PeerId.
        c4.unchecked_refused(UncheckedRefused::OtherIdentity);
        assert_eq!(c4.start_expects(), None, "nothing held");
        assert_eq!(c4.primary(), [Effect::ReadBack(mine())]);
        assert_eq!(
            c4.sealed(Ok(())),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        assert_eq!(c4.start_expects(), None, "Try again's start");
    }

    /// Its results (design section 6, Its results; acceptance 12): another
    /// `PeerId` than C4's starts nothing and shows `IdStorageUnknown` with
    /// Try again only, raised for focus; a platform error shows it with
    /// both controls, raised; a record that does not open is K1, with the
    /// record's `PeerId` as the expected one, asking for the flow's lock
    /// as K1 at a launch does. Each answers only the held start it is
    /// for. Mutations: the mismatch not withdrawing the control, or not
    /// raising the notice, or `Unavailable` withdrawing it, or `Recovery`
    /// staying on C4, fail here.
    #[test]
    fn start_uncheckeds_results_are_a_launchs_and_a_mismatch_fails_closed() {
        let mut other = c4_unchecked_offered();
        assert_eq!(
            other.unchecked_refused(UncheckedRefused::OtherIdentity),
            [],
            "not held: nothing"
        );
        assert!(other.offers_unchecked(), "control: still offered");
        other.start_unchecked();
        let raised = other.notices_raised();
        assert_eq!(other.unchecked_refused(UncheckedRefused::OtherIdentity), []);
        assert_eq!(other.screen(), ScreenId::C4, "nothing started");
        assert!(!other.start_held(), "Try again enabled again");
        assert_eq!(other.notice(), Some(&Notice::StorageUnknown));
        assert_eq!(other.notices_raised(), raised + 1, "focus on the notice");
        assert!(!other.offers_unchecked(), "Try again only");
        assert_eq!(other.start_unchecked(), [], "and it starts nothing");
        assert_eq!(other.start_expects(), None);

        let mut unavailable = c4_unchecked_offered();
        unavailable.start_unchecked();
        let raised = unavailable.notices_raised();
        unavailable.unchecked_refused(UncheckedRefused::Unavailable);
        assert!(!unavailable.start_held());
        assert_eq!(unavailable.notice(), Some(&Notice::StorageUnknown));
        assert_eq!(unavailable.notices_raised(), raised + 1, "focus");
        assert!(unavailable.offers_unchecked(), "both controls");
        assert_eq!(
            unavailable.start_unchecked(),
            [Effect::Discard, Effect::StartService],
            "pressed again"
        );

        let mut recovery = c4_unchecked_offered();
        recovery.start_unchecked();
        assert_eq!(
            recovery.unchecked_refused(UncheckedRefused::Recovery {
                reason: RecoveryReason::Key,
                expected: Some(mine()),
                new_keys: false,
            }),
            [Effect::Hold],
            "K1 takes the flow's lock again"
        );
        assert_eq!(recovery.screen(), ScreenId::K1);
        assert_eq!(
            recovery.recovery(),
            Some((RecoveryReason::Key, Some(&mine()))),
            "the record's PeerId, C4's, as the expected one"
        );
        assert!(recovery.can_restore());
        assert_eq!(recovery.start_expects(), None);

        // Under `IdStartBusy` after it, Try again's start answers the same.
        let mut busy = c4_unchecked_offered();
        busy.start_unchecked();
        busy.start_busy();
        busy.primary();
        busy.unchecked_refused(UncheckedRefused::OtherIdentity);
        assert_eq!(busy.notice(), Some(&Notice::StorageUnknown));
        assert!(!busy.offers_unchecked(), "withdrawn from IdStartBusy too");

        // A held start C4 does not answer: nothing (every C4 start that
        // shows a PeerId answers, `every_compared_start_held_on_c4_answers_on_c4`).
        for (place, mut flow) in held_starts() {
            if place == "C4 after a seal" {
                continue;
            }
            assert_eq!(
                flow.unchecked_refused(UncheckedRefused::OtherIdentity),
                [],
                "{place}"
            );
            assert!(flow.start_held(), "{place}: still held");
        }
    }

    /// A start made from C4 that compared and was refused answers on C4
    /// whichever C4 start it was (design section 6, An answer after the
    /// bound: "A start made from C4 that answers before the bound answers
    /// on C4, as Its results says, wherever it comes from"): the start
    /// after Try again's read-back found the record -- after an
    /// `Unavailable` that kept the control, or after a mismatch that
    /// withdrew it -- Continue's after a seal, `IdStartBusy`'s Try again,
    /// and a showing's retry under it. A mismatch shows `IdStorageUnknown`
    /// with Try again only, the control withdrawn; `Unavailable` keeps the
    /// control only where it was offered; either raises the notice for
    /// focus; `RecoveryRequired` is K1 with the record's `PeerId`. C4 under
    /// `IdAlreadyCreated` (no `PeerId` to read back), R4, R3, R2 and an
    /// unheld C4 answer nothing. Mutations: `answers_compared` reading the
    /// `unchecked` flag (the rule before), or not the `PeerId`, or not
    /// `held`, and `unchecked_refused` not withdrawing on a mismatch from
    /// another C4 start, fail here.
    #[test]
    fn every_compared_start_held_on_c4_answers_on_c4() {
        // C4's IdStartUnchecked refused with `first`, then Try again's
        // read-back finds the record: its start compares.
        let read_back_found = |first: UncheckedRefused| {
            let mut c4 = c4_unchecked_offered();
            c4.start_unchecked();
            c4.unchecked_refused(first);
            assert_eq!(c4.primary(), [Effect::ReadBack(mine())], "Try again");
            assert_eq!(
                c4.sealed(Ok(())),
                [Effect::VoidPaper(false), Effect::StartService],
                "found: its start"
            );
            assert_eq!(c4.start_expects(), None, "the flow names nothing");
            assert!(c4.answers_compared(), "yet C4 answers its refusal");
            c4
        };

        for first in [
            UncheckedRefused::Unavailable,
            UncheckedRefused::OtherIdentity,
        ] {
            let offered = first == UncheckedRefused::Unavailable;
            // A mismatch: Try again only, focus on the notice.
            let mut c4 = read_back_found(first.clone());
            let raised = c4.notices_raised();
            assert_eq!(c4.unchecked_refused(UncheckedRefused::OtherIdentity), []);
            assert_eq!(c4.screen(), ScreenId::C4, "{first:?}: nothing ran");
            assert!(!c4.start_held(), "{first:?}: Try again enabled");
            assert_eq!(c4.notice(), Some(&Notice::StorageUnknown));
            assert_eq!(c4.notices_raised(), raised + 1, "{first:?}: focus");
            assert!(!c4.offers_unchecked(), "{first:?}: withdrawn");
            assert_eq!(c4.peer(), Some(&mine()), "{first:?}: C4's PeerId");
            // Unavailable: the control only while it is offered.
            let mut c4 = read_back_found(first.clone());
            let raised = c4.notices_raised();
            assert_eq!(c4.unchecked_refused(UncheckedRefused::Unavailable), []);
            assert_eq!(c4.notice(), Some(&Notice::StorageUnknown));
            assert_eq!(c4.notices_raised(), raised + 1, "{first:?}: focus");
            assert_eq!(c4.offers_unchecked(), offered, "{first:?}: as offered");
            // RecoveryRequired: K1 with the record's PeerId.
            let mut c4 = read_back_found(first.clone());
            assert_eq!(
                c4.unchecked_refused(UncheckedRefused::Recovery {
                    reason: RecoveryReason::Key,
                    expected: Some(mine()),
                    new_keys: false,
                }),
                [Effect::Hold]
            );
            assert_eq!(
                c4.recovery(),
                Some((RecoveryReason::Key, Some(&mine()))),
                "{first:?}: K1"
            );
        }

        // Continue's start after a seal, IdStartBusy's Try again after
        // it, and a showing's retry under IdStartBusy.
        let sealed = || {
            let mut c4 = c4_sealing();
            c4.sealed(Ok(()));
            c4
        };
        let mut continued = sealed();
        let mut try_again = sealed();
        try_again.start_busy();
        assert!(!try_again.answers_compared(), "unheld under IdStartBusy");
        assert_eq!(try_again.primary(), [Effect::StartService]);
        let mut retried = sealed();
        retried.start_busy();
        assert!(retried.showing_started(), "control: a return's retry");
        for (place, c4) in [
            ("Continue", &mut continued),
            ("IdStartBusy's Try again", &mut try_again),
            ("a return's retry", &mut retried),
        ] {
            assert!(c4.answers_compared(), "{place}");
            assert_eq!(c4.unchecked_refused(UncheckedRefused::OtherIdentity), []);
            assert_eq!(c4.notice(), Some(&Notice::StorageUnknown), "{place}");
            assert!(!c4.start_held(), "{place}");
            assert!(!c4.offers_unchecked(), "{place}: Try again only");
            // Not offered later on this C4 either: the mismatch withdrew it.
            c4.primary();
            c4.sealed(Err(StepFailed::StorageUnknown));
            assert!(!c4.offers_unchecked(), "{place}: withdrawn for C4's life");
        }

        // No C4 to answer on, or no PeerId to read back.
        for (place, flow) in held_starts() {
            if place == "C4 after a seal" {
                continue;
            }
            assert!(!flow.answers_compared(), "{place}");
        }
        let mut unheld = c4_sealing();
        unheld.sealed(Err(StepFailed::StorageUnknown));
        assert!(!unheld.answers_compared(), "an unheld C4");
        assert_eq!(
            unheld.unchecked_refused(UncheckedRefused::OtherIdentity),
            []
        );
        assert!(unheld.offers_unchecked(), "nothing changed");
    }

    /// R4's Continue after K1 ends the comparison with C4's `PeerId`, at
    /// that press (architect-cto's ruling on Q17): the start it holds R4
    /// for, and a Try again after it, say so. Nothing before the press
    /// does -- K1 showing, R2 open, R2's words entered then Back, a
    /// cancelled restore prompt, R4 shown but not pressed, K1 left with no
    /// restore -- nor `IdRestoreOther`'s Continue on R2 after K1, nor R4's
    /// Continue after a fresh install's restore. Mutations:
    /// `start_ends_comparison` reading no `held`, or no `after_k1`, or
    /// matching R2's held `IdRestoreOther` start, or `restored` setting
    /// `after_k1` for R3's restore, fail here.
    #[test]
    fn only_r4s_continue_after_k1_ends_the_comparison() {
        let k1 = || {
            IdentityFlow::new(
                LIST,
                interweave_profile_identity::FORMAT,
                Start::Recovery {
                    reason: RecoveryReason::Key,
                    expected: Some(mine()),
                    new_keys: false,
                },
            )
        };
        let mut flow = k1();
        assert!(!flow.start_ends_comparison(), "K1 showing");
        flow.primary();
        assert_eq!(flow.screen(), ScreenId::R2, "control: R2");
        assert!(!flow.start_ends_comparison(), "R2 open");
        enter(&mut flow, "acid");
        flow.back();
        flow.discard(true);
        assert_eq!(flow.screen(), ScreenId::K1, "control: back on K1");
        assert!(!flow.start_ends_comparison(), "words entered, then Back");
        assert_eq!(flow.back(), [Effect::Finish], "control: K1 left");
        assert!(!flow.start_ends_comparison(), "K1 left with no restore");

        let mut cancelled = r2_after_k1_restoring();
        cancelled.cancelled_step();
        assert_eq!(cancelled.screen(), ScreenId::R2, "control: kept");
        assert!(!cancelled.start_ends_comparison(), "a cancelled prompt");

        let mut other_one = r2_after_k1_restoring();
        other_one.restored(Err(StepFailed::RestoreOther(other())));
        assert_eq!(
            other_one.notice_action(),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        assert!(other_one.start_held(), "control: IdRestoreOther's start");
        assert!(!other_one.start_ends_comparison(), "IdRestoreOther");

        let mut r4 = r2_after_k1_restoring();
        r4.restored(Ok(()));
        assert_eq!(r4.screen(), ScreenId::R4, "control: at R4");
        assert!(!r4.start_ends_comparison(), "R4 shown, not pressed");
        assert_eq!(
            r4.primary(),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        assert!(r4.start_ends_comparison(), "R4's Continue after K1");
        r4.start_busy();
        assert!(!r4.start_ends_comparison(), "unheld under IdStartBusy");
        assert_eq!(r4.primary(), [Effect::StartService], "its Try again");
        assert!(r4.start_ends_comparison(), "ended already; says so again");

        let mut fresh = at_r3();
        fresh.primary();
        fresh.restored(Ok(()));
        assert_eq!(fresh.screen(), ScreenId::R4, "control: fresh R4");
        fresh.primary();
        assert!(fresh.start_held(), "control: held");
        assert!(!fresh.start_ends_comparison(), "not after K1");
    }

    /// After a mismatch the control is not offered again while C4 is open,
    /// whatever Try again answers: its read-back fails again, and the
    /// start would meet the same comparison (design section 6). Mutation:
    /// `sealed` setting the offer back on a failed read-back whatever it
    /// was fails here.
    #[test]
    fn a_mismatch_withdraws_start_unchecked_for_c4s_life() {
        let mut c4 = c4_unchecked_offered();
        c4.start_unchecked();
        c4.unchecked_refused(UncheckedRefused::OtherIdentity);
        for _ in 0..2 {
            assert_eq!(c4.primary(), [Effect::ReadBack(mine())], "Try again");
            assert!(!c4.offers_unchecked(), "not while it runs");
            c4.sealed(Err(StepFailed::StorageUnknown));
            assert_eq!(c4.notice(), Some(&Notice::StorageUnknown));
            assert!(!c4.offers_unchecked(), "nor after it failed again");
        }
    }

    /// While `IdStorageUnknown` shows, R2's `IdCheckWords` and R3's
    /// `IdR3Yes` -- the primary -- do what its Try again does: read back,
    /// with no check and no restore, the screen held while it runs; found,
    /// R4 opens (design section 6, the read-back row; acceptance 12). Once
    /// a word is edited the notice is gone and the primary checks again.
    /// Mutation: the `StorageUnknown` delegation at the top of `primary`
    /// removed (R2 checks, R3 restores again) fails here.
    #[test]
    fn storage_unknowns_primary_on_r2_and_r3_reads_back_and_writes_nothing() {
        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::StorageUnknown));
        assert_eq!(r2.primary(), [Effect::ReadBack(mine())]);
        assert!(r2.waiting(), "held while it reads back");
        assert_eq!(r2.primary(), [], "one read-back at a time");
        assert_eq!(r2.restored(Err(StepFailed::StorageUnknown)), []);
        assert_eq!(r2.notice(), Some(&Notice::StorageUnknown), "shown again");
        assert_eq!(r2.primary(), [Effect::ReadBack(mine())], "and again");
        r2.restored(Ok(()));
        assert_eq!(r2.screen(), ScreenId::R4, "found: R4");

        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::StorageUnknown));
        r2.edit(0);
        assert_eq!(r2.notice(), None, "control: an edit takes the notice");

        let mut r3 = at_r3();
        assert!(matches!(
            r3.primary().as_slice(),
            [Effect::RestoreConfirmed(..)]
        ));
        r3.restored(Err(StepFailed::StorageUnknown));
        assert_eq!(r3.primary(), [Effect::ReadBack(mine())]);
        assert!(r3.waiting(), "held while it reads back");
        assert_eq!(r3.primary(), [], "one read-back at a time");
        r3.restored(Ok(()));
        assert_eq!(r3.screen(), ScreenId::R4, "found: R4");
    }

    /// On R2 after K1, an edit that takes `IdStorageUnknown` away does not
    /// end the read-back: until a read-back opens the record to that
    /// `PeerId` or R2 closes, a check whose words derive the `PeerId` the
    /// refused restore confirmed reads
    /// back instead of restoring, while words of another `PeerId` are
    /// checked as any words (design section 6, the read-back row;
    /// acceptance 12). A new R2, after Back to K1, restores again.
    /// Mutations: the `unknown` arm of `checked` removed (a second
    /// restore), and `unknown` not set where `restored` raises the notice,
    /// each fail here.
    #[test]
    fn an_edit_after_storage_unknown_keeps_the_read_back() {
        // As acceptance 12 runs it: Try again first, its read-back
        // failing, then the edit; the read-back after the edit answered
        // failing (the notice again, raised so it takes focus) and, in a
        // second run, opening (R4).
        for opens in [false, true] {
            let mut r2 = r2_after_k1_restoring();
            r2.restored(Err(StepFailed::StorageUnknown));
            assert_eq!(r2.notice_action(), [Effect::ReadBack(mine())]);
            assert_eq!(r2.restored(Err(StepFailed::StorageUnknown)), []);
            assert_eq!(r2.notice(), Some(&Notice::StorageUnknown));
            r2.edit(0);
            assert_eq!(r2.notice(), None, "the edit takes the notice");
            assert!(matches!(r2.primary().as_slice(), [Effect::Check(_)]));
            assert_eq!(
                r2.checked(Some(mine())),
                [Effect::ReadBack(mine())],
                "one more read-back, no restore"
            );
            assert!(r2.waiting(), "held while it reads back");
            let raised = r2.notices_raised();
            if opens {
                assert_eq!(r2.restored(Ok(())), []);
                assert_eq!(r2.screen(), ScreenId::R4, "opening: R4");
            } else {
                assert_eq!(r2.restored(Err(StepFailed::StorageUnknown)), []);
                assert_eq!(r2.notice(), Some(&Notice::StorageUnknown));
                assert_eq!(r2.notices_raised(), raised + 1, "raised: focus");
            }
        }

        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::StorageUnknown));
        r2.edit(0);
        r2.primary();
        assert_eq!(r2.checked(Some(other())), []);
        assert!(
            matches!(r2.notice(), Some(Notice::OtherIdentity { .. })),
            "another PeerId's words are checked as any words"
        );

        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::Storage));
        r2.edit(0);
        r2.primary();
        assert_eq!(
            r2.checked(Some(mine())),
            [Effect::RestoreRecorded(words().join(" "))],
            "control: after a refusal that wrote nothing, a restore"
        );
    }

    /// A read-back refused because authentication is needed -- the one
    /// after a refused write, or `IdStorageUnknown`'s Try again of it --
    /// asks for the prompt and keeps the screen held, its notice as it was
    /// (design section 6, Authentication needed). Granted, the root reads
    /// back (`Effect::ReadBack`, the gate's resume) and the answer is the
    /// step's: found, the screen goes on. Cancelled, denied or dropped,
    /// `IdStorageUnknown` shows, raised so it takes focus, and C4's
    /// primary is Try again -- a read-back, never a second seal -- while a
    /// seal's own prompt cancelled leaves C4 as it was. On C4, R2 after K1
    /// and R3 alike (acceptance 12). Mutations: `sealed`'s or `restored`'s
    /// `ReadBackNeedsAuthentication` arm removed (the notice shows at once,
    /// nothing prompts), `step_not_run`'s `ReadBack` arm calling
    /// `cancelled_step` (C4 back to Continue, which seals), or not counting
    /// the notice raised, fail here.
    #[test]
    fn a_read_backs_prompt_not_answered_shows_storage_unknown() {
        // C4: the seal refused after the publish, its read-back refused
        // for the person.
        let mut c4 = c4_sealing();
        assert_eq!(
            c4.sealed(Err(StepFailed::ReadBackNeedsAuthentication)),
            [Effect::AuthenticateReadBack(mine())]
        );
        assert!(c4.start_held(), "held under the prompt");
        assert_eq!(c4.notice(), None, "no notice while it asks");
        let raised = c4.notices_raised();
        c4.step_not_run(&Effect::ReadBack(mine()));
        assert_eq!(c4.notice(), Some(&Notice::StorageUnknown));
        assert_eq!(c4.notices_raised(), raised + 1, "raised: focus on it");
        assert!(!c4.start_held(), "C4 given back");
        assert_eq!(
            c4.primary(),
            [Effect::ReadBack(mine())],
            "Try again reads back, not Continue's seal"
        );
        // The Try again's read-back refused for the person: asked again,
        // granted, found.
        assert_eq!(
            c4.sealed(Err(StepFailed::ReadBackNeedsAuthentication)),
            [Effect::AuthenticateReadBack(mine())]
        );
        assert!(c4.start_held());
        assert_eq!(
            c4.sealed(Ok(())),
            [Effect::VoidPaper(false), Effect::StartService],
            "granted and found: C4 goes on"
        );

        // Control: the seal's own prompt cancelled leaves C4 as it was.
        let mut c4 = c4_sealing();
        c4.step_not_run(&Effect::Seal);
        assert_eq!(c4.notice(), None);
        assert_eq!(c4.primary(), [Effect::Seal], "Continue seals");

        // R2 after K1.
        let mut r2 = r2_after_k1_restoring();
        assert_eq!(
            r2.restored(Err(StepFailed::ReadBackNeedsAuthentication)),
            [Effect::AuthenticateReadBack(mine())]
        );
        assert!(r2.waiting(), "held under the prompt");
        assert_eq!(r2.notice(), None);
        let raised = r2.notices_raised();
        r2.step_not_run(&Effect::ReadBack(mine()));
        assert_eq!(r2.notice(), Some(&Notice::StorageUnknown));
        assert_eq!(r2.notices_raised(), raised + 1);
        assert!(!r2.waiting());
        assert_eq!(r2.primary(), [Effect::ReadBack(mine())]);
        assert_eq!(
            r2.restored(Err(StepFailed::ReadBackNeedsAuthentication)),
            [Effect::AuthenticateReadBack(mine())]
        );
        r2.restored(Ok(()));
        assert_eq!(r2.screen(), ScreenId::R4, "granted and found");

        // R3.
        let mut r3 = at_r3();
        r3.primary();
        assert_eq!(
            r3.restored(Err(StepFailed::ReadBackNeedsAuthentication)),
            [Effect::AuthenticateReadBack(mine())]
        );
        assert!(r3.waiting(), "held under the prompt");
        let raised = r3.notices_raised();
        r3.step_not_run(&Effect::ReadBack(mine()));
        assert_eq!(r3.notice(), Some(&Notice::StorageUnknown));
        assert_eq!(r3.notices_raised(), raised + 1);
        assert!(!r3.waiting());
        assert_eq!(r3.notice_action(), [Effect::ReadBack(mine())]);
        r3.step_not_run(&Effect::ReadBack(mine()));
        assert_eq!(
            r3.notice(),
            Some(&Notice::StorageUnknown),
            "a Try again's prompt cancelled: the notice again"
        );
        assert_eq!(r3.notices_raised(), raised + 2);
    }

    /// The action under R2's and R3's notices (section 6): Try again
    /// under `IdBusy` and `IdStorageFailed` runs the step again -- R2's
    /// check, R3's restore --; under `IdRestoreOther`, Continue starts the
    /// service with the record's identity, writes nothing, and holds the
    /// screen as a held start (`start_held`) until it decides: no second
    /// start, no word changed, Back does nothing. The checksum banner and
    /// `IdOtherIdentity` have no action. Mutation: `notice_action`'s
    /// `RestoreOther` arm setting no `started`, or not `checking` /
    /// `restoring`, fails here.
    #[test]
    fn notice_actions_retry_read_back_or_open_the_record() {
        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::Storage));
        assert_eq!(r2.notice_action(), [Effect::Check(words().join(" "))]);

        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::RestoreOther(other())));
        assert_eq!(
            r2.notice(),
            Some(&Notice::RestoreOther { recorded: other() })
        );
        assert!(!r2.start_held(), "control: nothing held yet");
        assert_eq!(
            r2.notice_action(),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        assert!(r2.start_held() && r2.waiting(), "held until it decides");
        assert_eq!(r2.notice_action(), [], "no second start");
        assert_eq!(r2.primary(), [], "no check either");
        assert_eq!(r2.back(), [], "Back does nothing");
        assert!(!r2.press('a'), "no letter reaches the words");
        r2.edit(3);
        assert_eq!(r2.positions_chosen(), 0, "no position opens");
        r2.set_starting(true);
        assert!(r2.starting(), "ConnectStarting under that Continue");
        assert_eq!(r2.decided(), [Effect::OpenChats]);
        assert_eq!(r2.screen(), ScreenId::Done);

        let mut r3 = at_r3();
        r3.primary();
        r3.restored(Err(StepFailed::Busy));
        assert!(matches!(
            r3.notice_action().as_slice(),
            [Effect::RestoreConfirmed(..)]
        ));
        r3.restored(Err(StepFailed::RestoreOther(other())));
        assert_eq!(
            r3.notice_action(),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        assert!(r3.start_held() && r3.waiting());
        assert_eq!(r3.primary(), [], "Yes held");
        r3.recheck();
        assert_eq!(r3.back(), [], "Back does nothing");
        assert_eq!(r3.screen(), ScreenId::R3, "No and Back held");
        assert_eq!(r3.decided(), [Effect::OpenChats]);

        // Back from the notice, before Continue: as at any other time
        // (section 6, Back from a notice).
        let mut r3 = at_r3();
        r3.primary();
        r3.restored(Err(StepFailed::RestoreOther(other())));
        assert_eq!(r3.back(), []);
        assert_eq!(r3.screen(), ScreenId::R2, "R3's Back: R2, words kept");
        assert!(r3.entry().is_some_and(PhraseEntry::complete));
        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::RestoreOther(other())));
        r2.back();
        assert!(r2.discard_asked(), "R2's Back: the dialog first");
        assert_eq!(r2.notice_action(), [], "nothing behind the dialog");

        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::Busy));
        r2.primary();
        r2.checked(None);
        assert_eq!(r2.notice(), Some(&Notice::ChecksumFailed));
        assert_eq!(r2.notice_action(), [], "the checksum banner has none");
    }

    #[test]
    fn debug_output_carries_no_word() {
        let flow = created_at_c2();
        let printed = format!("{flow:?}");
        assert!(LIST.iter().all(|word| !printed.contains(word)), "{printed}");
    }

    /// The five places of a held service start that `IdStartBusy`
    /// answers (design section 6, the row for a service start refused
    /// with the profile in use; acceptance 15): each held with its start
    /// asked, by the press that asked it.
    fn held_starts() -> Vec<(&'static str, IdentityFlow)> {
        let mut sealed = c4_sealing();
        assert_eq!(
            sealed.sealed(Ok(())),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        let mut already = c4_sealing();
        already.sealed(Err(StepFailed::AlreadyProvisioned));
        assert_eq!(
            already.primary(),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        let mut r4 = at_r3();
        r4.primary();
        r4.restored(Ok(()));
        assert_eq!(r4.screen(), ScreenId::R4, "control: at R4");
        assert_eq!(
            r4.primary(),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        let mut r3 = at_r3();
        r3.primary();
        r3.restored(Err(StepFailed::RestoreOther(other())));
        assert_eq!(
            r3.notice_action(),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        let mut r2 = r2_after_k1_restoring();
        r2.restored(Err(StepFailed::RestoreOther(other())));
        assert_eq!(
            r2.notice_action(),
            [Effect::VoidPaper(false), Effect::StartService]
        );
        let places = vec![
            ("C4 after a seal", sealed),
            ("C4 under IdAlreadyCreated", already),
            ("R4", r4),
            ("R3", r3),
            ("R2 after K1", r2),
        ];
        for (place, flow) in &places {
            assert!(flow.start_held(), "control: {place} held");
        }
        places
    }

    /// What `IdStartBusy` keeps on each place: the `PeerId`
    /// `IdRestoreOther` showed on R2 and R3, and on C4 under
    /// `IdAlreadyCreated` that notice's title and hidden `PeerId`.
    fn busy_on(place: &str) -> Notice {
        match place {
            "R3" | "R2 after K1" => Notice::StartBusy {
                recorded: Some(other()),
                already: false,
            },
            "C4 under IdAlreadyCreated" => Notice::StartBusy {
                recorded: None,
                already: true,
            },
            _ => Notice::StartBusy {
                recorded: None,
                already: false,
            },
        }
    }

    /// Try again in the primary's place, as the view routes it: C4's and
    /// R4's primary, R2's and R3's notice action.
    fn try_again(place: &str, flow: &mut IdentityFlow) -> Vec<Effect> {
        match place {
            "R3" | "R2 after K1" => flow.notice_action(),
            _ => flow.primary(),
        }
    }

    /// A held start refused with the profile in use before the bound
    /// (design section 6; acceptance 15, the first block): on each of the
    /// five places the hold ends, `ConnectStarting` goes, and
    /// `IdStartBusy` shows, raised so it takes focus, with no other
    /// notice. C4 after a seal and R4 keep their `PeerId`; C4 under
    /// `IdAlreadyCreated` keeps that notice's title and hidden `PeerId`;
    /// R2 and R3 keep the `PeerId` `IdRestoreOther` showed. A screen that
    /// is not held is left as it was, and the refused start's bound
    /// decides nothing later: nothing retries on its own.
    ///
    /// Mutations: `start_busy` leaving the screen held, raising no notice
    /// count, or dropping `recorded` or `already`, fails here; so does
    /// `sealed(Ok)` dropping the `PeerId` again (`drop_created`).
    #[test]
    fn a_start_refused_busy_shows_start_busy_on_each_held_screen() {
        for (place, mut flow) in held_starts() {
            flow.set_starting(true);
            assert!(flow.starting(), "control: {place} ConnectStarting");
            let raised = flow.notices_raised();
            flow.start_busy();
            assert!(!flow.start_held(), "{place}: the hold ends");
            assert!(!flow.starting(), "{place}: no progress under Try again");
            assert!(!flow.waiting(), "{place}: Back as at any other time");
            assert_eq!(flow.notice(), Some(&busy_on(place)), "{place}");
            assert_eq!(flow.notices_raised(), raised + 1, "{place}: focus");
            match place {
                "C4 after a seal" | "R4" => {
                    assert_eq!(flow.peer(), Some(&mine()), "{place}: PeerId");
                }
                "C4 under IdAlreadyCreated" => {
                    assert_eq!(flow.peer(), None, "{place}: PeerId hidden");
                }
                _ => {}
            }
            // Not held: a second answer, or the bound, changes nothing.
            flow.start_busy();
            assert_eq!(flow.notices_raised(), raised + 1, "{place}: once");
            assert_eq!(flow.decided(), [], "{place}: nothing opens Chats");
            flow.set_starting(true);
            assert!(!flow.starting(), "{place}: nothing under Try again");
        }
    }

    /// `IdStartBusy`'s Try again (design section 6, section 8;
    /// acceptance 15): one service start and nothing else -- no seal, no
    /// restore, no read-back, no void-paper marker -- pressed as the
    /// Continue it replaces: the screen holds from the press, a second
    /// press starts nothing, Back does nothing, R2's words and R3's No
    /// are held, and the notice stays until the result. Answered
    /// `LockHeld` again, the notice shows again, focused; answered
    /// otherwise, the start decides as any held start does.
    ///
    /// Mutations: the C4/R4 `StartBusy` arm of `primary` removed (C4
    /// seals again, R4 marks the void paper again), or `notice_action`'s
    /// `StartBusy` arms removed (R2 and R3 start nothing), or either
    /// setting no hold, fails here.
    #[test]
    fn start_busys_try_again_starts_the_service_and_nothing_else() {
        for (place, mut flow) in held_starts() {
            flow.start_busy();
            assert_eq!(
                try_again(place, &mut flow),
                [Effect::StartService],
                "{place}: one start, nothing else"
            );
            assert!(flow.start_held(), "{place}: held from the press");
            assert_eq!(try_again(place, &mut flow), [], "{place}: one start");
            assert_eq!(flow.primary(), [], "{place}: no primary either");
            assert_eq!(flow.back(), [], "{place}: Back does nothing");
            assert_eq!(flow.notice(), Some(&busy_on(place)), "{place}: stays");
            match place {
                "R2 after K1" => {
                    assert!(!flow.press('a'), "R2: no letter reaches the words");
                    assert!(flow.waiting(), "R2: the back arrow inactive");
                }
                "R3" => {
                    flow.recheck();
                    assert_eq!(flow.screen(), ScreenId::R3, "R3: No held");
                    assert!(flow.waiting(), "R3: held");
                }
                _ => {}
            }
            flow.set_starting(true);
            assert!(flow.starting(), "{place}: ConnectStarting from the press");

            // `LockHeld` again: the notice again, focused.
            let raised = flow.notices_raised();
            flow.start_busy();
            assert_eq!(flow.notice(), Some(&busy_on(place)), "{place}: again");
            assert_eq!(flow.notices_raised(), raised + 1, "{place}: focus");
            assert_eq!(try_again(place, &mut flow), [Effect::StartService]);
            assert_eq!(flow.decided(), [Effect::OpenChats], "{place}: decided");
        }
    }

    /// A showing of the window that starts the service again while
    /// `IdStartBusy` shows (unlock note section 11, L3; identity note
    /// section 6, A return or a recreation): on each of the five places
    /// the screen holds as Try again's press would hold it -- Try again
    /// inert, Back doing nothing, R2's words and R3's No held, the notice
    /// staying, `ConnectStarting` from the showing -- and the flow asks
    /// for nothing, since the Activity's own start is the one start: a
    /// press meanwhile starts nothing. The hold raises no notice, a second
    /// showing holds nothing again, and the start's success decides as
    /// any held start's does. A screen showing no `IdStartBusy`, or one
    /// already held, is left as it was.
    ///
    /// Mutations: `showing_started`'s C4/R4 arm setting no `held`, its R2
    /// arm no `checking`, its R3 arm no `restoring`, or any arm dropping
    /// its `@ false` guard, fails here.
    #[test]
    fn a_showings_start_holds_as_try_again_and_asks_nothing() {
        for (place, mut flow) in held_starts() {
            assert!(!flow.showing_started(), "{place}: held already");
            flow.start_busy();
            let raised = flow.notices_raised();
            assert!(flow.showing_started(), "{place}: held by the showing");
            assert!(flow.start_held(), "{place}: held");
            assert!(!flow.showing_started(), "{place}: once");
            assert_eq!(try_again(place, &mut flow), [], "{place}: no second start");
            assert_eq!(flow.primary(), [], "{place}: no primary either");
            assert_eq!(flow.back(), [], "{place}: Back does nothing");
            assert_eq!(flow.notice(), Some(&busy_on(place)), "{place}: stays");
            assert_eq!(flow.notices_raised(), raised, "{place}: nothing raised");
            match place {
                "R2 after K1" => {
                    assert!(!flow.press('a'), "R2: no letter reaches the words");
                    assert!(flow.waiting(), "R2: the back arrow inactive");
                }
                "R3" => {
                    flow.recheck();
                    assert_eq!(flow.screen(), ScreenId::R3, "R3: No held");
                    assert!(flow.waiting(), "R3: held");
                }
                _ => {}
            }
            flow.set_starting(true);
            assert!(flow.starting(), "{place}: ConnectStarting");
            assert_eq!(flow.decided(), [Effect::OpenChats], "{place}: decided");
            assert_eq!(flow.screen(), ScreenId::Done, "{place}: handed over");
        }
        // No `IdStartBusy`: a failed seal's Try again, R3 asked, R4 before
        // its Continue.
        let mut failed = c4_sealing();
        failed.sealed(Err(StepFailed::Busy));
        let mut r3 = at_r3();
        let mut r4 = at_r3();
        r4.primary();
        r4.restored(Ok(()));
        for (place, flow) in [("C4", &mut failed), ("R3", &mut r3), ("R4", &mut r4)] {
            assert!(!flow.showing_started(), "{place}: nothing to hold");
            assert!(!flow.start_held(), "{place}: not held");
        }
    }

    /// `start_shows` reads the screen now shown (identity note section 6,
    /// A return or a recreation): on each of the five places a start
    /// shows while it is held and while `IdStartBusy` shows, and none
    /// once the flow is off that screen -- R3's Back to R2, R2's discard
    /// off it, a decided start to Done -- nor on a screen that never asked
    /// one. So the Activity's check of a recreation after R3's Back off
    /// `IdStartBusy` restarts nothing under R2 or R1.
    ///
    /// Mutations: `start_shows` without its `start_held` (a held C4 shows
    /// none), without its `StartBusy` arms (C4 under `IdStartBusy` shows
    /// none), or answering a latch of the start once asked (R3's Back
    /// still shows one), fails here.
    #[test]
    fn a_start_shows_on_its_screen_and_goes_with_it() {
        for (place, mut flow) in held_starts() {
            assert!(flow.start_shows(), "{place}: held");
            flow.start_busy();
            assert!(flow.start_shows(), "{place}: IdStartBusy");
            match place {
                "R3" => {
                    flow.back();
                    assert_eq!(flow.screen(), ScreenId::R2, "control: R3's Back");
                    assert!(!flow.start_shows(), "R3: Back to R2");
                    // R2's Back with words entered asks the discard dialog.
                    flow.back();
                    flow.discard(true);
                    assert_eq!(flow.screen(), ScreenId::R1, "control: R2's discard");
                    assert!(!flow.start_shows(), "R3: on to R1");
                }
                "R2 after K1" => {
                    flow.back();
                    assert!(flow.start_shows(), "R2: the dialog over IdStartBusy");
                    flow.discard(true);
                    assert_eq!(flow.screen(), ScreenId::K1, "control: R2's discard");
                    assert!(!flow.start_shows(), "R2: on to K1");
                }
                _ => {
                    assert_eq!(try_again(place, &mut flow), [Effect::StartService]);
                    assert_eq!(flow.decided(), [Effect::OpenChats], "{place}");
                    assert!(!flow.start_shows(), "{place}: Done");
                }
            }
        }
        let mut failed = c4_sealing();
        failed.sealed(Err(StepFailed::Busy));
        let mut r4 = at_r3();
        r4.primary();
        r4.restored(Ok(()));
        for (place, flow) in [("C4", &failed), ("R3", &at_r3()), ("R4", &r4)] {
            assert!(!flow.start_shows(), "{place}: none asked");
        }
    }

    /// The showing's start refused with the profile in use again: as
    /// Try again's, the hold ends and `IdStartBusy` shows again with Try
    /// again enabled -- but it answers no press, so the notice is not
    /// raised and focus stays where it is. The next press's own refusal
    /// raises it as before.
    ///
    /// Mutations: `showing_started` leaving `held_by` at `Press`, or
    /// `start_busy` raising the count whatever `held_by` says, fails
    /// here; so does `start_busy` reading `held_by` without putting it
    /// back to `Press` (the press's refusal after it raises nothing).
    #[test]
    fn a_showings_start_refused_busy_again_raises_nothing() {
        for (place, mut flow) in held_starts() {
            flow.start_busy();
            let raised = flow.notices_raised();
            assert!(flow.showing_started(), "control: {place} held");
            flow.start_busy();
            assert!(!flow.start_held(), "{place}: the hold ends");
            assert_eq!(flow.notice(), Some(&busy_on(place)), "{place}: again");
            assert_eq!(flow.notices_raised(), raised, "{place}: not raised");
            assert_eq!(
                try_again(place, &mut flow),
                [Effect::StartService],
                "{place}: Try again enabled"
            );
            flow.start_busy();
            assert_eq!(
                flow.notices_raised(),
                raised + 1,
                "{place}: a press's raises"
            );
        }
    }

    /// A showing's start refused for the person, whose granted prompt
    /// starts it again: that start is the person's, so its `LockHeld`
    /// raises the notice as Try again's does.
    ///
    /// Mutation: `person_started` leaving `held_by` at `Showing` fails
    /// here.
    #[test]
    fn a_granted_prompt_over_a_showings_start_answers_the_person() {
        for (place, mut flow) in held_starts() {
            flow.start_busy();
            let raised = flow.notices_raised();
            assert!(flow.showing_started(), "control: {place} held");
            flow.person_started();
            assert!(flow.start_held(), "{place}: still held");
            flow.start_busy();
            assert_eq!(flow.notice(), Some(&busy_on(place)), "{place}: again");
            assert_eq!(flow.notices_raised(), raised + 1, "{place}: raised");
        }
    }

    /// R2's discard dialog open under `IdStartBusy` when a showing starts
    /// the service: it closes as its Continue entering would, every word
    /// kept, so nothing takes the person off the held screen while its
    /// start runs.
    ///
    /// Mutation: `showing_started`'s R2 arm leaving `discard_asked` set
    /// fails here.
    #[test]
    fn a_showings_start_closes_r2s_discard_dialog_keeping_the_words() {
        let (_, mut flow) = held_starts()
            .into_iter()
            .find(|(place, _)| *place == "R2 after K1")
            .expect("R2 is a place");
        flow.start_busy();
        assert_eq!(flow.back(), []);
        assert!(flow.discard_asked(), "control: the dialog open");
        let words = flow.entry().and_then(PhraseEntry::phrase);
        assert!(words.is_some(), "control: words entered");
        assert!(flow.showing_started());
        assert!(!flow.discard_asked(), "the dialog closes");
        flow.discard(true);
        assert_eq!(flow.screen(), ScreenId::R2, "nothing leaves R2");
        assert_eq!(flow.entry().and_then(PhraseEntry::phrase), words, "kept");
    }
}

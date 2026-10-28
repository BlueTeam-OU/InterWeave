// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Peers by address (ADR-0062; `design/2026-10-22-add-a-peer.md`): the
//! addresses this device holds, the add page's two steps, a removal and
//! its Undo, and the no-reach condition. Pure, as the trust settings are:
//! the root carries the intents out and feeds back what the runtime
//! answered.
//!
//! An address grants nothing. The add page's second step trusts a
//! `PeerId` through [`TrustSettings`], the block on screen -- the full
//! `PeerId` and what trust allows -- being the confirmation, so trust has
//! one way in. What the runtime judges (the grammar, the special-use
//! floor, the bound, a list the configuration turns off) is never judged
//! here: the model sends the lines and renders the refusal by name. The
//! client's own rules are judged before anything is sent: at most
//! [`CODE_LINES`] lines in one add, scanned or typed ([`TYPED_LINES`];
//! ADR-0062 decision 4), and no DNS name from a scan (decision 1).
//!
//! An add is all or none and is never split: one line is one write, and
//! several the call that checks them all first (IPC 2.9's
//! `admin.peers.add_addresses`), which a runtime below 2.9 does not
//! serve, so there an add of several is not made and one line is. So
//! every failure is
//! one of the note's §5.7 outcomes, and "nothing was changed" is said
//! only where it is true: not written; unknown, where the answer was lost
//! after the request left and the list is read back before anything is
//! said; not permitted; and a runtime too old for the methods.

use std::collections::BTreeMap;

use interweave_human_client_api::{
    AddressAnswer, AddressChangeFailure, AddressList, AddressOrigin, AddressOutcome,
    AddressProblem, AddressRefused, PeerAddress, ShowAddresses, TrustProblem,
};
use interweave_transport_api::TransportIdentity;

use crate::model::Intent;
use crate::trust::{Held, TrustOutcome, TrustSettings};

/// The most lines one add carries: a code's bound (ADR-0062 decision 4),
/// and the add method's.
pub const CODE_LINES: usize = 16;

/// The most lines a typed or pasted add carries: a code's, so one add
/// stays all or none (`design/2026-10-22-add-a-peer.md` §5.3). Named
/// apart from [`CODE_LINES`] because more is refused in the field's own
/// words, with the lines past it selected, before anything is sent.
pub const TYPED_LINES: usize = CODE_LINES;

/// The most addresses a device holds, configured and added together
/// (ADR-0062 decision 2): the `{max}` the at-the-bound words fill. The
/// runtime's bound is the rule; this is its value for the words, held
/// equal to the configuration's by `app-core`'s test.
pub const ADDRESS_BOUND: usize = 64;

/// How many rows Peers keeps the place of: every row a list can hold,
/// and as many dropped by reads (`places_never_pass_their_bound`).
const PLACES_BOUND: usize = 2 * ADDRESS_BOUND;

/// Why an add stored nothing, by a rule with its own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// Not an InterWeave address.
    Grammar,
    /// An address no other device can reach.
    SpecialUse,
    /// A DNS name in a scanned code (ADR-0062 decision 1).
    DnsFromScan,
    /// A code of more than [`CODE_LINES`] lines.
    TooMany,
    /// The device holds [`ADDRESS_BOUND`] addresses.
    Limit,
    /// The configuration turns the list off.
    ListOff,
    /// This device's own address: it reaches no one, and its row could
    /// never be trusted (add-a-peer §5.3).
    OwnAddress,
}

impl Refusal {
    /// Whether the refusal is of one line, which it names -- the device's
    /// bound included -- not a refusal of the whole add, too many lines or
    /// the list off, which names none (add-a-peer §5.3).
    #[must_use]
    pub const fn names_line(self) -> bool {
        // The device's bound names a line -- the first new one past it, as
        // the method's index does (§5.3) -- while too many lines and the
        // list off are of the whole add
        // (`each_runtime_refusal_is_named_and_stores_nothing`).
        !matches!(self, Self::TooMany | Self::ListOff)
    }
}

/// Where an add's lines came from: the client's DNS rule applies to a
/// scan, and only a typed line is selected in the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineSource {
    /// Typed or pasted into the field.
    Typed,
    /// Read from a code: the camera, an image file or a pasted image.
    Scanned,
}

/// The control a scan started from, which focus goes back to when it
/// ends with no add (§8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanControl {
    /// Scan a code: the camera (Android).
    Camera,
    /// Open an image…: an image file (desktop).
    OpenImage,
    /// Paste an image: the clipboard's image (desktop).
    PasteImage,
}

/// Why the administrative boundary cannot be used at all: no Try again,
/// since no retry mends either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Barred {
    /// This app may not administer the addresses here.
    NotPermitted,
    /// The runtime predates the methods (below IPC 2.7): a mixed install.
    Incompatible,
}

/// The one note on the add page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddNote {
    /// Add with nothing in the field.
    AddressNeeded,
    /// A scan control pressed with an address in the field.
    ScanAfterField,
    /// An image, opened or pasted, holds no code that reads.
    ImageNoCode,
    /// An image's code is not an address list.
    NotAddress,
    /// The file opened, or the clipboard's image, could not be read as
    /// an image: never "no code", since it was never looked at.
    ImageUnreadable,
    /// Paste an image found no image on the clipboard.
    ClipboardNoImage,
    /// Paste an image could not read the clipboard at all, so what it
    /// holds is not known: never "no image" nor an image unreadable
    /// (`a_scan_line_names_its_cause_and_goes_at_the_next_press_or_edit`).
    ClipboardUnreadable,
    /// The camera permission refused: type or paste instead.
    CameraDenied,
    /// The camera permission refused for good: Open settings to allow it.
    CameraDeniedSettings,
    /// The add stored nothing: the first refused line, and why.
    Refused {
        /// The line, as sent; named only when the refusal is of one line
        /// ([`Refusal::names_line`]).
        line: String,
        /// Why.
        why: Refusal,
    },
    /// Typed or pasted lines past [`TYPED_LINES`], refused whole before
    /// anything is sent.
    TooManyLines {
        /// How many lines the field holds.
        lines: usize,
    },
    /// Not written: nothing changed. Add again.
    Failed,
    /// The answer was lost and the list is being read back.
    Checking,
    /// The answer was lost and the list could not be read back. Add
    /// again is safe: an address already stored answers unchanged.
    Unknown {
        /// How many lines the add carried.
        lines: usize,
    },
    /// Refused `superseded` (IPC 2.10): another admin connection's later
    /// write of a line stands, and the list read back lacks a line or
    /// could not be read. No Try again, and nothing said of the list,
    /// since it is also said when the read failed (§5.7).
    Superseded {
        /// How many lines the add carried.
        lines: usize,
    },
    /// An add of several lines, typed or scanned, not taken: nothing
    /// added. A runtime with the one-address add only (IPC 2.7 or 2.8)
    /// answers so, the facade sending nothing to it.
    SeveralIncompatible {
        /// How many lines.
        lines: usize,
    },
    /// The boundary cannot be used.
    Barred(Barred),
}

impl AddNote {
    /// Whether the note is a scan's own line, under the scan controls:
    /// announced once with focus kept on the control, and gone at the
    /// next scan press, a press of Add (which finds the field empty, a
    /// scan's line standing only while it is, and says `AddressNeeded`
    /// in its place), an edit of the field or the page closing (§5.5).
    #[must_use]
    pub const fn is_scan_line(&self) -> bool {
        matches!(
            self,
            Self::ImageNoCode
                | Self::NotAddress
                | Self::ImageUnreadable
                | Self::ClipboardNoImage
                | Self::ClipboardUnreadable
                | Self::CameraDenied
                | Self::CameraDeniedSettings
        )
    }
}

/// What the add page's first step came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddResult {
    /// Lines added for these peers.
    Added {
        /// How many lines were answered added.
        lines: usize,
        /// The peers they reach, each once, in the lines' order.
        peers: Vec<TransportIdentity>,
    },
    /// Every line was already there.
    Unchanged {
        /// How many.
        lines: usize,
    },
    /// The answer was lost and the list read back holds every line: the
    /// device has them, which does not say this add made them.
    Checked {
        /// How many.
        lines: usize,
    },
}

/// Show my address, as the view draws it (§5.4): everything on it comes
/// from one read of this device's own addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowState {
    /// The read on its way: the warning only, nothing else drawn.
    Reading,
    /// The code and the same lines as text, at most [`CODE_LINES`] of
    /// them in the runtime's order, and the `PeerId` alone.
    Shown {
        /// This device's `PeerId`.
        peer: TransportIdentity,
        /// The lines the code carries and the text lists.
        lines: Vec<String>,
    },
    /// No address another device can reach: no code, and what to do.
    /// No reason is given: the read reports none, and the transport's
    /// own state is not the device's network (§5.4).
    NoAddress,
    /// The read failed: never "no address", which it has not shown.
    Unread(ShowProblem),
}

/// Why Show's read failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShowProblem {
    /// Anything a retry may mend: Try again.
    Failed,
    /// The runtime does not serve the read (below IPC 2.8): no Try again.
    Incompatible,
    /// This app may not read them: no Try again.
    NotPermitted,
}

/// One step-two block's state, as the view shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockState {
    /// Trust or Not now still to choose.
    Undecided,
    /// Trust on its way.
    Trusting,
    /// Trust's answer was lost: the trust list is being read back.
    Checking,
    /// The peer is trusted.
    Trusted,
    /// Not now.
    Later,
    /// Not now, while the trust list is not the daemon's word: Trust was
    /// not chosen, and no trust state is claimed (§5.3).
    LaterUnknown,
}

/// Why a block's Trust did not trust its peer: shown in the block, which
/// stays undecided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockProblem {
    /// Trust's own class, in its words.
    Trust(TrustProblem),
    /// The answer was lost, or none came, and the trust list read back
    /// does not list the peer: the trust was not made.
    NotMade,
    /// The answer was lost and the trust list could not be read back.
    Unknown,
    /// Refused `superseded` (IPC 2.10): another admin connection's later
    /// change of the peer may stand, and the trust list read again does
    /// not list the peer. Trust stays offered (§5.3).
    Superseded,
    /// Refused `superseded`, and the trust list could not be read again:
    /// no trust state claimed, and never [`Self::Unknown`], whose retry
    /// §5.7 rules out (§5.3).
    SupersededUnread,
}

/// Why a block's trust list is read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadBack {
    /// Its Trust's answer was lost, or none came.
    Lost,
    /// Its Trust was refused `superseded` (§5.3).
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrustBlock {
    peer: TransportIdentity,
    later: bool,
    /// Trust sent and not answered.
    trusting: bool,
    /// The trust list is read back for its Trust, and why: the answer
    /// lost, or refused `superseded`, which says its own lines.
    checking: Option<ReadBack>,
    problem: Option<BlockProblem>,
    /// Whether its peer is trusted, as the latest thing that told this
    /// device succeeded (§5.3): the trust list on the daemon's word when
    /// the block opened or since, `None` once this block's Trust is lost
    /// until its read back answers. Kept per block, so one block's lost
    /// Trust leaves another's known answer standing.
    known: Option<bool>,
}

impl TrustBlock {
    fn new(peer: TransportIdentity, trust: &TrustSettings) -> Self {
        let known = trust.confirmed_list().map(|l| l.allows(&peer));
        Self {
            peer,
            later: false,
            trusting: false,
            checking: None,
            problem: None,
            known,
        }
    }

    /// What is known of its peer's trust: the trust list while it is the
    /// daemon's word, else the block's own last known answer.
    fn known(&self, trust: &TrustSettings) -> Option<bool> {
        if self.checking.is_some() {
            return None;
        }
        trust
            .confirmed_list()
            .map(|l| l.allows(&self.peer))
            .or(self.known)
    }

    fn allowed(&self, trust: &TrustSettings) -> bool {
        self.known(trust) == Some(true)
    }

    fn state(&self, trust: &TrustSettings) -> BlockState {
        if self.allowed(trust) {
            BlockState::Trusted
        } else if self.checking.is_some() {
            BlockState::Checking
        } else if self.trusting {
            BlockState::Trusting
        } else if self.later {
            if self.known(trust).is_some() {
                BlockState::Later
            } else {
                BlockState::LaterUnknown
            }
        } else {
            BlockState::Undecided
        }
    }
}

/// One step-two block, as the view shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockView {
    /// The peer.
    pub peer: TransportIdentity,
    /// Its state.
    pub state: BlockState,
    /// Why its last Trust did not trust it.
    pub problem: Option<BlockProblem>,
}

/// One row of the Peers list, as the view shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerRow {
    /// The address, as the runtime holds it.
    pub address: String,
    /// The peer it reaches.
    pub peer: Option<TransportIdentity>,
    /// Configured, or added here: only the second offers Remove.
    pub origin: AddressOrigin,
    /// Its state in words.
    pub state: RowState,
}

/// A row's state (§5.2): what the device does with the address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowState {
    /// Its peer is trusted, so the device dials it -- never that the
    /// peer is reachable.
    Trusted,
    /// Its peer is not trusted, so the device does not dial it.
    NeedsTrust,
    /// The configuration turns the list off: nothing dials it.
    ProviderOff,
    /// The trust list is not the daemon's word (unread, refused, or a
    /// Trust not read back): neither Trusted nor not is claimed, and
    /// Trust stays offered (§5.2's trust unread).
    TrustUnknown,
    /// Its peer is this device: no trust state and no Trust, whatever
    /// the trust list says (§5.2).
    ThisDevice,
}

/// Why the rows show no trust state while the list is on show: in place
/// of the line on what trust means (§5.2's trust unread).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrustLine {
    /// The trust list could not be read, or a change left it unread;
    /// `retry` unless no retry can mend it (an incompatible read).
    Unread {
        /// Whether Try again is offered.
        retry: bool,
    },
    /// This app may not read the trust list here.
    NotPermitted,
}

/// Why the list is not on show (§5.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListProblem {
    /// The runtime cannot be reached: Try again.
    Unavailable,
    /// Anything else: Try again.
    Failed,
    /// The boundary cannot be used: no Try again.
    Barred(Barred),
}

/// The Peers page's snackbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeersSnack {
    /// An address was removed.
    Removed {
        /// The address, which Undo adds again.
        address: String,
        /// Its peer is still trusted: the line says so.
        trusted: bool,
        /// Undo is offered: not when the list is off, since the add
        /// would be refused.
        undo: bool,
    },
    /// A removal was not made: the row stays. Try again.
    RemoveFailed {
        /// The address.
        address: String,
    },
    /// A removal's answer was lost and the list could not be read back.
    /// Try again.
    RemoveUnknown {
        /// The address.
        address: String,
    },
    /// An Undo was refused by name: the address stays removed.
    UndoRefused(Refusal),
    /// An Undo was not made: the address stays removed. Try again.
    UndoFailed {
        /// The address.
        address: String,
    },
    /// An Undo's answer was lost and the list could not be read back.
    /// Try again.
    UndoUnknown {
        /// The address.
        address: String,
    },
    /// A removal refused `superseded` (IPC 2.10), the list read back
    /// still listing the row. No action: a retry could undo a change made
    /// on purpose elsewhere (§5.7).
    RemoveSuperseded {
        /// The address.
        address: String,
    },
    /// An Undo refused `superseded`, the list read back not listing the
    /// row. No action, as for a removal.
    UndoSuperseded {
        /// The address.
        address: String,
    },
    /// The add page closed with an address added.
    Added,
    /// The add page closed on an add that added nothing: every line
    /// was already there (§5.3).
    AddedUnchanged {
        /// How many lines the add carried.
        lines: usize,
    },
    /// The add page closed on an add read back as checked: the device
    /// has these addresses, which the read cannot show this add made.
    AddedChecked {
        /// How many lines the add carried.
        lines: usize,
    },
    /// The add page closed with every block trusted.
    PeerTrusted,
}

/// Which of the field's lines to select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineSelection {
    /// This line.
    Line(String),
    /// The line at this index among the field's lines, and every one
    /// after it.
    From(usize),
    /// Nothing: the caret at the field's end.
    End,
}

/// Who chose the focus asked for (`Peers::focus_by`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum FocusBy {
    /// Asked after the person's act on its target.
    #[default]
    Act,
    /// Placed by the app: the row after a removed one, a closed
    /// snackbar's row menu.
    App,
}

/// A row's own Remove or Undo a read decided without moving focus: the
/// view sends focus to `focus` only if focus is on `row` or a control of
/// it, and says neither row-gone text (§8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowReadBack {
    /// The row whose own change the read decided.
    pub row: String,
    /// Where focus goes from it, on what is drawn now.
    pub focus: PeersFocus,
}

/// The control of a step-two block holding focus, as the view finds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockHeld {
    /// Its Trust or its Not now.
    Choice,
    /// Its problem line.
    Problem,
}

/// Where focus goes, as the view is asked (§8 item 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeersFocus {
    /// Peers' title.
    Title,
    /// The add page's field, with lines to select in it.
    Field {
        /// What to select.
        select: Option<LineSelection>,
    },
    /// The add page's note: under the scan controls, or a refused
    /// scan's.
    Note,
    /// The add page's Add.
    AddButton,
    /// The scan control a scan started from.
    Scan(ScanControl),
    /// A step-two block's title.
    Block(TransportIdentity),
    /// A step-two block's problem line.
    BlockProblem(TransportIdentity),
    /// A step-two block's state line: its Trust unconfirmed (§8).
    BlockState(TransportIdentity),
    /// The step-two Done.
    Done,
    /// The control that opened the add page.
    Opener,
    /// A row of the list.
    Row(String),
    /// A row's menu button.
    RowMenu(String),
    /// Peers' Add an address.
    AddAddress,
    /// Peers' unreadable line, in place of the list.
    ListProblem,
    /// Try again on the trust-unread line.
    TrustRetry,
    /// The line under Add on the add page.
    AddNote,
    /// The trust-unread line itself, when it offers no Try again.
    TrustLine,
    /// The snackbar's action: Try again on a removal or Undo whose
    /// outcome is unknown (§8).
    SnackAction,
    /// The note under Add an address (at the bound, or list off).
    AddAddressNote,
    /// Show's warning, first on the page.
    ShowWarning,
    /// Show's problem line, in place of the code.
    ShowProblem,
    /// The control that opened Show.
    ShowOpener,
}

/// What a person did on Peers or its add page, as a view hands it to the
/// root ([`crate::ViewEvent::Peers`]); the root applies it with
/// [`crate::UiModel::peers_input`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeersInput {
    /// Peers opened: read the list and trust.
    Opened,
    /// Try again on an unread list.
    ReadAgain,
    /// Try again on the trust-unread line: the trust list read again.
    TrustReadAgain,
    /// Add an address pressed.
    OpenAdd,
    /// Connect's Add an address pressed (§5.1).
    OpenAddFromConnect,
    /// Connect's Trust pressed: step two for these peers, each one
    /// Connect's line holds as not trusted or not known (§5.1).
    TrustFromConnect(Vec<TransportIdentity>),
    /// The field now reads this.
    FieldChanged(String),
    /// Add pressed.
    Add,
    /// A scan control pressed: the scanner -- the viewfinder, the image
    /// dialog, or the clipboard's image -- opens only with the field
    /// empty ([`Peers::scan_open`]).
    ScanPressed(ScanControl),
    /// The scanner closed with no code: the viewfinder's Close, or the
    /// image dialog cancelled.
    ScannerClosed,
    /// The image opened or pasted holds no code that reads.
    ImageNoCode,
    /// The file opened, or the clipboard's image, could not be decoded.
    ImageUnreadable,
    /// Paste an image found no image on the clipboard.
    ClipboardNoImage,
    /// Paste an image could not read the clipboard at all: held by another
    /// program, not offered to the app, or the read failed (§5.5).
    ClipboardUnreadable,
    /// The camera permission was refused at Scan a code; `for_good` when
    /// the platform will not ask again, so only Settings can allow it.
    CameraDenied {
        /// Refused for good.
        for_good: bool,
    },
    /// Back from the app's system settings, which the denied-for-good
    /// line's Open settings opened.
    SettingsReturned,
    /// A code read, or an image decoded: its text. A view that must know
    /// whether the viewfinder stays open calls
    /// [`crate::UiModel::scanned`] instead.
    Scanned(String),
    /// Back or Escape on step one, Done on step two.
    Close,
    /// Trust on a step-two block.
    Trust(TransportIdentity),
    /// Trust on a row: the row's address, so the step it opens returns
    /// focus to that row though another row names the same peer (§8).
    TrustRow(String),
    /// Not now on a block.
    NotNow(TransportIdentity),
    /// Remove on an added row.
    Remove(String),
    /// The snackbar's Undo.
    Undo,
    /// The snackbar's Try again.
    TryAgain,
    /// The snackbar closed.
    SnackClosed {
        /// Focus was on its action as it went.
        from_action: bool,
    },
    /// Show my address pressed: Show opens and reads.
    OpenShow,
    /// Try again on Show's failed read.
    ShowTryAgain,
    /// Show closed: Back, or the app leaving the screen (§5.4).
    CloseShow,
}

/// What a scan's text came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanVerdict {
    /// Not an address list: the viewfinder stays open.
    NotAddress,
    /// Taken: added at once, or refused by name on the page.
    Taken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InFlight {
    /// The add page's add of these lines.
    Add {
        lines: Vec<String>,
        source: LineSource,
    },
    /// An Undo, or its Try again, of this removal.
    Undo(String),
    /// A removal, or its Try again, with the list as it stood when sent:
    /// its answer is told against that, whatever a read since did to the
    /// list -- a read failing meanwhile leaves none
    /// (`a_removal_answered_after_a_failed_read_is_told_against_its_own_list`).
    Remove {
        address: String,
        before: AddressList,
    },
}

/// Whose Try again a read on its way answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Again {
    /// Peers opening: its read failing leaves focus on the title (§8).
    Open,
    List,
    Trust,
}

/// A change whose answer was lost: what to decide once the list is read
/// back.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Check {
    Add(Vec<String>),
    Undo(String),
    Remove {
        address: String,
        before: AddressList,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AddPage {
    field: String,
    note: Option<AddNote>,
    result: Option<AddResult>,
    blocks: Vec<TrustBlock>,
    scanning: bool,
    /// The control the last scan started from.
    scan_from: Option<ScanControl>,
    /// Opened from this row's Trust, not from Add an address.
    from_row: Option<String>,
    /// Opened from Connect (§5.1): an add answered unchanged reads as
    /// checked, and what it came to is kept for Connect's line.
    from_connect: bool,
    /// The peers the add's lines carry, this device's own left out: what
    /// Connect counts as added once the page closes.
    add_peers: Vec<TransportIdentity>,
    /// The lines an add left unknown -- its answer lost and the read back
    /// failed: a retry of them that writes nothing leaves them so (§5.7).
    unknown: Option<Vec<String>>,
    /// Bumped at every note set, so the same note twice is still news.
    notes: u64,
}

/// The Peers page and its add page, as a view renders them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Peers {
    list: Option<AddressList>,
    list_problem: Option<ListProblem>,
    reading: bool,
    /// The read on its way is a Try again's: the list's, which once it
    /// succeeds sends focus as when Peers opens, or the trust line's.
    again: Option<Again>,
    reread: bool,
    /// A trust step an add opened, the trust list not the daemon's word:
    /// the step reads the set itself (§5.2, §5.3), as a row's does, once
    /// nothing else is asked ([`Self::take_trust_read`]).
    trust_read_owed: bool,
    /// Bumped when the trust line's Try again failed with the line still
    /// on show: said again, its words perhaps unchanged (§8).
    trust_line_said: u64,
    /// The lines of an add whose page Back closed while it was on its way:
    /// its answer is no page's to show, whatever page is open by then
    /// (`a_closed_pages_add_lands_on_no_page_opened_after_it`).
    orphan: Option<Vec<String>>,
    /// A change may have reached the runtime since the list was last
    /// read: its rows stay on show as last read (§5.7), but its
    /// emptiness is no fact until a read succeeds.
    held: Held,
    check: Option<Check>,
    page: Option<AddPage>,
    add_note: Option<Refusal>,
    in_flight: Option<InFlight>,
    snack: Option<PeersSnack>,
    snacks: u64,
    focus: Option<PeersFocus>,
    focuses: u64,
    /// Whether the focus asked for came after the person's act on the
    /// row it names or was placed by the app: a read dropping a placed
    /// one says no `PeerRowGoneAfterAct` (section 8's first row-gone row).
    focus_by: FocusBy,
    /// A row's own Remove or Undo decided by a read with no focus asked:
    /// where focus goes if it is on that row, else it stays (section 8),
    /// and a count that changes with each.
    read_back: Option<RowReadBack>,
    read_backs: u64,
    /// The rows in the order last drawn, a row a read dropped kept at its
    /// place: where focus goes for a row removed elsewhere (§8). At most
    /// [`PLACES_BOUND`] while a list holds no more, the row dropped
    /// longest ago going first.
    places: Vec<String>,
    /// Each dropped row in `places`, with the count of drops when it
    /// was: the order the bound evicts them in.
    dropped: BTreeMap<String, u64>,
    /// Rows dropped so far.
    drops: u64,
    /// Where focus went after the last removal, for an Undo refused.
    after_remove: Option<PeersFocus>,
    /// The last removal sent, with the list it was sent against: its
    /// Try again is told against that while no list is on show -- a read
    /// failing since leaves none
    /// (`a_removal_try_again_after_a_failed_read_is_told_against_its_removals_list`).
    removal_sent: Option<(String, AddressList)>,
    /// A row an Undo restored while its list was unread, focused once a
    /// read lists it.
    restored: Option<String>,
    /// The removal or Undo left unknown -- its answer lost, its read back
    /// failed -- as the check a read decides it by. It stays unknown until
    /// a read shows it or the runtime answers it (§5.7); the read speaks
    /// for it only where its line still holds its place
    /// ([`Self::read_decides`]).
    unknown_change: Option<Check>,
    /// Every removal and Undo left unknown and not yet decided, oldest
    /// first, one per address: each stays unknown until a read shows it,
    /// an answer for it decides it, or a list another answer brings shows
    /// the device as it is, whichever line is on show (§5.7),
    /// so a retry of an earlier one that writes nothing is still unknown,
    /// never "still in the list"
    /// (`every_removal_left_unknown_stays_unknown_until_decided`). At most
    /// [`PLACES_BOUND`], the oldest going first.
    pending: Vec<Check>,
    /// The focus requests counted when the unknown line was said: a
    /// request since is a later act's, whose focus a
    /// verdict on the earlier change never takes
    /// (`a_verdict_on_an_earlier_unknown_change_never_takes_a_later_ones_place`).
    unknown_mark: u64,
    /// This node's `PeerId`, from the session; else the trust list as
    /// last read names it.
    this_device: Option<TransportIdentity>,
    /// Show, while it is open.
    show: Option<ShowState>,
    /// Show's read is on its way, and whether it is a Try again's, which
    /// sends focus as when Show opens once it succeeds (§8).
    show_reading: Option<bool>,
    /// The `PeerId` whose addresses Show drew since this was last taken:
    /// Connect's `ConnectDoneShown` (§5.1).
    show_drew: Option<TransportIdentity>,
    /// What add pages opened from Connect came to, in the order they
    /// closed, since this was last taken ([`Self::take_connect_return`]):
    /// `Some` once one closed having done something §5.1 counts -- an
    /// add's answer, or a step two.
    connect_return: Option<Vec<ConnectAdd>>,
}

/// A peer an add opened from Connect added, and whether it is trusted
/// as the page closed (§5.1): `None` when that could not be known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectAdd {
    /// The peer.
    pub peer: TransportIdentity,
    /// Trusted, known not trusted, or not known.
    pub trusted: Option<bool>,
}

/// Whether a code's text is an address list at all -- every line, once
/// trimmed, a multiaddr -- before any rule judges the lines: a code that
/// is not leaves the viewfinder open with `ScanNotAddress` (§5.5).
#[must_use]
pub fn is_address_list(text: &str) -> bool {
    let lines = lines_of(text);
    !lines.is_empty() && lines.iter().all(|l| l.starts_with('/'))
}

/// The field's lines as the add sends them: each line ending (LF, CR LF
/// or CR) ends a line, each line trimmed of spaces and tabs, empty lines
/// dropped.
#[must_use]
pub fn lines_of(text: &str) -> Vec<String> {
    text.split(['\n', '\r'])
        .map(|line| line.trim_matches([' ', '\t']))
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether `line` names a host by DNS: a `dns`, `dns4`, `dns6` or
/// `dnsaddr` component anywhere in it. No value in the grammar is one of
/// those names, so a component that is one is the protocol.
fn names_dns(line: &str) -> bool {
    line.split('/')
        .any(|c| matches!(c, "dns" | "dns4" | "dns6" | "dnsaddr"))
}

/// The address a removal or Undo is of; an add has none.
fn pending_address(check: &Check) -> Option<&str> {
    match check {
        Check::Remove { address, .. } | Check::Undo(address) => Some(address),
        Check::Add(_) => None,
    }
}

/// The add page's refusal for the runtime's, or `None` for one that
/// names no rule of the address: a removal's own (`Configured`), which
/// an add never meets, and `Superseded`, which each change decides on
/// its list read back before it gets here ([`superseded`]).
const fn refusal_of(why: AddressRefused) -> Option<Refusal> {
    Some(match why {
        AddressRefused::Grammar => Refusal::Grammar,
        AddressRefused::SpecialUse => Refusal::SpecialUse,
        AddressRefused::Bound => Refusal::Limit,
        AddressRefused::ListOff => Refusal::ListOff,
        AddressRefused::OwnPeer => Refusal::OwnAddress,
        AddressRefused::Configured | AddressRefused::Superseded => return None,
    })
}

/// Whether an answer is the refusal `superseded` (IPC 2.10): nothing
/// written, and the list read back decides what is said, never the
/// not-written lines and never a Try again (§5.7).
const fn superseded(outcome: &AddressOutcome) -> bool {
    matches!(
        outcome,
        AddressOutcome::Refused(AddressRefused::Superseded)
            | AddressOutcome::RefusedLine {
                why: AddressRefused::Superseded,
                ..
            }
    )
}

/// Peers' unreadable state for a read that failed.
const fn list_problem(problem: AddressProblem) -> ListProblem {
    match problem {
        AddressProblem::Unavailable => ListProblem::Unavailable,
        AddressProblem::Internal => ListProblem::Failed,
        AddressProblem::NotPermitted => ListProblem::Barred(Barred::NotPermitted),
        AddressProblem::Incompatible => ListProblem::Barred(Barred::Incompatible),
    }
}

const fn barred(problem: AddressProblem) -> Option<Barred> {
    match problem {
        AddressProblem::NotPermitted => Some(Barred::NotPermitted),
        AddressProblem::Incompatible => Some(Barred::Incompatible),
        AddressProblem::Unavailable | AddressProblem::Internal => None,
    }
}

fn listed(list: &AddressList, address: &str) -> bool {
    list.addresses.iter().any(|a| a.address == address)
}

/// Whether the trust list, on the daemon's word, allows `peer`: whether
/// an add's peer needs no block. A list kept after a failed read is no
/// such word, so every peer of the add gets a block then; a block's own
/// state reads `TrustBlock::known`.
fn trust_allows(trust: &TrustSettings, peer: &TransportIdentity) -> bool {
    trust.confirmed_list().is_some_and(|l| l.allows(peer))
}

/// `list` in the order Peers shows it: the addresses added here newest
/// first, then the configuration's, in its order. Focus after a removal
/// walks this order, never the runtime's.
fn shown(list: &AddressList) -> impl Iterator<Item = &PeerAddress> {
    let added = list
        .addresses
        .iter()
        .filter(|a| a.origin == AddressOrigin::AddedHere)
        .rev();
    let configured = list
        .addresses
        .iter()
        .filter(|a| a.origin == AddressOrigin::Configured);
    added.chain(configured)
}

/// The first line naming this device's own `PeerId`, which one refuses
/// the whole add before anything is sent (add-a-peer §5.3).
fn own_line(lines: &[String], own: Option<&TransportIdentity>) -> Option<String> {
    let own = own?;
    lines
        .iter()
        .find(|l| PeerAddress::peer_of(l).as_ref() == Some(own))
        .cloned()
}

/// The peers `lines` reach, each once, in order.
fn peers_of(lines: &[String]) -> Vec<TransportIdentity> {
    let mut peers: Vec<TransportIdentity> = Vec::new();
    for peer in lines.iter().filter_map(|l| PeerAddress::peer_of(l)) {
        if !peers.contains(&peer) {
            peers.push(peer);
        }
    }
    peers
}

impl Peers {
    /// Apply what the person did; `trust` is the trust settings, through
    /// which step two's Trust passes.
    pub fn input(&mut self, input: PeersInput, trust: &mut TrustSettings) -> Vec<Intent> {
        match input {
            PeersInput::Opened => {
                let mut intents: Vec<Intent> = trust.opened().into_iter().collect();
                let read = self.read();
                if read.is_some() {
                    self.again = Some(Again::Open);
                }
                intents.extend(read);
                // The add page still held is what shows: focus goes back
                // where it last asked, so a block answered while Peers was
                // off screen says its line on return (§8).
                match (&self.page, self.focus.clone()) {
                    (Some(_), Some(focus)) => self.ask_focus(focus),
                    _ => self.ask_focus(PeersFocus::Title),
                }
                intents
            }
            PeersInput::TrustReadAgain => {
                let read = trust.opened();
                if read.is_some() {
                    self.again = Some(Again::Trust);
                }
                read.into_iter().collect()
            }
            // Unreadable Peers' Try again reads both lists (§5.2): the
            // trust list too while it is not the daemon's word.
            PeersInput::ReadAgain => {
                let read = self.read();
                self.again = read.is_some().then_some(Again::List);
                let mut intents: Vec<Intent> = read.into_iter().collect();
                if trust.confirmed_list().is_none() {
                    intents.extend(trust.opened());
                }
                intents
            }
            PeersInput::OpenAdd => {
                self.open_add();
                Vec::new()
            }
            PeersInput::OpenAddFromConnect => {
                // No bound or list-off note stops the page as Add an
                // address does on Peers (`add_blocked`): that note is drawn
                // on Peers' list, which is not on show under Connect. The
                // add's own answer decides, and its refusal is drawn on the
                // page (§5.3). Connect opens on a list read empty, so
                // whatever rows the list holds are this visit's adds.
                if self.page.is_none() {
                    self.add_note = None;
                    self.page = Some(AddPage {
                        from_connect: true,
                        ..AddPage::default()
                    });
                    self.ask_focus(PeersFocus::Field { select: None });
                }
                Vec::new()
            }
            PeersInput::TrustFromConnect(peers) => self.trust_from_connect(peers, trust),
            PeersInput::FieldChanged(text) => {
                self.field_changed(text);
                Vec::new()
            }
            PeersInput::Add => self.add_typed(trust).into_iter().collect(),
            PeersInput::ScanPressed(control) => {
                self.scan_pressed(control);
                Vec::new()
            }
            PeersInput::ScannerClosed => {
                self.scan_ended(None);
                Vec::new()
            }
            PeersInput::ImageNoCode => {
                self.scan_ended(Some(AddNote::ImageNoCode));
                Vec::new()
            }
            PeersInput::ImageUnreadable => {
                self.scan_ended(Some(AddNote::ImageUnreadable));
                Vec::new()
            }
            PeersInput::ClipboardNoImage => {
                self.scan_ended(Some(AddNote::ClipboardNoImage));
                Vec::new()
            }
            PeersInput::ClipboardUnreadable => {
                self.scan_ended(Some(AddNote::ClipboardUnreadable));
                Vec::new()
            }
            PeersInput::CameraDenied { for_good } => {
                self.camera_denied(for_good);
                Vec::new()
            }
            PeersInput::SettingsReturned => {
                // Focus back on Scan a code, its line kept until the next
                // press says what changed (§8).
                if self.page.is_some() {
                    self.ask_focus(PeersFocus::Scan(ScanControl::Camera));
                }
                Vec::new()
            }
            PeersInput::Scanned(text) => self.scanned(&text, trust).1.into_iter().collect(),
            PeersInput::Close => {
                self.close(trust);
                Vec::new()
            }
            PeersInput::OpenShow => self.open_show().into_iter().collect(),
            PeersInput::ShowTryAgain => self.show_again().into_iter().collect(),
            // A read on its way stays the one in flight: a page opened
            // again meanwhile is drawn from its answer.
            PeersInput::CloseShow => {
                if self.show.take().is_some() {
                    self.ask_focus(PeersFocus::ShowOpener);
                }
                Vec::new()
            }
            PeersInput::Trust(peer) => self.trust(&peer, trust).into_iter().collect(),
            PeersInput::TrustRow(address) => self.trust_row(&address, trust).into_iter().collect(),
            PeersInput::NotNow(peer) => {
                if let Some(block) = self.block_mut(&peer)
                    && !block.trusting
                    && block.checking.is_none()
                {
                    // A block whose Trust was pressed keeps its last line
                    // (§5.3): its problem text stays as it stands.
                    block.later = true;
                }
                self.settle(trust);
                Vec::new()
            }
            PeersInput::Remove(address) => self.remove(address).into_iter().collect(),
            PeersInput::Undo => self.undo().into_iter().collect(),
            PeersInput::TryAgain => self.try_again().into_iter().collect(),
            PeersInput::SnackClosed { from_action } => {
                // A removal unknown is read again once its snackbar goes,
                // so the row shows what the runtime holds; focus on its
                // Try again as it went waits on the row's menu button
                // while the read runs (section 8).
                if let Some(PeersSnack::RemoveUnknown { address }) = self.snack.clone() {
                    self.reread = true;
                    if from_action && self.list.as_ref().is_some_and(|l| listed(l, &address)) {
                        // Its own focus, not a later act's: the read still
                        // speaks for the removal.
                        let unmoved = self.focuses == self.unknown_mark;
                        self.place_focus(PeersFocus::RowMenu(address));
                        if unmoved {
                            self.unknown_mark = self.focuses;
                        }
                    }
                }
                self.snack = None;
                Vec::new()
            }
        }
    }

    /// Read the list, unless a read or a change is on its way: what the
    /// root asks at start, for the no-reach notice.
    pub fn read(&mut self) -> Option<Intent> {
        if self.reading || self.in_flight.is_some() {
            return None;
        }
        self.reading = true;
        Some(Intent::ReadAddresses)
    }

    /// Open the add page, or say why not: at the bound, or with the list
    /// off, the note under Add an address and no page.
    fn open_add(&mut self) {
        if self.page.is_some() {
            return;
        }
        if let Some(why) = self.add_blocked() {
            self.add_note = Some(why);
            self.ask_focus(PeersFocus::AddAddressNote);
            return;
        }
        self.add_note = None;
        self.page = Some(AddPage::default());
        self.ask_focus(PeersFocus::Field { select: None });
    }

    /// Why an add cannot be made, known from Peers' list alone. With none
    /// read, nothing is known: the add's own answer decides.
    fn add_blocked(&self) -> Option<Refusal> {
        let list = self.list.as_ref()?;
        if list.addresses.iter().any(|a| !a.dialled) {
            Some(Refusal::ListOff)
        } else if list.addresses.len() >= ADDRESS_BOUND {
            Some(Refusal::Limit)
        } else {
            None
        }
    }

    /// The field now reads `text`; a note about an empty or a full field
    /// goes once the field no longer is.
    fn field_changed(&mut self, text: String) {
        // Held while any change is on its way (`busy`). For the page's own
        // add or its check, the answer clears or selects in what was sent,
        // and an edit made meanwhile would be cleared unsent or leave no
        // line to select. For an add Back left out, a page opened since
        // waits as the page that sent it would: one add at a time, its Add
        // held too, the field read-only while an add is on its way (#396
        // §5.3), rather than taking a line its Add cannot send yet.
        if self.busy() {
            return;
        }
        let Some(page) = self.page.as_mut() else {
            return;
        };
        let lines = lines_of(&text);
        let empty = lines.is_empty();
        match page.note {
            Some(AddNote::AddressNeeded) if !empty => page.note = None,
            Some(AddNote::ScanAfterField) if empty => page.note = None,
            // A scan's own line goes at an edit of the field (§5.5).
            Some(ref note) if note.is_scan_line() && text != page.field => page.note = None,
            _ => {}
        }
        // An add left unknown speaks for the lines it sent: an edit away
        // from them is the person's own later act. Its note, about lines
        // the field no longer shows, goes with it, unannounced, focus kept
        // in the field, and editing back does not restore it, the claim on
        // the page having ended (ux-designer-01, REPLY 01a1694a). With the
        // note gone no read decides the earlier add over the page --
        // clearing the edit or moving it on (§5.7, "the field kept";
        // `read_decides` asks the note) -- and Peers' next read shows a
        // stored address as a row
        // (`an_unknown_add_decided_after_the_field_was_edited_keeps_the_edit`).
        // The lines are kept: a later Add of them that writes nothing is
        // still unknown, since the first write may have landed (§5.7;
        // `an_unknown_add_edited_away_and_back_is_still_unknown_on_a_retry`).
        if page.unknown.as_ref().is_some_and(|sent| *sent != lines)
            && matches!(page.note, Some(AddNote::Unknown { .. }))
        {
            page.note = None;
        }
        page.field = text;
    }

    /// Whether step one can act: the page open, nothing on its way, and
    /// no result yet.
    fn step_one(&self) -> bool {
        // A closed page's add still out holds the next page's Add too
        // (`busy`): its answer must not be taken for this one's.
        !self.busy() && self.step_one_shown()
    }

    /// Add pressed: the field's lines, or why not.
    fn add_typed(&mut self, trust: &TrustSettings) -> Option<Intent> {
        if !self.step_one() {
            return None;
        }
        let own = self.own(trust).cloned();
        let page = self.page.as_mut()?;
        let lines = lines_of(&page.field);
        // Each note set through page_note, so the view says it once
        // when it appears, focus moving or not (§5.3).
        if lines.is_empty() {
            self.page_note(AddNote::AddressNeeded, PeersFocus::Field { select: None });
            return None;
        }
        if lines.len() > TYPED_LINES {
            self.page_note(
                AddNote::TooManyLines { lines: lines.len() },
                PeersFocus::Field {
                    select: Some(LineSelection::From(TYPED_LINES)),
                },
            );
            return None;
        }
        if let Some(line) = own_line(&lines, own.as_ref()) {
            let focus = PeersFocus::Field {
                select: Some(LineSelection::Line(line.clone())),
            };
            self.page_note(
                AddNote::Refused {
                    line,
                    why: Refusal::OwnAddress,
                },
                focus,
            );
            return None;
        }
        page.note = None;
        Some(self.send(lines, LineSource::Typed))
    }

    /// A scan control pressed: the scanner opens with the field empty;
    /// otherwise the note says to add or clear it first.
    fn scan_pressed(&mut self, control: ScanControl) {
        if !self.step_one() {
            return;
        }
        let Some(page) = self.page.as_mut() else {
            return;
        };
        // A press while a scan is open starts no reader, so it changes
        // nothing: the open scan's result and focus stay with the control
        // that started it (§5.5, §8; #409 r3 F4). The desktop's file
        // dialog leaves the window live, so Paste can be pressed under it.
        if page.scanning {
            return;
        }
        page.scan_from = Some(control);
        // A scan's own line goes at the next scan press (§5.5).
        if page.note.as_ref().is_some_and(AddNote::is_scan_line) {
            page.note = None;
        }
        if lines_of(&page.field).is_empty() {
            page.scanning = true;
        } else {
            page.note = Some(AddNote::ScanAfterField);
            self.ask_focus(PeersFocus::Note);
        }
    }

    /// The camera permission refused: back to the add page, its line under
    /// the scan controls, focus on the field so typing is the next step
    /// (§5.5, §8).
    fn camera_denied(&mut self, for_good: bool) {
        let Some(page) = self.page.as_mut() else {
            return;
        };
        if !page.scanning {
            return;
        }
        page.scanning = false;
        let note = if for_good {
            AddNote::CameraDeniedSettings
        } else {
            AddNote::CameraDenied
        };
        self.page_note(note, PeersFocus::Field { select: None });
    }

    /// A scan that adds nothing ended: closed, or an image with no code
    /// or none that is an address list. Focus goes back to the control
    /// it started from, the note -- if any -- under the scan controls
    /// and read after them (§8).
    fn scan_ended(&mut self, note: Option<AddNote>) {
        let Some(page) = self.page.as_mut() else {
            return;
        };
        if !page.scanning {
            return;
        }
        page.scanning = false;
        let control = page.scan_from.unwrap_or(ScanControl::Camera);
        if let Some(note) = note {
            self.page_note(note, PeersFocus::Scan(control));
        } else {
            self.ask_focus(PeersFocus::Scan(control));
        }
    }

    fn open_show(&mut self) -> Option<Intent> {
        if self.show.is_some() {
            return None;
        }
        self.show = Some(ShowState::Reading);
        self.ask_focus(PeersFocus::ShowWarning);
        self.read_own(false)
    }

    fn show_again(&mut self) -> Option<Intent> {
        if self.show != Some(ShowState::Unread(ShowProblem::Failed)) {
            return None;
        }
        self.show = Some(ShowState::Reading);
        self.read_own(true)
    }

    fn read_own(&mut self, again: bool) -> Option<Intent> {
        if self.show_reading.is_some() {
            return None;
        }
        self.show_reading = Some(again);
        Some(Intent::ReadOwnAddresses)
    }

    /// The runtime's answer to Show's read. A page closed meanwhile shows
    /// nothing of it.
    pub fn own_answer(&mut self, answer: Result<ShowAddresses, AddressProblem>) {
        let Some(again) = self.show_reading.take() else {
            return;
        };
        if self.show.is_none() {
            return;
        }
        let state = match answer {
            Ok(own) if own.addresses.is_empty() => ShowState::NoAddress,
            Ok(own) => {
                self.show_drew = Some(own.peer.clone());
                ShowState::Shown {
                    peer: own.peer,
                    lines: own.addresses.into_iter().take(CODE_LINES).collect(),
                }
            }
            Err(AddressProblem::Incompatible) => ShowState::Unread(ShowProblem::Incompatible),
            Err(AddressProblem::NotPermitted) => ShowState::Unread(ShowProblem::NotPermitted),
            Err(AddressProblem::Unavailable | AddressProblem::Internal) => {
                ShowState::Unread(ShowProblem::Failed)
            }
        };
        let unread = matches!(state, ShowState::Unread(_));
        self.show = Some(state);
        if unread {
            self.ask_focus(PeersFocus::ShowProblem);
        } else if again {
            self.ask_focus(PeersFocus::ShowWarning);
        }
    }

    /// Show, while it is open.
    #[must_use]
    pub const fn show(&self) -> Option<&ShowState> {
        self.show.as_ref()
    }

    /// Whether Show drew this device's addresses since the last take:
    /// for Connect's line (§5.1).
    pub fn take_show_drew(&mut self) -> bool {
        self.show_drew.take().is_some()
    }

    /// A code read, or an image decoded: `text` is all it carries. Taken
    /// whole as one add, never through the field: the client's two rules
    /// first, then the runtime's.
    pub fn scanned(&mut self, text: &str, trust: &TrustSettings) -> (ScanVerdict, Option<Intent>) {
        let lines = lines_of(text);
        if !is_address_list(text) {
            // The viewfinder stays open on such a code; an image is
            // read once, so its scan ends with the reason (§5.5).
            let from_image = self.page.as_ref().is_some_and(|p| {
                p.scanning
                    && matches!(
                        p.scan_from,
                        Some(ScanControl::OpenImage | ScanControl::PasteImage)
                    )
            });
            if from_image {
                self.scan_ended(Some(AddNote::NotAddress));
            }
            return (ScanVerdict::NotAddress, None);
        }
        let ready = self.step_one();
        let own = self.own(trust).cloned();
        let Some(page) = self.page.as_mut() else {
            return (ScanVerdict::NotAddress, None);
        };
        // A code is taken only by the scan open now: a reader's result
        // that outlived its scan -- the page closed, or opened again, while
        // a file was read -- adds nothing (#409 C4).
        if !page.scanning {
            return (ScanVerdict::Taken, None);
        }
        page.scanning = false;
        if !ready || !lines_of(&page.field).is_empty() {
            return (ScanVerdict::Taken, None);
        }
        let refused = if lines.len() > CODE_LINES {
            Some((lines[CODE_LINES].clone(), Refusal::TooMany))
        } else {
            lines
                .iter()
                .find(|l| names_dns(l))
                .map(|l| (l.clone(), Refusal::DnsFromScan))
                .or_else(|| own_line(&lines, own.as_ref()).map(|l| (l, Refusal::OwnAddress)))
        };
        if let Some((line, why)) = refused {
            page.note = Some(AddNote::Refused { line, why });
            self.ask_focus(PeersFocus::Note);
            return (ScanVerdict::Taken, None);
        }
        page.note = None;
        let intent = self.send(lines, LineSource::Scanned);
        // The scanner gone, focus waits on Add, read as waiting (§5.3).
        self.ask_focus(PeersFocus::AddButton);
        (ScanVerdict::Taken, Some(intent))
    }

    fn send(&mut self, lines: Vec<String>, source: LineSource) -> Intent {
        self.in_flight = Some(InFlight::Add {
            lines: lines.clone(),
            source,
        });
        Intent::AddAddresses(lines)
    }

    fn block_mut(&mut self, peer: &TransportIdentity) -> Option<&mut TrustBlock> {
        self.page
            .as_mut()?
            .blocks
            .iter_mut()
            .find(|b| &b.peer == peer)
    }

    /// Trust on a block: proposed and confirmed at once, the block being
    /// the confirmation. A row's Trust is [`Self::trust_row`].
    fn trust(&mut self, peer: &TransportIdentity, trust: &mut TrustSettings) -> Option<Intent> {
        if let Some(block) = self.block_mut(peer) {
            if block.trusting || block.checking.is_some() || trust.in_flight().is_some() {
                return None;
            }
            let change = trust.propose_allow_of(peer)?;
            let intent = trust.confirm_in_step(&change)?;
            block.trusting = true;
            block.later = false;
            block.problem = None;
            // Its Trust and Not now give way to its waiting line, which
            // takes focus while the answer is awaited (§5.3, §8).
            self.ask_focus(PeersFocus::BlockState(peer.clone()));
            return Some(intent);
        }
        None
    }

    /// Trust on the row of `address`: the trust step for the peer that
    /// row names, which returns focus to that row.
    fn trust_row(&mut self, address: &str, trust: &mut TrustSettings) -> Option<Intent> {
        if self.page.is_some() {
            return None;
        }
        // Only a row that offers Trust as drawn now opens the step: one
        // needing trust, or whose trust is not known (§5.2, §5.3). A press
        // taken after a read showed its peer trusted -- or on this
        // device's own row, which offers none -- opens nothing, since a
        // block already decided would leave a page with nothing to press
        // (`a_rows_trust_taken_after_its_peer_is_trusted_opens_nothing`).
        let peer = self
            .rows(trust)
            .into_iter()
            .find(|r| r.address == address)
            .filter(|r| matches!(r.state, RowState::NeedsTrust | RowState::TrustUnknown))?
            .peer?;
        self.page = Some(AddPage {
            from_row: Some(address.to_owned()),
            blocks: vec![TrustBlock::new(peer.clone(), trust)],
            ..AddPage::default()
        });
        self.ask_focus(PeersFocus::Block(peer.clone()));
        // The trust step reads the set itself (§5.2) while the list is
        // not the daemon's word; its block takes Trust meanwhile, and a
        // read that lists the peer decides it Trusted.
        if trust.confirmed_list().is_none() {
            return trust.opened();
        }
        None
    }

    /// Back or Escape on step one discards what was typed; on step two it
    /// is Done, leaving any undecided block as Not now. While the page's
    /// add is on its way it closes the page at once, the add going on
    /// unshown (`back_on_an_add_on_its_way_closes_its_page_and_a_refused_scan_focuses_its_note`).
    fn close(&mut self, trust: &mut TrustSettings) {
        // Back while the page's own add, or that add's check, is on its
        // way closes the page at once: the add goes on, its outcome
        // unknown to the page, and Peers' next read shows what is stored
        // (§5.3, as a Trust awaited when its page closes).
        let page_add = matches!(self.in_flight, Some(InFlight::Add { .. }))
            || matches!(self.check, Some(Check::Add(_)));
        if page_add && self.page.is_some() {
            // Out of the page's slot, so a page opened later is its own:
            // the add's answer comes as the orphan's; a check's read still
            // shows the rows.
            if let Some(InFlight::Add { lines, .. }) = self.in_flight.take() {
                self.orphan = Some(lines);
            }
            if matches!(self.check, Some(Check::Add(_))) {
                self.check = None;
            }
            self.page = None;
            self.ask_focus(PeersFocus::Opener);
            return;
        }
        // Any other change on its way -- a Remove, an Undo, or its read
        // back -- is the list's, said by its snackbar: it never holds the
        // page open, since a Back that does nothing breaks the platform's
        // Back (§5.3; `a_page_closes_while_a_removal_is_on_its_way`).
        let Some(page) = self.page.as_mut() else {
            return;
        };
        if page.blocks.is_empty() && page.result.is_none() {
            self.page = None;
            self.ask_focus(PeersFocus::Opener);
            return;
        }
        for block in &mut page.blocks {
            if block.state(trust) == BlockState::Undecided {
                block.later = true;
            }
            // A Trust still awaited goes with its block: its answer is
            // lost to the page, and the peer's trust not known (§5.3).
            if block.trusting {
                trust.answer_lost(&block.peer);
            }
        }
        self.finish(trust);
    }

    /// Close the page once every block is decided; else move focus to the
    /// next undecided block, or to Done with several.
    fn settle(&mut self, trust: &TrustSettings) {
        let Some(page) = self.page.as_ref() else {
            return;
        };
        if page.blocks.is_empty() {
            return;
        }
        let next = page
            .blocks
            .iter()
            .find(|b| b.state(trust) == BlockState::Undecided)
            .map(|b| b.peer.clone());
        let waiting = page
            .blocks
            .iter()
            .any(|b| matches!(b.state(trust), BlockState::Trusting | BlockState::Checking));
        let several = page.blocks.len() > 1;
        match next {
            Some(peer) => self.ask_focus(PeersFocus::Block(peer)),
            // None undecided while a Trust is awaited: Done, with several
            // (§8); one block keeps focus where its Trust put it.
            None if waiting && several => self.ask_focus(PeersFocus::Done),
            None if waiting => {}
            None => self.finish(trust),
        }
    }

    fn finish(&mut self, trust: &TrustSettings) {
        let Some(page) = self.page.take() else {
            return;
        };
        let all_trusted = !page.blocks.is_empty()
            && page
                .blocks
                .iter()
                .all(|b| b.state(trust) == BlockState::Trusted);
        // One snackbar: Trusted when every block was; else step one's
        // own word on the add -- "added" only when it read so (§5.3). A
        // row's trust step, with no add, closes with none, and so does a
        // page opened from Connect: Connect's line says what was done,
        // said on the return, and a snackbar would say it twice.
        if page.from_connect {
            self.keep_for_connect(&page, trust);
        } else if all_trusted {
            self.say(PeersSnack::PeerTrusted);
        } else {
            match page.result {
                Some(AddResult::Added { .. }) => self.say(PeersSnack::Added),
                Some(AddResult::Unchanged { lines }) => {
                    self.say(PeersSnack::AddedUnchanged { lines });
                }
                Some(AddResult::Checked { lines }) => self.say(PeersSnack::AddedChecked { lines }),
                None => {}
            }
        }
        match page.from_row {
            Some(row) => self.ask_focus(PeersFocus::Row(row)),
            None => self.ask_focus(PeersFocus::Opener),
        }
    }

    /// What a page opened from Connect came to, as it closes: each peer
    /// its add carried (none when the add made nothing) and each peer a
    /// block was opened for, with its trust as the block reads it -- a
    /// peer with no block was trusted already (§5.3).
    fn keep_for_connect(&mut self, page: &AddPage, trust: &TrustSettings) {
        let added = page.result.is_some().then_some(&page.add_peers);
        let peers = added
            .into_iter()
            .flatten()
            .chain(page.blocks.iter().map(|b| &b.peer));
        // A page reaches here with an add's answer or a step two (`close`,
        // `settle` and `accepted` call `finish` for no other): either is
        // something §5.1 counts, a step two closed by Back included, since
        // Back there is Done (§5.3;
        // `a_return_is_said_only_when_its_page_did_something_counted`).
        let adds = self.connect_return.get_or_insert_with(Vec::new);
        for peer in peers {
            let trusted = match page.blocks.iter().find(|b| &b.peer == peer) {
                None => Some(true),
                Some(b) if b.state(trust) == BlockState::Trusted => Some(true),
                // A Trust still awaited as the page closed counts as one
                // whose answer was lost, as does one lost earlier that no
                // read has answered since: not known (§5.3), whatever the
                // block last knew.
                Some(b) if b.trusting || trust.is_pending(peer) => None,
                Some(b) => b.known(trust),
            };
            adds.retain(|a| &a.peer != peer);
            adds.push(ConnectAdd {
                peer: peer.clone(),
                trusted,
            });
        }
    }

    /// What add pages opened from Connect came to since the last take:
    /// `None` when none closed having done something §5.1 counts.
    pub fn take_connect_return(&mut self) -> Option<Vec<ConnectAdd>> {
        self.connect_return.take()
    }

    /// Connect's Trust: step two for `peers`, opened as a row's is, the
    /// blocks Connect's line counts as not trusted or not known (§5.1). A
    /// peer read trusted since gets no block; with none left nothing opens.
    fn trust_from_connect(
        &mut self,
        peers: Vec<TransportIdentity>,
        trust: &mut TrustSettings,
    ) -> Vec<Intent> {
        if self.page.is_some() {
            return Vec::new();
        }
        let blocks: Vec<TrustBlock> = peers
            .into_iter()
            .filter(|p| !trust_allows(trust, p))
            .map(|p| TrustBlock::new(p, trust))
            .collect();
        let Some(first) = blocks.first().map(|b| b.peer.clone()) else {
            return Vec::new();
        };
        self.page = Some(AddPage {
            from_connect: true,
            blocks,
            ..AddPage::default()
        });
        self.ask_focus(PeersFocus::Block(first));
        if trust.confirmed_list().is_none() {
            return trust.opened().into_iter().collect();
        }
        Vec::new()
    }

    /// Whether the list's read has answered since the start: `Some(true)`
    /// when the list is the runtime's word, `Some(false)` when the read
    /// failed, `None` while it is unread or a change may have altered it.
    #[must_use]
    pub fn list_answered(&self) -> Option<bool> {
        if self.busy() || self.reading {
            return None;
        }
        if self.list_problem.is_some() {
            return Some(false);
        }
        (self.list.is_some() && self.held == Held::AsRead).then_some(true)
    }

    /// Remove on an added row: at once, with Undo.
    fn remove(&mut self, address: String) -> Option<Intent> {
        if self.busy() {
            return None;
        }
        let row = self
            .list
            .as_ref()?
            .addresses
            .iter()
            .find(|a| a.address == address)?;
        if row.origin != AddressOrigin::AddedHere {
            return None;
        }
        let before = self.list.clone().unwrap_or_default();
        self.removal_sent = Some((address.clone(), before.clone()));
        self.in_flight = Some(InFlight::Remove {
            address: address.clone(),
            before,
        });
        Some(Intent::RemoveAddress(address))
    }

    /// The snackbar's Undo: adds the removed address again.
    fn undo(&mut self) -> Option<Intent> {
        let Some(PeersSnack::Removed {
            address,
            undo: true,
            ..
        }) = self.snack.clone()
        else {
            return None;
        };
        self.re_add(address)
    }

    /// The snackbar's Try again: the failed change once more, which is
    /// safe either way -- an address present answers unchanged, and one
    /// absent answers unchanged to a removal.
    fn try_again(&mut self) -> Option<Intent> {
        if self.busy() {
            return None;
        }
        match self.snack.clone()? {
            PeersSnack::UndoFailed { address } | PeersSnack::UndoUnknown { address } => {
                self.re_add(address)
            }
            PeersSnack::RemoveFailed { address } | PeersSnack::RemoveUnknown { address } => {
                // The list on show, else the one this removal was sent
                // against.
                let before = self.list.clone().or_else(|| match &self.removal_sent {
                    Some((a, before)) if *a == address => Some(before.clone()),
                    _ => None,
                });
                let before = before.unwrap_or_default();
                self.removal_sent = Some((address.clone(), before.clone()));
                self.in_flight = Some(InFlight::Remove {
                    address: address.clone(),
                    before,
                });
                Some(Intent::RemoveAddress(address))
            }
            _ => None,
        }
    }

    /// Whether this address's removal (`undo` false) or Undo (`undo`
    /// true) was left unknown and is not decided yet: a retry of it that
    /// writes nothing leaves it so. A change of the other kind is no retry
    /// of it, and says its own outcome (§5.7; `a_removal_is_no_retry_of_an_unknown_undo`).
    fn still_unknown(&self, address: &str, undo: bool) -> bool {
        self.pending.iter().any(|c| match c {
            Check::Undo(a) => undo && a == address,
            Check::Remove { address: a, .. } => !undo && a == address,
            Check::Add(_) => false,
        })
    }

    /// An address's removal or Undo left unknown, kept until decided.
    fn pend(&mut self, check: Check) {
        let Some(address) = pending_address(&check).map(str::to_owned) else {
            return;
        };
        self.pending
            .retain(|c| pending_address(c) != Some(address.as_str()));
        self.pending.push(check);
        if self.pending.len() > PLACES_BOUND {
            self.pending.remove(0);
        }
    }

    /// This address's unknown change decided, by its answer.
    fn settle_pending(&mut self, address: &str) {
        self.pending.retain(|c| pending_address(c) != Some(address));
        if self
            .unknown_change
            .as_ref()
            .is_some_and(|c| pending_address(c) == Some(address))
        {
            self.unknown_change = None;
        }
    }

    /// A retry of an unknown change that wrote nothing: its unknown line
    /// is said again -- the snackbar shown anew if it had closed or another
    /// had taken its place -- read to the screen reader once more, focus on
    /// its Try again (§5.7, §8).
    fn still_unknown_again(&mut self, address: &str, undo: bool) {
        let change = self
            .pending
            .iter()
            .find(|c| match c {
                Check::Undo(a) => undo && a == address,
                Check::Remove { address: a, .. } => !undo && a == address,
                Check::Add(_) => false,
            })
            .cloned();
        match &change {
            Some(Check::Undo(a)) => self.say(PeersSnack::UndoUnknown { address: a.clone() }),
            Some(Check::Remove { address: a, .. }) => {
                self.say(PeersSnack::RemoveUnknown { address: a.clone() });
            }
            Some(Check::Add(_)) | None => {}
        }
        self.unknown_change = change;
        self.ask_focus(PeersFocus::SnackAction);
        self.unknown_mark = self.focuses;
    }

    fn re_add(&mut self, address: String) -> Option<Intent> {
        if self.busy() {
            return None;
        }
        self.in_flight = Some(InFlight::Undo(address.clone()));
        Some(Intent::AddAddresses(vec![address]))
    }

    // --- the runtime's answers -----------------------------------------

    /// The runtime's answer to a read, and, after a lost answer, what the
    /// change it checks came to.
    pub fn read_answer(
        &mut self,
        answer: Result<AddressList, AddressProblem>,
        trust: &TrustSettings,
    ) {
        self.reading = false;
        let check = self.check.take();
        let again = self.again.take_if(|a| *a != Again::Trust);
        match answer {
            Ok(list) => {
                self.set_list(list);
                self.list_problem = None;
                self.held = Held::AsRead;
                // A read shows every change left unknown (§5.7): none is
                // pending after it, its verdict said where its line is
                // (`read_decides`).
                self.pending.clear();
                if again == Some(Again::List) {
                    self.ask_focus(PeersFocus::Title);
                }
                if self.add_note.is_some() && self.add_blocked().is_none() {
                    self.add_note = None;
                }
                if let Some(check) = check {
                    self.checked(check, trust);
                }
                self.read_decides(trust);
                // An Undo made, its row read in at last: focus on it, or,
                // a read having dropped it, as for any row gone -- its
                // place's next row, PeerRowGoneAfterAct said (§8).
                if let Some(address) = self.restored.take() {
                    self.ask_focus(PeersFocus::Row(address));
                }
            }
            // A lost answer's read back failing leaves the list as last
            // read: the change is unknown, said where it was made.
            Err(_) if check.is_some() => {
                if let Some(check) = check {
                    self.unknown(check);
                }
            }
            Err(problem) => {
                // An unreadable list is unknown, never empty: the list on
                // show goes, so nothing reads its emptiness, and focus
                // that was in it goes to the line that says why -- once
                // the add page is closed, if one is open (`ask_focus`).
                // A failed Try again goes to the line saying why (§8);
                // a failed opening read leaves focus on the title.
                let had_list = self.list.take().is_some();
                match again {
                    Some(Again::List) => self.ask_focus(PeersFocus::ListProblem),
                    Some(Again::Open) => {}
                    _ if had_list => self.ask_focus(PeersFocus::ListProblem),
                    _ => {}
                }
                // A row restored while unread is not drawn with no list;
                // its Try again succeeding goes as on opening (§8).
                self.restored = None;
                self.list_problem = Some(list_problem(problem));
            }
        }
    }

    /// The runtime's answer to an add of `lines`: the page's, or an
    /// Undo's.
    pub fn add_answer(
        &mut self,
        lines: &[String],
        answer: Result<AddressAnswer, AddressChangeFailure>,
        trust: &TrustSettings,
    ) {
        if self.orphan.as_deref() == Some(lines) {
            self.orphan = None;
            self.orphan_answered(lines, answer);
            return;
        }
        match self.in_flight.take() {
            Some(InFlight::Add {
                lines: sent,
                source,
            }) if sent == lines => self.page_answered(sent, source, answer, trust),
            Some(InFlight::Undo(address)) if lines == [address.as_str()] => {
                self.undo_answered(address, answer);
            }
            other => self.in_flight = other,
        }
    }

    /// The runtime's answer to a removal of `address`.
    pub fn remove_answer(
        &mut self,
        address: &str,
        answer: Result<AddressAnswer, AddressChangeFailure>,
        trust: &TrustSettings,
    ) {
        let before = match self.in_flight.take() {
            Some(InFlight::Remove {
                address: sent,
                before,
            }) if sent == address => before,
            other => {
                self.in_flight = other;
                return;
            }
        };
        match answer {
            Ok(answer) if superseded(&answer.outcome) => match answer.list {
                Ok(list) => {
                    self.read_back_superseded(address, list);
                    if self.list.as_ref().is_some_and(|l| listed(l, address)) {
                        self.say(PeersSnack::RemoveSuperseded {
                            address: address.to_owned(),
                        });
                        self.ask_focus(PeersFocus::RowMenu(address.to_owned()));
                    } else {
                        self.removed(address, &before, trust);
                    }
                }
                Err(problem) => self.unreadable(problem),
            },
            Ok(answer) => {
                // An answer decides it: nothing is unknown about it now.
                self.settle_pending(address);
                self.take_list(answer.list);
                match answer.outcome {
                    AddressOutcome::Removed | AddressOutcome::Unchanged => {
                        self.removed(address, &before, trust);
                    }
                    // A configured row offers no Remove; any other answer
                    // removed nothing.
                    AddressOutcome::Refused(_)
                    | AddressOutcome::RefusedLine { .. }
                    | AddressOutcome::Added { .. }
                    | AddressOutcome::AddedLines(_) => {
                        self.remove_failed(address);
                    }
                }
            }
            Err(AddressChangeFailure::NotMade(problem)) => match barred(problem) {
                Some(b) => self.bar(b),
                // A Try again that wrote nothing settles nothing: after an
                // unknown removal the change stays unknown (§5.7).
                None if self.still_unknown(address, false) => {
                    self.still_unknown_again(address, false);
                }
                None => self.remove_failed(address),
            },
            Err(AddressChangeFailure::Unconfirmed(_)) => {
                self.check = Some(Check::Remove {
                    address: address.to_owned(),
                    before,
                });
                self.reread = true;
                self.held = Held::MayHaveChanged;
            }
        }
    }

    /// An add whose page closed while it was on its way, answered: no
    /// page's to show -- no note, no result, no snackbar -- the list it
    /// brings, or the read a lost answer owes, shows what is stored, and
    /// a barred answer still says Peers cannot be used (§5.3).
    fn orphan_answered(
        &mut self,
        lines: &[String],
        answer: Result<AddressAnswer, AddressChangeFailure>,
    ) {
        match answer {
            Ok(answer) => self.take_list(answer.list),
            Err(AddressChangeFailure::Unconfirmed(_)) => {
                self.reread = true;
                self.held = Held::MayHaveChanged;
            }
            Err(AddressChangeFailure::NotMade(problem)) => match barred(problem) {
                // A runtime with the one-address add only: Peers still
                // reads, as for an open page (§5.7).
                Some(Barred::Incompatible) if lines.len() > 1 => {}
                // Focus to the line saying why, only where a list was on
                // show to have held it (§8), as a failed read does.
                Some(b) => {
                    let had_list = self.list.is_some();
                    self.list = None;
                    self.list_problem = Some(ListProblem::Barred(b));
                    self.snack = None;
                    if had_list {
                        self.ask_focus(PeersFocus::ListProblem);
                    }
                }
                None => {}
            },
        }
    }

    /// The list read back after a removal or an Undo of `address` refused
    /// `superseded`: a read started after it, which decides what it said
    /// and leaves nothing of it unknown. The refusal alone settles
    /// nothing, so an unreadable list leaves it as it was (§5.7).
    fn read_back_superseded(&mut self, address: &str, list: AddressList) {
        self.settle_pending(address);
        self.take_list(Ok(list));
    }

    /// The read back of a removal or an Undo refused `superseded` failed:
    /// Peers in its unreadable state with the read's line, focus on it
    /// (§5.7, §8), no read owed beyond the one made.
    fn unreadable(&mut self, problem: AddressProblem) {
        self.list = None;
        self.list_problem = Some(list_problem(problem));
        self.snack = None;
        self.restored = None;
        self.ask_focus(PeersFocus::ListProblem);
    }

    /// A Remove or an Undo answered not permitted or incompatible: Peers
    /// goes to its unreadable state with that line, focus on it.
    fn bar(&mut self, b: Barred) {
        self.list = None;
        self.list_problem = Some(ListProblem::Barred(b));
        self.snack = None;
        self.ask_focus(PeersFocus::ListProblem);
    }

    fn page_answered(
        &mut self,
        lines: Vec<String>,
        source: LineSource,
        answer: Result<AddressAnswer, AddressChangeFailure>,
        trust: &TrustSettings,
    ) {
        let answer = match answer {
            Ok(answer) if superseded(&answer.outcome) => {
                self.add_superseded(&lines, answer.list, trust);
                return;
            }
            Ok(answer) => {
                // A real answer decides the add: nothing is unknown now.
                if let Some(page) = self.page.as_mut() {
                    page.unknown = None;
                }
                answer
            }
            Err(AddressChangeFailure::NotMade(problem)) => {
                let note = match barred(problem) {
                    // A runtime with the one-address add only: an add of
                    // several lines is not taken there, one line is.
                    Some(Barred::Incompatible) if lines.len() > 1 => {
                        AddNote::SeveralIncompatible { lines: lines.len() }
                    }
                    Some(b) => {
                        self.list = None;
                        self.list_problem = Some(ListProblem::Barred(b));
                        AddNote::Barred(b)
                    }
                    // A retry of an add left unknown that writes nothing
                    // settles nothing: still unknown, said again (§5.7).
                    None if self.page.as_ref().and_then(|p| p.unknown.as_ref()) == Some(&lines) => {
                        AddNote::Unknown { lines: lines.len() }
                    }
                    None => AddNote::Failed,
                };
                // Not permitted or incompatible: the line under Add
                // (§8); not written: Add, to add again.
                let focus = if matches!(note, AddNote::Barred(_)) {
                    PeersFocus::AddNote
                } else {
                    PeersFocus::AddButton
                };
                let still_unknown = matches!(note, AddNote::Unknown { .. });
                self.page_note(note, focus);
                // The unknown line said again is its own, not a later act.
                if still_unknown {
                    self.unknown_mark = self.focuses;
                }
                return;
            }
            Err(AddressChangeFailure::Unconfirmed(_)) => {
                self.check = Some(Check::Add(lines));
                self.reread = true;
                self.held = Held::MayHaveChanged;
                self.page_note(AddNote::Checking, PeersFocus::AddButton);
                return;
            }
        };
        self.take_list(answer.list);
        match answer.outcome {
            AddressOutcome::Added { peer } => {
                let peers = peer.map_or_else(|| peers_of(&lines), |p| vec![p]);
                let result = AddResult::Added {
                    lines: lines.len(),
                    peers: peers.clone(),
                };
                self.accepted(result, &peers, trust);
            }
            AddressOutcome::AddedLines(rows) => {
                // Only the lines answered added are counted and named; a
                // line already there beside them is not (§5.3), while
                // every peer of the add is offered its trust step.
                let mut peers = Vec::new();
                let mut added = 0;
                for (row, line) in rows.iter().zip(&lines) {
                    if !row.added {
                        continue;
                    }
                    added += 1;
                    let peer = row.peer.clone().or_else(|| PeerAddress::peer_of(line));
                    if let Some(peer) = peer
                        && !peers.contains(&peer)
                    {
                        peers.push(peer);
                    }
                }
                let result = AddResult::Added {
                    lines: added,
                    peers,
                };
                self.accepted(result, &peers_of(&lines), trust);
            }
            AddressOutcome::Unchanged => {
                let result = AddResult::Unchanged { lines: lines.len() };
                self.accepted(result, &peers_of(&lines), trust);
            }
            AddressOutcome::Refused(why) | AddressOutcome::RefusedLine { why, .. } => {
                // A removal's refusal, never an add's: nothing written.
                let Some(why) = refusal_of(why) else {
                    self.page_note(AddNote::Failed, PeersFocus::AddButton);
                    return;
                };
                // The add of several names the line it refuses; one line
                // is its own.
                let index = match answer.outcome {
                    AddressOutcome::RefusedLine { line, .. } => line,
                    _ => 0,
                };
                let line = lines.get(index).cloned().unwrap_or_default();
                // A refusal of the whole add names no line: the caret at
                // the field's end, nothing selected (§5.3).
                let select = if why.names_line() {
                    LineSelection::Line(line.clone())
                } else {
                    LineSelection::End
                };
                let focus = match source {
                    LineSource::Typed => PeersFocus::Field {
                        select: Some(select),
                    },
                    LineSource::Scanned => PeersFocus::Note,
                };
                self.page_note(AddNote::Refused { line, why }, focus);
            }
            AddressOutcome::Removed => self.page_note(AddNote::Failed, PeersFocus::AddButton),
        }
    }

    /// An add refused `superseded`, decided on its list read back as an
    /// unknown add's is: every line listed, as checked, step two after
    /// it; else `AddSuperseded`, focus on Add. The read settles the lines
    /// an earlier add left unknown, listed or not; when it failed they
    /// stay unknown, and the line is still `AddSuperseded`, never
    /// `AddUnknown`, whose retry §5.7 rules out
    /// (`a_superseded_add_settles_unknown_lines_only_by_its_read`).
    fn add_superseded(
        &mut self,
        lines: &[String],
        list: Result<AddressList, AddressProblem>,
        trust: &TrustSettings,
    ) {
        let note = AddNote::Superseded { lines: lines.len() };
        let Ok(list) = list else {
            // The one read made, and it failed: nothing more is read for
            // it (acceptance 10).
            self.held = Held::MayHaveChanged;
            self.page_note(note, PeersFocus::AddButton);
            return;
        };
        self.take_list(Ok(list));
        // `take_list` settles only a line it shows missing; this read
        // settles the listed ones too (§5.7, "listed or not").
        if let Some(page) = self.page.as_mut() {
            page.unknown = None;
        }
        if self
            .list
            .as_ref()
            .is_some_and(|l| lines.iter().all(|line| listed(l, line)))
        {
            let result = AddResult::Checked { lines: lines.len() };
            self.accepted(result, &peers_of(lines), trust);
        } else {
            self.page_note(note, PeersFocus::AddButton);
        }
    }

    fn page_note(&mut self, note: AddNote, focus: PeersFocus) {
        if let Some(page) = self.page.as_mut() {
            page.note = Some(note);
            page.notes = page.notes.wrapping_add(1);
        }
        self.ask_focus(focus);
    }

    /// Step one done: the result line, the field cleared, and one block
    /// per peer of the add not trusted -- every peer while the trust list
    /// is not the daemon's word (unread, or kept after a failed read),
    /// since a needless block costs a tap and a missing one leaves a peer
    /// untrusted unprompted -- and none for this device.
    fn accepted(&mut self, result: AddResult, peers: &[TransportIdentity], trust: &TrustSettings) {
        let own = self.own(trust).cloned();
        let Some(page) = self.page.as_mut() else {
            return;
        };
        // From Connect a line already there is one this visit's earlier
        // add wrote (Connect opens on a list read empty), so "Nothing was
        // added" would contradict Connect's own line (§5.3).
        let result = match result {
            AddResult::Unchanged { lines } if page.from_connect => AddResult::Checked { lines },
            other => other,
        };
        page.result = Some(result);
        page.add_peers = peers
            .iter()
            .filter(|p| Some(*p) != own.as_ref())
            .cloned()
            .collect();
        page.field.clear();
        page.note = None;
        for peer in peers {
            if Some(peer) == own.as_ref()
                || trust_allows(trust, peer)
                || page.blocks.iter().any(|b| &b.peer == peer)
            {
                continue;
            }
            page.blocks.push(TrustBlock::new(peer.clone(), trust));
        }
        match page.blocks.first() {
            Some(block) => {
                let peer = block.peer.clone();
                self.trust_read_owed = trust.confirmed_list().is_none();
                self.ask_focus(PeersFocus::Block(peer));
            }
            None => self.finish(trust),
        }
    }

    /// Whether a trust step an add opened owes its read of the trust set,
    /// once: the root asks it with [`TrustSettings::opened`], which a read
    /// already on its way answers instead.
    pub fn take_trust_read(&mut self) -> bool {
        std::mem::take(&mut self.trust_read_owed)
    }

    fn undo_answered(
        &mut self,
        address: String,
        answer: Result<AddressAnswer, AddressChangeFailure>,
    ) {
        match answer {
            Ok(answer) if superseded(&answer.outcome) => match answer.list {
                Ok(list) => {
                    self.read_back_superseded(&address, list);
                    if self.list.as_ref().is_some_and(|l| listed(l, &address)) {
                        self.undone(address);
                    } else {
                        self.undo_not_made(PeersSnack::UndoSuperseded { address });
                    }
                }
                Err(problem) => self.unreadable(problem),
            },
            Ok(answer) => {
                self.settle_pending(&address);
                self.take_list(answer.list);
                match answer.outcome {
                    // An Undo is one line, so the add of several answers
                    // it only as it answers that line.
                    AddressOutcome::Added { .. }
                    | AddressOutcome::AddedLines(_)
                    | AddressOutcome::Unchanged => {
                        self.undone(address);
                    }
                    // Refused by name -- the bound, the list off, this
                    // device's own address on a row written before that
                    // rule (ADR-0062 decision 1) -- the refusal's own words
                    // and no Try again, which the same refusal would meet
                    // (§5.2; `an_undo_refused_by_name_says_why_and_offers_no_retry`).
                    AddressOutcome::Refused(why) | AddressOutcome::RefusedLine { why, .. }
                        if let Some(why) = refusal_of(why) =>
                    {
                        self.undo_not_made(PeersSnack::UndoRefused(why));
                    }
                    AddressOutcome::Refused(_)
                    | AddressOutcome::RefusedLine { .. }
                    | AddressOutcome::Removed => {
                        let snack = PeersSnack::UndoFailed {
                            address: address.clone(),
                        };
                        self.undo_not_made(snack);
                    }
                }
            }
            Err(AddressChangeFailure::NotMade(problem)) => {
                if let Some(b) = barred(problem) {
                    self.bar(b);
                } else if self.still_unknown(&address, true) {
                    // As for a removal: an unknown Undo stays unknown.
                    self.still_unknown_again(&address, true);
                } else {
                    let snack = PeersSnack::UndoFailed {
                        address: address.clone(),
                    };
                    self.undo_not_made(snack);
                }
            }
            Err(AddressChangeFailure::Unconfirmed(_)) => {
                self.check = Some(Check::Undo(address));
                self.reread = true;
                self.held = Held::MayHaveChanged;
            }
        }
    }

    fn undone(&mut self, address: String) {
        self.snack = None;
        self.after_remove = None;
        // Restored with its list unread: the row is not drawn yet, so
        // focus goes to it once a read lists it.
        if self.held == Held::MayHaveChanged
            && !self.list.as_ref().is_some_and(|l| listed(l, &address))
        {
            self.restored = Some(address);
            return;
        }
        self.ask_focus(PeersFocus::Row(address));
    }

    /// An Undo not made: focus as after the Remove it undid, while that
    /// row is still listed; else the first row shown, else Add an
    /// address.
    fn undo_not_made(&mut self, snack: PeersSnack) {
        self.say(snack);
        let list = self.list.clone().unwrap_or_default();
        // Read, not taken: a Try again refused or failed again goes the
        // same way (section 8), until an Undo lands or a new Remove.
        let kept = self.after_remove.clone().filter(|f| match f {
            PeersFocus::Row(row) => listed(&list, row),
            _ => true,
        });
        let focus = kept.unwrap_or_else(|| {
            shown(&list).next().map_or(PeersFocus::AddAddress, |a| {
                PeersFocus::Row(a.address.clone())
            })
        });
        // As after the Remove: a target the app chose, not a row acted on.
        self.place_focus(focus);
    }

    fn removed(&mut self, address: &str, before: &AddressList, trust: &TrustSettings) {
        let row = before.addresses.iter().find(|a| a.address == address);
        // "You still trust this peer" only on the daemon's word.
        let trusted = row
            .and_then(|r| r.peer.as_ref())
            .is_some_and(|p| trust.confirmed_list().is_some_and(|l| l.allows(p)));
        let undo = row.is_none_or(|r| r.dialled);
        self.say(PeersSnack::Removed {
            address: address.to_owned(),
            trusted,
            undo,
        });
        self.focus_after(before, address);
    }

    fn remove_failed(&mut self, address: &str) {
        self.say(PeersSnack::RemoveFailed {
            address: address.to_owned(),
        });
        self.ask_focus(PeersFocus::RowMenu(address.to_owned()));
    }

    /// A lost answer, decided on the list read back. An add is all or
    /// none, so a list lacking any of its lines shows one not made.
    fn checked(&mut self, check: Check, trust: &TrustSettings) {
        let Some(list) = self.list.clone() else {
            return;
        };
        match check {
            Check::Add(lines) => {
                if lines.iter().all(|l| listed(&list, l)) {
                    let result = AddResult::Checked { lines: lines.len() };
                    self.accepted(result, &peers_of(&lines), trust);
                } else {
                    self.page_note(AddNote::Failed, PeersFocus::AddButton);
                }
            }
            Check::Undo(address) => {
                if listed(&list, &address) {
                    self.undone(address);
                } else {
                    let snack = PeersSnack::UndoFailed {
                        address: address.clone(),
                    };
                    self.undo_not_made(snack);
                }
            }
            Check::Remove { address, before } => {
                if listed(&list, &address) {
                    self.remove_failed(&address);
                } else {
                    self.removed(&address, &before, trust);
                }
            }
        }
    }

    /// A lost answer whose list could not be read back either.
    fn unknown(&mut self, check: Check) {
        match check {
            Check::Add(lines) => {
                let count = lines.len();
                if let Some(page) = self.page.as_mut() {
                    page.unknown = Some(lines);
                }
                self.page_note(AddNote::Unknown { lines: count }, PeersFocus::AddButton);
                self.unknown_mark = self.focuses;
            }
            // Unknown, the snackbar's Try again is where to go next: the
            // row menu that opened it may be gone (§8).
            Check::Undo(address) => {
                self.unknown_change = Some(Check::Undo(address.clone()));
                self.pend(Check::Undo(address.clone()));
                self.say(PeersSnack::UndoUnknown { address });
                self.ask_focus(PeersFocus::SnackAction);
                self.unknown_mark = self.focuses;
            }
            Check::Remove { address, before } => {
                let check = Check::Remove {
                    address: address.clone(),
                    before,
                };
                self.unknown_change = Some(check.clone());
                self.pend(check);
                self.say(PeersSnack::RemoveUnknown { address });
                self.ask_focus(PeersFocus::SnackAction);
                self.unknown_mark = self.focuses;
            }
        }
    }

    /// A change's list read back, or, when it failed, a read owed.
    fn take_list(&mut self, list: Result<AddressList, AddressProblem>) {
        if let Ok(list) = list {
            self.set_list(list);
            self.list_problem = None;
            self.held = Held::AsRead;
            self.shown_not_made();
        } else {
            self.reread = true;
            self.held = Held::MayHaveChanged;
        }
    }

    /// A read of the list decides what was unknown (§5.7, "until a read
    /// shows it"), as a lost answer's read back does: an add's lines all
    /// listed are checked, any missing is not made; a removal or Undo by
    /// whether its address is listed.
    ///
    /// Only for a change whose own line still holds its place: on show, or
    /// a snackbar closed by itself with no act since. Its verdict replaces
    /// that line, and takes focus only if nothing asked for it since the
    /// line was said; a change a later act has taken the place of ends
    /// unsaid, that act's outcome standing alone.
    fn read_decides(&mut self, trust: &TrustSettings) {
        let moved = self.focuses != self.unknown_mark;
        if let Some(change) = self.unknown_change.clone() {
            let on_show = self.unknown_on_show();
            self.unknown_change = None;
            // A removal whose snackbar has left with no act since: this
            // read is its read back, and focus is the view's to move,
            // only where it is on that row (§5.7, §8).
            let left = !on_show && matches!(change, Check::Remove { .. });
            if on_show || (self.snack.is_none() && !moved) {
                let row = match &change {
                    Check::Remove { address, .. } | Check::Undo(address) => Some(address.clone()),
                    Check::Add(_) => None,
                };
                let kept = moved || left;
                let asked = self.decide_keeping_focus(change, kept, trust);
                if kept && let (Some(row), Some(focus)) = (row, asked) {
                    self.read_back = Some(RowReadBack { row, focus });
                    self.read_backs = self.read_backs.wrapping_add(1);
                }
            }
        }
        let note_on_show = self
            .page
            .as_ref()
            .is_some_and(|p| matches!(p.note, Some(AddNote::Unknown { .. })));
        if let Some(lines) = self.page.as_mut().and_then(|p| p.unknown.take())
            && note_on_show
        {
            let _ = self.decide_keeping_focus(Check::Add(lines), moved, trust);
        }
    }

    /// `checked`, leaving focus where a later request put it if `kept`;
    /// the focus `checked` asked for, if any.
    fn decide_keeping_focus(
        &mut self,
        change: Check,
        kept: bool,
        trust: &TrustSettings,
    ) -> Option<PeersFocus> {
        let focus = (self.focus.clone(), self.focuses, self.focus_by);
        self.checked(change, trust);
        let asked = (self.focuses != focus.1)
            .then(|| self.focus.clone())
            .flatten();
        if kept {
            (self.focus, self.focuses, self.focus_by) = focus;
        }
        asked
    }

    /// A list that came with another change's answer. The change whose
    /// line is on show -- and an add left unknown on the page -- it ends
    /// only where it shows it not made: an add's line missing, a removed
    /// address still listed, an undone one absent; that line is the
    /// read's to decide aloud. Every other change left unknown it decides
    /// as a read does, showing the device as it is.
    fn shown_not_made(&mut self) {
        let Some(list) = self.list.as_ref() else {
            return;
        };
        let not_made = match &self.unknown_change {
            Some(Check::Remove { address, .. }) => listed(list, address),
            Some(Check::Undo(address)) => !listed(list, address),
            _ => false,
        };
        if not_made {
            self.unknown_change = None;
        }
        // Every other change left unknown is decided by this list as by a
        // read: it shows the device as it is now, after each was sent.
        // Only the line on show waits, as above, to be said where it is
        // (`an_answers_list_decides_the_changes_left_unknown_off_show`).
        let on_show = self.unknown_change.clone();
        self.pending.retain(|c| on_show.as_ref() == Some(c));
        let missing = self
            .page
            .as_ref()
            .and_then(|p| p.unknown.as_ref())
            .is_some_and(|lines| lines.iter().any(|l| !listed(list, l)));
        if missing && let Some(page) = self.page.as_mut() {
            page.unknown = None;
        }
    }

    /// Focus after a removal: the next row shown, else the previous,
    /// else Add an address, in `list` as it stood -- kept, so an Undo
    /// refused goes as after the Remove (§8).
    fn focus_after(&mut self, list: &AddressList, address: &str) {
        let rows: Vec<&str> = shown(list).map(|a| a.address.as_str()).collect();
        // Only a row the list now on show still has: another administrator
        // may have removed a neighbour meanwhile. The nearest after, else
        // the nearest before.
        let drawn = |row: &&&str| self.list.as_ref().is_none_or(|l| listed(l, row));
        let target = rows.iter().position(|r| *r == address).and_then(|at| {
            rows[at + 1..]
                .iter()
                .find(drawn)
                .or_else(|| rows[..at].iter().rev().find(drawn))
        });
        let focus = target.map_or(PeersFocus::AddAddress, |row| {
            PeersFocus::Row((*row).to_owned())
        });
        self.after_remove = Some(focus.clone());
        self.place_focus(focus);
    }

    /// What the root carries out between turns: the read a change left
    /// owed, and step two's blocks once trust answered.
    pub fn turn(&mut self, trust: &TrustSettings) -> Option<Intent> {
        self.trust_answers(trust);
        // The trust line's Try again answered: the rows with their states
        // and focus on the title, or the line again and focus kept on Try
        // again (§8).
        // A trust step opened meanwhile keeps the focus it has.
        if self.again == Some(Again::Trust) && !trust.reading() {
            self.again = None;
            if self.page.is_none() {
                // Only a control the render draws: Try again while it is
                // offered, else the line that says why it is not.
                match self.trust_line(trust) {
                    None => self.ask_focus(PeersFocus::Title),
                    Some(TrustLine::Unread { retry: true }) => {
                        self.trust_line_said = self.trust_line_said.wrapping_add(1);
                        self.ask_focus(PeersFocus::TrustRetry);
                    }
                    Some(_) => self.ask_focus(PeersFocus::TrustLine),
                }
            }
        }
        if !self.reread || self.reading || self.in_flight.is_some() {
            return None;
        }
        self.reread = false;
        self.read()
    }

    /// A block whose Trust is no longer on its way takes trust's answer:
    /// trusted -- also when answered as made with its list not read back
    /// (§5.3) --, a problem in the block, or a lost answer checked on the
    /// trust list read again. A trusted peer's block keeps no problem,
    /// whatever read listed it.
    fn trust_answers(&mut self, trust: &TrustSettings) {
        let Some(page) = self.page.as_mut() else {
            return;
        };
        // Focus goes only to a block this turn changed: its new problem
        // line, or its state line once its Trust is unconfirmed; never an
        // earlier block's problem (§8).
        let mut problem = None;
        let mut checking = None;
        let mut moved = false;
        for block in &mut page.blocks {
            let moved_before = moved;
            moved = false;
            let known_before = block.known;
            if let Some(list) = trust.confirmed_list() {
                block.known = Some(list.allows(&block.peer));
            }
            // A block whose Trust was pressed and left not known: a read
            // the person did not ask for decides it in place -- the page
            // stays open and the model asks no focus move; focus on a
            // control the decision took away is the view's to move, by
            // `focus_after_block_decided` (§5.3, §8).
            let left_unknown = matches!(
                block.problem,
                Some(BlockProblem::Unknown | BlockProblem::SupersededUnread)
            ) && !block.trusting
                && block.checking.is_none();
            // A read that shows the peer trusted decides the block as its
            // own Trust would: the step settles (§5.3).
            if block.known == Some(true)
                && known_before != Some(true)
                && !block.trusting
                && block.checking.is_none()
                && !left_unknown
            {
                moved = true;
            }
            let on_its_way = trust.in_flight().is_some_and(|c| c.peer == block.peer);
            if block.problem.is_some() && block.allowed(trust) {
                block.problem = None;
                // (`a_superseded_trust_left_unread_is_decided_by_a_later_read`)
                moved |= !left_unknown;
            }
            // An unknown Trust stays unknown only until a read shows it
            // (§5.7): a confirmed list not naming the peer says it was not
            // made, and its block offers Trust again
            // (`an_unknown_trust_a_later_read_decides_is_no_longer_unknown`).
            // The model asks no focus move for a read the person did not
            // ask for: focus stays, save where it was on a control the
            // decision took away, which the view finds and moves through
            // `focus_after_block_decided` (§8).
            // A superseded Trust left unread is decided the same way by a
            // read started after its refusal, in its own lines (§5.3).
            if matches!(
                block.problem,
                Some(BlockProblem::Unknown | BlockProblem::SupersededUnread)
            ) && !on_its_way
                && !block.trusting
                && block.checking.is_none()
                && trust
                    .confirmed_list()
                    .is_some_and(|l| !l.allows(&block.peer))
            {
                block.problem = Some(if block.problem == Some(BlockProblem::Unknown) {
                    BlockProblem::NotMade
                } else {
                    BlockProblem::Superseded
                });
            }
            if block.trusting && !on_its_way {
                block.trusting = false;
                moved = true;
                if !block.allowed(trust) {
                    match trust.outcome().0 {
                        // Answered as made, though its list was not read
                        // back: known, trusted (§5.3).
                        Some(TrustOutcome::Changed(c)) if c.peer == block.peer => {
                            block.known = Some(true);
                        }
                        // Unconfirmed, or refused superseded: the list read
                        // again decides, a superseded one in its own lines.
                        Some(
                            outcome @ (TrustOutcome::Unconfirmed(c) | TrustOutcome::Superseded(c)),
                        ) if c.peer == block.peer => {
                            block.checking = Some(match outcome {
                                TrustOutcome::Superseded(_) => ReadBack::Superseded,
                                _ => ReadBack::Lost,
                            });
                            block.known = None;
                            checking.get_or_insert_with(|| block.peer.clone());
                        }
                        // Written nothing -- unavailable or failed -- after an
                        // unknown Trust: still unknown (§5.7). Refused, not
                        // permitted or incompatible are answers, and say so.
                        // While a Trust on this PeerId is pending -- lost, not
                        // read back -- one that writes nothing is a retry of
                        // an unknown change, wherever it is made (§5.7).
                        Some(TrustOutcome::Problem(p)) => {
                            let wrote_nothing =
                                matches!(p, TrustProblem::Unavailable | TrustProblem::Internal);
                            block.problem =
                                Some(if wrote_nothing && trust.is_pending(&block.peer) {
                                    BlockProblem::Unknown
                                } else {
                                    BlockProblem::Trust(*p)
                                });
                        }
                        _ => block.problem = Some(BlockProblem::NotMade),
                    }
                }
            } else if block.checking.is_some() && !trust.reading() && !trust.reread_owed() {
                let superseded = block.checking.take() == Some(ReadBack::Superseded);
                moved = true;
                if !block.allowed(trust) {
                    let read = trust.confirmed_list().is_some();
                    block.problem = Some(match trust.outcome().0 {
                        _ if superseded && read => BlockProblem::Superseded,
                        _ if superseded => BlockProblem::SupersededUnread,
                        Some(TrustOutcome::NotReadAgain(_)) => BlockProblem::Unknown,
                        _ => BlockProblem::NotMade,
                    });
                }
            }
            if moved && problem.is_none() && block.problem.is_some() && block.checking.is_none() {
                problem = Some(block.peer.clone());
            }
            moved |= moved_before;
        }
        if !moved {
            return;
        }
        match (problem, checking) {
            (Some(peer), _) => self.ask_focus(PeersFocus::BlockProblem(peer)),
            (None, Some(peer)) => self.ask_focus(PeersFocus::BlockState(peer)),
            (None, None) => self.settle(trust),
        }
    }

    fn say(&mut self, snack: PeersSnack) {
        self.snack = Some(snack);
        self.snacks = self.snacks.wrapping_add(1);
    }

    /// Whether the snackbar on show is the unknown change's own line.
    fn unknown_on_show(&self) -> bool {
        match (&self.unknown_change, &self.snack) {
            (
                Some(Check::Remove { address, .. }),
                Some(PeersSnack::RemoveUnknown { address: on }),
            )
            | (Some(Check::Undo(address)), Some(PeersSnack::UndoUnknown { address: on })) => {
                address == on
            }
            _ => false,
        }
    }

    /// The list read in, its rows' order kept in [`Self::places`] with
    /// every row it dropped at its place among them.
    fn set_list(&mut self, list: AddressList) {
        let mut places: Vec<String> = shown(&list).map(|a| a.address.clone()).collect();
        for (at, row) in self.places.iter().enumerate() {
            if places.contains(row) {
                continue;
            }
            // After the nearest row before it that is placed, else first.
            let after = self.places[..at]
                .iter()
                .rev()
                .find_map(|before| places.iter().position(|p| p == before));
            places.insert(after.map_or(0, |i| i + 1), row.clone());
        }
        for row in &places {
            if listed(&list, row) {
                self.dropped.remove(row);
            } else if !self.dropped.contains_key(row) {
                self.dropped.insert(row.clone(), self.drops);
                self.drops = self.drops.wrapping_add(1);
            }
        }
        while places.len() > PLACES_BOUND {
            let Some(oldest) = self
                .dropped
                .iter()
                .min_by_key(|(_, at)| **at)
                .map(|(row, _)| row.clone())
            else {
                break;
            };
            self.dropped.remove(&oldest);
            places.retain(|p| *p != oldest);
        }
        self.places = places;
        self.list = Some(list);
    }

    fn ask_focus(&mut self, focus: PeersFocus) {
        // A control of the list pane asked for while the add page is open
        // is not drawn: the page's own focus stands, and closing the page
        // sends focus to the list (§8). A Remove, an Undo or a read can be
        // answered under the page
        // (`a_list_control_asked_under_the_add_page_leaves_its_focus`).
        // Exhaustive, so a variant added later is placed on purpose.
        let list_pane = match focus {
            PeersFocus::Opener
            | PeersFocus::Row(_)
            | PeersFocus::RowMenu(_)
            | PeersFocus::AddAddress
            | PeersFocus::AddAddressNote
            | PeersFocus::ListProblem
            | PeersFocus::TrustLine
            | PeersFocus::TrustRetry => true,
            PeersFocus::Title
            | PeersFocus::Field { .. }
            | PeersFocus::Note
            | PeersFocus::AddButton
            | PeersFocus::Scan(_)
            | PeersFocus::Block(_)
            | PeersFocus::BlockProblem(_)
            | PeersFocus::BlockState(_)
            | PeersFocus::Done
            | PeersFocus::AddNote
            | PeersFocus::SnackAction
            | PeersFocus::ShowWarning
            | PeersFocus::ShowProblem
            | PeersFocus::ShowOpener => false,
        };
        if list_pane && self.page.is_some() {
            return;
        }
        self.focus = Some(focus);
        self.focuses = self.focuses.wrapping_add(1);
        self.focus_by = FocusBy::Act;
    }

    /// `ask_focus` for a target the app chose rather than the row the
    /// person acted on: the next row after a Remove, a closed snackbar's
    /// row menu.
    fn place_focus(&mut self, focus: PeersFocus) {
        let asked = self.focuses;
        self.ask_focus(focus);
        if self.focuses != asked {
            self.focus_by = FocusBy::App;
        }
    }

    /// `focus` on what is drawn now: it is judged when it is read, not
    /// when it was asked, since a read may replace the list in between
    /// (`a_row_focus_asked_before_a_read_lands_on_what_is_drawn`).
    fn drawn_focus(&self, focus: &PeersFocus) -> PeersFocus {
        // A list control asked for once Peers became unreadable is not
        // drawn: the line saying why takes its place (§8).
        let list_control = matches!(
            focus,
            PeersFocus::Opener
                | PeersFocus::Row(_)
                | PeersFocus::RowMenu(_)
                | PeersFocus::AddAddress
                | PeersFocus::AddAddressNote
        );
        if list_control && self.page.is_none() && self.list.is_none() {
            return PeersFocus::ListProblem;
        }
        // A row the list on show no longer has -- removed elsewhere since
        // it was chosen -- is drawn nowhere: the row after its place as
        // last drawn, else the one before, else Add an address, as after
        // a Remove (§8).
        match (focus, self.list.as_ref()) {
            (PeersFocus::Row(row) | PeersFocus::RowMenu(row), Some(list)) if !listed(list, row) => {
                let drawn = |p: &&String| listed(list, p);
                let at = self.places.iter().position(|p| p == row);
                at.and_then(|at| {
                    self.places[at + 1..]
                        .iter()
                        .find(drawn)
                        .or_else(|| self.places[..at].iter().rev().find(drawn))
                })
                .map_or(PeersFocus::AddAddress, |p| PeersFocus::Row(p.clone()))
            }
            (focus, _) => focus.clone(),
        }
    }

    /// Where focus goes from `address`'s row, or a control in it, holding
    /// focus when a read dropped it: the row after its place as last
    /// drawn, else the one before, else Add an address (§7, §8); `None`
    /// while it is listed, or no list is on show.
    #[must_use]
    pub fn focus_after_gone(&self, address: &str) -> Option<PeersFocus> {
        let list = self.list.as_ref()?;
        if listed(list, address) {
            return None;
        }
        Some(self.drawn_focus(&PeersFocus::Row(address.to_owned())))
    }

    /// Whether step two shows Done: with several blocks always; with one,
    /// only once it is decided -- a read the person did not ask for
    /// decided it and the page stays open, Done its exit -- since Done on
    /// an undecided block would do what its Not now does (§5.3).
    #[must_use]
    pub fn done_shown(&self, trust: &TrustSettings) -> bool {
        self.page
            .as_ref()
            .is_some_and(|page| match page.blocks.as_slice() {
                [] => false,
                [only] => matches!(
                    only.state(trust),
                    BlockState::Trusted | BlockState::Later | BlockState::LaterUnknown
                ),
                _ => true,
            })
    }

    /// Where focus goes from `peer`'s block's control `held`, holding
    /// focus when it went with no move asked for -- a read the person did
    /// not ask for decided the block trusted (§5.3, §8's row for that
    /// read): from its Trust or Not now, the next undecided block's title,
    /// else Done; from its problem line, the line that replaced it, the
    /// block's new one (asked of ux-designer-01). `None` while the block
    /// is not decided trusted, or no add page shows it, since then the
    /// control is still drawn.
    #[must_use]
    pub fn focus_after_block_decided(
        &self,
        trust: &TrustSettings,
        peer: &TransportIdentity,
        held: BlockHeld,
    ) -> Option<PeersFocus> {
        let page = self.page.as_ref()?;
        let block = page.blocks.iter().find(|b| &b.peer == peer)?;
        if block.state(trust) != BlockState::Trusted {
            return None;
        }
        Some(match held {
            BlockHeld::Problem => PeersFocus::BlockState(peer.clone()),
            BlockHeld::Choice => page
                .blocks
                .iter()
                .find(|b| b.state(trust) == BlockState::Undecided)
                .map_or(PeersFocus::Done, |b| PeersFocus::Block(b.peer.clone())),
        })
    }

    /// Whether the focus asked for after the person's act on a row is
    /// that row, which a read showed gone: `PeerRowGoneAfterAct` is
    /// announced before focus moves (§8). A target the app placed is not.
    #[must_use]
    pub fn focus_row_gone(&self) -> bool {
        if self.focus_by == FocusBy::App {
            return false;
        }
        match (self.focus.as_ref(), self.list.as_ref()) {
            (Some(PeersFocus::Row(row) | PeersFocus::RowMenu(row)), Some(list)) => {
                !listed(list, row)
            }
            _ => false,
        }
    }

    // --- what a view reads -------------------------------------------

    /// The list as last read; `None` before the first read and after a
    /// read or a change that found the boundary unusable.
    #[must_use]
    pub const fn list(&self) -> Option<&AddressList> {
        self.list.as_ref()
    }

    /// Why the list is not on show.
    #[must_use]
    pub const fn list_problem(&self) -> Option<ListProblem> {
        self.list_problem
    }

    /// Whether a read is on its way.
    #[must_use]
    pub const fn reading(&self) -> bool {
        self.reading
    }

    /// The rows, newest added first, then the configuration's (§5.2), in
    /// their states against `trust`'s list.
    #[must_use]
    pub fn rows(&self, trust: &TrustSettings) -> Vec<PeerRow> {
        let Some(list) = self.list.as_ref() else {
            return Vec::new();
        };
        let own = self.own(trust);
        let row = |a: &PeerAddress| PeerRow {
            address: a.address.clone(),
            peer: a.peer.clone(),
            origin: a.origin,
            state: match trust.confirmed_list() {
                _ if !a.dialled => RowState::ProviderOff,
                // This node's PeerId as `own` gives it -- the session's,
                // else the trust list's as kept.
                _ if a.peer.is_some() && a.peer.as_ref() == own => RowState::ThisDevice,
                // A Trust from this page answered as made, nothing of its
                // peer since: the daemon's last word on it (§5.2).
                None if a.peer.as_ref().is_some_and(|p| trust.trusted_here(p)) => RowState::Trusted,
                None => RowState::TrustUnknown,
                Some(l) if a.peer.as_ref().is_some_and(|p| l.allows(p)) => RowState::Trusted,
                Some(_) => RowState::NeedsTrust,
            },
        };
        shown(list).map(row).collect()
    }

    /// Whether the add page is open.
    #[must_use]
    pub const fn add_open(&self) -> bool {
        self.page.is_some()
    }

    /// Whether the open page's own add awaits its answer: Add keeps focus,
    /// labelled `AddWaiting` beside a progress mark, and takes no press
    /// (§5.3). An add Back left out is no page's.
    #[must_use]
    pub const fn add_waiting(&self) -> bool {
        self.page.is_some() && matches!(self.in_flight, Some(InFlight::Add { .. }))
    }

    /// Whether the add page is at step one -- the field, its helper and
    /// Add -- rather than its result or a trust step: open with no result
    /// and no block. A row's Trust opens the trust step alone, so the
    /// view draws no step-one control a press would not reach.
    #[must_use]
    pub fn step_one_shown(&self) -> bool {
        self.page
            .as_ref()
            .is_some_and(|p| p.result.is_none() && p.blocks.is_empty())
    }

    /// The field's text.
    #[must_use]
    pub fn field(&self) -> &str {
        self.page.as_ref().map_or("", |p| p.field.as_str())
    }

    /// The add page's note and a count bumped at every note set: the same
    /// note set again is said again.
    #[must_use]
    pub fn add_note_said(&self) -> (Option<&AddNote>, u64) {
        self.page
            .as_ref()
            .map_or((None, 0), |p| (p.note.as_ref(), p.notes))
    }

    /// The note on the add page.
    #[must_use]
    pub fn add_page_note(&self) -> Option<&AddNote> {
        self.page.as_ref()?.note.as_ref()
    }

    /// The note under Peers' Add an address.
    #[must_use]
    pub const fn add_note(&self) -> Option<Refusal> {
        self.add_note
    }

    /// Step one's result, once it is done.
    #[must_use]
    pub fn add_result(&self) -> Option<&AddResult> {
        self.page.as_ref()?.result.as_ref()
    }

    /// Whether the scanner is open.
    #[must_use]
    pub fn scan_open(&self) -> bool {
        self.page.as_ref().is_some_and(|p| p.scanning)
    }

    /// Step two's blocks.
    #[must_use]
    pub fn blocks(&self, trust: &TrustSettings) -> Vec<BlockView> {
        self.page.as_ref().map_or_else(Vec::new, |p| {
            p.blocks
                .iter()
                .map(|b| BlockView {
                    peer: b.peer.clone(),
                    state: b.state(trust),
                    problem: b.problem,
                })
                .collect()
        })
    }

    /// A count that changes each time the trust line's Try again failed
    /// with the line still on show, for a view to say the line again.
    #[must_use]
    pub const fn trust_line_said(&self) -> u64 {
        self.trust_line_said
    }

    /// Whether an add -- its page's, or one Back left out -- a removal,
    /// an Undo or the check of a lost answer is on its way: one change at a
    /// time, since the root sends no command equal to one still out
    /// (`an_add_left_out_by_back_holds_every_other_change`).
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.in_flight.is_some() || self.check.is_some() || self.orphan.is_some()
    }

    /// The snackbar, and a count that changes with each.
    #[must_use]
    pub const fn snack(&self) -> (Option<&PeersSnack>, u64) {
        (self.snack.as_ref(), self.snacks)
    }

    /// The last row's own read back that left focus to the view, on what
    /// is drawn now, and a count that changes with each.
    #[must_use]
    pub fn read_back(&self) -> (Option<RowReadBack>, u64) {
        let back = self.read_back.as_ref().map(|b| RowReadBack {
            row: b.row.clone(),
            focus: self.drawn_focus(&b.focus),
        });
        (back, self.read_backs)
    }

    /// Where focus is asked to go, on what is drawn now, and a count that
    /// changes with each request.
    #[must_use]
    pub fn focus(&self) -> (Option<PeersFocus>, u64) {
        (
            self.focus.as_ref().map(|f| self.drawn_focus(f)),
            self.focuses,
        )
    }

    /// Why the rows show no trust state, while the list is on show and
    /// the trust list is not the daemon's word (§5.2's trust unread):
    /// none while the list itself is unreadable, whose line stands alone,
    /// and none while the first trust read is still on its way.
    #[must_use]
    pub fn trust_line(&self, trust: &TrustSettings) -> Option<TrustLine> {
        self.list.as_ref()?;
        if trust.confirmed_list().is_some() {
            return None;
        }
        match trust.outcome().0 {
            Some(TrustOutcome::Problem(p) | TrustOutcome::NotReadAgain(p)) => Some(match p {
                TrustProblem::NotPermitted => TrustLine::NotPermitted,
                TrustProblem::Incompatible => TrustLine::Unread { retry: false },
                _ => TrustLine::Unread { retry: true },
            }),
            _ if trust.list().is_none() && trust.reading() => None,
            _ => Some(TrustLine::Unread { retry: true }),
        }
    }

    /// This node's `PeerId`, as the session knows it.
    pub fn this_device(&mut self, peer: TransportIdentity) {
        self.this_device = Some(peer);
    }

    /// This node's `PeerId`: the session's, which a ready session gave,
    /// else the trust list's as last read when no session has said.
    fn own<'a>(&'a self, trust: &'a TrustSettings) -> Option<&'a TransportIdentity> {
        self.this_device
            .as_ref()
            .or_else(|| trust.list().and_then(|l| l.local_peer.as_ref()))
    }

    /// ADR-0062 decision 8's condition: the list read and empty, and
    /// `trust`'s read and empty -- two facts the client holds whole,
    /// never a connection or path signal. An unread list, one whose read
    /// failed, or one a change may have altered since it was read, is
    /// unknown and makes no claim -- as does either list while a change of
    /// it is on its way (`no_reach_claims_nothing_while_a_change_is_out`).
    #[must_use]
    pub fn no_reach(&self, trust: &TrustSettings) -> bool {
        !self.busy()
            && trust.in_flight().is_none()
            && self.held == Held::AsRead
            && self.list.as_ref().is_some_and(AddressList::is_empty)
            && trust.confirmed_list().is_some_and(|l| l.allowed.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(from: usize, to: usize) -> AddressList {
        AddressList {
            addresses: (from..to)
                .map(|i| PeerAddress {
                    address: format!("/ip4/192.168.1.{}/tcp/{i}", i % 250),
                    peer: None,
                    origin: AddressOrigin::AddedHere,
                    dialled: true,
                })
                .collect(),
        }
    }

    /// Changes left unknown with no read between keep at most
    /// `PLACES_BOUND` pending, the oldest going first and the latest
    /// kept; one address is pending once, its latest change.
    #[test]
    fn pending_changes_never_pass_their_bound() {
        let mut peers = Peers::default();
        let address = |n: usize| format!("/ip4/10.0.0.1/tcp/{n}");
        for n in 0..PLACES_BOUND + 10 {
            peers.pend(Check::Undo(address(n)));
            assert!(peers.pending.len() <= PLACES_BOUND, "after {n}");
        }
        assert!(!peers.still_unknown(&address(0), true), "the oldest went");
        assert!(
            peers.still_unknown(&address(PLACES_BOUND + 9), true),
            "the latest kept"
        );
        peers.pend(Check::Undo(address(PLACES_BOUND + 9)));
        assert_eq!(
            peers
                .pending
                .iter()
                .filter(|c| pending_address(c) == Some(address(PLACES_BOUND + 9).as_str()))
                .count(),
            1,
            "once per address"
        );
    }

    /// Reads that each drop every row keep the places of at most
    /// `PLACES_BOUND` rows, every listed one among them.
    #[test]
    fn places_never_pass_their_bound() {
        let mut peers = Peers::default();
        for read in 0..10 {
            let list = rows(read * ADDRESS_BOUND, (read + 1) * ADDRESS_BOUND);
            peers.set_list(list.clone());
            assert!(peers.places.len() <= PLACES_BOUND, "read {read}");
            assert!(
                list.addresses
                    .iter()
                    .all(|a| peers.places.contains(&a.address)),
                "read {read}"
            );
        }
        let newest_dropped = &rows(8 * ADDRESS_BOUND, 9 * ADDRESS_BOUND).addresses;
        assert!(
            newest_dropped
                .iter()
                .all(|a| peers.places.contains(&a.address)),
            "the latest dropped kept"
        );
    }

    /// The row dropped just now keeps its place when the bound evicts,
    /// wherever it stands among the rows: the bound goes by when a row
    /// was dropped, not by its place in the order.
    #[test]
    fn the_bound_evicts_the_row_dropped_longest_ago() {
        let mut peers = Peers::default();
        // Added rows show newest first: the older half shows last, so
        // when a read drops it, it stays after the rows still listed.
        let all = rows(0, PLACES_BOUND);
        let old = rows(0, ADDRESS_BOUND);
        let new = rows(ADDRESS_BOUND, PLACES_BOUND);
        peers.set_list(all);
        peers.set_list(new.clone());
        assert_eq!(peers.places.len(), PLACES_BOUND);
        // The row shown first dropped now, another added: one place too
        // many, and the row dropped now stands before every older one.
        let mut next = new;
        let gone = next.addresses.pop().expect("a row").address;
        next.addresses
            .extend(rows(PLACES_BOUND, PLACES_BOUND + 1).addresses);
        peers.set_list(next);
        assert_eq!(peers.places.first(), Some(&gone), "it stands first");
        assert_eq!(peers.places.len(), PLACES_BOUND);
        let old_left = old
            .addresses
            .iter()
            .filter(|a| peers.places.contains(&a.address))
            .count();
        assert_eq!(old_left, ADDRESS_BOUND - 1, "one dropped long ago went");
    }
}

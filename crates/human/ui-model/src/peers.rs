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
//! [`CODE_LINES`] lines in one add (ADR-0062 decision 4), at most
//! [`TYPED_LINES`] typed, and no DNS name from a scan (decision 1).
//!
//! An add is all or none and is never split: one line is one write, and
//! several need the call that checks them all first (IPC 2.9's
//! `admin.peers.add_addresses`), which this build does not bind; until
//! it does, the facade answers an add of several as a runtime without
//! that call answers. So every failure is
//! one of the note's §5.7 outcomes, and "nothing was changed" is said
//! only where it is true: not written; unknown, where the answer was lost
//! after the request left and the list is read back before anything is
//! said; not permitted; and a runtime too old for the methods.

use interweave_human_client_api::{
    AddressAnswer, AddressChangeFailure, AddressList, AddressOrigin, AddressOutcome,
    AddressProblem, AddressRefused, PeerAddress, TrustProblem,
};
use interweave_transport_api::TransportIdentity;

use crate::model::Intent;
use crate::trust::{Held, TrustOutcome, TrustSettings};

/// The most lines one add carries: a code's bound (ADR-0062 decision 4),
/// and the add method's.
pub const CODE_LINES: usize = 16;

/// The most lines a typed add carries while the add of several lines is
/// not bound: the runtime's add takes one address, an add is never split
/// into single writes, and the note designs no interim state for typed
/// lines (`design/2026-10-22-add-a-peer.md` §5.7), so a second line is
/// refused in the field's own words before anything is sent. It becomes
/// [`CODE_LINES`] when `admin.peers.add_addresses` (IPC 2.9) is bound.
pub const TYPED_LINES: usize = 1;

/// The most addresses a device holds, configured and added together
/// (ADR-0062 decision 2): the `{max}` the at-the-bound words fill. The
/// runtime's bound is the rule; this is its value for the words, held
/// equal to the configuration's by `app-core`'s test.
pub const ADDRESS_BOUND: usize = 64;

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
    /// The add stored nothing: the first refused line, and why.
    Refused {
        /// The line, as sent.
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
    /// An add of several lines, from a scan, not taken: nothing added.
    /// A runtime with the one-address add only (IPC 2.7 or 2.8) answers
    /// so, and so does this build's facade for every such add until the
    /// add of several lines is bound.
    SeveralIncompatible {
        /// How many lines.
        lines: usize,
    },
    /// The boundary cannot be used.
    Barred(Barred),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrustBlock {
    peer: TransportIdentity,
    later: bool,
    /// Trust sent and not answered.
    trusting: bool,
    /// Its answer was lost: the trust list is read back.
    checking: bool,
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
            checking: false,
            problem: None,
            known,
        }
    }

    /// What is known of its peer's trust: the trust list while it is the
    /// daemon's word, else the block's own last known answer.
    fn known(&self, trust: &TrustSettings) -> Option<bool> {
        if self.checking {
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
        } else if self.checking {
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
    /// A step-two block's title.
    Block(TransportIdentity),
    /// A step-two block's problem line.
    BlockProblem(TransportIdentity),
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
    /// The snackbar's action: Try again on a removal or Undo whose
    /// outcome is unknown (§8).
    SnackAction,
    /// The note under Add an address (at the bound, or list off).
    AddAddressNote,
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
    /// The field now reads this.
    FieldChanged(String),
    /// Add pressed.
    Add,
    /// A scan control pressed: the scanner opens only with the field
    /// empty ([`Peers::scan_open`]).
    ScanPressed,
    /// The scanner closed with no code.
    ScannerClosed,
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
    SnackClosed,
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
    /// A removal, or its Try again.
    Remove(String),
}

/// Whose Try again a read on its way answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Again {
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
    /// Opened from this row's Trust, not from Add an address.
    from_row: Option<String>,
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
    /// Where focus went after the last removal, for an Undo refused.
    after_remove: Option<PeersFocus>,
    /// A row an Undo restored while its list was unread, focused once a
    /// read lists it.
    restored: Option<String>,
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
                intents.extend(self.read());
                self.ask_focus(PeersFocus::Title);
                intents
            }
            PeersInput::TrustReadAgain => {
                let read = trust.opened();
                if read.is_some() {
                    self.again = Some(Again::Trust);
                }
                read.into_iter().collect()
            }
            PeersInput::ReadAgain => {
                let read = self.read();
                self.again = read.is_some().then_some(Again::List);
                read.into_iter().collect()
            }
            PeersInput::OpenAdd => {
                self.open_add();
                Vec::new()
            }
            PeersInput::FieldChanged(text) => {
                self.field_changed(text);
                Vec::new()
            }
            PeersInput::Add => self.add_typed().into_iter().collect(),
            PeersInput::ScanPressed => {
                self.scan_pressed();
                Vec::new()
            }
            PeersInput::ScannerClosed => {
                if let Some(page) = self.page.as_mut() {
                    page.scanning = false;
                }
                Vec::new()
            }
            PeersInput::Scanned(text) => self.scanned(&text).1.into_iter().collect(),
            PeersInput::Close => {
                self.close(trust);
                Vec::new()
            }
            PeersInput::Trust(peer) => self.trust(&peer, trust).into_iter().collect(),
            PeersInput::TrustRow(address) => self.trust_row(&address, trust).into_iter().collect(),
            PeersInput::NotNow(peer) => {
                if let Some(block) = self.block_mut(&peer)
                    && !block.trusting
                    && !block.checking
                {
                    block.later = true;
                    block.problem = None;
                }
                self.settle(trust);
                Vec::new()
            }
            PeersInput::Remove(address) => self.remove(address).into_iter().collect(),
            PeersInput::Undo => self.undo().into_iter().collect(),
            PeersInput::TryAgain => self.try_again().into_iter().collect(),
            PeersInput::SnackClosed => {
                // A removal unknown is read again once its snackbar goes,
                // so the row shows what the runtime holds.
                if matches!(self.snack, Some(PeersSnack::RemoveUnknown { .. })) {
                    self.reread = true;
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
        let Some(page) = self.page.as_mut() else {
            return;
        };
        let empty = lines_of(&text).is_empty();
        match page.note {
            Some(AddNote::AddressNeeded) if !empty => page.note = None,
            Some(AddNote::ScanAfterField) if empty => page.note = None,
            _ => {}
        }
        page.field = text;
    }

    /// Whether step one can act: the page open, nothing on its way, and
    /// no result yet.
    fn step_one(&self) -> bool {
        self.in_flight.is_none() && self.check.is_none() && self.step_one_shown()
    }

    /// Add pressed: the field's lines, or why not.
    fn add_typed(&mut self) -> Option<Intent> {
        if !self.step_one() {
            return None;
        }
        let page = self.page.as_mut()?;
        let lines = lines_of(&page.field);
        if lines.is_empty() {
            page.note = Some(AddNote::AddressNeeded);
            self.ask_focus(PeersFocus::Field { select: None });
            return None;
        }
        if lines.len() > TYPED_LINES {
            page.note = Some(AddNote::TooManyLines { lines: lines.len() });
            self.ask_focus(PeersFocus::Field {
                select: Some(LineSelection::From(TYPED_LINES)),
            });
            return None;
        }
        page.note = None;
        Some(self.send(lines, LineSource::Typed))
    }

    /// A scan control pressed: the scanner opens with the field empty;
    /// otherwise the note says to add or clear it first.
    fn scan_pressed(&mut self) {
        if !self.step_one() {
            return;
        }
        let Some(page) = self.page.as_mut() else {
            return;
        };
        if lines_of(&page.field).is_empty() {
            page.scanning = true;
        } else {
            page.note = Some(AddNote::ScanAfterField);
            self.ask_focus(PeersFocus::Note);
        }
    }

    /// A code read, or an image decoded: `text` is all it carries. Taken
    /// whole as one add, never through the field: the client's two rules
    /// first, then the runtime's.
    pub fn scanned(&mut self, text: &str) -> (ScanVerdict, Option<Intent>) {
        let lines = lines_of(text);
        if lines.is_empty() || lines.iter().any(|l| !l.starts_with('/')) {
            return (ScanVerdict::NotAddress, None);
        }
        let ready = self.step_one();
        let Some(page) = self.page.as_mut() else {
            return (ScanVerdict::NotAddress, None);
        };
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
        };
        if let Some((line, why)) = refused {
            page.note = Some(AddNote::Refused { line, why });
            self.ask_focus(PeersFocus::Note);
            return (ScanVerdict::Taken, None);
        }
        page.note = None;
        (
            ScanVerdict::Taken,
            Some(self.send(lines, LineSource::Scanned)),
        )
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
            if block.trusting || block.checking || trust.in_flight().is_some() {
                return None;
            }
            let change = trust.propose_allow_of(peer)?;
            let intent = trust.confirm(&change)?;
            block.trusting = true;
            block.later = false;
            block.problem = None;
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
        let peer = self
            .list
            .as_ref()?
            .addresses
            .iter()
            .find(|a| a.address == address)?
            .peer
            .clone()?;
        // This device's own row offers no Trust and gets no block (§5.3).
        if trust.list().and_then(|l| l.local_peer.as_ref()) == Some(&peer) {
            return None;
        }
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
    /// is Done, leaving any undecided block as Not now. Nothing closes
    /// while an add is on its way, so its answer has a page to land on.
    fn close(&mut self, trust: &TrustSettings) {
        if self.in_flight.is_some() || self.check.is_some() {
            return;
        }
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
        match next {
            Some(peer) => self.ask_focus(PeersFocus::Block(peer)),
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
        // row's trust step, with no add, closes with none.
        if all_trusted {
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

    /// Remove on an added row: at once, with Undo.
    fn remove(&mut self, address: String) -> Option<Intent> {
        if self.in_flight.is_some() || self.check.is_some() {
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
        self.in_flight = Some(InFlight::Remove(address.clone()));
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
        if self.in_flight.is_some() || self.check.is_some() {
            return None;
        }
        match self.snack.clone()? {
            PeersSnack::UndoFailed { address } | PeersSnack::UndoUnknown { address } => {
                self.re_add(address)
            }
            PeersSnack::RemoveFailed { address } | PeersSnack::RemoveUnknown { address } => {
                self.in_flight = Some(InFlight::Remove(address.clone()));
                Some(Intent::RemoveAddress(address))
            }
            _ => None,
        }
    }

    fn re_add(&mut self, address: String) -> Option<Intent> {
        if self.in_flight.is_some() || self.check.is_some() {
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
        let again = self.again.take_if(|a| *a == Again::List).is_some();
        match answer {
            Ok(list) => {
                self.list = Some(list);
                self.list_problem = None;
                self.held = Held::AsRead;
                if again {
                    self.ask_focus(PeersFocus::Title);
                }
                if self.add_note.is_some() && self.add_blocked().is_none() {
                    self.add_note = None;
                }
                if let Some(check) = check {
                    self.checked(check, trust);
                }
                if let Some(address) = self.restored.take() {
                    let focus = self.list.as_ref().and_then(|l| {
                        if listed(l, &address) {
                            Some(PeersFocus::Row(address))
                        } else {
                            shown(l).next().map(|a| PeersFocus::Row(a.address.clone()))
                        }
                    });
                    self.ask_focus(focus.unwrap_or(PeersFocus::AddAddress));
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
                // that was in it goes to the line that says why.
                if self.list.take().is_some() {
                    self.ask_focus(PeersFocus::ListProblem);
                }
                // A row restored while unread is not drawn with no list;
                // its Try again succeeding goes as on opening (§8).
                self.restored = None;
                self.list_problem = Some(match problem {
                    AddressProblem::Unavailable => ListProblem::Unavailable,
                    AddressProblem::Internal => ListProblem::Failed,
                    AddressProblem::NotPermitted => ListProblem::Barred(Barred::NotPermitted),
                    AddressProblem::Incompatible => ListProblem::Barred(Barred::Incompatible),
                });
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
        match &self.in_flight {
            Some(InFlight::Remove(sent)) if sent == address => self.in_flight = None,
            _ => return,
        }
        let before = self.list.clone().unwrap_or_default();
        match answer {
            Ok(answer) => {
                self.take_list(answer.list);
                match answer.outcome {
                    AddressOutcome::Removed | AddressOutcome::Unchanged => {
                        self.removed(address, &before, trust);
                    }
                    // A configured row offers no Remove; any other answer
                    // removed nothing.
                    AddressOutcome::Refused(_) | AddressOutcome::Added { .. } => {
                        self.remove_failed(address);
                    }
                }
            }
            Err(AddressChangeFailure::NotMade(problem)) => match barred(problem) {
                Some(b) => self.bar(b),
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
            Ok(answer) => answer,
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
                    None => AddNote::Failed,
                };
                self.page_note(note, PeersFocus::AddButton);
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
            AddressOutcome::Unchanged => {
                let result = AddResult::Unchanged { lines: lines.len() };
                self.accepted(result, &peers_of(&lines), trust);
            }
            AddressOutcome::Refused(why) => {
                let why = match why {
                    AddressRefused::Grammar => Refusal::Grammar,
                    AddressRefused::SpecialUse => Refusal::SpecialUse,
                    AddressRefused::Bound => Refusal::Limit,
                    AddressRefused::ListOff => Refusal::ListOff,
                    // A removal's refusal, never an add's: nothing written.
                    AddressRefused::Configured => {
                        self.page_note(AddNote::Failed, PeersFocus::AddButton);
                        return;
                    }
                };
                let line = lines.first().cloned().unwrap_or_default();
                let focus = match source {
                    LineSource::Typed => PeersFocus::Field {
                        select: Some(LineSelection::Line(line.clone())),
                    },
                    LineSource::Scanned => PeersFocus::Note,
                };
                self.page_note(AddNote::Refused { line, why }, focus);
            }
            AddressOutcome::Removed => self.page_note(AddNote::Failed, PeersFocus::AddButton),
        }
    }

    fn page_note(&mut self, note: AddNote, focus: PeersFocus) {
        if let Some(page) = self.page.as_mut() {
            page.note = Some(note);
        }
        self.ask_focus(focus);
    }

    /// Step one done: the result line, the field cleared, and one block
    /// per peer of the add not trusted -- every peer while the trust list
    /// is not the daemon's word (unread, or kept after a failed read), since a needless block costs a tap and a missing one
    /// leaves a peer untrusted unprompted.
    fn accepted(&mut self, result: AddResult, peers: &[TransportIdentity], trust: &TrustSettings) {
        let Some(page) = self.page.as_mut() else {
            return;
        };
        page.result = Some(result);
        page.field.clear();
        page.note = None;
        let own = trust.list().and_then(|l| l.local_peer.as_ref());
        for peer in peers {
            if Some(peer) == own
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
                self.ask_focus(PeersFocus::Block(peer));
            }
            None => self.finish(trust),
        }
    }

    fn undo_answered(
        &mut self,
        address: String,
        answer: Result<AddressAnswer, AddressChangeFailure>,
    ) {
        match answer {
            Ok(answer) => {
                self.take_list(answer.list);
                match answer.outcome {
                    AddressOutcome::Added { .. } | AddressOutcome::Unchanged => {
                        self.undone(address);
                    }
                    AddressOutcome::Refused(AddressRefused::Bound) => {
                        self.undo_not_made(PeersSnack::UndoRefused(Refusal::Limit));
                    }
                    AddressOutcome::Refused(AddressRefused::ListOff) => {
                        self.undo_not_made(PeersSnack::UndoRefused(Refusal::ListOff));
                    }
                    AddressOutcome::Refused(_) | AddressOutcome::Removed => {
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
        self.ask_focus(focus);
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
            Check::Add(lines) => self.page_note(
                AddNote::Unknown { lines: lines.len() },
                PeersFocus::AddButton,
            ),
            // Unknown, the snackbar's Try again is where to go next: the
            // row menu that opened it may be gone (§8).
            Check::Undo(address) => {
                self.say(PeersSnack::UndoUnknown { address });
                self.ask_focus(PeersFocus::SnackAction);
            }
            Check::Remove { address, .. } => {
                self.say(PeersSnack::RemoveUnknown { address });
                self.ask_focus(PeersFocus::SnackAction);
            }
        }
    }

    /// A change's list read back, or, when it failed, a read owed.
    fn take_list(&mut self, list: Result<AddressList, AddressProblem>) {
        if let Ok(list) = list {
            self.list = Some(list);
            self.list_problem = None;
            self.held = Held::AsRead;
        } else {
            self.reread = true;
            self.held = Held::MayHaveChanged;
        }
    }

    /// Focus after a removal: the next row shown, else the previous,
    /// else Add an address, in `list` as it stood -- kept, so an Undo
    /// refused goes as after the Remove (§8).
    fn focus_after(&mut self, list: &AddressList, address: &str) {
        let rows: Vec<&str> = shown(list).map(|a| a.address.as_str()).collect();
        let target = rows.iter().position(|r| *r == address).and_then(|at| {
            rows.get(at + 1)
                .or_else(|| at.checked_sub(1).and_then(|p| rows.get(p)))
        });
        let focus = target.map_or(PeersFocus::AddAddress, |row| {
            PeersFocus::Row((*row).to_owned())
        });
        self.after_remove = Some(focus.clone());
        self.ask_focus(focus);
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
                match self.trust_line(trust) {
                    None => self.ask_focus(PeersFocus::Title),
                    Some(_) => self.ask_focus(PeersFocus::TrustRetry),
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
        let mut problem = None;
        let mut moved = false;
        for block in &mut page.blocks {
            let known_before = block.known;
            if let Some(list) = trust.confirmed_list() {
                block.known = Some(list.allows(&block.peer));
            }
            // A read that shows the peer trusted decides the block as its
            // own Trust would: the step settles (§5.3).
            if block.known == Some(true)
                && known_before != Some(true)
                && !block.trusting
                && !block.checking
            {
                moved = true;
            }
            let on_its_way = trust.in_flight().is_some_and(|c| c.peer == block.peer);
            if block.problem.is_some() && block.allowed(trust) {
                block.problem = None;
                moved = true;
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
                        Some(TrustOutcome::Unconfirmed(c)) if c.peer == block.peer => {
                            block.checking = true;
                            block.known = None;
                        }
                        Some(TrustOutcome::Problem(p)) => {
                            block.problem = Some(BlockProblem::Trust(*p));
                        }
                        _ => block.problem = Some(BlockProblem::NotMade),
                    }
                }
            } else if block.checking && !trust.reading() && !trust.reread_owed() {
                block.checking = false;
                moved = true;
                if !block.allowed(trust) {
                    block.problem = Some(match trust.outcome().0 {
                        Some(TrustOutcome::NotReadAgain(_)) => BlockProblem::Unknown,
                        _ => BlockProblem::NotMade,
                    });
                }
            }
            if moved && problem.is_none() && block.problem.is_some() && !block.checking {
                problem = Some(block.peer.clone());
            }
        }
        if !moved {
            return;
        }
        match problem {
            Some(peer) => self.ask_focus(PeersFocus::BlockProblem(peer)),
            None => self.settle(trust),
        }
    }

    fn say(&mut self, snack: PeersSnack) {
        self.snack = Some(snack);
        self.snacks = self.snacks.wrapping_add(1);
    }

    fn ask_focus(&mut self, focus: PeersFocus) {
        self.focus = Some(focus);
        self.focuses = self.focuses.wrapping_add(1);
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
        let own = trust.list().and_then(|l| l.local_peer.as_ref());
        let row = |a: &PeerAddress| PeerRow {
            address: a.address.clone(),
            peer: a.peer.clone(),
            origin: a.origin,
            state: match trust.confirmed_list() {
                _ if !a.dialled => RowState::ProviderOff,
                // This profile's identity never changes, so the list as
                // kept answers it.
                _ if a.peer.is_some() && a.peer.as_ref() == own => RowState::ThisDevice,
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

    /// Whether an add, a removal or the check of a lost answer is on its
    /// way.
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.in_flight.is_some() || self.check.is_some()
    }

    /// The snackbar, and a count that changes with each.
    #[must_use]
    pub const fn snack(&self) -> (Option<&PeersSnack>, u64) {
        (self.snack.as_ref(), self.snacks)
    }

    /// Where focus is asked to go, and a count that changes with each.
    #[must_use]
    pub const fn focus(&self) -> (Option<&PeersFocus>, u64) {
        (self.focus.as_ref(), self.focuses)
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

    /// ADR-0062 decision 8's condition: the list read and empty, and
    /// `trust`'s read and empty -- two facts the client holds whole,
    /// never a connection or path signal. An unread list, one whose read
    /// failed, or one a change may have altered since it was read, is
    /// unknown and makes no claim.
    #[must_use]
    pub fn no_reach(&self, trust: &TrustSettings) -> bool {
        self.held == Held::AsRead
            && self.list.as_ref().is_some_and(AddressList::is_empty)
            && trust.confirmed_list().is_some_and(|l| l.allowed.is_empty())
    }
}

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

use std::collections::BTreeMap;

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
    /// Opened from this row's Trust, not from Add an address.
    from_row: Option<String>,
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
    /// A row an Undo restored while its list was unread, focused once a
    /// read lists it.
    restored: Option<String>,
    /// The removal or Undo left unknown -- its answer lost, its read back
    /// failed -- as the check a read decides it by. It stays unknown until
    /// a read shows it or the runtime answers it (§5.7); the read speaks
    /// for it only where its line still holds its place
    /// ([`Self::read_decides`]).
    unknown_change: Option<Check>,
    /// The focus requests counted when the unknown line was said: a
    /// request since is a later act's, whose focus a
    /// verdict on the earlier change never takes
    /// (`a_verdict_on_an_earlier_unknown_change_never_takes_a_later_ones_place`).
    unknown_mark: u64,
    /// Peers a Trust from this page was answered as made for while the
    /// trust list is not the daemon's word: their rows read Trusted
    /// (§5.2) until a confirmed list speaks for them. At most one per
    /// step-two block.
    trusted_here: Vec<TransportIdentity>,
    /// This node's `PeerId`, from the session; else the trust list as
    /// last read names it.
    this_device: Option<TransportIdentity>,
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
/// The first line naming this device's own `PeerId`, which one refuses
/// the whole add before anything is sent (add-a-peer §5.3).
fn own_line(lines: &[String], own: Option<&TransportIdentity>) -> Option<String> {
    let own = own?;
    lines
        .iter()
        .find(|l| PeerAddress::peer_of(l).as_ref() == Some(own))
        .cloned()
}

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
            PeersInput::FieldChanged(text) => {
                self.field_changed(text);
                Vec::new()
            }
            PeersInput::Add => self.add_typed(trust).into_iter().collect(),
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
            PeersInput::Scanned(text) => self.scanned(&text, trust).1.into_iter().collect(),
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
    fn add_typed(&mut self, trust: &TrustSettings) -> Option<Intent> {
        if !self.step_one() {
            return None;
        }
        let own = self.own(trust).cloned();
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
    pub fn scanned(&mut self, text: &str, trust: &TrustSettings) -> (ScanVerdict, Option<Intent>) {
        let lines = lines_of(text);
        if lines.is_empty() || lines.iter().any(|l| !l.starts_with('/')) {
            return (ScanVerdict::NotAddress, None);
        }
        let ready = self.step_one();
        let own = self.own(trust).cloned();
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
                .or_else(|| own_line(&lines, own.as_ref()).map(|l| (l, Refusal::OwnAddress)))
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
        let peer = self
            .list
            .as_ref()?
            .addresses
            .iter()
            .find(|a| a.address == address)?
            .peer
            .clone()?;
        // This device's own row offers no Trust and gets no block (§5.3).
        if self.own(trust) == Some(&peer) {
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
    fn close(&mut self, trust: &mut TrustSettings) {
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

    /// Whether this address's removal or Undo was left unknown: a retry
    /// of it that writes nothing leaves it so.
    fn still_unknown(&self, address: &str) -> bool {
        matches!(
            &self.unknown_change,
            Some(Check::Remove { address: a, .. } | Check::Undo(a)) if a == address
        )
    }

    /// A retry of an unknown change that wrote nothing: its unknown line
    /// is said again -- the snackbar shown anew if it had closed -- read to
    /// the screen reader once more, focus on its Try again (§5.7, §8).
    fn still_unknown_again(&mut self) {
        match self.unknown_change.clone() {
            Some(Check::Undo(address)) => self.say(PeersSnack::UndoUnknown { address }),
            Some(Check::Remove { address, .. }) => self.say(PeersSnack::RemoveUnknown { address }),
            Some(Check::Add(_)) | None => {}
        }
        self.ask_focus(PeersFocus::SnackAction);
        self.unknown_mark = self.focuses;
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
        let again = self.again.take_if(|a| *a != Again::Trust);
        match answer {
            Ok(list) => {
                self.set_list(list);
                self.list_problem = None;
                self.held = Held::AsRead;
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
                // An answer decides it: nothing is unknown about it now.
                if self.still_unknown(address) {
                    self.unknown_change = None;
                }
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
                // A Try again that wrote nothing settles nothing: after an
                // unknown removal the change stays unknown (§5.7).
                None if self.still_unknown(address) => self.still_unknown_again(),
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
                    AddressRefused::OwnPeer => Refusal::OwnAddress,
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
        page.result = Some(result);
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
                if self.still_unknown(&address) {
                    self.unknown_change = None;
                }
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
                } else if self.still_unknown(&address) {
                    // As for a removal: an unknown Undo stays unknown.
                    self.still_unknown_again();
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
                self.say(PeersSnack::UndoUnknown { address });
                self.ask_focus(PeersFocus::SnackAction);
                self.unknown_mark = self.focuses;
            }
            Check::Remove { address, before } => {
                self.unknown_change = Some(Check::Remove {
                    address: address.clone(),
                    before,
                });
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
            if on_show || (self.snack.is_none() && !moved) {
                self.decide_keeping_focus(change, moved, trust);
            }
        }
        let note_on_show = self
            .page
            .as_ref()
            .is_some_and(|p| matches!(p.note, Some(AddNote::Unknown { .. })));
        if let Some(lines) = self.page.as_mut().and_then(|p| p.unknown.take())
            && note_on_show
        {
            self.decide_keeping_focus(Check::Add(lines), moved, trust);
        }
    }

    /// `checked`, leaving focus where a later request put it if `kept`.
    fn decide_keeping_focus(&mut self, change: Check, kept: bool, trust: &TrustSettings) {
        let focus = (self.focus.clone(), self.focuses);
        self.checked(change, trust);
        if kept {
            (self.focus, self.focuses) = focus;
        }
    }

    /// A list that came with another change's answer: it ends what was
    /// unknown only where it shows that change not made -- an add's line
    /// missing, a removed address still listed, an undone one absent --
    /// and leaves the rest unknown, its answer being another row's.
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
        self.ask_focus(focus);
    }

    /// What the root carries out between turns: the read a change left
    /// owed, and step two's blocks once trust answered.
    pub fn turn(&mut self, trust: &TrustSettings) -> Option<Intent> {
        self.trust_answers(trust);
        if trust.confirmed_list().is_some() {
            self.trusted_here.clear();
        }
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
                            if !self.trusted_here.contains(&block.peer) {
                                self.trusted_here.push(block.peer.clone());
                            }
                        }
                        Some(TrustOutcome::Unconfirmed(c)) if c.peer == block.peer => {
                            block.checking = true;
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
        self.focus = Some(focus);
        self.focuses = self.focuses.wrapping_add(1);
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

    /// Whether the focus asked for is a row a read showed gone, removed
    /// elsewhere: `PeerRowGone` is announced before focus moves (§8).
    #[must_use]
    pub fn focus_row_gone(&self) -> bool {
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
                None if a
                    .peer
                    .as_ref()
                    .is_some_and(|p| self.trusted_here.contains(p)) =>
                {
                    RowState::Trusted
                }
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
    /// unknown and makes no claim.
    #[must_use]
    pub fn no_reach(&self, trust: &TrustSettings) -> bool {
        self.held == Held::AsRead
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

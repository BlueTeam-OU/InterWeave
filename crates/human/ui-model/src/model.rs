// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The UI model: presentation state reduced from what the composition
//! root tells it, and the [`Intent`]s a person's actions raise. Pure and
//! synchronous (agreed item 1): it holds no facade and no store, does no
//! I/O, and never re-opens anything -- it shows `Reconnecting` while the
//! facade does.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use interweave_human_chat_protocol::{HumanChatV2, Rendered, is_allowed_link_scheme, render};
use interweave_human_client_api::{
    AddressAnswer, AddressChangeFailure, AddressList, AddressProblem, ClientEvent, Connectivity,
    Destination, Diagnostics, Origin, OutboundStatus, OutboundUpdate, SendError, SessionState,
    ShowAddresses,
};
use interweave_human_core::RowId;
use interweave_transport_api::{
    ChannelId, EndpointId, PeerPath, TransportError, TransportIdentity,
};

use crate::connect::{Connect, ConnectFocus, ConnectGate, ConnectInput, ConnectState};
use crate::labels::{
    ErrorClass, LabelKey, UiText, fill, outbound_label, placeholder_en, send_error_class,
    session_problem_class, short_peer,
};
use crate::look::Avatar;
use crate::peers::{Peers, PeersInput, ScanVerdict};
use crate::trust::{TrustChange, TrustInput, TrustSettings};

pub(crate) mod cards;
pub(crate) mod directory;
pub(crate) mod summaries;
mod swipe;
pub(crate) mod window;

pub use swipe::{
    Announce, Confirmation, ContactChange, FocusTarget, Outcome, Released, RowAction, Snackbar,
    SwipeOffer, UndoClosed,
};

/// How many items the model holds that the store no longer does --
/// read-and-unkept inbound and terminal outbound -- before it evicts the
/// oldest of them (agreed item 3g). Everything else can be re-listed from
/// the store, so nothing else is evicted.
pub const SESSION_ITEM_CAP: usize = 1_024;

/// The bounds the model holds itself to: the published caps, which a test
/// build alone may set lower (`set_bounds_for_test`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Bounds {
    /// [`SESSION_ITEM_CAP`].
    pub(super) session_items: usize,
    /// [`window::HELD_ROW_CAP`].
    pub(super) held_rows: usize,
    /// [`cards::READ_PROPOSAL_CAP`].
    pub(super) read_proposals: usize,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            session_items: SESSION_ITEM_CAP,
            held_rows: window::HELD_ROW_CAP,
            read_proposals: cards::READ_PROPOSAL_CAP,
        }
    }
}

/// How many outbound updates the model holds for rows not listed yet,
/// latest per row, the oldest dropped past it (agreed U3b).
pub const HELD_UPDATE_CAP: usize = 1_024;

/// How many (origin, application id) pairs the model remembers, to attach
/// a copy to the item already shown, oldest forgotten first. The same
/// bound holds the released rows a stale listing cannot bring back.
pub const DEDUP_CAP: usize = 4_096;

/// A conversation: a direct route or a channel (`human-client-ui.md` §4).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConversationKey {
    /// A direct conversation, keyed by the remote route.
    Direct {
        /// The remote peer.
        peer: TransportIdentity,
        /// The remote endpoint, or `None` for the peer's default.
        endpoint: Option<EndpointId>,
    },
    /// A channel conversation.
    Channel(ChannelId),
}

/// An `EndpointId` as a view shows it: a routing label, never an
/// identity, a person or an authenticated role (`human-client-ui.md` §3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteLabel(String);

impl RouteLabel {
    /// The label's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&EndpointId> for RouteLabel {
    fn from(endpoint: &EndpointId) -> Self {
        Self(endpoint.as_str().to_owned())
    }
}

/// What trust the view may show for a peer: every peer is "not verified
/// by this client", never an invented value. The trust settings' allowlist
/// says which peers this profile admits, not who a peer is, so it is no
/// source for this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// No verification by this client.
    NotVerifiedByThisClient,
}

/// What a view asks of the composition root: a legal intent, an edit to
/// a draft, or an input to the trust settings. It is the view's output,
/// defined here rather than in the toolkit crate so a root that names no
/// toolkit can handle it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewEvent {
    /// A legal intent, resolved against the model at take time.
    Intent(Intent),
    /// The person edited a conversation's draft: the root passes it to
    /// [`UiModel::draft_changed`] before taking the view's events again.
    DraftChanged {
        /// The conversation.
        key: ConversationKey,
        /// The draft as it now reads.
        draft: String,
    },
    /// The person acted in the trust settings: the root passes it to
    /// [`TrustSettings::input`] before taking the view's events again, and
    /// carries out the intent it returns.
    Trust(TrustInput),
    /// The person acted on Peers or its add page: the root passes it to
    /// [`UiModel::peers_input`] and carries out the intents it returns.
    Peers(PeersInput),
    /// The person acted on onboarding's Connect: the root passes it to
    /// [`UiModel::connect_input`] and carries out the intents it returns.
    Connect(ConnectInput),
    /// The person pressed the Undo of the snackbar with this id: the root
    /// passes it to [`UiModel::undo`] and carries out what it returns.
    Undo(u64),
    /// The snackbar with this id closed without an Undo: the root passes
    /// it to [`UiModel::snackbar_closed`].
    SnackbarClosed(u64),
    /// The person copied this conversation's identifier from Details, on a
    /// platform that confirms nothing itself: the root passes it to
    /// [`UiModel::identifier_copied`] (#304 F11).
    IdentifierCopied(ConversationKey),
}

/// Which table an item's row is in: a row id names a row within one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Table {
    /// `pending_outbound`.
    Pending,
    /// `unread_inbound`.
    Unread,
    /// `kept_inbound`.
    Kept,
}

/// A stable key for a list item, so a view keeps focus and selection
/// when the list changes (agreed item 3b). An item keeps the key it was
/// first shown under for the session, even when its row moves table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemKey(Table, RowId);

/// An item's key as text: its table's letter and its row, `p12` for a
/// pending row, `u7` unread, `k3` kept. A process that lists the store
/// afresh shows each item under the row it holds now, so the text names
/// the same item to the next process while the row stays in its table --
/// what a command line or a script hands back (`apps/human-cli`), and
/// [`UiModel::item_named`] finds again. Nothing parses it into a row: a
/// row id comes only from the store.
impl std::fmt::Display for ItemKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let letter = match self.0 {
            Table::Pending => 'p',
            Table::Unread => 'u',
            Table::Kept => 'k',
        };
        write!(f, "{letter}{}", self.1.get())
    }
}

/// An inbound message's retention state (`human-client-ui.md` §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retention {
    /// Durable until read.
    Unread,
    /// Read and not kept: shown this session only.
    ReadEphemeral,
    /// Read and kept by this receiver.
    Kept,
}

/// Which way a message went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Sent by this client.
    Outbound,
    /// Received.
    Inbound,
}

/// A message's status, for its label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemStatus {
    /// An outbound row's status, as the facade reports it.
    Outbound(OutboundStatus),
    /// An inbound message's retention state.
    Inbound(Retention),
}

/// What a message replies to, if anything (`human-client-ui.md` §15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The referenced message is shown in this session.
    Present(ItemKey),
    /// It is not held here: show a neutral placeholder. Never fetched,
    /// never a reason to doubt the message.
    Unavailable(String),
}

/// One message as a view renders it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageItem {
    /// Its stable key.
    pub key: ItemKey,
    /// Which way it went.
    pub direction: Direction,
    /// Its status.
    pub status: ItemStatus,
    /// The status's label key.
    pub label: LabelKey,
    /// Whether a kept row holds this message too: an unread re-sent copy
    /// of a kept message shows both (agreed U1b).
    pub kept: bool,
    /// The text, rendered within the subset.
    pub body: Rendered,
    /// The text as written: always viewable (`human-client-ui.md` §4).
    pub source: String,
    /// The authenticated sender of an inbound message, or `None` for
    /// this client's own.
    pub author: Option<TransportIdentity>,
    /// The route label the sender asserted, display only.
    pub route_label: Option<RouteLabel>,
    /// What it replies to.
    pub reply: Option<Reply>,
    /// When the sender says it sent it: peer-asserted, display only,
    /// never an order (A3).
    pub sent_at_ms: Option<u64>,
    /// The local wall-clock time the list is ordered by.
    pub local_at: u64,
    /// The identity's own card this row sent, drawn in place of a text
    /// (contact-cards section 5.5); `None` for a message.
    pub card: Option<cards::SentCard>,
}

/// One conversation in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSummary {
    /// Its key.
    pub key: ConversationKey,
    /// Its title: a channel's name, or for a direct route the
    /// authenticated short `PeerId` and the route label the peer asserts,
    /// shown as a label (agreed item 3c). Never the envelope's
    /// `from_endpoint` or anything in the text.
    pub title: String,
    /// The title as a view draws it: the same, a contact's name inside a
    /// first-strong isolate ([`crate::labels::isolated`]), since a drawn
    /// title sits in other text -- a row, a header, an announcement. A
    /// terminal or a script gets [`ConversationSummary::title`], plain.
    pub drawn_title: String,
    /// How many of its messages are unread.
    pub unread: usize,
    /// The latest local time of any of its messages.
    pub last_activity: u64,
    /// Its avatar (design/2026-10-16-android-look.md section 5.1).
    pub avatar: Avatar,
    /// The last message it holds in this session, which the list
    /// previews (section 5.2). There is always one: the list holds a
    /// conversation only while it holds one of its messages. Previewing it
    /// reads nothing: this is the model's state, handed out through
    /// `&self`, and only a focused view of the conversation marks a
    /// message read.
    pub last: LastMessage,
}

/// The last message of a conversation, as the list previews it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastMessage {
    /// Its key.
    pub key: ItemKey,
    /// Which way it went: one's own is previewed as such.
    pub direction: Direction,
    /// Its status's label.
    pub label: LabelKey,
    /// The text, rendered within the subset: the preview is its first
    /// line, as the conversation draws it.
    pub body: Rendered,
    /// The local time it reached this app: never the time a sender
    /// asserts.
    pub local_at: u64,
    /// A card row, previewed as that: one's own, or in the windowed mode
    /// one someone sent (`direction` tells which), never by its text.
    pub card: bool,
    /// Its first line, cut to the preview's bound ([`crate::preview`]):
    /// the same whether the row is held or only the store's newest.
    pub preview: crate::preview::Preview,
}

impl LastMessage {
    /// Whether it is one's own and still on its way: the list shows a
    /// clock in the badge's place for it when nothing is unread (section
    /// 5) -- the two statuses a conversation draws with a clock (section
    /// 6). One waiting for the person is not: it shows its error in the
    /// conversation.
    #[must_use]
    pub const fn pending(&self) -> bool {
        matches!(self.label, LabelKey::Sending | LabelKey::NotConfirmed)
    }
}

/// A session state a person can act on (agreed item 3f).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionNotice {
    /// No transport daemon serves this profile: the window connects on its
    /// own once one starts. Guidance only -- starting the daemon is the
    /// operator's (architect-cto's Q9 ruling, relay seq 11163).
    NoDaemon,
    /// The facade is re-opening on its own.
    Reconnecting,
    /// Storage cannot hold new messages; [`Intent::RecheckStorage`]
    /// resolves it once space is freed.
    StorageDegraded,
    /// Opening was refused; [`Intent::Reopen`] tries again.
    Refused(ErrorClass),
    /// The platform gives this app no network access: reading and
    /// keeping go on, as local writes, and nothing is sent
    /// (human-client-android.md, "Runtime permissions and the
    /// network-denied state"); [`Intent::AllowNetwork`] asks the person
    /// for it.
    NetworkDenied,
    /// The profile's key needs the person and the window of their last
    /// authentication has passed: nothing runs until they authenticate,
    /// which [`Intent::Authenticate`] asks of them. Nothing runs on its
    /// own meanwhile, so nothing here says the app will connect
    /// (android-key-custody.md's failure list; #309 acceptance 10).
    AuthenticationNeeded,
}

impl SessionNotice {
    /// The intent that resolves it, if a person can.
    #[must_use]
    pub const fn resolution(self) -> Option<Intent> {
        match self {
            Self::NoDaemon | Self::Reconnecting => None,
            Self::StorageDegraded => Some(Intent::RecheckStorage),
            Self::Refused(_) => Some(Intent::Reopen),
            Self::NetworkDenied => Some(Intent::AllowNetwork),
            Self::AuthenticationNeeded => Some(Intent::Authenticate),
        }
    }
}

/// What a person's action asks the composition root to do (agreed items
/// 1, 3e). The only way anything leaves the model: a view never calls the
/// facade or the store. None of these touches trust, administration or
/// recovery -- `human-client-ui.md` §13's tests assert that by
/// exhaustive match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Mark an unread row read (deletes its durable copy unless kept).
    MarkRead(RowId),
    /// Keep a read message: only after it was read.
    Keep {
        /// The item to keep.
        item: ItemKey,
        /// The row whose read or unkeep handed this session the content:
        /// the root keeps the copy that row's store call returned.
        from: (Table, RowId),
    },
    /// Remove Keep from a kept row.
    Unkeep(RowId),
    /// Retry a pending row now.
    Retry(RowId),
    /// Cancel a pending row.
    Cancel(RowId),
    /// Hide a read, unkept message for the rest of the session: writes
    /// nothing, and the root carries it out on the model
    /// ([`UiModel::hide`]).
    Hide(ItemKey),
    /// Hide a conversation from the list, every message in it as `Hide`
    /// hides one ([`UiModel::hide_conversation`]).
    HideConversation(ConversationKey),
    /// Take a cancel back: restore the pending record the cancel of this
    /// row deleted. Raised only by [`UiModel::undo`].
    Restore(RowId),
    /// Send the conversation's draft.
    Send {
        /// Where.
        key: ConversationKey,
        /// What.
        draft: String,
    },
    /// Open an allowlisted link a person activated.
    OpenLink(String),
    /// Leave a refused session: try opening again.
    Reopen,
    /// Re-check storage now.
    RecheckStorage,
    /// Ask the person for the network access the platform withholds: its
    /// prompt while it still offers one, else the app's settings. Carried
    /// out by the platform, never by the facade.
    AllowNetwork,
    /// Raise the platform's authentication prompt again, for a profile
    /// whose key needs the person. Carried out by the platform, never by
    /// the facade.
    Authenticate,
    /// Read the trust allowlist: the trust settings opened, or a change's
    /// list was not read back ([`crate::TrustSettings::take_reread`]).
    ReadTrust,
    /// Accept the card a proposal holds: write the contact it said it
    /// would. Only [`UiModel::card_actions`] offers it.
    AcceptCard(RowId),
    /// Decline it: discard the card, writing nothing.
    DeclineCard(RowId),
    /// Take back the Accept or Decline of this proposal: only the open
    /// snackbar's Undo makes one ([`UiModel::undo`]).
    UndoCard(RowId),
    /// Take back the delete of this contact, or the removal of its
    /// photo: only the open snackbar's Undo makes one ([`UiModel::undo`]),
    /// and the facade side writes back what it held for it.
    UndoContact {
        /// The contact.
        contact_id: String,
        /// What is taken back.
        change: swipe::ContactChange,
        /// The facade side's hold for this change: one per change, so
        /// a release or an Undo names this change and no other of the
        /// same contact.
        hold: u64,
    },
    /// Send this conversation's identity's own card to its route. Only
    /// [`UiModel::share_card`] makes one, for a direct conversation.
    ShareCard(ConversationKey),
    /// Ask what making a new key for the identity in front would cancel:
    /// the facade answers before the confirmation opens
    /// ([`UiModel::new_key_asked`]).
    AskNewKey,
    /// Make a new key for the identity in front, as the person confirmed
    /// (design/2026-10-24-identity-key.md section 5).
    MakeNewKey,
    /// Open the identity in front's key once more (section 6).
    RetryKey,
    /// Close what the card page shows of a key step: the confirmation
    /// cancelled, or the page left. The model's alone.
    CloseKeyStep,
    /// A photo chosen for the own card, as its file holds it: the facade
    /// side prepares it (contact-cards design section 5.2), and the editor
    /// shows what it came to ([`UiModel::own_photo`]).
    ChooseOwnPhoto(Vec<u8>),
    /// The own card's editor closed without saving its chosen photo.
    ForgetOwnPhoto,
    /// Remove photo in the own card's editor: no photo once saved, and a
    /// photo being prepared is let go.
    RemoveOwnPhoto,
    /// The person's edit of their identity's own card, from its settings.
    SetOwnCard {
        /// The name.
        name: String,
        /// The surname, if any.
        surname: Option<String>,
        /// What happens to the photo.
        photo: cards::PhotoEdit,
    },
    /// The person's own name for a contact: theirs from now on, and no
    /// card replaces it.
    RenameContact {
        /// The contact.
        contact_id: String,
        /// The name.
        name: String,
    },
    /// The person's own photo for a contact, or its removal: theirs from
    /// now on, and no card replaces it.
    SetContactPhoto {
        /// The contact.
        contact_id: String,
        /// What happens to the photo.
        photo: cards::PhotoEdit,
    },
    /// Delete a contact, its routes and their recorded keys: the only way
    /// a contact goes (ADR-0059 decision 3). Only [`UiModel::delete_contact`]
    /// makes one, for a contact the model holds.
    DeleteContact {
        /// The contact.
        contact_id: String,
    },
    /// Delete one route of a contact and the key it recorded, the contact
    /// staying with its others (ADR-0059 decision 9). Only
    /// [`UiModel::delete_route`] makes one, for a route the model holds
    /// under a contact with another: the last route goes only with its
    /// contact.
    DeleteRoute {
        /// The contact.
        contact_id: String,
        /// The route's `PeerId`.
        peer: TransportIdentity,
        /// The route's `EndpointId`.
        endpoint: EndpointId,
    },
    /// Read the addresses this device holds for reaching peers (ADR-0062):
    /// Peers opened, the app started, or a change's list was not read
    /// back.
    ReadAddresses,
    /// Add these lines, all or none: the add page's typed lines or a
    /// scanned code, or an Undo's one address. Only [`crate::Peers`]
    /// makes one, after the client's own two rules.
    AddAddresses(Vec<String>),
    /// Remove an address added here.
    RemoveAddress(String),
    /// Read this device's own addresses for Show (ADR-0062 decision 4):
    /// only on the person's act, never otherwise.
    ReadOwnAddresses,
    /// Make a trust change the person confirmed in the trust settings,
    /// which showed its exact `PeerId` and scope (`human-client-ui.md` §8).
    /// Only [`crate::TrustSettings::confirm`] makes one.
    SetTrust(TrustChange),
}

/// How often a composer was edited, and at which count Send was pressed.
#[derive(Debug, Clone, Copy, Default)]
struct Edits {
    revision: u64,
    pressed: Option<u64>,
}

/// A conversation's composer: its draft and why the last send was
/// refused, if it was (agreed item 2b).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Composer {
    /// The text being written.
    pub draft: String,
    /// Why the facade refused the last send, with no row.
    pub refused: Option<ErrorClass>,
}

/// An outbound message the store holds pending, as the root read it
/// (agreed item 2a: at start, so a restart has content to show).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedOutbound {
    /// Its pending row.
    pub row: RowId,
    /// Where it is going.
    pub destination: Destination,
    /// The envelope it carries: for an own card, one holding only the
    /// card's id, its text empty.
    pub envelope: HumanChatV2,
    /// When it was composed, on the wall clock.
    pub created_at: u64,
    /// The identity's own card it carries, as sent; `None` for a message.
    pub card: Option<cards::SentCard>,
}

impl ListedOutbound {
    /// A pending row holding the identity's own card, read back as it went:
    /// held under an envelope that carries only the card's id, as
    /// [`UiModel::card_queued`] holds one queued this session.
    #[must_use]
    pub fn own_card(
        row: RowId,
        destination: Destination,
        app_message_id: String,
        card: cards::SentCard,
        created_at: u64,
    ) -> Self {
        Self {
            row,
            destination,
            envelope: card_envelope(app_message_id),
            created_at,
            card: Some(card),
        }
    }
}

/// An inbound message the store holds, as the root read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedInbound {
    /// Its row.
    pub row: RowId,
    /// Who sent it.
    pub origin: Origin,
    /// The envelope.
    pub envelope: HumanChatV2,
    /// When it was received, on the wall clock.
    pub received_at: u64,
}

/// An outbound row's raw failure, for the diagnostics view only: kept
/// off [`MessageItem`] so a view cannot render a raw code by accident
/// (`human-client-ui.md` §12, agreed U2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemDiagnostics {
    /// The last transport failure the facade reported for the row.
    pub last_code: Option<TransportError>,
}

#[derive(Debug, Clone)]
struct Item {
    conversation: ConversationKey,
    direction: Direction,
    /// The store rows this item stands for: its own, and any duplicate
    /// attached to it (agreed U1). Empty once only this session holds it.
    rows: Vec<(Table, RowId)>,
    envelope: HumanChatV2,
    body: Rendered,
    author: Option<TransportIdentity>,
    route_label: Option<RouteLabel>,
    outbound: Option<OutboundStatus>,
    last_code: Option<TransportError>,
    local_at: u64,
    /// The last row whose release handed this session the content -- a
    /// read of an unread row, or an unkeep of a kept one. Keep is offered
    /// only with one, since the store keeps only content it handed back.
    released_from: Option<(Table, RowId)>,
    /// The pending row an outbound message last held: its first, or the
    /// one an undone cancel restored it as. A cancel names it.
    last_pending: Option<RowId>,
    /// Hidden by the person (Hide, a conversation hide, a cancel): not
    /// shown, and dropped once the snackbar that could undo it closes.
    hidden: bool,
    /// The own card an outbound row carries, as sent.
    card: Option<cards::SentCard>,
}

impl Item {
    fn has(&self, table: Table) -> bool {
        self.rows.iter().any(|(t, _)| *t == table)
    }

    fn row_in(&self, table: Table) -> Option<RowId> {
        self.rows.iter().find(|(t, _)| *t == table).map(|(_, r)| *r)
    }

    /// An inbound item is Unread while ANY of its rows is, Kept while any
    /// is kept and none is unread, and read-ephemeral once none is held
    /// (agreed U1b).
    fn retention(&self) -> Retention {
        if self.has(Table::Unread) {
            Retention::Unread
        } else if self.has(Table::Kept) {
            Retention::Kept
        } else {
            Retention::ReadEphemeral
        }
    }

    fn status(&self) -> ItemStatus {
        self.outbound.as_ref().map_or_else(
            || ItemStatus::Inbound(self.retention()),
            |s| ItemStatus::Outbound(s.clone()),
        )
    }

    fn label(&self) -> LabelKey {
        match &self.outbound {
            Some(status) => outbound_label(status),
            None => match self.retention() {
                Retention::Unread => LabelKey::Unread,
                Retention::ReadEphemeral => LabelKey::ReadNotKept,
                Retention::Kept => LabelKey::Kept,
            },
        }
    }
}

/// The human client's presentation state.
#[derive(Debug)]
pub struct UiModel {
    items: BTreeMap<ItemKey, Item>,
    /// Which item a store row belongs to: a row is looked up, never
    /// searched for (review F9).
    by_row: HashMap<(Table, RowId), ItemKey>,
    /// The items holding an application id in a conversation, for replies.
    by_app: HashMap<(ConversationKey, String), Vec<ItemKey>>,
    /// Session-only items (no store row), oldest first, each once.
    ephemeral: VecDeque<ItemKey>,
    ephemeral_set: HashSet<ItemKey>,
    /// Every item an (origin, application id) is shown as: one per
    /// distinct envelope, since new text under an old id is a new item.
    seen: HashMap<(String, String), Vec<ItemKey>>,
    seen_order: VecDeque<(String, String)>,
    /// Rows the store no longer holds (read, unkept, terminal): a listing
    /// taken before the release cannot bring one back. Row ids are never
    /// reused (AUTOINCREMENT), so a released id is never a new row.
    released: HashSet<(Table, RowId)>,
    released_order: VecDeque<(Table, RowId)>,
    /// Outbound updates for rows not listed yet (agreed U3).
    held: HashMap<RowId, OutboundUpdate>,
    held_order: VecDeque<RowId>,
    composers: BTreeMap<ConversationKey, Composer>,
    /// Each composer's edit count and the count at its pending press, so
    /// an answer to a send tells an edit made after the press from the
    /// same text left unedited -- which comparing text cannot.
    edits: BTreeMap<ConversationKey, Edits>,
    /// No daemon serves the profile, as the root last saw it.
    daemon_absent: bool,
    /// The platform withholds network access, as the root last saw it.
    network_denied: bool,
    authentication_needed: bool,
    connectivity: Connectivity,
    /// The path to each peer a direct conversation is with, once the
    /// runtime has said: the route indicator's (`human-client-ui.md` §7).
    paths: HashMap<TransportIdentity, PeerPath>,
    session: SessionState,
    diagnostics: Diagnostics,
    trust: TrustSettings,
    peers: Peers,
    /// Onboarding's Connect: owed, open, or done with (add-a-peer §5.1).
    connect: ConnectState,
    /// The snackbar, its Undo and the latest outcome.
    swipe: swipe::SwipeState,
    /// Card proposals, the contacts and the identity in front.
    cards: cards::Cards,
    /// The windowed mode's summaries, once the model started from them
    /// ([`UiModel::started`]); `None` for a model given a whole listing.
    windowed: Option<summaries::Summaries>,
    /// The open conversation's window, in the windowed mode.
    windows: window::Windows,
    /// The contacts as the windowed mode reads them, and their photos.
    directory: directory::Directory,
    /// Stored rows the last listing or start could not decode for
    /// display: held in the store, and Chats' closing line says how many
    /// (design section 6.1).
    rows_not_shown: usize,
    /// The bounds held to (`the_default_bounds_are_the_published_caps`).
    bounds: Bounds,
}

impl Default for UiModel {
    fn default() -> Self {
        Self::new()
    }
}

impl UiModel {
    /// An empty model: connectivity unknown, the facade reconnecting.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
            by_row: HashMap::new(),
            by_app: HashMap::new(),
            ephemeral: VecDeque::new(),
            ephemeral_set: HashSet::new(),
            seen: HashMap::new(),
            seen_order: VecDeque::new(),
            released: HashSet::new(),
            released_order: VecDeque::new(),
            held: HashMap::new(),
            held_order: VecDeque::new(),
            composers: BTreeMap::new(),
            edits: BTreeMap::new(),
            daemon_absent: false,
            network_denied: false,
            authentication_needed: false,
            connectivity: Connectivity::Unknown,
            paths: HashMap::new(),
            session: SessionState::Reconnecting {
                attempt: 0,
                next_at: 0,
            },
            diagnostics: Diagnostics::default(),
            trust: TrustSettings::default(),
            peers: Peers::default(),
            connect: ConnectState::NotOwed,
            swipe: swipe::SwipeState::default(),
            cards: cards::Cards::default(),
            windowed: None,
            windows: window::Windows::default(),
            directory: directory::Directory::default(),
            rows_not_shown: 0,
            bounds: Bounds::default(),
        }
    }

    /// TEST BUILDS ONLY (the `test-hooks` feature): hold the model to a
    /// session bound, a held-row ceiling and a read-proposal bound below
    /// the published caps, so a test reaches each with a few rows.
    #[cfg(feature = "test-hooks")]
    pub fn set_bounds_for_test(
        &mut self,
        session_items: usize,
        held_rows: usize,
        read_proposals: usize,
    ) {
        self.bounds = Bounds {
            session_items,
            held_rows,
            read_proposals,
        };
        self.cards.set_read_cap(read_proposals);
    }

    /// TEST BUILDS ONLY (the `test-hooks` feature): open Connect now,
    /// whatever this start decided, so a test shows it after a
    /// conversation was opened -- where the app itself opens it only on a
    /// first start with nobody to reach (design/2026-10-27-start-window.md
    /// section 10 item 12).
    #[cfg(feature = "test-hooks")]
    pub fn open_connect_for_test(&mut self) {
        self.connect = ConnectState::Open(Connect::opened());
    }

    // --- inputs ------------------------------------------------------------

    /// An event the facade emitted.
    pub fn client_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Outbound(update) => {
                if self.by_row.contains_key(&(Table::Pending, update.row)) {
                    self.apply(update);
                } else {
                    // Not listed or sent yet: held, latest wins per row,
                    // the oldest dropped past the cap (agreed U3).
                    self.hold(update);
                }
            }
            // A route is a session's (`LOCAL-CLIENT.md` §2), and a path is
            // said only for a route and only on a change, so no path
            // outlives the session it was said in: every session event
            // clears them, `Ready` included -- the facade raises one only
            // on a change, so a `Ready` is a new session, and the queue may
            // have folded the ending into it
            // (`a_session_event_clears_every_path`).
            ClientEvent::Session(state) => {
                if matches!(state, SessionState::Ready { .. }) {
                    self.trust.session_ready();
                }
                self.session = state;
                self.paths.clear();
            }
            ClientEvent::Connectivity(connectivity) => self.connectivity = connectivity,
            // A path notice -- a route's begin, a change, or the peer's
            // return -- updates the route indicator and nothing else: no
            // item, no unread count, no conversation (`human-client-ui.md`
            // §7 and §13). Kept for a peer a direct conversation is with --
            // one holding a row here, or in the windowed mode one the start
            // summarised, whose rows stay in the store
            // (`a_summarised_conversations_path_is_kept`); the facade hands
            // a path notice over after the messages of the same take, so the
            // conversation a route's first message starts is there.
            ClientEvent::PeerPath { peer, path } => {
                let with = |c: &ConversationKey| matches!(c, ConversationKey::Direct { peer: p, .. } if *p == peer);
                let known = self.items.values().any(|item| with(&item.conversation))
                    || self
                        .windowed
                        .as_ref()
                        .is_some_and(|w| w.iter().any(|s| with(&s.conversation)));
                if known {
                    self.paths.insert(peer, path);
                }
            }
            // A peer's connection ending changes no conversation and adds
            // no message; its path is no longer known.
            ClientEvent::PeerDisconnected { peer } => {
                self.paths.remove(&peer);
            }
            ClientEvent::UnreadInStore { .. } => {}
        }
    }

    /// The facade's diagnostic counts, for the diagnostics view.
    pub const fn diagnostics_updated(&mut self, diagnostics: Diagnostics) {
        self.diagnostics = diagnostics;
    }

    /// A message `drain` handed over: committed unread already.
    pub fn received(&mut self, received: interweave_human_client_api::Received) {
        let conversation = conversation_of_origin(&received.origin);
        if let Some(windowed) = self.windowed.as_mut()
            && windowed.handed_over(received.row)
            && !self.by_row.contains_key(&(Table::Unread, received.row))
            && !self.released.contains(&(Table::Unread, received.row))
        {
            windowed.arrived(
                &conversation,
                summaries::NewestRow::received(
                    received.row,
                    received.received_at,
                    &received.envelope.text,
                ),
            );
        }
        // A closed conversation holds none of its stored rows in the
        // windowed mode, only its summary: the row stays unread in the
        // store and its opening's page brings it (design section 3).
        if self.windowed.is_some() && self.windows.open_conversation() != Some(&conversation) {
            return;
        }
        self.inbound(
            received.row,
            Table::Unread,
            &received.origin,
            received.envelope,
            received.received_at,
        );
        // Held in the open conversation: its place, and the held-row
        // ceiling, which a flood while the window has no focus reaches.
        if self.windowed.is_some() && self.by_row.contains_key(&(Table::Unread, received.row)) {
            self.windows.holds(
                Table::Unread,
                received.row,
                window::PageMark {
                    received_at: received.received_at,
                    kept: false,
                    row: received.row,
                },
            );
            self.hold_ceiling();
        }
    }

    /// The store's unread rows, at start or after `UnreadInStore`: merged
    /// by row id with what is already shown (agreed A5, rule 2).
    pub fn unread_listed(&mut self, rows: Vec<ListedInbound>) {
        for row in rows {
            self.inbound(
                row.row,
                Table::Unread,
                &row.origin,
                row.envelope,
                row.received_at,
            );
        }
    }

    /// The store's kept rows, at start.
    pub fn kept_listed(&mut self, rows: Vec<ListedInbound>) {
        for row in rows {
            self.inbound(
                row.row,
                Table::Kept,
                &row.origin,
                row.envelope,
                row.received_at,
            );
        }
    }

    /// A message this client sent: the facade committed `row`.
    ///
    /// The composer is cleared only if it was not edited since the press
    /// that sent it ([`send_pressed`](Self::send_pressed)): the commit is
    /// answered after the press, and anything typed in between -- the same
    /// text typed again included -- is the person's next message, not this
    /// one's. A send with no recorded press clears nothing.
    pub fn sent(&mut self, row: RowId, destination: &Destination, envelope: HumanChatV2, at: u64) {
        let conversation = conversation_of(destination);
        let unedited = self.take_press(&conversation);
        if let Some(composer) = self.composers.get_mut(&conversation) {
            if unedited {
                composer.draft.clear();
                self.edits.entry(conversation.clone()).or_default().revision += 1;
            }
            composer.refused = None;
        }
        self.outbound(row, conversation, envelope, None, at);
    }

    /// Share queued the identity's own card on `key` as pending `row`: its
    /// own card row (contact-cards section 5.5), the card as it went,
    /// with the status of any outbound message from here.
    pub fn card_queued(
        &mut self,
        row: RowId,
        key: ConversationKey,
        app_message_id: String,
        card: cards::SentCard,
        at: u64,
    ) {
        self.outbound(row, key, card_envelope(app_message_id), Some(card), at);
    }

    /// The person pressed Send in conversation `key`, for the draft as it
    /// reads now: the facade's answer may touch the composer only if it
    /// was not edited after this. The root calls it when it issues the
    /// send; one send per conversation is in flight at a time.
    pub fn send_pressed(&mut self, key: &ConversationKey) {
        let edit = self.edits.entry(key.clone()).or_default();
        edit.pressed = Some(edit.revision);
    }

    /// Whether the composer of `key` is unedited since the recorded press,
    /// forgetting the press: an answer is the press's only one.
    fn take_press(&mut self, key: &ConversationKey) -> bool {
        self.edits.get_mut(key).is_some_and(|edit| {
            let unedited = edit.pressed == Some(edit.revision);
            edit.pressed = None;
            unedited
        })
    }

    /// The store's pending rows, at start (agreed item 2a). Authoritative:
    /// a held update for a row it does not list is discarded, a terminal
    /// one included (agreed U3a).
    pub fn pending_listed(&mut self, rows: Vec<ListedOutbound>) {
        for row in rows {
            let conversation = conversation_of(&row.destination);
            self.outbound(
                row.row,
                conversation,
                row.envelope,
                row.card,
                row.created_at,
            );
        }
        self.held.clear();
        self.held_order.clear();
    }

    /// The facade refused a send with no row: the composer keeps the
    /// draft and shows why (agreed item 2b).
    ///
    /// An edit made after the press -- clearing the composer included --
    /// is newer than the refusal and is never replaced by it. With no
    /// edit since the press the composer already holds the refused text;
    /// with no recorded press, the text is put back only into an empty
    /// composer.
    pub fn send_refused(&mut self, key: ConversationKey, draft: String, error: &SendError) {
        let pressed = self
            .edits
            .get(&key)
            .is_some_and(|edit| edit.pressed.is_some());
        let unedited = self.take_press(&key);
        let composer = self.composers.entry(key).or_default();
        if (pressed && unedited) || (!pressed && composer.draft.is_empty()) {
            composer.draft = draft;
        }
        composer.refused = Some(send_error_class(error));
    }

    /// Whether a transport daemon serves this profile, as the root sees
    /// it: `Some(true)`, `Some(false)`, or `None` when it cannot tell.
    /// Only `Some(false)` makes a reconnecting session's notice say no
    /// daemon runs; cannot-tell clears it, since the guidance to start
    /// the daemon may then be wrong.
    pub fn daemon_seen(&mut self, present: Option<bool>) {
        self.daemon_absent = present == Some(false);
    }

    /// Whether the platform gives this app network access, as the root
    /// sees it. While it does not, the session's notice is
    /// [`SessionNotice::NetworkDenied`] whatever the facade reports, and
    /// no draft is offered for sending; reading and keeping go on.
    pub fn network_access(&mut self, allowed: bool) {
        self.network_denied = !allowed;
    }

    /// Whether the profile waits for the person's authentication before
    /// anything can run: while it does, the session's notice is
    /// [`SessionNotice::AuthenticationNeeded`] and connectivity reads
    /// offline, whatever else is known.
    pub fn authentication_needed(&mut self, needed: bool) {
        self.authentication_needed = needed;
    }

    /// Whether a person may send now: false while the platform withholds
    /// network access, so the view offers no composer.
    #[must_use]
    pub const fn sending_offered(&self) -> bool {
        !self.network_denied
    }

    /// The person edited a draft.
    pub fn draft_changed(&mut self, key: ConversationKey, draft: String) {
        self.edits.entry(key.clone()).or_default().revision += 1;
        let composer = self.composers.entry(key).or_default();
        composer.draft = draft;
        composer.refused = None;
    }

    /// The root marked an unread row read: its durable copy is gone, and
    /// the content stays for this session unless another row holds it.
    pub fn read(&mut self, row: RowId) {
        // The store's count loses the row once, whichever kind it is.
        let conversation = self
            .by_row
            .get(&(Table::Unread, row))
            .and_then(|key| self.items.get(key))
            .map(|item| item.conversation.clone())
            .or_else(|| self.cards.unread_conversation(row));
        if let (Some(windowed), Some(conversation)) = (self.windowed.as_mut(), conversation) {
            windowed.left_unread(&conversation);
        }
        self.windows.forget(Table::Unread, row);
        self.release(Table::Unread, row);
        self.cards.read(row);
    }

    /// The content a Keep of `key` would keep is no longer held this
    /// session -- the root dropped its copy past a bound -- so Keep is no
    /// longer offered for it. The message stays shown, read, until it is
    /// evicted or the session ends.
    pub fn copy_gone(&mut self, key: ItemKey) {
        if let Some(item) = self.items.get_mut(&key) {
            item.released_from = None;
        }
    }

    /// The root kept the read message `key` as `kept_row`.
    pub fn kept(&mut self, key: ItemKey, kept_row: RowId) {
        self.keep_answered(key);
        if let Some(item) = self.items.get_mut(&key) {
            item.rows.push((Table::Kept, kept_row));
            self.by_row.insert((Table::Kept, kept_row), key);
            // A kept row keeps its stored copy at the time the message
            // reached this app: its place in the window.
            self.windows.holds(
                Table::Kept,
                kept_row,
                window::PageMark {
                    received_at: item.local_at,
                    kept: true,
                    row: kept_row,
                },
            );
            if self.ephemeral_set.remove(&key) {
                self.ephemeral.retain(|k| *k != key);
            }
            self.row_attached(key);
            self.swipe_kept(key, kept_row);
        }
    }

    /// The root removed Keep from `kept_row`.
    pub fn unkept(&mut self, kept_row: RowId) {
        let key = self.by_row.get(&(Table::Kept, kept_row)).copied();
        self.windows.forget(Table::Kept, kept_row);
        self.release(Table::Kept, kept_row);
        if let Some(key) = key {
            self.swipe_unkept(key);
        }
    }

    /// The view shows conversation `key`: if the window has focus, its
    /// unread rows are read now. Read is a retention act -- it deletes the
    /// durable copy -- so it is raised from here alone, never on receipt,
    /// from a notification, or while unfocused (agreed 2c); `actions`
    /// does not offer it (review F1).
    #[must_use]
    pub fn conversation_viewed(&self, key: &ConversationKey, focused: bool) -> Vec<Intent> {
        if !focused {
            return Vec::new();
        }
        self.items
            .values()
            .filter(|i| &i.conversation == key)
            .flat_map(|i| {
                i.rows
                    .iter()
                    .filter(|(t, _)| *t == Table::Unread)
                    .map(|(_, r)| Intent::MarkRead(*r))
            })
            .collect()
    }

    /// A person activated a link in an item: an intent only for an
    /// allowlisted scheme (`HUMAN-CHAT.md`). Remote text alone raises
    /// nothing -- there is no path from receipt to an intent.
    #[must_use]
    pub fn link_activated(&self, destination: &str) -> Option<Intent> {
        is_allowed_link_scheme(destination).then(|| Intent::OpenLink(destination.to_owned()))
    }

    // --- outputs -----------------------------------------------------------

    /// Every conversation, most recent activity first.
    #[must_use]
    pub fn conversations(&self) -> Vec<ConversationSummary> {
        let shared = self.peers_with_routes();
        let row = |key: &ConversationKey, last: LastMessage| ConversationSummary {
            key: key.clone(),
            title: self.contact_title(key, &shared, Isolate::No),
            drawn_title: self.contact_title(key, &shared, Isolate::Yes),
            unread: 0,
            last_activity: last.local_at,
            avatar: Avatar::of(key),
            last,
        };
        let mut by: BTreeMap<&ConversationKey, ConversationSummary> = BTreeMap::new();
        // The windowed mode lists every conversation the store shows a row
        // of, held or not, counted as the store counts it: a held unread
        // row is among the store's, so it is not counted again below.
        if let Some(windowed) = &self.windowed {
            for stored in windowed.iter() {
                if let Some(newest) = &stored.newest {
                    let mut summary = row(&stored.conversation, newest.last());
                    summary.unread = usize::try_from(stored.unread).unwrap_or(usize::MAX);
                    by.insert(&stored.conversation, summary);
                }
            }
        }
        for (key, item) in self.items.iter().filter(|(_, i)| !i.hidden) {
            let last = || LastMessage {
                key: *key,
                direction: item.direction,
                label: item.label(),
                body: item.body.clone(),
                local_at: item.local_at,
                card: item.card.is_some(),
                preview: summaries::held_preview(&item.body),
            };
            let summary = by
                .entry(&item.conversation)
                .or_insert_with(|| row(&item.conversation, last()));
            if self.windowed.is_none() && item.outbound.is_none() && item.has(Table::Unread) {
                summary.unread += 1;
            }
            summary.last_activity = summary.last_activity.max(item.local_at);
            // The conversation's own order (`messages`): local time, then
            // key.
            if (item.local_at, *key) > (summary.last.local_at, summary.last.key) {
                summary.last = last();
            }
        }
        let mut list: Vec<_> = by.into_values().collect();
        list.sort_by(|a, b| {
            b.last_activity
                .cmp(&a.last_activity)
                .then_with(|| a.key.cmp(&b.key))
        });
        list
    }

    /// A conversation's title, as the list shows it: for a conversation
    /// the list no longer holds too -- every message in it hidden while it
    /// is open.
    #[must_use]
    pub fn title(&self, key: &ConversationKey) -> String {
        self.contact_title(key, &self.peers_with_routes(), Isolate::No)
    }

    /// [`UiModel::title`] as a view draws it: a contact's name isolated
    /// ([`ConversationSummary::drawn_title`]).
    #[must_use]
    pub fn drawn_title(&self, key: &ConversationKey) -> String {
        self.contact_title(key, &self.peers_with_routes(), Isolate::Yes)
    }

    /// The title, its contact's name once the route belongs to one (plan
    /// §29 step 2, `human-client-ui.md` §4), with the short `PeerId` beside
    /// it when another contact has that name; else as before. `isolate`
    /// wraps the name for a view that draws it in other text.
    fn contact_title(
        &self,
        key: &ConversationKey,
        shared: &BTreeSet<&TransportIdentity>,
        isolate: Isolate,
    ) -> String {
        let name = |n: &str| match isolate {
            Isolate::Yes => crate::labels::isolated(n),
            Isolate::No => n.to_owned(),
        };
        if let ConversationKey::Direct {
            peer,
            endpoint: Some(at),
        } = key
            && let Some(contact) = self.cards.contact_for(peer, at)
        {
            if self
                .cards
                .name_is_shared(&contact.contact_id, &contact.name)
            {
                return fill(
                    placeholder_en::text(UiText::ContactTitle),
                    &[
                        ("name", &name(&contact.name)),
                        ("peer", &short_peer(peer.as_str())),
                    ],
                );
            }
            return name(&contact.name);
        }
        title(key, shared)
    }

    /// The peers this session holds direct conversations with on more than
    /// one route: their titles carry the route (human-client-ui.md §4).
    /// Every conversation the session holds counts, a hidden one too: it
    /// is still there to be told apart when it comes back.
    fn peers_with_routes(&self) -> BTreeSet<&TransportIdentity> {
        let mut routes: BTreeMap<&TransportIdentity, BTreeSet<&ConversationKey>> = BTreeMap::new();
        for item in self.items.values() {
            if let ConversationKey::Direct { peer, .. } = &item.conversation {
                routes.entry(peer).or_default().insert(&item.conversation);
            }
        }
        routes
            .into_iter()
            .filter(|(_, keys)| keys.len() > 1)
            .map(|(peer, _)| peer)
            .collect()
    }

    /// The item a text from [`ItemKey`]'s `Display` names: one of the
    /// rows the model holds now under that table letter and row -- an
    /// item that moved table answers to its new row too -- else the key
    /// it was first shown under. `None` for a text naming nothing held.
    #[must_use]
    pub fn item_named(&self, text: &str) -> Option<ItemKey> {
        self.by_row
            .iter()
            .find(|((table, row), _)| ItemKey(*table, *row).to_string() == text)
            .map(|(_, key)| *key)
            .or_else(|| {
                self.items
                    .keys()
                    .find(|key| key.to_string() == text)
                    .copied()
            })
    }

    /// A conversation's messages, in LOCAL time order (A3). Replies are
    /// resolved here, within the conversation, so a target that was
    /// evicted or is ambiguous reads `Unavailable` (review F5).
    #[must_use]
    pub fn messages(&self, key: &ConversationKey) -> Vec<MessageItem> {
        let mut list: Vec<MessageItem> = self
            .items
            .iter()
            .filter(|(_, i)| &i.conversation == key && !i.hidden)
            .map(|(k, i)| MessageItem {
                key: *k,
                direction: i.direction,
                status: i.status(),
                label: i.label(),
                kept: i.has(Table::Kept),
                body: i.body.clone(),
                source: i.envelope.text.clone(),
                author: i.author.clone(),
                route_label: i.route_label.clone(),
                reply: self.reply(&i.conversation, i.envelope.reply_to.as_deref()),
                sent_at_ms: i.envelope.sent_at_ms,
                local_at: i.local_at,
                card: i.card.clone(),
            })
            .collect();
        list.sort_by(|a, b| a.local_at.cmp(&b.local_at).then_with(|| a.key.cmp(&b.key)));
        list
    }

    /// The intents legal on `key` now (agreed item 3e). Never `MarkRead`:
    /// that comes from a focused view alone (review F1).
    #[must_use]
    pub fn actions(&self, key: ItemKey) -> Vec<Intent> {
        // A hidden item offers nothing: it is on its way out of the session.
        let Some(item) = self.items.get(&key).filter(|i| !i.hidden) else {
            return Vec::new();
        };
        if let Some(status) = &item.outbound {
            return match item.row_in(Table::Pending) {
                Some(row) if !status.is_terminal() => vec![Intent::Retry(row), Intent::Cancel(row)],
                _ => Vec::new(),
            };
        }
        if let Some(kept_row) = item.row_in(Table::Kept) {
            vec![Intent::Unkeep(kept_row)]
        } else if let (true, Some(from)) = (item.rows.is_empty(), item.released_from) {
            // Read and not kept: Keep is offered only here, after read,
            // and only with the row whose release handed the content back.
            vec![Intent::Keep { item: key, from }]
        } else {
            Vec::new()
        }
    }

    /// Whether the model offers `intent` NOW. A view judges a whole take
    /// against the model as it was rendered, so the root asks again as it
    /// issues each: a Dismiss taken earlier in the same take may have
    /// changed what is legal (review P-1 on #289, round 4).
    #[must_use]
    pub fn offers(&self, intent: &Intent) -> bool {
        let key = match intent {
            Intent::Keep { item, .. } | Intent::Hide(item) => Some(*item),
            Intent::Unkeep(row) => self.by_row.get(&(Table::Kept, *row)).copied(),
            Intent::Retry(row) | Intent::Cancel(row) => {
                self.by_row.get(&(Table::Pending, *row)).copied()
            }
            Intent::HideConversation(key) => return self.conversation_hide_offered(key),
            // Destructive: offered only while the contact is still held,
            // so a delete of a contact already gone is not issued. Two in
            // one take are one command by the root's in-flight set.
            Intent::DeleteContact { contact_id } => return self.contact(contact_id).is_some(),
            Intent::DeleteRoute {
                contact_id,
                peer,
                endpoint,
            } => return self.delete_route(contact_id, peer, endpoint).is_some(),
            // Not while the contacts are unread (design section 11).
            Intent::AcceptCard(row) => return self.card_actions(*row).contains(intent),
            _ => return true,
        };
        key.is_some_and(|key| match intent {
            Intent::Hide(_) => self.row_actions(key).contains(&RowAction::Hide),
            _ => self.actions(key).contains(intent),
        })
    }

    /// An outbound row's raw failure, for the diagnostics view only.
    #[must_use]
    pub fn item_diagnostics(&self, key: ItemKey) -> Option<ItemDiagnostics> {
        self.items.get(&key).map(|i| ItemDiagnostics {
            last_code: i.last_code,
        })
    }

    /// A conversation's composer.
    #[must_use]
    pub fn composer(&self, key: &ConversationKey) -> Composer {
        self.composers.get(key).cloned().unwrap_or_default()
    }

    /// The send intent for a conversation's draft, if there is one.
    #[must_use]
    pub fn send_draft(&self, key: &ConversationKey) -> Option<Intent> {
        if !self.sending_offered() {
            return None;
        }
        let draft = &self.composers.get(key)?.draft;
        (!draft.is_empty()).then(|| Intent::Send {
            key: key.clone(),
            draft: draft.clone(),
        })
    }

    /// Connectivity: always present, `Unknown` never shown as `Offline`.
    #[must_use]
    pub const fn connectivity(&self) -> Connectivity {
        // Offline, not unknown: the cause is known, and the person can
        // act on it (#309 acceptance 10, condition 3).
        if self.authentication_needed {
            return Connectivity::Offline;
        }
        self.connectivity
    }

    /// The path to the peer a direct conversation is with, once the
    /// runtime has said; `None` for a channel, or before it has.
    #[must_use]
    pub fn path(&self, key: &ConversationKey) -> Option<PeerPath> {
        match key {
            ConversationKey::Direct { peer, .. } => self.paths.get(peer).copied(),
            ConversationKey::Channel(_) => None,
        }
    }

    /// The session state a person can act on, if any.
    #[must_use]
    pub const fn session_notice(&self) -> Option<SessionNotice> {
        // First: nothing runs at all until the person authenticates.
        if self.authentication_needed {
            return Some(SessionNotice::AuthenticationNeeded);
        }
        // Before what the facade says: with no network access it can only
        // ever be re-opening, and the person can act on the cause.
        if self.network_denied {
            return Some(SessionNotice::NetworkDenied);
        }
        match &self.session {
            SessionState::Ready { .. } | SessionState::Closed => None,
            // In place of "reconnecting" only: re-opening cannot succeed
            // while nothing serves the profile, and the person can act on
            // that. Storage trouble and a refusal are said as they are.
            SessionState::Reconnecting { .. } if self.daemon_absent => {
                Some(SessionNotice::NoDaemon)
            }
            SessionState::Reconnecting { .. } => Some(SessionNotice::Reconnecting),
            SessionState::StorageDegraded => Some(SessionNotice::StorageDegraded),
            SessionState::Refused { problem } => {
                Some(SessionNotice::Refused(session_problem_class(*problem)))
            }
        }
    }

    /// The trust settings (`human-client-ui.md` §8).
    #[must_use]
    pub const fn trust_settings(&self) -> &TrustSettings {
        &self.trust
    }

    /// The trust settings, for the settings view's inputs and the
    /// daemon's answers.
    pub const fn trust_settings_mut(&mut self) -> &mut TrustSettings {
        &mut self.trust
    }

    /// Peers by address (ADR-0062).
    #[must_use]
    pub const fn peers(&self) -> &Peers {
        &self.peers
    }

    /// Apply what the person did on Peers: the intents it asks for. Trust
    /// on step two passes through the trust settings, the one way a trust
    /// change is made.
    pub fn peers_input(&mut self, input: PeersInput) -> Vec<Intent> {
        let intents = self.peers.input(input, &mut self.trust);
        self.connect_sync();
        intents
    }

    /// Connect is owed: a new or restored identity's first start, or a
    /// later one whose earlier start left it undecided (§5.1). The
    /// platform says so before the session's first reads answer.
    pub fn connect_owed(&mut self) {
        if self.connect == ConnectState::NotOwed {
            self.connect = ConnectState::Owed;
        }
    }

    /// What this start does about an owed Connect, said once each: open
    /// it when both reads succeeded on two empty lists, settle it when
    /// either list has an entry or once the person left it, defer it
    /// when a read failed (§5.1, §5.6). `None` while nothing is to say.
    /// While Connect is open, each turn reads its added peers' trust from
    /// the list again, so its line follows an answer that came late.
    pub fn connect_turn(&mut self) -> Option<ConnectGate> {
        match self.connect {
            ConnectState::Owed => {
                let list = self.peers.list_answered()?;
                let trust = self.trust.read_answered()?;
                if !(list && trust) {
                    self.connect = ConnectState::Decided;
                    return Some(ConnectGate::Deferred);
                }
                if self.no_reach() {
                    self.connect = ConnectState::Open(Connect::opened());
                    Some(ConnectGate::Open)
                } else {
                    self.connect = ConnectState::Decided;
                    Some(ConnectGate::Settled)
                }
            }
            ConnectState::Left => {
                self.connect = ConnectState::Decided;
                Some(ConnectGate::Settled)
            }
            ConnectState::Open(ref mut connect) => {
                connect.trust_read(&self.trust);
                None
            }
            ConnectState::NotOwed | ConnectState::Decided => None,
        }
    }

    /// Connect, while it is open.
    #[must_use]
    pub const fn connect(&self) -> Option<&Connect> {
        match &self.connect {
            ConnectState::Open(connect) => Some(connect),
            _ => None,
        }
    }

    /// Apply what the person did on Connect: the intents it asks for.
    pub fn connect_input(&mut self, input: ConnectInput) -> Vec<Intent> {
        let ConnectState::Open(connect) = &mut self.connect else {
            return Vec::new();
        };
        connect.trust_read(&self.trust);
        // One page at a time: a press reaching Connect while a page it
        // opened is still up is a stale one.
        if self.peers.add_open() || self.peers.show().is_some() {
            return Vec::new();
        }
        let (from, peers_input) = match input {
            ConnectInput::Add => (ConnectFocus::Add, PeersInput::OpenAddFromConnect),
            ConnectInput::Show => (ConnectFocus::Show, PeersInput::OpenShow),
            ConnectInput::Trust => {
                let peers = connect.to_trust();
                if peers.is_empty() {
                    return Vec::new();
                }
                (ConnectFocus::Trust, PeersInput::TrustFromConnect(peers))
            }
            ConnectInput::Leave => {
                self.connect = ConnectState::Left;
                return Vec::new();
            }
        };
        connect.opening(from);
        // Show drawn before this visit -- from Peers -- is not this
        // visit's.
        self.peers.take_show_drew();
        let intents = self.peers.input(peers_input, &mut self.trust);
        self.connect_sync();
        intents
    }

    /// Hand Connect what a page it opened came to, once that page is
    /// gone; with Connect not open, what Show drew is nobody's.
    fn connect_sync(&mut self) {
        let ConnectState::Open(connect) = &mut self.connect else {
            self.peers.take_show_drew();
            return;
        };
        if self.peers.add_open() || self.peers.show().is_some() {
            return;
        }
        let adds = self.peers.take_connect_return();
        let shown = self.peers.take_show_drew();
        connect.returned(adds, shown, &self.trust);
    }

    /// This node's `PeerId`, as the session that became ready knows it.
    pub fn this_device(&mut self, peer: TransportIdentity) {
        self.peers.this_device(peer);
    }

    /// Read the addresses: at start, for the no-reach notice.
    pub fn read_addresses(&mut self) -> Option<Intent> {
        self.peers.read()
    }

    /// The runtime's answer to a read of the addresses.
    pub fn addresses_read(&mut self, answer: Result<AddressList, AddressProblem>) {
        self.peers.read_answer(answer, &self.trust);
    }

    /// The runtime's answer to Show's read of this device's addresses.
    pub fn own_addresses_read(&mut self, answer: Result<ShowAddresses, AddressProblem>) {
        self.peers.own_answer(answer);
    }

    /// A code read, or an image decoded: whether the viewfinder stays
    /// open, and the add it asks for.
    pub fn scanned(&mut self, text: &str) -> (ScanVerdict, Option<Intent>) {
        self.peers.scanned(text, &self.trust)
    }

    /// The runtime's answer to an add of `lines`.
    pub fn addresses_added(
        &mut self,
        lines: &[String],
        answer: Result<AddressAnswer, AddressChangeFailure>,
    ) {
        self.peers.add_answer(lines, answer, &self.trust);
        self.connect_sync();
    }

    /// The runtime's answer to a removal of `address`.
    pub fn address_removed(
        &mut self,
        address: &str,
        answer: Result<AddressAnswer, AddressChangeFailure>,
    ) {
        self.peers.remove_answer(address, answer, &self.trust);
    }

    /// What Peers asks between turns: a read a change left owed, the
    /// trust set's read a trust step an add opened owes, and closing the
    /// add page once trust's answers decided it.
    pub fn peers_turn(&mut self) -> Option<Intent> {
        let intent = self.peers.turn(&self.trust);
        self.connect_sync();
        // A trust step an add opened reads the trust set itself, once the
        // turn asks nothing else.
        if intent.is_none() && self.peers.take_trust_read() {
            return self.trust.opened();
        }
        intent
    }

    /// Whether Chats shows the no-reach notice (ADR-0062 decision 8).
    #[must_use]
    pub fn no_reach(&self) -> bool {
        self.peers.no_reach(&self.trust)
    }

    /// What trust to show for `peer`.
    #[must_use]
    pub const fn trust(&self, _peer: &TransportIdentity) -> Trust {
        Trust::NotVerifiedByThisClient
    }

    /// The facade's counts, for the diagnostics view.
    #[must_use]
    pub const fn diagnostics(&self) -> Diagnostics {
        self.diagnostics
    }

    /// How many items the model holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How many (conversation, application id) entries the reply index
    /// holds: at most one per item held, so it shrinks with eviction.
    #[must_use]
    pub fn indexed_ids(&self) -> usize {
        self.by_app.len()
    }

    /// How many item keys the copy index holds across every (origin,
    /// application id): an evicted item's key is dropped when its pair is
    /// next seen, so one id reused with ever-new text stays bounded.
    #[must_use]
    pub fn copy_index_keys(&self) -> usize {
        self.seen.values().map(Vec::len).sum()
    }

    /// How many released rows are remembered, at most [`DEDUP_CAP`].
    #[must_use]
    pub fn released_rows(&self) -> usize {
        self.released.len()
    }

    /// How many outbound updates are held for rows not listed yet.
    #[must_use]
    pub fn held_updates(&self) -> usize {
        self.held.len()
    }

    // --- internals ---------------------------------------------------------

    fn inbound(
        &mut self,
        row: RowId,
        table: Table,
        origin: &Origin,
        envelope: HumanChatV2,
        at: u64,
    ) {
        // Merged by row: the same row listed again is the same item, and a
        // row already released is not listed back by a stale snapshot.
        if self.by_row.contains_key(&(table, row)) || self.released.contains(&(table, row)) {
            return;
        }
        let (conversation, author, route_label, origin_key) = match origin {
            Origin::Direct { peer, endpoint } => (
                ConversationKey::Direct {
                    peer: peer.clone(),
                    endpoint: Some(endpoint.clone()),
                },
                peer.clone(),
                Some(RouteLabel::from(endpoint)),
                format!("direct:{}:{}", peer.as_str(), endpoint.as_str()),
            ),
            Origin::Channel { channel, publisher } => (
                ConversationKey::Channel(channel.clone()),
                publisher.clone(),
                None,
                format!("channel:{}:{}", channel.as_str(), publisher.as_str()),
            ),
        };
        // A second copy of one message is one item -- but a store row is
        // never hidden: the copy's row is ATTACHED to the item, so it is
        // counted, read when viewed, and unkept, like the first (agreed
        // U1). Only an identical envelope is a copy: the id is chosen by
        // the peer, and new text under an old id is a new item (U1a).
        let dedup = (origin_key, envelope.app_message_id.clone());
        let copy_of = self.seen.get(&dedup).and_then(|keys| {
            keys.iter()
                .copied()
                .find(|k| self.items.get(k).is_some_and(|i| i.envelope == envelope))
        });
        if let Some(existing) = copy_of
            && let Some(item) = self.items.get_mut(&existing)
        {
            item.rows.push((table, row));
            self.by_row.insert((table, row), existing);
            if self.ephemeral_set.remove(&existing) {
                self.ephemeral.retain(|k| *k != existing);
            }
            self.row_attached(existing);
            return;
        }
        let key = ItemKey(table, row);
        self.remember(dedup, key);
        self.by_app
            .entry((conversation.clone(), envelope.app_message_id.clone()))
            .or_default()
            .push(key);
        self.by_row.insert((table, row), key);
        let conversation_key = conversation.clone();
        self.items.insert(
            key,
            Item {
                conversation,
                direction: Direction::Inbound,
                rows: vec![(table, row)],
                body: render(&envelope.text),
                envelope,
                author: Some(author),
                route_label,
                outbound: None,
                last_code: None,
                local_at: at,
                released_from: None,
                last_pending: None,
                hidden: false,
                card: None,
            },
        );
        self.arrived_in(&conversation_key);
    }

    fn outbound(
        &mut self,
        row: RowId,
        conversation: ConversationKey,
        envelope: HumanChatV2,
        card: Option<cards::SentCard>,
        at: u64,
    ) {
        // Merged by row, and a row already terminal is not listed back by
        // a snapshot taken before its terminal update.
        if self.by_row.contains_key(&(Table::Pending, row))
            || self.released.contains(&(Table::Pending, row))
        {
            return;
        }
        let key = ItemKey(Table::Pending, row);
        self.by_app
            .entry((conversation.clone(), envelope.app_message_id.clone()))
            .or_default()
            .push(key);
        self.by_row.insert((Table::Pending, row), key);
        let route_label = match &conversation {
            ConversationKey::Direct {
                endpoint: Some(endpoint),
                ..
            } => Some(RouteLabel::from(endpoint)),
            _ => None,
        };
        let conversation_key = conversation.clone();
        self.items.insert(
            key,
            Item {
                conversation,
                direction: Direction::Outbound,
                rows: vec![(Table::Pending, row)],
                body: render(&envelope.text),
                envelope,
                author: None,
                route_label,
                outbound: Some(OutboundStatus::Sending {
                    attempts: 0,
                    next_retry_at: None,
                    last_problem: None,
                }),
                last_code: None,
                local_at: at,
                released_from: None,
                last_pending: Some(row),
                hidden: false,
                card,
            },
        );
        self.arrived_in(&conversation_key);
        // A status the facade reported before the row was listed.
        if let Some(update) = self.held.remove(&row) {
            self.held_order.retain(|r| *r != row);
            self.apply(update);
        }
    }

    fn apply(&mut self, update: OutboundUpdate) {
        let Some(key) = self.by_row.get(&(Table::Pending, update.row)).copied() else {
            return;
        };
        let terminal = update.status.is_terminal();
        if let Some(item) = self.items.get_mut(&key) {
            item.outbound = Some(update.status);
            if update.last_code.is_some() {
                item.last_code = update.last_code;
            }
        }
        if terminal {
            self.release(Table::Pending, update.row);
        }
    }

    fn hold(&mut self, update: OutboundUpdate) {
        let row = update.row;
        if self.held.insert(row, update).is_none() {
            self.held_order.push_back(row);
            while self.held_order.len() > HELD_UPDATE_CAP {
                if let Some(oldest) = self.held_order.pop_front() {
                    self.held.remove(&oldest);
                }
            }
        }
    }

    /// A store row went: the item keeps showing, and becomes session-only
    /// once it holds no row.
    fn release(&mut self, table: Table, row: RowId) {
        let Some(key) = self.by_row.remove(&(table, row)) else {
            return;
        };
        if self.released.insert((table, row)) {
            self.released_order.push_back((table, row));
            while self.released_order.len() > DEDUP_CAP {
                if let Some(oldest) = self.released_order.pop_front() {
                    self.released.remove(&oldest);
                }
            }
        }
        let now_session_only = self.items.get_mut(&key).is_some_and(|item| {
            item.rows.retain(|r| *r != (table, row));
            if table != Table::Pending {
                item.released_from = Some((table, row));
            }
            item.rows.is_empty()
        });
        if now_session_only && self.ephemeral_set.insert(key) {
            self.ephemeral.push_back(key);
            self.evict();
        }
    }

    fn reply(&self, conversation: &ConversationKey, target: Option<&str>) -> Option<Reply> {
        let target = target?;
        let holders = self
            .by_app
            .get(&(conversation.clone(), target.to_owned()))
            .map(|keys| {
                // A hidden message is not held as far as a reply is
                // concerned: the neutral placeholder (§15).
                keys.iter()
                    .filter(|k| self.items.get(k).is_some_and(|i| !i.hidden))
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Some(match holders.as_slice() {
            [only] => Reply::Present(*only),
            _ => Reply::Unavailable(target.to_owned()),
        })
    }

    fn remember(&mut self, pair: (String, String), key: ItemKey) {
        let keys = self.seen.entry(pair.clone()).or_default();
        let new_pair = keys.is_empty();
        // Only keys still held: an evicted item can no longer be a copy.
        keys.retain(|k| self.items.contains_key(k));
        keys.push(key);
        if new_pair {
            self.seen_order.push_back(pair);
            while self.seen_order.len() > DEDUP_CAP {
                if let Some(oldest) = self.seen_order.pop_front() {
                    self.seen.remove(&oldest);
                }
            }
        }
    }

    /// Evict the oldest session-only items past [`SESSION_ITEM_CAP`]:
    /// only those, since everything else is re-listed from the store, and
    /// never one an open snackbar's Undo needs (it goes once that closes).
    fn evict(&mut self) {
        // The item the person moved focus to is never dropped while focused:
        // the next in the order goes instead (design section 7.1).
        let focused = self.windows.focused();
        while self.ephemeral.len() > self.bounds.session_items {
            let Some(at) = self
                .ephemeral
                .iter()
                .position(|k| !self.swipe.protects(k) && Some(*k) != focused)
            else {
                return;
            };
            let Some(key) = self.ephemeral.remove(at) else {
                return;
            };
            // A row of the open conversation leaves a ReadNotShown line
            // where it stood; another conversation's goes with nothing said.
            if self
                .items
                .get(&key)
                .is_some_and(|i| Some(&i.conversation) == self.windows.open_conversation())
            {
                if let Some(at) = self.drop_pos(key) {
                    self.windows.dropped(at);
                }
                self.drop_items(&[key]);
                self.join_runs();
                continue;
            }
            self.drop_items(&[key]);
        }
    }

    /// Stop holding `conversation`'s stored rows -- unread and kept -- which
    /// stay in the store unchanged and load again when reached: not
    /// released, which would refuse them for good (design sections 3 and
    /// 5.1). An item a pending row holds stays, as does a session-only one.
    pub(super) fn let_go(&mut self, conversation: &ConversationKey) {
        let keys: Vec<ItemKey> = self
            .items
            .iter()
            .filter(|(_, i)| {
                &i.conversation == conversation
                    && !i.rows.is_empty()
                    && i.rows.iter().all(|(t, _)| *t != Table::Pending)
            })
            .map(|(k, _)| *k)
            .collect();
        self.drop_items(&keys);
    }

    /// Forget `keys` entirely: shown nowhere, offered nothing, no reply
    /// resolved to them. Only items no store row holds.
    fn drop_items(&mut self, keys: &[ItemKey]) {
        for key in keys {
            if self.ephemeral_set.remove(key) {
                self.ephemeral.retain(|k| k != key);
            }
            if let Some(item) = self.items.remove(key) {
                for row in &item.rows {
                    self.by_row.remove(row);
                }
                let index = (item.conversation, item.envelope.app_message_id);
                if let Some(keys) = self.by_app.get_mut(&index) {
                    keys.retain(|k| k != key);
                    if keys.is_empty() {
                        self.by_app.remove(&index);
                    }
                }
            }
        }
    }
}

/// The conversation a received row belongs to, as [`UiModel::received`]'s
/// item takes it: a direct route by its peer and endpoint, a channel by its
/// id whoever published.
fn conversation_of_origin(origin: &Origin) -> ConversationKey {
    match origin {
        Origin::Direct { peer, endpoint } => ConversationKey::Direct {
            peer: peer.clone(),
            endpoint: Some(endpoint.clone()),
        },
        Origin::Channel { channel, .. } => ConversationKey::Channel(channel.clone()),
    }
}

fn conversation_of(destination: &Destination) -> ConversationKey {
    match destination {
        Destination::Direct { peer, endpoint } => ConversationKey::Direct {
            peer: peer.clone(),
            endpoint: endpoint.clone(),
        },
        Destination::Broadcast(channel) => ConversationKey::Channel(channel.clone()),
    }
}

/// A conversation's title: a channel's name, or a direct route's
/// authenticated short `PeerId` -- with the route label the peer asserts
/// beside it only while this session holds another conversation with the
/// same peer on another route, which the label then tells apart
/// (human-client-ui.md §4 at a5ecfc04; #304 F6). One conversation with a
/// peer has no route in its title; its details show it. The envelope's
/// own `from_endpoint` and the message text never enter it.
/// Whether a title wraps a contact's name in a first-strong isolate.
#[derive(Clone, Copy)]
enum Isolate {
    Yes,
    No,
}

fn title(key: &ConversationKey, shared: &BTreeSet<&TransportIdentity>) -> String {
    match key {
        ConversationKey::Channel(channel) => format!("#{}", channel.as_str()),
        ConversationKey::Direct { peer, endpoint } => {
            let short = short_peer(peer.as_str());
            if !shared.contains(peer) {
                return short;
            }
            endpoint.as_ref().map_or_else(
                || short.clone(),
                |e| {
                    fill(
                        placeholder_en::text(UiText::DirectTitle),
                        &[("peer", &short), ("route", e.as_str())],
                    )
                },
            )
        }
    }
}

/// The envelope an own card row is held under: the card's id, which the
/// item is keyed by as any outbound row is, and no text -- the row draws
/// the card ([`MessageItem::card`]).
fn card_envelope(app_message_id: String) -> HumanChatV2 {
    HumanChatV2 {
        v: 2,
        kind: interweave_human_chat_protocol::MessageKind::Text,
        app_message_id,
        text: String::new(),
        reply_to: None,
        sent_at_ms: None,
        from_endpoint: None,
    }
}

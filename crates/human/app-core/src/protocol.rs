// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the model side asks of the facade side, and what the facade side
//! reports back. The two sides may live on different threads -- on the
//! desktop the facade runs on its own runtime, because `ipc-client` loses
//! its lease when a session's events go undrained -- so everything that
//! crosses is a plain value.

use interweave_human_chat_protocol::HumanChatV2;
use interweave_human_client_api::{
    AddressAnswer, AddressChangeFailure, AddressList, AddressProblem, ClientEvent, Destination,
    Diagnostics, Received, SendError, ShowAddresses, TrustList, TrustProblem, TrustSetFailure,
};
use interweave_human_store::RowId;
use interweave_human_ui_model::{
    CardOutcome, ContactView, ConversationKey, IdentityView, ItemKey, ListedInbound,
    ListedOutbound, PhotoEdit, Proposal, ProposalContext, Table, TrustChange,
};
use interweave_transport_api::TransportIdentity;

/// A person's intent, resolved by the model, for the facade side to carry
/// out against the facade and the store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    /// Send `draft` to the conversation `key`.
    Send {
        /// The conversation.
        key: ConversationKey,
        /// The text as the person wrote it.
        draft: String,
    },
    /// Mark an unread row read.
    MarkRead(RowId),
    /// Keep the read message `item`, from the copy `from`'s read or
    /// unkeep handed back.
    Keep {
        /// The item.
        item: ItemKey,
        /// The row whose read or unkeep handed back the content.
        from: (Table, RowId),
    },
    /// Remove Keep from a kept row.
    Unkeep(RowId),
    /// Retry a pending row now.
    Retry(RowId),
    /// Cancel a pending row.
    Cancel(RowId),
    /// Take a cancel back: restore the record the cancel of this row
    /// deleted, if the facade side still holds it.
    Restore(RowId),
    /// Let go of the record held for the Undo of this row's cancel: its
    /// snackbar closed.
    ForgetCancelled(RowId),
    /// Let go of the read copy a Keep of a now hidden message would have
    /// used, by the row whose read or unkeep handed it back.
    ForgetCopy((Table, RowId)),
    /// Try opening the session again.
    Reopen,
    /// Re-check storage now.
    RecheckStorage,
    /// Read the trust allowlist.
    ReadTrust,
    /// Make a trust change the person confirmed.
    SetTrust(TrustChange),
    /// Read the addresses this device holds for reaching peers.
    ReadAddresses,
    /// Add these lines, all or none (ADR-0062 decision 4).
    AddAddresses(Vec<String>),
    /// Remove an address added here.
    RemoveAddress(String),
    /// Read this device's own addresses, for Show.
    ReadOwnAddresses,
    /// Accept the card an unread row holds: the row is read, the card
    /// verified again, and the contact written as its proposal said.
    AcceptCard {
        /// The row.
        row: RowId,
        /// The context the person was shown. The facade reads it again
        /// from the store and writes nothing when the two differ
        /// ([`Failure::ProposalChanged`]).
        shown: Box<ProposalContext>,
    },
    /// Decline it: the row is read, and nothing more is written.
    DeclineCard(RowId),
    /// Take back the last Accept or Decline, of this row: an accepted
    /// card's contact goes back to what it was, and the proposal returns,
    /// read, for the person to decide again (contact-cards design
    /// sections 5.4.2 and 5.4.3). Only while the facade side holds it.
    UndoCard(RowId),
    /// Let go of what the Undo of this row's Accept or Decline would have
    /// needed: its snackbar closed.
    ForgetCard(RowId),
    /// Send this identity's own card on a direct conversation's route.
    ShareCard(ConversationKey),
    /// Write the identity's own card as the person edited it.
    SetOwnCard {
        /// The name.
        name: String,
        /// The surname, if any.
        surname: Option<String>,
        /// What happens to the photo.
        photo: PhotoEdit,
    },
    /// The person's own name for a contact.
    RenameContact {
        /// The contact.
        contact_id: String,
        /// The name.
        name: String,
    },
    /// The person's own photo for a contact, or its removal.
    SetContactPhoto {
        /// The contact.
        contact_id: String,
        /// What happens to the photo.
        photo: PhotoEdit,
    },
    /// Delete a contact, its routes and their recorded keys.
    DeleteContact(String),
    /// Say whether a new key would cancel own cards on their way.
    AskNewKey,
    /// Make a new key for the identity in front, as the person confirmed.
    MakeNewKey,
    /// Open the identity in front's key once more.
    RetryKey,
}

/// The store's rows at start, decoded for the model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    /// Pending outbound rows.
    pub pending: Vec<ListedOutbound>,
    /// Unread inbound rows.
    pub unread: Vec<ListedInbound>,
    /// Kept inbound rows.
    pub kept: Vec<ListedInbound>,
    /// Unread rows the store holds that could not be decoded for display:
    /// still held, never shown, counted so the root can say so.
    pub undecodable_unread: usize,
    /// Pending and kept rows likewise.
    pub undecodable_other: usize,
    /// Unread rows that hold a card: proposals, with what Accept would do.
    pub unread_cards: Vec<Proposal>,
    /// The contacts.
    pub contacts: Vec<ContactView>,
    /// The identity in front, when the root holds its key.
    pub identity: Option<IdentityView>,
}

/// What the facade side did, for the model side to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    /// The store's rows at start. Applied before anything else, since the
    /// model treats the pending listing as authoritative.
    Listed(Listing),
    /// The store's unread rows again, after the facade reported unread
    /// content it holds but did not hand over (`ClientEvent::UnreadInStore`).
    UnreadListed {
        /// The rows that could be decoded.
        rows: Vec<ListedInbound>,
        /// Rows held but not decodable for display: never shown, counted.
        undecodable: usize,
    },
    /// The unread rows could not be listed again: content the store holds
    /// stays unshown until the next listing.
    UnreadNotListed(Failure),
    /// A message committed unread and handed over.
    Received(Received),
    /// A contact card committed unread and handed over: a proposal, which
    /// nothing writes to the contacts before the person accepts it.
    ReceivedCard(Box<Proposal>),
    /// A held proposal judged again after a contact changed
    /// ([`UiModel::card_judged`](interweave_human_ui_model::UiModel::card_judged)):
    /// it replaces a proposal the model holds and brings back none.
    ProposalJudged(Box<Proposal>),
    /// A proposal was accepted or declined: it is gone.
    CardResolved {
        /// Its row.
        row: RowId,
        /// What became of it.
        outcome: CardOutcome,
    },
    /// An Undo brought a resolved proposal back, read, judged as the
    /// store stands now.
    ProposalRestored(Box<Proposal>),
    /// An Undo of this row's answer was carried out, but its card could not
    /// be judged again, so its proposal does not return.
    ProposalNotRestored(RowId),
    /// A card's answer or its Undo was carried out, and reading the cards
    /// or contacts back afterwards failed: what is shown may lag the store.
    CardsNotRead(Failure),
    /// The contacts, after a change to them.
    ContactsListed(Vec<ContactView>),
    /// The identity in front, after its own card changed.
    IdentityShown(IdentityView),
    /// Whether a new key would cancel own cards still on their way.
    NewKeyAsked {
        /// Own card rows not yet accepted by the remote transport.
        pending: bool,
    },
    /// The new key was made.
    KeyMade {
        /// The identity, with its new fingerprint, when it could be read.
        identity: Option<IdentityView>,
        /// The new key's fingerprint, whatever could be read.
        fingerprint: String,
        /// Where its counterparts hold or may hold the old key, most
        /// recently shared first.
        share_again: Vec<ConversationKey>,
    },
    /// No new key was made.
    KeyNotMade {
        /// The identity, as its key's open now finds it, when it could be
        /// read.
        identity: Option<IdentityView>,
        /// Why its key cannot be used now.
        problem: interweave_human_ui_model::KeyProblem,
        /// The old file was removed before the failure.
        gone: bool,
    },
    /// The key was opened once more.
    KeyRetried(IdentityView),
    /// This identity's card was shared on `key` as pending `row`.
    CardShared {
        /// The conversation.
        key: ConversationKey,
        /// Its pending row.
        row: RowId,
        /// When, on the wall clock.
        at: u64,
    },
    /// An event from the facade.
    Client(ClientEvent),
    /// The facade's counters changed.
    Diagnostics(Diagnostics),
    /// This node's `PeerId`, once a session is ready and the facade
    /// knows it: Peers reads an address naming it as this device's own,
    /// whether or not the trust list was ever read (design section 5.2).
    ThisDevice(TransportIdentity),
    /// A send was committed as `row`.
    Sent {
        /// The conversation it was sent from.
        key: ConversationKey,
        /// Its pending row.
        row: RowId,
        /// Where it went.
        destination: Destination,
        /// What was sent.
        envelope: HumanChatV2,
        /// When, on the wall clock.
        at: u64,
    },
    /// A send was refused with no row: the composer keeps its text.
    SendRefused {
        /// The conversation.
        key: ConversationKey,
        /// The text that was refused.
        draft: String,
        /// Why.
        error: SendError,
    },
    /// An unread row was marked read.
    Read(RowId),
    /// `item` was kept as `row`.
    Kept {
        /// The item.
        item: ItemKey,
        /// Its kept row.
        row: RowId,
    },
    /// A kept row was unkept.
    Unkept(RowId),
    /// A pending row was cancelled as the person asked. `restorable`
    /// says whether the facade side holds the record an Undo restores.
    Cancelled {
        /// The cancelled row.
        row: RowId,
        /// Whether an Undo can restore it.
        restorable: bool,
    },
    /// The record cancelled as `cancelled` was restored as pending `row`.
    Restored {
        /// The row the cancel ended.
        cancelled: RowId,
        /// The restored row.
        row: RowId,
    },
    /// The daemon's answer to reading the trust allowlist. A failure is
    /// the settings view's to say, not a command's: the command is `Done`.
    TrustRead(Result<TrustList, TrustProblem>),
    /// The daemon's answer to a trust change: the allowlist read back, or
    /// why not and whether the change was made.
    TrustSet {
        /// The change.
        change: TrustChange,
        /// The answer.
        answer: Result<TrustList, TrustSetFailure>,
    },
    /// The runtime's answer to reading the addresses.
    AddressesRead(Result<AddressList, AddressProblem>),
    /// The runtime's answer to reading this device's own addresses.
    OwnAddressesRead(Result<ShowAddresses, AddressProblem>),
    /// The runtime's answer to an add of `lines`.
    AddressesAdded {
        /// The lines, as sent.
        lines: Vec<String>,
        /// The answer.
        answer: Result<AddressAnswer, AddressChangeFailure>,
    },
    /// The runtime's answer to a removal of `address`.
    AddressRemoved {
        /// The address.
        address: String,
        /// The answer.
        answer: Result<AddressAnswer, AddressChangeFailure>,
    },
    /// A command finished. Every command ends with this or `Failed`, so
    /// the model side can offer the same action again; an admin command
    /// whose answer is an update of its own is freed by that answer too.
    Done(Command),
    /// A command could not be carried out; nothing changed.
    Failed {
        /// The command.
        command: Command,
        /// Why, as a class: never content.
        why: Failure,
    },
}

/// Why a command, or a listing, could not be carried out. A class, so it
/// can be logged and shown without carrying a message's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Failure {
    /// The content a Keep needs is no longer held this session: the
    /// facade side dropped its copy past [`crate::READ_COPY_CAP`].
    CopyGone,
    /// No such row: already read, kept, unkept, sent or cancelled.
    NoSuchRow,
    /// The store cannot record it now (unavailable, full, degraded).
    StorageUnavailable,
    /// The store refused it: a kept copy with this message's id already
    /// holds different content, or Keep is not allowed for it.
    Refused,
    /// A stored row could not be read back.
    Corrupt,
    /// Too many of the same action are still on their way: the cancels
    /// whose records an Undo may need ([`crate::CANCELLED_CAP`]).
    TooMany,
    /// The row held no card this build verifies on its route: nothing was
    /// written to the contacts.
    NotACard,
    /// The identity has no key to sign its card with, or no session says
    /// this node's `PeerId`: nothing was sent.
    NoIdentityKey,
    /// The card could not be sent: too large for the route, or refused
    /// before any row was made.
    CardNotSent,
    /// An edit outside a card's bounds: a name or surname too long, empty
    /// or holding a control character, or a photo that cannot be read or
    /// made to fit. Nothing was written.
    OutOfBounds,
    /// What Accept would do changed since the proposal was shown -- another
    /// card accepted on its route, a key recorded, a contact renamed.
    /// Nothing was written, and the proposal is given again as it stands
    /// ([`Update::ProposalJudged`]) for the person to decide.
    ProposalChanged,
}

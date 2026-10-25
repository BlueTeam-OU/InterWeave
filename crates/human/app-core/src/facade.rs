// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade side: owns the transport facade (and through it the store),
//! turns the model side's [`Command`]s into facade and store calls, and
//! reports what happened as [`Update`]s.
//!
//! It is driven, never self-driving: the root calls [`FacadeSide::turn`]
//! often enough that the session's events never wait long -- on the
//! desktop, from its own runtime, since a session whose events go
//! undrained stops answering keepalive pings and loses its lease.

use std::collections::{BTreeMap, HashMap, VecDeque};

use interweave_human_chat_protocol::Card;
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{
    ClientEvent, Destination, Diagnostics, Inbound, OutboundUpdate, RowError, SendError,
    SessionState,
};
use interweave_human_store::identity_key::IdentityKey;
use interweave_human_store::{
    AcceptUndo, ContactRestored, DeletedContact, HumanStore, ReadEphemeral, RemovedPhoto, RowId,
    StoreError,
};
use interweave_human_transport_client::{CancelledSend, TransportClient};
use interweave_human_ui_model::{
    ContactView, ConversationKey, PhotoEdit, Proposal, ProposalContext, Table,
};
use interweave_local_client_api::{AdminBinding, DataSessionBinding};
use interweave_transport_api::{EndpointId, TransportIdentity};

use crate::protocol::{Command, Failure, Listing, Update};
use crate::{cards, listing};
use interweave_human_ui_model::CardOutcome;

/// How many read-and-unkept copies the facade side holds for a later
/// Keep, at most. Each is a message's content, up to the 48 KiB payload
/// limit, so the bound is on memory as much as on count; past it the
/// oldest copy is dropped and a Keep of that message fails (reported as
/// [`Update::Failed`] with [`Failure::CopyGone`], nothing changed, and the
/// model stops offering it).
pub const READ_COPY_CAP: usize = 256;

/// How many cancelled records the facade side holds for an Undo, at most:
/// past it the oldest goes. The open snackbar's record is the newest answer
/// the model has applied, so only records of cancels still in flight are
/// newer, and the model side keeps at most this many minus two of them
/// (`ModelSide::turn`): the open record is never the one evicted, however
/// late the answers come. Records of closed snackbars are let go with
/// `ForgetCancelled` at the next turn, and any still held are older, so
/// evicted first. Each holds a message's bytes, up to the 48 KiB payload
/// limit, so this is about 3 MiB at most.
pub const CANCELLED_CAP: usize = 65;

/// How many received messages one turn hands over, at most.
const DRAIN_PER_TURN: usize = 64;

/// The facade side of the root.
pub struct FacadeSide<B: DataSessionBinding, A: AdminBinding> {
    client: TransportClient<B, A>,
    wall: Box<dyn Fn() -> u64 + Send + Sync>,
    copies: ReadCopies<ReadEphemeral>,
    /// The records cancels deleted, for their snackbars' Undo, by row: let
    /// go when a snackbar closes or is replaced, or its Undo runs. More than
    /// one, since a cancel can run before the Undo of the snackbar it
    /// replaces (review F2 on #289); at most [`CANCELLED_CAP`], the oldest
    /// going first.
    cancelled: VecDeque<CancelledSend>,
    reported: Option<Diagnostics>,
    /// Each pending row's last reported status, until it is terminal: a
    /// listing shows a pending row as a first attempt, and the facade
    /// reports a row only when it changes, so a view listing after the
    /// facade has run is told these ([`Self::outbound_now`]), or a row
    /// the facade holds as not confirmed reads "Sending" until its next
    /// attempt fails (#304 F10).
    outbound: BTreeMap<RowId, OutboundUpdate>,
    /// The identity's own key, which signs its card: opened or made from
    /// the root's [`crate::KeySource`] ([`Self::set_key_source`]), made
    /// anew by `MakeNewKey`, or set directly by a test
    /// ([`Self::set_identity_key`]); without it nothing is shared.
    identity: Option<IdentityKey>,
    /// Where the key lives, when the root gave it ([`Self::set_key_source`]):
    /// what opens it again and makes a new one.
    key_source: Option<crate::KeySource>,
    /// Why the key could not be used, while it cannot.
    key_problem: Option<interweave_human_ui_model::KeyProblem>,
    /// The last Accept or Decline, held for its Undo until its snackbar
    /// closes ([`Command::ForgetCard`]) or the next one replaces it: one
    /// snackbar is up at a time, so one is held (message-swipe design
    /// section 6).
    card_undo: Option<Box<CardUndo>>,
    /// The last delete of a contact or removal of its photo, held for its
    /// Undo until its snackbar closes ([`Command::ForgetContact`]) or
    /// another change replaces it, as `card_undo` is.
    contact_undo: Option<Box<HeldContact>>,
    /// The last hold handed out: each change gets the next.
    contact_holds: u64,
}

/// A contact's change held for its Undo, by the hold the model names it by.
struct HeldContact {
    hold: u64,
    undo: ContactUndo,
}

/// What the Undo of a contact's change writes back.
enum ContactUndo {
    /// A deleted contact, with the routes it held: an Accept that puts one
    /// of them under a contact withdraws it.
    Deleted {
        deleted: DeletedContact,
        routes: Vec<(TransportIdentity, EndpointId)>,
    },
    /// A removed photo.
    Photo(RemovedPhoto),
}

impl ContactUndo {
    fn contact_id(&self) -> &str {
        match self {
            Self::Deleted { deleted, .. } => deleted.contact_id(),
            Self::Photo(removed) => removed.contact_id(),
        }
    }

    const fn change(&self) -> interweave_human_ui_model::ContactChange {
        match self {
            Self::Deleted { .. } => interweave_human_ui_model::ContactChange::Deleted,
            Self::Photo(_) => interweave_human_ui_model::ContactChange::PhotoRemoved,
        }
    }
}

/// What the Undo of an Accept or a Decline needs: the card's read copy,
/// which the proposal returns with, and for an Accept what the store
/// writes back.
struct CardUndo {
    row: RowId,
    held: ReadEphemeral,
    accept: Option<AcceptUndo>,
}

impl<B: DataSessionBinding, A: AdminBinding> FacadeSide<B, A> {
    /// Wrap a facade, `wall` the wall clock in Unix milliseconds. A sent
    /// message names as its `from_endpoint` the endpoint the facade
    /// leases ([`TransportClient::endpoint`]), which an open may move.
    pub fn new(
        client: TransportClient<B, A>,
        wall: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            client,
            wall: Box::new(wall),
            copies: ReadCopies::default(),
            cancelled: VecDeque::new(),
            reported: None,
            outbound: BTreeMap::new(),
            identity: None,
            key_source: None,
            key_problem: None,
            card_undo: None,
            contact_undo: None,
            contact_holds: 0,
        }
    }

    /// The identity's own key, which signs the card Share sends. Its
    /// endpoint must be the one the facade leases.
    pub fn set_identity_key(&mut self, key: IdentityKey) {
        self.identity = Some(key);
        self.key_problem = None;
    }

    /// Where the identity's key lives: it is opened now, by the root's
    /// rule, and a refusal is kept for the identity's page to say
    /// (design/2026-10-24-identity-key.md section 3), never a key made in
    /// its place. Its endpoint must be the one the facade leases.
    pub fn set_key_source(&mut self, source: crate::KeySource) {
        self.key_source = Some(source);
        self.open_key();
    }

    /// Why the identity's key cannot be used, while it cannot: what a
    /// root that logs says.
    #[must_use]
    pub const fn key_problem(&self) -> Option<&interweave_human_ui_model::KeyProblem> {
        self.key_problem.as_ref()
    }

    /// Open the key from its source: held, or its refusal kept.
    fn open_key(&mut self) {
        let Some(source) = &self.key_source else {
            return;
        };
        match source.open(self.client.store_mut()) {
            Ok(key) => {
                self.identity = Some(key);
                self.key_problem = None;
            }
            Err(e) => {
                self.identity = None;
                self.key_problem = Some(source.problem(&e));
            }
        }
    }

    /// The identity in front as its page shows it: its card, and its key
    /// or why it cannot be used. `None` with neither a key nor a source.
    fn own_view(&mut self) -> Result<Option<interweave_human_ui_model::IdentityView>, StoreError> {
        use interweave_human_ui_model::{KeyProblem, OwnKey};
        let (endpoint, key) = match (&self.identity, &self.key_source) {
            (Some(key), _) => (
                key.endpoint().clone(),
                OwnKey::Ready {
                    fingerprint: key.fingerprint().to_string(),
                },
            ),
            (None, Some(source)) => (
                source.endpoint().clone(),
                OwnKey::Unusable(self.key_problem.clone().unwrap_or(KeyProblem::NotNow)),
            ),
            (None, None) => return Ok(None),
        };
        let peer = self.client.local_peer().cloned();
        let at = (self.wall)();
        cards::identity_view(self.client.store_mut(), &endpoint, key, peer.as_ref(), at).map(Some)
    }

    /// The facade, to read its state.
    #[must_use]
    pub const fn client(&self) -> &TransportClient<B, A> {
        &self.client
    }

    /// Every pending row's status as last reported, for a view that lists
    /// after the facade has run: applied after the listing.
    pub fn outbound_now(&self) -> impl Iterator<Item = Update> + '_ {
        self.outbound
            .values()
            .map(|update| Update::Client(ClientEvent::Outbound(update.clone())))
    }

    /// The store's rows, decoded: the first update a root applies.
    ///
    /// # Errors
    /// The store's, when it cannot be read.
    pub fn listing(&mut self) -> Result<Listing, StoreError> {
        let mut listing = listing::list_all(self.client.store_mut())?;
        listing.identity = self.own_view()?;
        Ok(listing)
    }

    /// One turn of the facade: run it, hand over what it received, and
    /// report its events and any change in its counters. `now` is the
    /// caller's monotonic clock in milliseconds.
    pub async fn turn(&mut self, now: u64) -> Vec<Update> {
        let mut updates = Vec::new();
        // Boxed: the facade's futures are large.
        Box::pin(self.client.tick(now)).await;
        let received = Box::pin(self.client.drain_inbound(DRAIN_PER_TURN, now)).await;
        for inbound in received {
            updates.push(match inbound {
                Inbound::Message(received) => Update::Received(received),
                Inbound::Card(card) => match cards::proposal(self.client.store_mut(), card) {
                    Ok(proposal) => Update::ReceivedCard(Box::new(proposal)),
                    // Committed unread: the next listing shows it.
                    Err(e) => Update::UnreadNotListed(store_failure(&e)),
                },
            });
        }
        while let Some(event) = self.client.next_event() {
            self.on_event(event, &mut updates);
        }
        let diagnostics = self.client.diagnostics();
        if self.reported != Some(diagnostics) {
            self.reported = Some(diagnostics);
            updates.push(Update::Diagnostics(diagnostics));
        }
        updates
    }

    /// One facade event, and what follows from it.
    fn on_event(&mut self, event: ClientEvent, updates: &mut Vec<Update>) {
        let relist = matches!(event, ClientEvent::UnreadInStore { .. });
        if let ClientEvent::Outbound(update) = &event {
            if update.status.is_terminal() {
                self.outbound.remove(&update.row);
            } else {
                self.outbound.insert(update.row, update.clone());
            }
        }
        if matches!(event, ClientEvent::Session(SessionState::Ready { .. }))
            && let Some(peer) = self.client.local_peer()
        {
            updates.push(Update::ThisDevice(peer.clone()));
        }
        updates.push(Update::Client(event));
        if relist {
            // What the facade committed but could not hand over is in the
            // store: list it, so the person sees it now.
            match listing::list_unread(self.client.store_mut()) {
                Ok((rows, cards, undecodable)) => {
                    updates.push(Update::UnreadListed { rows, undecodable });
                    for card in cards {
                        updates.push(match cards::proposal(self.client.store_mut(), card) {
                            Ok(proposal) => Update::ReceivedCard(Box::new(proposal)),
                            Err(e) => Update::UnreadNotListed(store_failure(&e)),
                        });
                    }
                }
                Err(e) => updates.push(Update::UnreadNotListed(store_failure(&e))),
            }
        }
    }

    /// Carry out `command`. The updates end with [`Update::Done`] or
    /// [`Update::Failed`] for it, whatever happened.
    pub async fn execute(&mut self, command: Command, now: u64) -> Vec<Update> {
        let mut updates = Vec::new();
        let outcome: Result<(), Failure> = match &command {
            Command::Send { key, draft } => {
                updates.push(self.send(key, draft, now).await);
                Ok(())
            }
            Command::MarkRead(row) => {
                let at = (self.wall)();
                self.client
                    .store_mut()
                    .mark_read(*row, at)
                    .map(|held| {
                        self.copies.insert((Table::Unread, *row), held);
                        updates.push(Update::Read(*row));
                    })
                    .map_err(|e| store_failure(&e))
            }
            Command::Keep { item, from } => {
                let at = (self.wall)();
                match self.copies.get(from) {
                    None => Err(Failure::CopyGone),
                    // A card has no Keep, asked for through any path
                    // (RETENTION.md): its copy stays for Accept or Decline.
                    Some(held) if cards::holds_card(held) => Err(Failure::Refused),
                    Some(held) => match self.client.store_mut().keep(held, at) {
                        Ok(row) => {
                            self.copies.remove(from);
                            updates.push(Update::Kept { item: *item, row });
                            Ok(())
                        }
                        Err(e) => Err(store_failure(&e)),
                    },
                }
            }
            Command::Unkeep(row) => self
                .client
                .store_mut()
                .unkeep(*row, (self.wall)())
                .map(|held| {
                    if let Some(held) = held {
                        self.copies.insert((Table::Kept, *row), held);
                    }
                    updates.push(Update::Unkept(*row));
                })
                .map_err(|e| store_failure(&e)),
            Command::Retry(row) => self.client.retry(*row, now).map_err(|e| row_failure(&e)),
            Command::Cancel(row) => match self.client.cancel(*row) {
                Ok(record) => {
                    // The row's Cancelled status first, so the model reads
                    // whether it may have reached the remote from it.
                    while let Some(event) = self.client.next_event() {
                        self.on_event(event, &mut updates);
                    }
                    updates.push(Update::Cancelled {
                        row: *row,
                        restorable: record.is_some(),
                    });
                    if let Some(record) = record {
                        self.cancelled.push_back(record);
                        while self.cancelled.len() > CANCELLED_CAP {
                            self.cancelled.pop_front();
                        }
                    }
                    Ok(())
                }
                Err(e) => Err(row_failure(&e)),
            },
            Command::Restore(row) => match self
                .cancelled
                .iter()
                .position(|c| c.row() == *row)
                .and_then(|at| self.cancelled.remove(at))
            {
                None => Err(Failure::NoSuchRow),
                Some(record) => match Box::pin(self.client.restore(record, now)).await {
                    Ok(restored) => {
                        updates.push(Update::Restored {
                            cancelled: *row,
                            row: restored,
                        });
                        Ok(())
                    }
                    Err(e) => Err(restore_failure(&e)),
                },
            },
            Command::ForgetCancelled(row) => {
                self.cancelled.retain(|c| c.row() != *row);
                Ok(())
            }
            Command::ForgetCopy(from) => {
                self.copies.remove(from);
                Ok(())
            }
            Command::Reopen => {
                self.client.reopen(now);
                Ok(())
            }
            Command::RecheckStorage => {
                self.client.recheck(now);
                Ok(())
            }
            Command::ReadTrust => {
                updates.push(Update::TrustRead(Box::pin(self.client.trust()).await));
                Ok(())
            }
            Command::SetTrust(change) => {
                let answer =
                    Box::pin(self.client.set_trust(change.peer.clone(), change.allowed)).await;
                updates.push(Update::TrustSet {
                    change: change.clone(),
                    answer,
                });
                Ok(())
            }
            Command::ReadAddresses => {
                let answer = Box::pin(self.client.addresses()).await;
                updates.push(Update::AddressesRead(answer));
                Ok(())
            }
            Command::ReadOwnAddresses => {
                let answer = Box::pin(self.client.own_addresses()).await;
                updates.push(Update::OwnAddressesRead(answer));
                Ok(())
            }
            Command::AddAddresses(lines) => {
                // One add, all or none, whatever its lines (ADR-0062
                // decision 4): the facade chooses the method.
                let answer = Box::pin(self.client.add_addresses(lines.clone())).await;
                updates.push(Update::AddressesAdded {
                    lines: lines.clone(),
                    answer,
                });
                Ok(())
            }
            Command::RemoveAddress(address) => {
                let answer = Box::pin(self.client.remove_address(address.clone())).await;
                updates.push(Update::AddressRemoved {
                    address: address.clone(),
                    answer,
                });
                Ok(())
            }
            Command::AcceptCard { row, shown } => self.accept_card(*row, shown, &mut updates),
            Command::DeclineCard(row) => self.decline_card(*row, &mut updates),
            Command::UndoCard(row) => self.undo_card(*row, &mut updates),
            Command::AskNewKey => cards::own_cards_pending(self.client.store_mut())
                .map(|rows| {
                    updates.push(Update::NewKeyAsked {
                        pending: !rows.is_empty(),
                    });
                })
                .map_err(|e| store_failure(&e)),
            Command::MakeNewKey => self.make_new_key(&mut updates),
            Command::RetryKey => self.retry_key(&mut updates),
            Command::ForgetCard(row) => {
                if self.card_undo.as_ref().is_some_and(|u| u.row == *row) {
                    self.card_undo = None;
                }
                Ok(())
            }
            Command::ShareCard(key) => Box::pin(self.share_card(key, now, &mut updates)).await,
            Command::SetOwnCard {
                name,
                surname,
                photo,
            } => self.set_own_card(name, surname.as_deref(), photo, &mut updates),
            Command::RenameContact { contact_id, name } => {
                self.rename_contact(contact_id, name, &mut updates)
            }
            Command::SetContactPhoto { contact_id, photo } => {
                self.set_contact_photo(contact_id, photo, &mut updates)
            }
            Command::DeleteContact(contact_id) => self.delete_contact(contact_id, &mut updates),
            Command::UndoContact {
                contact_id,
                change,
                hold,
            } => self.undo_contact(contact_id, *change, *hold, &mut updates),
            Command::ForgetContact(hold) => {
                if self.contact_undo.as_ref().is_some_and(|h| h.hold == *hold) {
                    self.contact_undo = None;
                }
                Ok(())
            }
            Command::DeleteRoute {
                contact_id,
                peer,
                endpoint,
            } => self.delete_route(contact_id, peer, endpoint, &mut updates),
        };
        updates.push(match outcome {
            Ok(()) => Update::Done(command),
            Err(why) => Update::Failed { command, why },
        });
        updates
    }

    /// Accept: the row is read, which removes its unread copy (a card has
    /// no Keep), the card it held is verified again on its route, and the
    /// contact written as the proposal said -- only when the store still
    /// says what `shown` says. Whatever stops it short of the write puts
    /// the copy back for this session, so nothing the person can still
    /// decide is lost: a row that held no card, a context that changed
    /// (given again as it stands), a write that failed.
    fn accept_card(
        &mut self,
        row: RowId,
        shown: &ProposalContext,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        let at = (self.wall)();
        let (held, read_now) = self.held_card(row, at)?;
        if read_now {
            updates.push(Update::Read(row));
        }
        let Some((peer, endpoint, card)) = cards::card_of(&held) else {
            self.copies.insert((Table::Unread, row), held);
            return Err(Failure::NotACard);
        };
        let store = self.client.store_mut();
        let written = match cards::context(store, &peer, &endpoint, &card) {
            Err(e) => Err(store_failure(&e)),
            Ok(now) if now != *shown => {
                let received_at = held.received_at();
                // Judged again, not received: a row the model has let go
                // is not brought back.
                updates.push(Update::ProposalJudged(Box::new(Proposal {
                    row,
                    peer: peer.clone(),
                    endpoint: endpoint.clone(),
                    card: card.clone(),
                    received_at,
                    context: now,
                })));
                Err(Failure::ProposalChanged)
            }
            Ok(_) => {
                let mut accepted = cards::accepted(&peer, &endpoint, &card);
                accepted.photo = cards::kept_photo(&card);
                let photo_dropped = card.photo.is_some() && accepted.photo.is_none();
                store
                    .accept_card(&accepted, at)
                    .map(|a| (a, photo_dropped))
                    .map_err(|e| store_failure(&e))
            }
        };
        let (accepted, photo_dropped) = match written {
            Ok(written) => written,
            Err(why) => {
                self.copies.insert((Table::Unread, row), held);
                return Err(why);
            }
        };
        updates.push(Update::CardResolved {
            row,
            outcome: CardOutcome::Accepted {
                contact_id: accepted.contact_id,
                created: accepted.created,
                photo_dropped,
            },
        });
        self.card_undo = Some(Box::new(CardUndo {
            row,
            held,
            accept: Some(accepted.undo),
        }));
        // The route is under a contact now: a deleted contact that held it
        // cannot come back without putting it under two (design 6.3).
        let held_route = matches!(
            self.contact_undo.as_deref(),
            Some(HeldContact { undo: ContactUndo::Deleted { routes, .. }, .. })
                if routes.iter().any(|(p, e)| *p == peer && *e == endpoint)
        );
        if held_route && let Some(held) = self.contact_undo.take() {
            updates.push(Update::ContactUndoWithdrawn(held.hold));
        }
        // The Accept is written: a contacts list that cannot be read now is
        // a problem of its own, never a refusal of what was done (#414 F8).
        updates.push(contacts_update(self.client.store_mut()));
        self.proposals_again(updates);
        Ok(())
    }

    /// Try again: the key opened once more, and the identity as it finds
    /// it (design/2026-10-24-identity-key.md section 6).
    fn retry_key(&mut self, updates: &mut Vec<Update>) -> Result<(), Failure> {
        self.open_key();
        let view = self
            .own_view()
            .map_err(|e| store_failure(&e))?
            .ok_or(Failure::NoIdentityKey)?;
        updates.push(Update::KeyRetried(view));
        Ok(())
    }

    /// Make a new key, as the person confirmed (design/2026-10-24-
    /// identity-key.md section 5.2): every own card the facade still holds
    /// unsent is withdrawn first, with no Undo, since it carries a key the
    /// person no longer has -- a row the transport may hold already is
    /// left to arrive; where the store cannot withdraw one, no key is
    /// made. Then the old key goes and the new one is made, and the
    /// conversations whose counterparts hold or may hold the old key are
    /// listed for a share again. A failure after the old file went is said
    /// as missing, and nothing makes a key in its place this session.
    fn make_new_key(&mut self, updates: &mut Vec<Update>) -> Result<(), Failure> {
        use interweave_human_ui_model::KeyProblem;
        // Refused only before the step starts; from here every path ends
        // it made or not made, so the identity's page never waits on a
        // step that answered nothing (#423 F1): the start's two reads are
        // `a_new_key_starts_from_its_two_reads`, a made and a not-made end
        // the tests/cards.rs key tests, and a refusal the model's
        // `a_make_the_facade_refuses_ends_the_step_not_made`.
        if self.key_source.is_none() {
            return Err(Failure::NoIdentityKey);
        }
        let start = new_key_start(
            cards::own_cards_pending(self.client.store_mut()),
            self.client.store_mut().cards_shared(),
            updates,
        );
        let Some((pending, shared)) = start else {
            self.key_not_made(false, updates);
            return Ok(());
        };
        for (row, _, _) in &pending {
            // Not sent: the client holds it, so it is withdrawn. Sending
            // or unconfirmed: the transport may hold it, so it may arrive.
            let unsent = self.outbound.get(row).is_some_and(|u| {
                matches!(
                    u.status,
                    interweave_human_client_api::OutboundStatus::NeedsAttention { .. }
                )
            });
            if unsent {
                if self.client.cancel(*row).is_err() {
                    self.key_not_made(false, updates);
                    return Ok(());
                }
                while let Some(event) = self.client.next_event() {
                    self.on_event(event, updates);
                }
                updates.push(Update::Cancelled {
                    row: *row,
                    restorable: false,
                });
            }
        }
        match self.key_source.as_ref().map(crate::KeySource::replace) {
            Some(Ok(key)) => {
                let fingerprint = key.fingerprint().to_string();
                self.identity = Some(key);
                self.key_problem = None;
                let identity = self.own_view_or_said(updates);
                updates.push(Update::KeyMade {
                    identity,
                    fingerprint,
                    share_again: shared,
                });
            }
            Some(Err(failed)) => {
                let gone = matches!(
                    failed,
                    interweave_human_store::identity_key::NewKeyFailed::Gone(_)
                );
                if gone {
                    // Never opened again here: an identity that never
                    // shared would be given a key silently, and the person
                    // asked for one and must learn it was not made.
                    self.identity = None;
                    self.key_problem = Some(KeyProblem::Missing);
                } else {
                    self.open_key();
                }
                self.key_not_made(gone, updates);
            }
            None => self.key_not_made(false, updates),
        }
        Ok(())
    }

    /// The step ended with no new key: the identity as its key stands,
    /// and whether the old file went first.
    fn key_not_made(&mut self, gone: bool, updates: &mut Vec<Update>) {
        let problem = self
            .key_problem
            .clone()
            .unwrap_or(interweave_human_ui_model::KeyProblem::NotNow);
        let identity = self.own_view_or_said(updates);
        updates.push(Update::KeyNotMade {
            identity,
            problem,
            gone,
        });
    }

    /// The identity's view, or `None` with the read's failure said: what a
    /// step that has ended reports either way.
    fn own_view_or_said(
        &mut self,
        updates: &mut Vec<Update>,
    ) -> Option<interweave_human_ui_model::IdentityView> {
        match self.own_view() {
            Ok(view) => view,
            Err(e) => {
                updates.push(Update::CardsNotRead(store_failure(&e)));
                None
            }
        }
    }

    /// Decline: the row is read and dropped, which removes its unread copy
    /// and writes nothing to the contacts.
    fn decline_card(&mut self, row: RowId, updates: &mut Vec<Update>) -> Result<(), Failure> {
        let at = (self.wall)();
        let (held, _) = self.held_card(row, at)?;
        self.card_undo = Some(Box::new(CardUndo {
            row,
            held,
            accept: None,
        }));
        updates.push(Update::CardResolved {
            row,
            outcome: CardOutcome::Declined,
        });
        Ok(())
    }

    /// Undo the last Accept or Decline, of `row`, by [`undo_answer`]'s
    /// rule: refused only while the store wrote nothing, which keeps what
    /// the Undo needs; done once the store wrote it, whatever reading
    /// afterwards does.
    fn undo_card(&mut self, row: RowId, updates: &mut Vec<Update>) -> Result<(), Failure> {
        let Some(undo) = self.card_undo.take_if(|u| u.row == row) else {
            return Err(Failure::NoSuchRow);
        };
        let Some((peer, endpoint, card)) = cards::card_of(&undo.held) else {
            self.card_undo = Some(undo);
            return Err(Failure::NotACard);
        };
        let answer = UndoneAnswer {
            row,
            peer,
            endpoint,
            card,
            received_at: undo.held.received_at(),
        };
        match undo_answer(
            self.client.store_mut(),
            answer,
            undo.accept.as_ref(),
            updates,
        ) {
            Err(why) => {
                self.card_undo = Some(undo);
                Err(why)
            }
            Ok(restored) => {
                if restored {
                    self.copies.insert((Table::Unread, row), undo.held);
                }
                if undo.accept.is_some() {
                    self.proposals_again(updates);
                }
                Ok(())
            }
        }
    }

    /// The content of a card's row, its durable copy gone, and whether
    /// this call read it: the copy this session holds if showing the
    /// proposal read it, else read now. A proposal shown and past
    /// [`READ_COPY_CAP`] is gone, as a message's copy is
    /// ([`Failure::CopyGone`]).
    fn held_card(&mut self, row: RowId, at: u64) -> Result<(ReadEphemeral, bool), Failure> {
        if let Some(held) = self.copies.remove(&(Table::Unread, row)) {
            return Ok((held, false));
        }
        match self.client.store_mut().mark_read(row, at) {
            Ok(held) => Ok((held, true)),
            Err(StoreError::NoSuchRow | StoreError::AlreadyRead) => Err(Failure::CopyGone),
            Err(e) => Err(store_failure(&e)),
        }
    }

    /// Share: the identity's own card, signed by its key for the route it
    /// leaves on, this node's `PeerId` and the leased endpoint, sent to
    /// the conversation's route and recorded as shared there.
    async fn share_card(
        &mut self,
        key: &ConversationKey,
        now: u64,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        let ConversationKey::Direct {
            peer,
            endpoint: Some(to),
        } = key
        else {
            return Err(Failure::CardNotSent);
        };
        let identity = self.identity.as_ref().ok_or(Failure::NoIdentityKey)?;
        let (Some(own), Some(from)) = (
            self.client.local_peer().cloned(),
            self.client.endpoint().cloned(),
        ) else {
            return Err(Failure::NoIdentityKey);
        };
        if &from != identity.endpoint() {
            return Err(Failure::NoIdentityKey);
        }
        let at = (self.wall)();
        let own_card = self
            .client
            .store_mut()
            .ensure_own_card(&from, at)
            .map_err(|e| store_failure(&e))?;
        let text = interweave_human_chat_protocol::build_card(
            &format!("{:032x}", rand::random::<u128>()),
            interweave_human_chat_protocol::CardDraft {
                name: &own_card.name,
                surname: own_card.surname.as_deref(),
                photo: own_card.photo.as_deref().map(|p| ("image/jpeg", p)),
            },
            identity.public_key(),
            interweave_human_chat_protocol::CardRoute {
                source_peer: &own,
                source_endpoint: &from,
            },
            Some(at),
            |message| identity.sign(message),
        )
        .map_err(|_| Failure::CardNotSent)?;
        let row = Box::pin(self.client.send_card(peer.clone(), to.clone(), &text, now))
            .await
            .map_err(|_| Failure::CardNotSent)?;
        self.client
            .store_mut()
            .mark_card_shared(peer, to, at)
            .map_err(|e| store_failure(&e))?;
        updates.push(Update::CardShared {
            key: key.clone(),
            row,
            at,
        });
        Ok(())
    }

    /// The identity's own card, as the person edited it: held to the bounds
    /// a receiver holds a card to, a chosen photo re-encoded with no
    /// metadata block first (`card_photo`). Nothing is written otherwise.
    fn set_own_card(
        &mut self,
        name: &str,
        surname: Option<&str>,
        photo: &PhotoEdit,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        // The card is signed when it is shared, not when it is saved:
        // editing it needs no key (design/2026-10-24-identity-key.md
        // section 4.1), only the identity.
        let endpoint = match (&self.identity, &self.key_source) {
            (Some(key), _) => key.endpoint().clone(),
            (None, Some(source)) => source.endpoint().clone(),
            (None, None) => return Err(Failure::NoIdentityKey),
        };
        let at = (self.wall)();
        let store = self.client.store_mut();
        let current = store
            .ensure_own_card(&endpoint, at)
            .map_err(|e| store_failure(&e))?;
        let photo = match photo {
            PhotoEdit::Keep => current.photo,
            PhotoEdit::Remove => None,
            PhotoEdit::Chosen(chosen) => Some(
                crate::card_photo::prepare_photo(chosen)
                    .map_err(|_| Failure::OutOfBounds)?
                    .bytes,
            ),
        };
        interweave_human_chat_protocol::check_draft(interweave_human_chat_protocol::CardDraft {
            name,
            surname,
            photo: photo.as_deref().map(|p| ("image/jpeg", p)),
        })
        .map_err(|_| Failure::OutOfBounds)?;
        store
            .set_own_card(
                &interweave_human_store::OwnCard {
                    endpoint,
                    name: name.to_owned(),
                    surname: surname.map(str::to_owned),
                    photo,
                },
                at,
            )
            .map_err(|e| store_failure(&e))?;
        if let Some(view) = self.own_view().map_err(|e| store_failure(&e))? {
            updates.push(Update::IdentityShown(view));
        }
        Ok(())
    }

    /// Delete a contact (ADR-0059 decision 3): the store deletes it, its
    /// routes and their keys, and the contacts are listed again.
    fn delete_contact(
        &mut self,
        contact_id: &str,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        let store = self.client.store_mut();
        let routes = store
            .contact(contact_id)
            .map_err(|e| store_failure(&e))?
            .map(|c| c.routes.into_iter().map(|r| (r.peer, r.endpoint)).collect())
            .unwrap_or_default();
        let Some(deleted) = store
            .delete_contact_for_undo(contact_id)
            .map_err(|e| store_failure(&e))?
        else {
            return Err(Failure::NoSuchRow);
        };
        let hold = self.hold(ContactUndo::Deleted { deleted, routes });
        // The delete is written: its snackbar opens whatever the reads
        // after it find.
        updates.push(Update::ContactChanged {
            contact_id: contact_id.to_owned(),
            change: interweave_human_ui_model::ContactChange::Deleted,
            hold,
        });
        updates.push(contacts_update(self.client.store_mut()));
        self.proposals_again(updates);
        Ok(())
    }

    /// The Undo of a contact's delete or photo removal (design section
    /// 6.3): what the store removed is written back as it was. Refused,
    /// nothing written, when the facade side no longer holds it, when a
    /// route the deleted contact held is under another contact now, or
    /// when a photo was set since the removal.
    fn undo_contact(
        &mut self,
        contact_id: &str,
        change: interweave_human_ui_model::ContactChange,
        hold: u64,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        let taken = self
            .contact_undo
            .take_if(|h| {
                h.hold == hold && h.undo.contact_id() == contact_id && h.undo.change() == change
            })
            .ok_or(Failure::NoSuchRow)?;
        let store = self.client.store_mut();
        let restored = match &taken.undo {
            ContactUndo::Deleted { deleted, .. } => store
                .restore_contact(deleted)
                .map(|r| r == ContactRestored::Restored),
            ContactUndo::Photo(removed) => store.restore_contact_photo(removed),
        };
        // A refusal and a storage error alike write nothing, and the change
        // stands: the snackbar closed when its Undo was pressed, and what
        // was held goes with the taken hold (the model's refusal path).
        match restored {
            Ok(true) => {}
            Ok(false) => return Err(Failure::Refused),
            Err(e) => return Err(store_failure(&e)),
        }
        updates.push(contacts_update(self.client.store_mut()));
        updates.push(Update::ContactRestored {
            contact_id: contact_id.to_owned(),
            change,
        });
        self.proposals_again(updates);
        Ok(())
    }

    /// Delete one route of a contact: the contacts read back, and the
    /// proposals judged again, since a card from that route now makes a
    /// contact rather than updating this one. The store refuses the
    /// contact's last route, which the model never offers
    /// (`a_route_is_deleted_and_its_contact_keeps_the_other`).
    fn delete_route(
        &mut self,
        contact_id: &str,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        use interweave_human_store::RouteDeleted;
        let store = self.client.store_mut();
        match store
            .delete_route(contact_id, peer, endpoint)
            .map_err(|e| store_failure(&e))?
        {
            RouteDeleted::Deleted => {}
            RouteDeleted::LastRoute => return Err(Failure::Refused),
            RouteDeleted::NoSuchRoute => return Err(Failure::NoSuchRow),
        }
        updates.push(Update::ContactsListed(
            cards::contact_views(store).map_err(|e| store_failure(&e))?,
        ));
        self.proposals_again(updates);
        Ok(())
    }

    /// The person's own name for a contact, held to a card name's bounds.
    fn rename_contact(
        &mut self,
        contact_id: &str,
        name: &str,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        interweave_human_chat_protocol::check_draft(interweave_human_chat_protocol::CardDraft {
            name,
            surname: None,
            photo: None,
        })
        .map_err(|_| Failure::OutOfBounds)?;
        let at = (self.wall)();
        let store = self.client.store_mut();
        if !store
            .set_contact_name(contact_id, name, at)
            .map_err(|e| store_failure(&e))?
        {
            return Err(Failure::NoSuchRow);
        }
        updates.push(Update::ContactsListed(
            cards::contact_views(store).map_err(|e| store_failure(&e))?,
        ));
        self.proposals_again(updates);
        Ok(())
    }

    /// The person's own photo for a contact, or its removal.
    fn set_contact_photo(
        &mut self,
        contact_id: &str,
        photo: &PhotoEdit,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        let photo = match photo {
            PhotoEdit::Keep => return Ok(()),
            PhotoEdit::Remove => return self.remove_contact_photo(contact_id, updates),
            PhotoEdit::Chosen(chosen) => Some(
                crate::card_photo::prepare_photo(chosen)
                    .map_err(|_| Failure::OutOfBounds)?
                    .bytes,
            ),
        };
        let at = (self.wall)();
        let store = self.client.store_mut();
        if !store
            .set_contact_photo(contact_id, photo.as_deref(), at)
            .map_err(|e| store_failure(&e))?
        {
            return Err(Failure::NoSuchRow);
        }
        updates.push(Update::ContactsListed(
            cards::contact_views(store).map_err(|e| store_failure(&e))?,
        ));
        self.proposals_again(updates);
        Ok(())
    }

    /// Hold a contact's change for its Undo, replacing the one held: one
    /// snackbar is up at a time. Its hold, the next, names it.
    fn hold(&mut self, undo: ContactUndo) -> u64 {
        self.contact_holds += 1;
        let hold = self.contact_holds;
        self.contact_undo = Some(Box::new(HeldContact { hold, undo }));
        hold
    }

    /// The person's removal of a contact's photo, held for its Undo. A
    /// contact with no photo still takes the removal as the person's
    /// choice, so no card's photo replaces it (design section 6.3), with
    /// no Undo, since no bytes went (the command line's
    /// `a_contact_is_renamed_its_photo_removed_and_deleted_by_the_person`).
    fn remove_contact_photo(
        &mut self,
        contact_id: &str,
        updates: &mut Vec<Update>,
    ) -> Result<(), Failure> {
        let at = (self.wall)();
        let store = self.client.store_mut();
        match store
            .remove_contact_photo(contact_id, at)
            .map_err(|e| store_failure(&e))?
        {
            Some(removed) => {
                let hold = self.hold(ContactUndo::Photo(removed));
                updates.push(Update::ContactChanged {
                    contact_id: contact_id.to_owned(),
                    change: interweave_human_ui_model::ContactChange::PhotoRemoved,
                    hold,
                });
            }
            None => {
                if !store
                    .set_contact_photo(contact_id, None, at)
                    .map_err(|e| store_failure(&e))?
                {
                    return Err(Failure::NoSuchRow);
                }
            }
        }
        updates.push(contacts_update(self.client.store_mut()));
        self.proposals_again(updates);
        Ok(())
    }

    /// Every card proposal still held, judged again against the store
    /// after the person changed a contact -- an Accept, a rename, a photo,
    /// a delete: a proposal shows what Accept would do as the store stood
    /// when it was judged, and a card that matched its contact (case S,
    /// Close alone) must offer Accept again once the contact is gone or
    /// another card on its route changed it. Those not yet shown
    /// are in the store; those shown and undecided are this session's
    /// copies. The model replaces each it still holds, by its row
    /// ([`Update::ProposalJudged`]).
    fn proposals_again(&mut self, updates: &mut Vec<Update>) {
        match listing::list_unread(self.client.store_mut()) {
            Ok((_, unshown, _)) => {
                for card in unshown {
                    updates.push(judged(cards::proposal(self.client.store_mut(), card)));
                }
            }
            Err(e) => updates.push(Update::UnreadNotListed(store_failure(&e))),
        }
        let shown: Vec<_> = self
            .copies
            .by_row
            .iter()
            .filter(|((table, _), _)| *table == Table::Unread)
            .filter_map(|((_, row), held)| {
                cards::card_of(held).map(|card| (*row, held.received_at(), card))
            })
            .collect();
        for (row, received_at, (peer, endpoint, card)) in shown {
            let context = cards::context(self.client.store_mut(), &peer, &endpoint, &card);
            updates.push(judged(context.map(|context| Proposal {
                row,
                peer,
                endpoint,
                card,
                received_at,
                context,
            })));
        }
    }

    /// Close the session: the endpoint lease is released. The daemon is
    /// not asked to stop -- closing a window never stops the transport
    /// (ADR-0040).
    pub async fn close(&mut self) {
        Box::pin(self.client.close()).await;
    }

    async fn send(&mut self, key: &ConversationKey, draft: &str, now: u64) -> Update {
        let destination = match key {
            ConversationKey::Direct { peer, endpoint } => Destination::Direct {
                peer: peer.clone(),
                endpoint: endpoint.clone(),
            },
            ConversationKey::Channel(channel) => Destination::Broadcast(channel.clone()),
        };
        let at = (self.wall)();
        let envelope = HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{:032x}", rand::random::<u128>()),
            text: draft.to_owned(),
            reply_to: None,
            sent_at_ms: Some(at),
            from_endpoint: self.client.endpoint().cloned(),
        };
        match Box::pin(self.client.send(destination.clone(), &envelope, now)).await {
            Ok(row) => Update::Sent {
                key: key.clone(),
                row,
                destination,
                envelope,
                at,
            },
            Err(error) => Update::SendRefused {
                key: key.clone(),
                draft: draft.to_owned(),
                error,
            },
        }
    }
}

/// A store error's class: what a person or a log may be told.
fn store_failure(error: &StoreError) -> Failure {
    match error {
        StoreError::NoSuchRow => Failure::NoSuchRow,
        StoreError::IdentityConflict { .. } | StoreError::KeepRefused(_) => Failure::Refused,
        StoreError::Corrupt(_)
        | StoreError::Migration(_)
        | StoreError::MalformedAppMessageId { .. }
        | StoreError::TimestampOutOfRange { .. } => Failure::Corrupt,
        _ => Failure::StorageUnavailable,
    }
}

/// A refused restore's class. Nothing was committed or sent.
const fn restore_failure(error: &SendError) -> Failure {
    match error {
        SendError::StorageUnavailable => Failure::StorageUnavailable,
        // Another pending row holds the message, or this session can no
        // longer send there.
        _ => Failure::Refused,
    }
}

const fn row_failure(error: &RowError) -> Failure {
    match error {
        RowError::NoSuchRow => Failure::NoSuchRow,
        RowError::StorageUnavailable => Failure::StorageUnavailable,
    }
}

/// The read-and-unkept copies a Keep needs, keyed by the row whose read
/// or unkeep handed them back; the oldest goes first past the cap.
/// Generic only so the bound is testable: a `ReadEphemeral` is built by the
/// store alone.
struct ReadCopies<V> {
    by_row: HashMap<(Table, RowId), V>,
    order: VecDeque<(Table, RowId)>,
}

impl<V> Default for ReadCopies<V> {
    fn default() -> Self {
        Self {
            by_row: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

impl<V> ReadCopies<V> {
    fn insert(&mut self, from: (Table, RowId), held: V) {
        if self.by_row.insert(from, held).is_none() {
            self.order.push_back(from);
        }
        while self.order.len() > READ_COPY_CAP {
            if let Some(oldest) = self.order.pop_front() {
                self.by_row.remove(&oldest);
            }
        }
    }

    fn get(&self, from: &(Table, RowId)) -> Option<&V> {
        self.by_row.get(from)
    }

    fn remove(&mut self, from: &(Table, RowId)) -> Option<V> {
        let held = self.by_row.remove(from)?;
        self.order.retain(|k| k != from);
        Some(held)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.by_row.len()
    }
}

/// A held proposal judged again: given again as judged, or, when the
/// store cannot judge it, a failure to read the cards back -- never an
/// unread listing, which no judging is (#414 round 4, carried).
fn judged(proposal: Result<Proposal, StoreError>) -> Update {
    match proposal {
        Ok(proposal) => Update::ProposalJudged(Box::new(proposal)),
        Err(e) => Update::CardsNotRead(store_failure(&e)),
    }
}

/// An own card row on its way, with the route it goes to.
type PendingCard = (RowId, TransportIdentity, EndpointId);

/// The start of Make a new key from its two reads, as the store gave them
/// (#423 F4, F5): `None` when the own cards on their way cannot be read --
/// none can be withdrawn, so no key is made -- else those cards and where
/// to share the card again. The list is every conversation that recorded a
/// share, newest first, then the conversations of own cards on their way;
/// when the shares cannot be read there is no list at all, never a part of
/// one passed off as whole (section 5.3's "those, and nobody else"). A read
/// that fails is said in `updates` as [`Update::CardsNotRead`].
fn new_key_start(
    pending: Result<Vec<PendingCard>, StoreError>,
    shared: Result<Vec<(TransportIdentity, EndpointId, u64)>, StoreError>,
    updates: &mut Vec<Update>,
) -> Option<(Vec<PendingCard>, Vec<ConversationKey>)> {
    let pending = match pending {
        Ok(pending) => pending,
        Err(e) => {
            updates.push(Update::CardsNotRead(store_failure(&e)));
            return None;
        }
    };
    let direct = |peer: &TransportIdentity, endpoint: &EndpointId| ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(endpoint.clone()),
    };
    let share_again = match shared {
        Ok(rows) => {
            let mut list: Vec<ConversationKey> = rows
                .iter()
                .map(|(peer, endpoint, _)| direct(peer, endpoint))
                .collect();
            for (_, peer, endpoint) in &pending {
                let key = direct(peer, endpoint);
                if !list.contains(&key) {
                    list.push(key);
                }
            }
            list
        }
        Err(e) => {
            updates.push(Update::CardsNotRead(store_failure(&e)));
            Vec::new()
        }
    };
    Some((pending, share_again))
}

/// What an Undo of a card's answer reads and writes in the store: the
/// human store, or a test's that fails at the step it is told to.
pub(crate) trait CardStore {
    /// What an Accept handed back for its Undo.
    type Undo;
    /// Write back what the Accept changed.
    fn undo_accept(&mut self, undo: &Self::Undo) -> Result<bool, StoreError>;
    /// Judge the card again, as the store stands.
    fn judge(
        &mut self,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
        card: &Card,
    ) -> Result<ProposalContext, StoreError>;
    /// The contacts, for the model.
    fn contacts(&mut self) -> Result<Vec<ContactView>, StoreError>;
}

impl CardStore for HumanStore {
    type Undo = AcceptUndo;

    fn undo_accept(&mut self, undo: &AcceptUndo) -> Result<bool, StoreError> {
        Self::undo_accept(self, undo)
    }

    fn judge(
        &mut self,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
        card: &Card,
    ) -> Result<ProposalContext, StoreError> {
        cards::context(self, peer, endpoint, card)
    }

    fn contacts(&mut self) -> Result<Vec<ContactView>, StoreError> {
        cards::contact_views(self)
    }
}

/// The answered card an Undo brings back.
struct UndoneAnswer {
    row: RowId,
    peer: TransportIdentity,
    endpoint: EndpointId,
    card: Card,
    received_at: u64,
}

/// Undo an answer (#414 F2, F7, F8): `accept` is what an Accept handed
/// back, `None` for a Decline.
///
/// - Refused, `Err`, only while the store wrote nothing: an Accept's
///   write-back the store refused, or a Decline's card that cannot be
///   judged again -- the answer stands as the snackbar said.
/// - Done, `Ok`, once an Accept's write-back is in the store: the
///   proposal returns judged as the store stands (`Ok(true)`), or, when it
///   cannot be judged, [`Update::CardsNotRead`] and
///   [`Update::ProposalNotRestored`] say so (`Ok(false)`); a contacts list
///   that cannot be read is [`Update::CardsNotRead`] too, never a refusal.
fn undo_answer<S: CardStore>(
    store: &mut S,
    answer: UndoneAnswer,
    accept: Option<&S::Undo>,
    updates: &mut Vec<Update>,
) -> Result<bool, Failure> {
    if let Some(accept) = accept {
        store.undo_accept(accept).map_err(|e| store_failure(&e))?;
    }
    let restored = match store.judge(&answer.peer, &answer.endpoint, &answer.card) {
        Ok(context) => {
            updates.push(Update::ProposalRestored(Box::new(Proposal {
                row: answer.row,
                peer: answer.peer,
                endpoint: answer.endpoint,
                card: answer.card,
                received_at: answer.received_at,
                context,
            })));
            true
        }
        Err(e) if accept.is_none() => return Err(store_failure(&e)),
        Err(e) => {
            updates.push(Update::CardsNotRead(store_failure(&e)));
            updates.push(Update::ProposalNotRestored(answer.row));
            false
        }
    };
    if accept.is_some() {
        updates.push(contacts_update(store));
    }
    Ok(restored)
}

/// The contacts after a write to them that is done: listed, or, when they
/// cannot be read, [`Update::CardsNotRead`] -- never a refusal of the
/// write (#414 F8).
fn contacts_update(store: &mut impl CardStore) -> Update {
    match store.contacts() {
        Ok(contacts) => Update::ContactsListed(contacts),
        Err(e) => Update::CardsNotRead(store_failure(&e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use interweave_human_client_api::{ClientEvent, Connectivity, OutboundStatus};
    use interweave_human_store::{
        AppMessageId, HumanStore, InboundOrigin, NewInbound, StoreOptions,
    };
    use interweave_human_transport_client::ClientConfig;
    use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
    use interweave_profile_identity::ProfileIdentity;
    use interweave_transport_api::MediaType;

    /// The facade reports unread content it holds but did not hand over:
    /// the facade side lists the unread rows again, decoded, with the
    /// undecodable ones counted.
    #[test]
    fn unread_in_store_lists_the_unread_rows_again() {
        let human = interweave_transport_api::EndpointId::parse("human").expect("endpoint");
        let config = || FakeConfig {
            peer: ProfileIdentity::generate()
                .transport_identity()
                .expect("peer"),
            endpoints: vec![FakeEndpoint::open(human.clone(), false)],
            default_endpoint: Some(human.clone()),
            queue_bound: 4,
        };
        let (node, _) = FakeNetwork::pair(config(), config());
        let mut store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
        let media =
            MediaType::parse("application/vnd.interweave-human-chat+json;v=2").expect("media");
        let held = br#"{"v":2,"kind":"text","app_message_id":"000000000000000000000000000000aa","text":"held back"}"#;
        for (id, payload) in [
            ("000000000000000000000000000000aa", held.to_vec()),
            ("000000000000000000000000000000ab", b"garbled".to_vec()),
        ] {
            store
                .commit_unread_inbound(&NewInbound {
                    app_message_id: AppMessageId::parse(id.to_owned()).expect("id"),
                    origin: InboundOrigin {
                        peer: node.peer().clone(),
                        endpoint: Some(human.clone()),
                        channel: None,
                    },
                    media_type: Some(media.clone()),
                    payload,
                    received_at: 1,
                })
                .expect("committed");
        }
        let client = TransportClient::new(
            node.clone(),
            node,
            store,
            ClientConfig {
                client_kind: "human-client".to_owned(),
                endpoints: vec![human.clone()],
                channels: vec![],
                max_payload_bytes: 49_152,
            },
            Box::new(|| 1),
            0,
        )
        .expect("facade");
        let mut side = FacadeSide::new(client, || 1);

        let mut updates = Vec::new();
        side.on_event(
            ClientEvent::UnreadInStore { not_handed_over: 2 },
            &mut updates,
        );
        match updates.as_slice() {
            [
                Update::Client(ClientEvent::UnreadInStore { .. }),
                Update::UnreadListed { rows, undecodable },
            ] => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].envelope.text, "held back");
                assert_eq!(*undecodable, 1);
            }
            other => panic!("expected the event and a relist: {other:?}"),
        }

        // Any other event lists nothing.
        let mut updates = Vec::new();
        side.on_event(
            ClientEvent::Connectivity(Connectivity::Unknown),
            &mut updates,
        );
        assert_eq!(
            updates,
            [Update::Client(ClientEvent::Connectivity(
                Connectivity::Unknown
            ))]
        );
    }

    /// A facade over a fake network, its store empty.
    fn quiet_side() -> FacadeSide<FakeNode, FakeNode> {
        let human = interweave_transport_api::EndpointId::parse("human").expect("endpoint");
        let config = || FakeConfig {
            peer: ProfileIdentity::generate()
                .transport_identity()
                .expect("peer"),
            endpoints: vec![FakeEndpoint::open(human.clone(), false)],
            default_endpoint: Some(human.clone()),
            queue_bound: 4,
        };
        let (node, _) = FakeNetwork::pair(config(), config());
        let store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
        let client = TransportClient::new(
            node.clone(),
            node,
            store,
            ClientConfig {
                client_kind: "human-client".to_owned(),
                endpoints: vec![human],
                channels: vec![],
                max_payload_bytes: 49_152,
            },
            Box::new(|| 1),
            0,
        )
        .expect("facade");
        FacadeSide::new(client, || 1)
    }

    /// A view listing after the facade has run reads each pending row's
    /// last status, not the listing's first attempt, and a row that went
    /// terminal is not told again (#304 F10).
    #[test]
    fn a_late_view_is_told_each_pending_rows_last_status() {
        let mut side = quiet_side();
        let update = |row: i64, status: OutboundStatus| OutboundUpdate {
            row: RowId::from_stored(row),
            app_message_id: AppMessageId::parse(format!("{row:032x}")).expect("id"),
            status,
            last_code: None,
        };
        let unconfirmed = OutboundStatus::Unconfirmed {
            next_retry_at: Some(300_000),
            last_problem: None,
        };
        let mut sink = Vec::new();
        for event in [
            update(
                1,
                OutboundStatus::Sending {
                    attempts: 1,
                    next_retry_at: Some(1_000),
                    last_problem: None,
                },
            ),
            update(2, OutboundStatus::Published),
            update(1, unconfirmed.clone()),
            update(3, unconfirmed.clone()),
            update(
                3,
                OutboundStatus::Cancelled {
                    may_have_reached: true,
                },
            ),
        ] {
            side.on_event(ClientEvent::Outbound(event), &mut sink);
        }
        assert_eq!(
            side.outbound_now().collect::<Vec<_>>(),
            [Update::Client(ClientEvent::Outbound(update(
                1,
                unconfirmed
            )))],
            "row 1's latest only; rows 2 and 3 ended"
        );
    }

    #[test]
    fn the_copies_never_hold_more_than_the_cap_and_drop_the_oldest() {
        let row = |n: usize| {
            (
                Table::Unread,
                RowId::from_stored(i64::try_from(n).expect("small")),
            )
        };
        let mut copies = ReadCopies::default();
        for n in 0..=READ_COPY_CAP {
            copies.insert(row(n), n);
        }
        assert_eq!(copies.len(), READ_COPY_CAP, "never past the cap");
        assert!(copies.get(&row(0)).is_none(), "the oldest went first");
        assert_eq!(copies.get(&row(READ_COPY_CAP)), Some(&READ_COPY_CAP));
        // A re-insert of a held row does not count twice.
        copies.insert(row(READ_COPY_CAP), 0);
        assert_eq!(copies.len(), READ_COPY_CAP);
        assert_eq!(copies.order.len(), READ_COPY_CAP);
        copies.remove(&row(READ_COPY_CAP));
        assert_eq!(
            copies.order.len(),
            READ_COPY_CAP - 1,
            "removal clears the order too"
        );
    }

    /// The store step a test store fails at.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Step {
        Write,
        Judge,
        Contacts,
    }

    /// A store that fails at the step it is told to, recording a write.
    #[derive(Default)]
    struct Failing {
        at: Option<Step>,
        written: bool,
    }

    impl Failing {
        fn at(step: Step) -> Self {
            Self {
                at: Some(step),
                written: false,
            }
        }

        fn fails(&self, step: Step) -> Result<(), StoreError> {
            if self.at == Some(step) {
                return Err(StoreError::Corrupt("told to".to_owned()));
            }
            Ok(())
        }
    }

    impl super::CardStore for Failing {
        type Undo = ();

        fn undo_accept(&mut self, (): &()) -> Result<bool, StoreError> {
            self.fails(Step::Write)?;
            self.written = true;
            Ok(true)
        }

        fn judge(
            &mut self,
            _: &TransportIdentity,
            _: &EndpointId,
            _: &Card,
        ) -> Result<ProposalContext, StoreError> {
            self.fails(Step::Judge)?;
            Ok(ProposalContext {
                updates: None,
                shares_node: Vec::new(),
                same_name: Vec::new(),
                key_matches: Vec::new(),
                key: interweave_human_ui_model::KeyView::New,
                fingerprint: "0123456789abcdef".to_owned(),
                changes_nothing: false,
                values: interweave_human_ui_model::ValueChanges::default(),
            })
        }

        fn contacts(&mut self) -> Result<Vec<ContactView>, StoreError> {
            self.fails(Step::Contacts)?;
            Ok(Vec::new())
        }
    }

    fn answer() -> super::UndoneAnswer {
        super::UndoneAnswer {
            row: RowId::from_stored(7),
            peer: TransportIdentity::parse("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN")
                .expect("a PeerId"),
            endpoint: EndpointId::parse("human").expect("an endpoint"),
            card: Card {
                app_message_id: "0".repeat(32),
                name: "Ada".to_owned(),
                surname: None,
                photo: None,
                identity_key: [1; 32],
                sent_at_ms: None,
                from_endpoint: None,
            },
            received_at: 1,
        }
    }

    fn kinds(updates: &[Update]) -> Vec<&'static str> {
        updates
            .iter()
            .map(|u| match u {
                Update::ProposalRestored(_) => "restored",
                Update::ProposalNotRestored(_) => "not restored",
                Update::ContactsListed(_) => "contacts",
                Update::CardsNotRead(_) => "not read",
                _ => "other",
            })
            .collect()
    }

    /// #414 F2, F7, F8: an Undo is refused only while the store wrote
    /// nothing; once an Accept's write-back is in, it is done, and what
    /// could not be read afterwards is said as a problem of its own.
    #[test]
    fn an_undo_is_refused_only_while_the_store_wrote_nothing() {
        let run = |store: &mut Failing, accept: bool| {
            let mut updates = Vec::new();
            let end = super::undo_answer(store, answer(), accept.then_some(&()), &mut updates);
            (end, kinds(&updates))
        };
        // An Accept's write-back refused: nothing written, refused.
        let mut s = Failing::at(Step::Write);
        assert_eq!(run(&mut s, true), (Err(Failure::Corrupt), vec![]));
        // A Decline whose card cannot be judged: it wrote nothing, refused.
        let mut s = Failing::at(Step::Judge);
        assert_eq!(run(&mut s, false), (Err(Failure::Corrupt), vec![]));
        // An Accept written, its card not judged: done, not back, said.
        let mut s = Failing::at(Step::Judge);
        assert_eq!(
            run(&mut s, true),
            (Ok(false), vec!["not read", "not restored", "contacts"])
        );
        assert!(s.written);
        // An Accept written and judged, its contacts unread: done, back.
        let mut s = Failing::at(Step::Contacts);
        assert_eq!(run(&mut s, true), (Ok(true), vec!["restored", "not read"]));
        // Everything read: back, with the contacts.
        let mut s = Failing::default();
        assert_eq!(run(&mut s, true), (Ok(true), vec!["restored", "contacts"]));
        assert_eq!(
            run(&mut Failing::default(), false),
            (Ok(true), vec!["restored"])
        );
    }

    /// #423 F4, F5: Make a new key starts from its two reads -- no key
    /// when the own cards on their way cannot be read, no list when the
    /// shares cannot be, each failure said; otherwise every share newest
    /// first and then the conversations of cards on their way, once each.
    #[test]
    fn a_new_key_starts_from_its_two_reads() {
        use super::new_key_start;
        let peer = |s: &str| TransportIdentity::parse(s).expect("a PeerId");
        let (p, q) = (
            peer("12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN"),
            peer("12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5"),
        );
        let human = EndpointId::parse("human").expect("an endpoint");
        let key = |p: &TransportIdentity| ConversationKey::Direct {
            peer: p.clone(),
            endpoint: Some(human.clone()),
        };
        let corrupt = || StoreError::Corrupt("told to".to_owned());
        let pending = vec![
            (RowId::from_stored(1), q.clone(), human.clone()),
            (RowId::from_stored(2), p.clone(), human.clone()),
        ];

        let mut said = Vec::new();
        assert_eq!(
            new_key_start(Err(corrupt()), Ok(Vec::new()), &mut said),
            None
        );
        assert_eq!(said, vec![Update::CardsNotRead(Failure::Corrupt)]);

        let mut said = Vec::new();
        assert_eq!(
            new_key_start(Ok(pending.clone()), Err(corrupt()), &mut said),
            Some((pending.clone(), Vec::new())),
            "no list, never the pending part of one"
        );
        assert_eq!(
            said,
            vec![Update::CardsNotRead(Failure::Corrupt)],
            "and said"
        );

        let mut said = Vec::new();
        assert_eq!(
            new_key_start(
                Ok(pending.clone()),
                Ok(vec![(p.clone(), human.clone(), 20)]),
                &mut said
            ),
            Some((pending, vec![key(&p), key(&q)])),
            "every share, then the cards on their way, once each"
        );
        assert!(said.is_empty());
    }

    /// A proposal the store cannot judge again is a failure to read the
    /// cards back, not an unread listing that never ran.
    #[test]
    fn a_proposal_not_judged_again_is_a_card_read_failure() {
        assert_eq!(
            super::judged(Err(StoreError::Corrupt("told to".to_owned()))),
            Update::CardsNotRead(Failure::Corrupt)
        );
    }
}

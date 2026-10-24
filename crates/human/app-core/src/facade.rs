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

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{
    AddressChangeFailure, AddressProblem, ClientEvent, Destination, Diagnostics, Inbound,
    OutboundUpdate, RowError, SendError, SessionState,
};
use interweave_human_store::identity_key::IdentityKey;
use interweave_human_store::{ReadEphemeral, RowId, StoreError};
use interweave_human_transport_client::{CancelledSend, TransportClient};
use interweave_human_ui_model::{ConversationKey, PhotoEdit, Proposal, ProposalContext, Table};
use interweave_local_client_api::{AdminBinding, DataSessionBinding};

use crate::protocol::{Command, Failure, Listing, Update};
use crate::{cards, listing};

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
    /// The identity's own key, which signs its card: the root opens or
    /// creates it at start (`identity_key`), and without it nothing is
    /// shared.
    identity: Option<IdentityKey>,
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
        }
    }

    /// The identity's own key, which signs the card Share sends. Its
    /// endpoint must be the one the facade leases.
    pub fn set_identity_key(&mut self, key: IdentityKey) {
        self.identity = Some(key);
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
        if let Some(key) = &self.identity {
            let peer = self.client.local_peer().cloned();
            let at = (self.wall)();
            listing.identity = Some(cards::identity_view(
                self.client.store_mut(),
                key,
                peer.as_ref(),
                at,
            )?);
        }
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
            Command::AddAddresses(lines) => {
                let answer = match lines.as_slice() {
                    [line] => Box::pin(self.client.add_address(line.clone())).await,
                    // Several lines are one add, all or none, and never
                    // one call per line (ADR-0062 decision 4): the method
                    // that takes them comes at IPC 2.9, so until then the
                    // add is not made, as a runtime without it answers.
                    _ => Err(AddressChangeFailure::NotMade(AddressProblem::Incompatible)),
                };
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
                updates.push(Update::ReceivedCard(Box::new(Proposal {
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
                store
                    .accept_card(&accepted, at)
                    .map(|_| ())
                    .map_err(|e| store_failure(&e))
            }
        };
        if let Err(why) = written {
            self.copies.insert((Table::Unread, row), held);
            return Err(why);
        }
        updates.push(Update::CardResolved(row));
        updates.push(Update::ContactsListed(
            cards::contact_views(self.client.store_mut()).map_err(|e| store_failure(&e))?,
        ));
        self.proposals_again(updates);
        Ok(())
    }

    /// Decline: the row is read and dropped, which removes its unread copy
    /// and writes nothing to the contacts.
    fn decline_card(&mut self, row: RowId, updates: &mut Vec<Update>) -> Result<(), Failure> {
        let at = (self.wall)();
        self.held_card(row, at)?;
        updates.push(Update::CardResolved(row));
        Ok(())
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
        let identity = self.identity.as_ref().ok_or(Failure::NoIdentityKey)?;
        let at = (self.wall)();
        let store = self.client.store_mut();
        let current = store
            .ensure_own_card(identity.endpoint(), at)
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
                    endpoint: identity.endpoint().clone(),
                    name: name.to_owned(),
                    surname: surname.map(str::to_owned),
                    photo,
                },
                at,
            )
            .map_err(|e| store_failure(&e))?;
        let peer = self.client.local_peer().cloned();
        let view = cards::identity_view(self.client.store_mut(), identity, peer.as_ref(), at)
            .map_err(|e| store_failure(&e))?;
        updates.push(Update::IdentityShown(view));
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
        if !store
            .delete_contact(contact_id)
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
            PhotoEdit::Remove => None,
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

    /// Every card proposal still held, judged again against the store
    /// after the person changed a contact -- an Accept, a rename, a photo,
    /// a delete: a proposal shows what Accept would do as the store stood
    /// when it was judged, and a card that matched its contact (case S,
    /// Close alone) must offer Accept again once the contact is gone or
    /// another card on its route changed it. Those not yet shown
    /// are in the store; those shown and undecided are this session's
    /// copies. The model replaces each by its row.
    fn proposals_again(&mut self, updates: &mut Vec<Update>) {
        match listing::list_unread(self.client.store_mut()) {
            Ok((_, unshown, _)) => {
                for card in unshown {
                    updates.push(match cards::proposal(self.client.store_mut(), card) {
                        Ok(proposal) => Update::ReceivedCard(Box::new(proposal)),
                        Err(e) => Update::UnreadNotListed(store_failure(&e)),
                    });
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
            updates.push(
                match cards::context(self.client.store_mut(), &peer, &endpoint, &card) {
                    Ok(context) => Update::ReceivedCard(Box::new(Proposal {
                        row,
                        peer,
                        endpoint,
                        card,
                        received_at,
                        context,
                    })),
                    Err(e) => Update::UnreadNotListed(store_failure(&e)),
                },
            );
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
}

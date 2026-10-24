// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Contact cards in the model (plan §29 step 2; `human-client-ui.md` §3
//! and §13; ADR-0059, ADR-0063).
//!
//! A received card is a PROPOSAL, held apart from the messages: shown in
//! its conversation with the full `PeerId` beside the name it claims,
//! never in the conversation list, a notification or an announcement,
//! and offering Accept and Decline and nothing else -- no Keep. What it
//! would do is told before it runs: whether Accept creates a contact or
//! updates one (whose own edits stay), a contact of the same name on
//! another node, a contact sharing its node, and a key change in plain
//! words with both fingerprints. The facade computes that context, since
//! only it holds the store and the key; the model only shows it.
//!
//! A conversation's title becomes its contact's name once the person
//! accepted a card on its route, told apart by the short `PeerId` when
//! another contact has the same name.

use std::collections::BTreeMap;

use interweave_human_chat_protocol::Card;
use interweave_human_core::RowId;
use interweave_transport_api::{EndpointId, TransportIdentity};

use super::swipe::{Confirmation, FocusTarget};
use super::{ConversationKey, Intent, RouteLabel, UiModel};
use crate::labels::UiText;
use unicode_normalization::UnicodeNormalization as _;

/// A photo decoded for display, after Accept or from the person's own
/// card: the facade decodes it within the card's 256 × 256 bound. A
/// received photo is not decoded before Accept: the model holds the
/// card's bytes as they came, and a proposal exposes only
/// [`ProposalItem::has_photo`] (app-core's
/// `a_photo_reaches_the_screens_decoded_only_once_it_is_accepted_with_its_time`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Photo {
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// Its pixels, RGBA, row by row.
    pub rgba: Vec<u8>,
}

/// Why a name typed for a card or a contact cannot be saved, checked on
/// Save and shown under its field (design section 5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    /// Empty, or spaces only.
    Needed,
    /// Over 64 characters after NFC.
    TooLong,
    /// A control, format, line or paragraph separator character.
    BadCharacters,
}

impl NameProblem {
    /// What its field says.
    #[must_use]
    pub const fn text(self) -> UiText {
        match self {
            Self::Needed => UiText::CardNameNeeded,
            Self::TooLong => UiText::CardNameTooLong,
            Self::BadCharacters => UiText::CardNameBadCharacters,
        }
    }
}

/// What is wrong with `name` as a card's name or surname, by the card's
/// own rule (`check_draft`, HUMAN-CHAT.md section Contact card), or
/// nothing: a name the facade would refuse is refused here first, with
/// the reason a person can act on.
#[must_use]
pub fn name_problem(name: &str) -> Option<NameProblem> {
    if name.trim().is_empty() {
        return Some(NameProblem::Needed);
    }
    let passes = |name: &str| {
        interweave_human_chat_protocol::check_draft(interweave_human_chat_protocol::CardDraft {
            name,
            surname: None,
            photo: None,
        })
        .is_ok()
    };
    if passes(name) {
        return None;
    }
    // The rule counts after NFC, which can lengthen a name. A character it
    // refuses is found first, in pieces short enough that length is not
    // the reason (a prefix of an NFC string is NFC), so a long name with a
    // line break is asked to lose the line break, which the person cannot
    // otherwise see; only a clean name is too long.
    let nfc: Vec<char> = name.nfc().collect();
    let max = interweave_human_chat_protocol::MAX_CARD_NAME_SCALARS;
    let clean = nfc
        .chunks(max)
        .all(|piece| passes(&piece.iter().collect::<String>()));
    if clean && nfc.len() > max {
        Some(NameProblem::TooLong)
    } else {
        Some(NameProblem::BadCharacters)
    }
}

/// What an edit does to a photo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhotoEdit {
    /// Leave it as it is.
    Keep,
    /// Remove it.
    Remove,
    /// The image the person chose, as its file holds it: re-encoded and
    /// stripped of every metadata block before anything keeps it.
    Chosen(Vec<u8>),
}

/// A contact named in a proposal's context.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContactRef {
    /// The store's id for it.
    pub contact_id: String,
    /// Its name, as accepted or edited.
    pub name: String,
}

/// How a card's identity key stands against its route's record.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyView {
    /// The route records no key: the first card on it, or a typed one.
    New,
    /// The card carries the recorded key.
    Same,
    /// The card carries another key: a key change, shown before Accept.
    Changed {
        /// The recorded key's fingerprint.
        recorded: String,
    },
}

/// What a received card would do, as the facade read it from the store.
/// Accept carries the one the person was shown, and the facade refuses it
/// when the store no longer agrees, so no Accept writes what was not shown.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProposalContext {
    /// The contact holding the card's route, which Accept updates.
    pub updates: Option<ContactRef>,
    /// Contacts on another endpoint of the card's `PeerId`.
    pub shares_node: Vec<ContactRef>,
    /// Contacts of the card's name on other `PeerId`s.
    pub same_name: Vec<ContactRef>,
    /// Contacts holding the card's key on another `PeerId`.
    pub key_matches: Vec<ContactRef>,
    /// The key against the route's record.
    pub key: KeyView,
    /// The card's key's fingerprint, 16 lowercase hex characters.
    pub fingerprint: String,
    /// Accept would write nothing (design section 5.4.1, case S): a
    /// contact holds the route under the key the card carries, and each
    /// of its values is the person's or already the card's. The proposal
    /// then offers Close alone.
    pub changes_nothing: bool,
}

/// What became of a proposal the person answered: what the confirmation
/// row and the snackbar say (contact-cards design sections 5.4.2, 5.4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardOutcome {
    /// Accepted: a contact was created or updated.
    Accepted {
        /// The contact.
        contact_id: String,
        /// Whether Accept created it (cases N, D and P) rather than
        /// updating one (U and E).
        created: bool,
        /// The card carried a photo that did not decode within its
        /// bound, so the contact was saved without it.
        photo_dropped: bool,
    },
    /// Declined: nothing was written.
    Declined,
}

/// A card committed unread, with its context: what [`UiModel::card_received`]
/// takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// Its unread row.
    pub row: RowId,
    /// The authenticated source peer.
    pub peer: TransportIdentity,
    /// The source endpoint: the remote identity it presents.
    pub endpoint: EndpointId,
    /// The parsed card.
    pub card: Card,
    /// When it was committed, on the local wall clock.
    pub received_at: u64,
    /// What Accept would do.
    pub context: ProposalContext,
}

/// What Accept's context line says it will do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptLine {
    /// Create a contact.
    Creates,
    /// Update this contact, whose own edits stay.
    Updates(String),
    /// Nothing: this contact already matches the card (case S).
    Matches(String),
}

/// A note a proposal shows before Accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalNote {
    /// A contact of the same name on another node.
    SameName(String),
    /// A contact on the same node, another identity of it.
    SharesNode(String),
    /// A contact holding the same key on another node.
    KeyMatches(String),
}

/// A proposal as its conversation shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalItem {
    /// Its unread row, which Accept and Decline name.
    pub row: RowId,
    /// The full `PeerId`, always beside the name.
    pub peer: TransportIdentity,
    /// The route label the peer asserts.
    pub route: RouteLabel,
    /// The name and surname the card claims, as plain text.
    pub name: String,
    /// Whether the card carries a photo; it is not decoded before Accept.
    pub has_photo: bool,
    /// The card's key's fingerprint.
    pub fingerprint: String,
    /// The key against the route's record.
    pub key: KeyView,
    /// What Accept will do.
    pub accept: AcceptLine,
    /// The notes to show before Accept.
    pub notes: Vec<ProposalNote>,
    /// When it arrived.
    pub received_at: u64,
}

impl ProposalItem {
    /// Accept's label, which says what it does before it runs (design
    /// section 5.4.1): Close when the card changes nothing, Update
    /// contact when a contact holds the route,
    /// Accept as a new contact beside one of the same name on another
    /// `PeerId` or one on the same device, else Accept.
    #[must_use]
    pub fn accept_text(&self) -> UiText {
        match self.accept {
            AcceptLine::Matches(_) => UiText::Close,
            AcceptLine::Updates(_) => UiText::UpdateContact,
            AcceptLine::Creates
                if self.notes.iter().any(|n| {
                    matches!(n, ProposalNote::SameName(_) | ProposalNote::SharesNode(_))
                }) =>
            {
                UiText::AcceptAsNew
            }
            AcceptLine::Creates => UiText::CardAccept,
        }
    }
}

/// One route of a contact, for its page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteView {
    /// The `PeerId`.
    pub peer: TransportIdentity,
    /// The remote identity's `EndpointId`.
    pub endpoint: EndpointId,
    /// The fingerprint of the key an accepted card recorded, if any: a
    /// typed route has none, and shows nothing shaped like one.
    pub fingerprint: Option<String>,
}

/// A contact, for its page and for titles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactView {
    /// The store's id for it.
    pub contact_id: String,
    /// Its name.
    pub name: String,
    /// Whether the name came from a card (else the person wrote it).
    pub name_from_card: bool,
    /// When the name was last set, if known.
    pub name_at: Option<u64>,
    /// Where the photo came from, if there is one: from a card or not.
    /// The person's removal is theirs too: a photo of `None` with
    /// `Some(false)` here is one the person took away.
    pub photo_from_card: Option<bool>,
    /// When the photo was last set or removed, if known.
    pub photo_at: Option<u64>,
    /// The photo, decoded for display.
    pub photo: Option<Photo>,
    /// Its routes.
    pub routes: Vec<RouteView>,
}

/// The identity in front, for its page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityView {
    /// The identity.
    pub endpoint: EndpointId,
    /// Its own card's name and surname.
    pub name: String,
    /// Its own card's name alone, as the person typed it.
    pub given: String,
    /// Its own card's surname, if any.
    pub surname: Option<String>,
    /// Whether its own card has a photo.
    pub has_photo: bool,
    /// Its own card's photo, decoded for display: exactly what a share
    /// sends.
    pub photo: Option<Photo>,
    /// Its key's fingerprint.
    pub fingerprint: String,
    /// This node's `PeerId`, beside the fingerprint, once known.
    pub peer: Option<TransportIdentity>,
}

/// How many proposals shown and read the model holds: as many as the
/// facade holds read copies of (app-core's `READ_COPY_CAP`, which a test
/// there holds equal), since past it an Accept could only fail.
pub const READ_PROPOSAL_CAP: usize = 256;

/// The model's card state.
#[derive(Debug, Clone, Default)]
pub(super) struct Cards {
    proposals: BTreeMap<RowId, Proposal>,
    /// The proposals shown and so read: their durable copy is gone, and
    /// they last until the session ends unless the person acts
    /// (RETENTION.md: unread until shown, gone at exit once shown).
    read: std::collections::BTreeSet<RowId>,
    /// The order they were read in, oldest first: the cap's eviction order.
    read_order: std::collections::VecDeque<RowId>,
    contacts: Vec<ContactView>,
    identity: Option<IdentityView>,
    /// When each conversation last shared its identity's card, this
    /// session: what the conversation's details show.
    shared: BTreeMap<ConversationKey, u64>,
    /// The confirmation rows Accepts left this session, in place of their
    /// proposals (contact-cards design 5.4.2), up to [`READ_PROPOSAL_CAP`],
    /// the oldest going first.
    answers: BTreeMap<RowId, CardAnswer>,
}

/// The confirmation row an Accept leaves in its proposal's place: "You
/// saved {name} as a contact." or "You updated your contact {name}.",
/// the name as the contact holds it now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardAnswer {
    /// The card's row: where it stands, and what an Undo takes back.
    pub row: RowId,
    /// The route the card came on.
    pub peer: TransportIdentity,
    /// The route's endpoint.
    pub endpoint: EndpointId,
    /// The contact saved or updated.
    pub contact_id: String,
    /// Whether it was created (cases N, D and P) rather than updated.
    pub created: bool,
    /// The card's photo did not decode and was not saved
    /// (`CardPhotoNotUsed`).
    pub photo_dropped: bool,
    /// When the card arrived: its place in the conversation.
    pub received_at: u64,
}

impl Cards {
    /// The unread proposals of a conversation: what showing it reads.
    pub(super) fn unread_in(&self, peer: &TransportIdentity, endpoint: &EndpointId) -> Vec<RowId> {
        self.proposals
            .values()
            .filter(|p| &p.peer == peer && &p.endpoint == endpoint && !self.read.contains(&p.row))
            .map(|p| p.row)
            .collect()
    }

    /// A row was marked read: if it is a proposal's, it is shown now.
    /// Past [`READ_PROPOSAL_CAP`], the oldest read one goes: the store no
    /// longer holds it, and the facade no longer holds its copy either.
    pub(super) fn read(&mut self, row: RowId) {
        if self.proposals.contains_key(&row) && self.read.insert(row) {
            self.read_order.push_back(row);
        }
        while self.read_order.len() > READ_PROPOSAL_CAP {
            if let Some(oldest) = self.read_order.pop_front() {
                self.read.remove(&oldest);
                self.proposals.remove(&oldest);
            }
        }
    }

    fn resolved(&mut self, row: RowId) -> Option<Proposal> {
        let proposal = self.proposals.remove(&row);
        if self.read.remove(&row) {
            self.read_order.retain(|r| *r != row);
        }
        proposal
    }

    /// Whether a proposal or its confirmation row stands at `row`.
    pub(super) fn shows(&self, row: RowId) -> bool {
        self.proposals.contains_key(&row) || self.answers.contains_key(&row)
    }

    fn answered(&mut self, answer: CardAnswer) {
        self.answers.insert(answer.row, answer);
        while self.answers.len() > READ_PROPOSAL_CAP {
            self.answers.pop_first();
        }
    }

    /// The contact holding a direct route, if any.
    pub(crate) fn contact_for(
        &self,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
    ) -> Option<&ContactView> {
        self.contacts.iter().find(|c| {
            c.routes
                .iter()
                .any(|r| &r.peer == peer && &r.endpoint == endpoint)
        })
    }

    /// Whether another contact than `contact_id` has `name`.
    pub(super) fn name_is_shared(&self, contact_id: &str, name: &str) -> bool {
        self.contacts
            .iter()
            .any(|c| c.contact_id != contact_id && c.name == name)
    }
}

fn card_name(card: &Card) -> String {
    card.surname
        .as_ref()
        .map_or_else(|| card.name.clone(), |s| format!("{} {s}", card.name))
}

impl UiModel {
    /// A card the facade committed unread, with its context.
    pub fn card_received(&mut self, proposal: Proposal) {
        self.cards.proposals.insert(proposal.row, proposal);
    }

    /// A proposal held here, judged again by the facade after a contact
    /// changed: its context replaces the one shown, and whether it was
    /// shown stays as it was. A row no longer held -- resolved, or past
    /// the read bound -- is not brought back.
    pub fn card_judged(&mut self, proposal: Proposal) {
        if let Some(held) = self.cards.proposals.get_mut(&proposal.row) {
            *held = proposal;
        }
    }

    /// What showing a conversation's card proposals reads: each unread
    /// one's [`Intent::MarkRead`], which removes its durable copy, so a
    /// proposal shown and left undecided is gone at the session's end
    /// (RETENTION.md: a card is unread content until shown). A view calls
    /// it when it renders the proposals, never from
    /// [`UiModel::conversation_viewed`]: a conversation shown without
    /// them has not shown the card.
    #[must_use]
    pub fn proposals_viewed(&self, key: &ConversationKey, focused: bool) -> Vec<Intent> {
        let ConversationKey::Direct {
            peer,
            endpoint: Some(at),
        } = key
        else {
            return Vec::new();
        };
        if !focused {
            return Vec::new();
        }
        self.cards
            .unread_in(peer, at)
            .into_iter()
            .map(Intent::MarkRead)
            .collect()
    }

    /// A proposal whose copy is gone: it leaves the conversation, with
    /// nothing to confirm or undo.
    pub fn card_gone(&mut self, row: RowId) {
        self.cards.resolved(row);
    }

    /// The person's answer to a proposal was carried out: an Accept leaves
    /// a confirmation row in its place and a Decline takes it away, each
    /// with the snackbar and its Undo; case S's Close takes it away with
    /// none (contact-cards design 5.4.1 to 5.4.3). A row the model no
    /// longer holds is confirmed by nothing, and what its Undo would have
    /// needed is let go.
    pub fn card_resolved(&mut self, row: RowId, outcome: CardOutcome) {
        let leaving = self.focus_after_proposal(row);
        // The facade side replaced the answer it held for an open card
        // snackbar's Undo with this one: that Undo is gone whatever this
        // answer opens.
        self.close_card_snackbar();
        let Some(proposal) = self.cards.resolved(row) else {
            self.swipe_release_card(row);
            return;
        };
        let conversation = ConversationKey::Direct {
            peer: proposal.peer.clone(),
            endpoint: Some(proposal.endpoint.clone()),
        };
        match outcome {
            CardOutcome::Accepted {
                contact_id,
                created,
                photo_dropped,
            } => {
                self.cards.answered(CardAnswer {
                    row,
                    peer: proposal.peer,
                    endpoint: proposal.endpoint,
                    contact_id,
                    created,
                    photo_dropped,
                    received_at: proposal.received_at,
                });
                let confirms = if created {
                    Confirmation::ContactSaved
                } else {
                    Confirmation::ContactUpdated
                };
                // The row is replaced in place, so a pressed button is
                // gone: focus goes to the confirmation row.
                self.open_card_snackbar(
                    confirms,
                    conversation,
                    row,
                    Some(FocusTarget::Proposal(row)),
                    false,
                );
            }
            CardOutcome::Declined
                if proposal.context.changes_nothing && proposal.context.updates.is_some() =>
            {
                self.swipe_release_card(row);
                self.card_outcome(None, leaving);
            }
            CardOutcome::Declined => {
                self.open_card_snackbar(
                    Confirmation::CardDeclined,
                    conversation,
                    row,
                    leaving,
                    true,
                );
            }
        }
    }

    /// An Undo brought a resolved proposal back, read, judged as the store
    /// stands now: its confirmation row goes, and the Undo is announced as
    /// done with focus on the proposal.
    pub fn card_restored(&mut self, proposal: Proposal) {
        let row = proposal.row;
        self.cards.answers.remove(&row);
        self.cards.proposals.insert(row, proposal);
        self.cards.read(row);
        self.card_undone(row);
    }

    /// Where focus goes when the proposal at `row` leaves its conversation:
    /// the next proposal, else the previous one, else the conversation's
    /// header title.
    fn focus_after_proposal(&self, row: RowId) -> Option<FocusTarget> {
        let proposal = self.cards.proposals.get(&row)?;
        let key = ConversationKey::Direct {
            peer: proposal.peer.clone(),
            endpoint: Some(proposal.endpoint.clone()),
        };
        let rows: Vec<RowId> = self.proposals(&key).iter().map(|p| p.row).collect();
        let at = rows.iter().position(|r| *r == row)?;
        Some(
            rows.get(at + 1)
                .or_else(|| at.checked_sub(1).and_then(|p| rows.get(p)))
                .map_or(FocusTarget::Header(key), |r| FocusTarget::Proposal(*r)),
        )
    }

    /// The confirmation rows Accepts left in a conversation this session,
    /// oldest first.
    #[must_use]
    pub fn card_answers(&self, key: &ConversationKey) -> Vec<CardAnswer> {
        let ConversationKey::Direct {
            peer,
            endpoint: Some(at),
        } = key
        else {
            return Vec::new();
        };
        let mut list: Vec<CardAnswer> = self
            .cards
            .answers
            .values()
            .filter(|a| &a.peer == peer && &a.endpoint == at)
            .cloned()
            .collect();
        list.sort_by_key(|a| (a.received_at, a.row));
        list
    }

    /// This identity's card was shared on `key` at `at`.
    pub fn card_shared(&mut self, key: ConversationKey, at: u64) {
        self.cards.shared.insert(key, at);
    }

    /// When this identity's card was last shared on `key`, this session.
    #[must_use]
    pub fn card_shared_at(&self, key: &ConversationKey) -> Option<u64> {
        self.cards.shared.get(key).copied()
    }

    /// The contacts, as the facade listed them after a change.
    pub fn contacts_listed(&mut self, contacts: Vec<ContactView>) {
        self.cards.contacts = contacts;
    }

    /// The identity in front, as the facade read it.
    pub fn identity_shown(&mut self, identity: IdentityView) {
        self.cards.identity = Some(identity);
    }

    /// The context a proposal was shown with: what its Accept carries.
    #[must_use]
    pub fn proposal_context(&self, row: RowId) -> Option<&ProposalContext> {
        self.cards.proposals.get(&row).map(|p| &p.context)
    }

    /// A conversation's proposals, oldest first.
    #[must_use]
    pub fn proposals(&self, key: &ConversationKey) -> Vec<ProposalItem> {
        let ConversationKey::Direct {
            peer,
            endpoint: Some(at),
        } = key
        else {
            return Vec::new();
        };
        let mut list: Vec<ProposalItem> = self
            .cards
            .proposals
            .values()
            .filter(|p| &p.peer == peer && &p.endpoint == at)
            .map(|p| {
                let c = &p.context;
                let mut notes = Vec::new();
                notes.extend(
                    c.same_name
                        .iter()
                        .map(|r| ProposalNote::SameName(r.name.clone())),
                );
                notes.extend(
                    c.shares_node
                        .iter()
                        .map(|r| ProposalNote::SharesNode(r.name.clone())),
                );
                notes.extend(
                    c.key_matches
                        .iter()
                        .map(|r| ProposalNote::KeyMatches(r.name.clone())),
                );
                ProposalItem {
                    row: p.row,
                    peer: p.peer.clone(),
                    route: RouteLabel::from(&p.endpoint),
                    name: card_name(&p.card),
                    has_photo: p.card.photo.is_some(),
                    fingerprint: c.fingerprint.clone(),
                    key: c.key.clone(),
                    accept: match &c.updates {
                        None => AcceptLine::Creates,
                        Some(u) if c.changes_nothing => AcceptLine::Matches(u.name.clone()),
                        Some(u) => AcceptLine::Updates(u.name.clone()),
                    },
                    notes,
                    received_at: p.received_at,
                }
            })
            .collect();
        list.sort_by_key(|p| (p.received_at, p.row));
        list
    }

    /// The direct conversations holding a proposal: a way to reach one
    /// that holds no message yet, which names nothing of the card.
    #[must_use]
    pub fn proposal_conversations(&self) -> Vec<ConversationKey> {
        let mut keys: Vec<ConversationKey> = self
            .cards
            .proposals
            .values()
            .map(|p| ConversationKey::Direct {
                peer: p.peer.clone(),
                endpoint: Some(p.endpoint.clone()),
            })
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// What a proposal offers: Accept and Decline, and no Keep; a card
    /// that changes nothing, Decline alone, which its row labels Close
    /// (case S): an Accept there would write nothing the person sees.
    #[must_use]
    pub fn card_actions(&self, row: RowId) -> Vec<Intent> {
        match self.cards.proposals.get(&row) {
            Some(p) if p.context.changes_nothing && p.context.updates.is_some() => {
                vec![Intent::DeclineCard(row)]
            }
            Some(_) => vec![Intent::AcceptCard(row), Intent::DeclineCard(row)],
            None => Vec::new(),
        }
    }

    /// Share this conversation's identity's card: in a direct
    /// conversation on a route, never a channel, and only by the person's
    /// act (this intent, which nothing raises on its own).
    #[must_use]
    pub fn share_card(&self, key: &ConversationKey) -> Option<Intent> {
        match key {
            ConversationKey::Direct {
                endpoint: Some(_), ..
            } => Some(Intent::ShareCard(key.clone())),
            _ => None,
        }
    }

    /// Delete a contact the model holds, by the person's act alone.
    #[must_use]
    pub fn delete_contact(&self, contact_id: &str) -> Option<Intent> {
        self.contact(contact_id).map(|c| Intent::DeleteContact {
            contact_id: c.contact_id.clone(),
        })
    }

    /// The contact a direct conversation's route belongs to, if any.
    #[must_use]
    pub fn contact_on(&self, key: &ConversationKey) -> Option<&ContactView> {
        match key {
            ConversationKey::Direct {
                peer,
                endpoint: Some(at),
            } => self.cards.contact_for(peer, at),
            _ => None,
        }
    }

    /// The other contacts of this identity with a route on `peer`: what a
    /// contact page names under a route sharing its device (ADR-0061
    /// decision 4).
    #[must_use]
    pub fn contacts_sharing(
        &self,
        contact_id: &str,
        peer: &TransportIdentity,
    ) -> Vec<&ContactView> {
        self.cards
            .contacts
            .iter()
            .filter(|c| c.contact_id != contact_id && c.routes.iter().any(|r| &r.peer == peer))
            .collect()
    }

    /// The contacts, by name.
    #[must_use]
    pub fn contacts(&self) -> &[ContactView] {
        &self.cards.contacts
    }

    /// The names of every contact of the identity in front with a route
    /// on `peer`, for Peers' rows (add-a-peer design section 5.2): one
    /// per line, never joined, since a typed name may hold "and" or a
    /// comma. Ordered case-insensitively, then by the exact name, in
    /// place of the locale's collation, which no part of the client has
    /// yet.
    #[must_use]
    pub fn contact_names_on(&self, peer: &TransportIdentity) -> Vec<String> {
        let mut names: Vec<String> = self
            .cards
            .contacts
            .iter()
            .filter(|c| c.routes.iter().any(|r| &r.peer == peer))
            .map(|c| c.name.clone())
            .collect();
        names.sort_by(|a, b| {
            a.to_lowercase()
                .cmp(&b.to_lowercase())
                .then_with(|| a.cmp(b))
        });
        names
    }

    /// One contact.
    #[must_use]
    pub fn contact(&self, contact_id: &str) -> Option<&ContactView> {
        self.cards
            .contacts
            .iter()
            .find(|c| c.contact_id == contact_id)
    }

    /// The identity in front.
    #[must_use]
    pub const fn identity(&self) -> Option<&IdentityView> {
        self.cards.identity.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::labels::{UiText, placeholder_en};

    const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
    const Q: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

    fn peer(s: &str) -> TransportIdentity {
        TransportIdentity::parse(s).expect("a PeerId")
    }

    fn endpoint(s: &str) -> EndpointId {
        EndpointId::parse(s).expect("an endpoint")
    }

    fn direct(p: &str, e: &str) -> ConversationKey {
        ConversationKey::Direct {
            peer: peer(p),
            endpoint: Some(endpoint(e)),
        }
    }

    fn row(n: i64) -> RowId {
        RowId::from_stored(n)
    }

    fn context() -> ProposalContext {
        ProposalContext {
            updates: None,
            shares_node: Vec::new(),
            same_name: Vec::new(),
            key_matches: Vec::new(),
            key: KeyView::New,
            fingerprint: "0123456789abcdef".to_owned(),
            changes_nothing: false,
        }
    }

    fn proposal(n: i64, p: &str, name: &str, context: ProposalContext) -> Proposal {
        Proposal {
            row: row(n),
            peer: peer(p),
            endpoint: endpoint("human"),
            card: Card {
                app_message_id: "0".repeat(32),
                name: name.to_owned(),
                surname: Some("Lovelace".to_owned()),
                photo: None,
                identity_key: [1; 32],
                sent_at_ms: None,
                from_endpoint: None,
            },
            received_at: u64::try_from(n).expect("small"),
            context,
        }
    }

    #[test]
    fn proposals_shown_and_read_are_held_up_to_the_cap_oldest_going_first() {
        let mut model = UiModel::new();
        let total = READ_PROPOSAL_CAP + 2;
        for n in 1..=i64::try_from(total).expect("small") {
            model.card_received(proposal(n, P, "Ada", context()));
            model.read(row(n));
        }
        let held = model.proposals(&direct(P, "human"));
        assert_eq!(held.len(), READ_PROPOSAL_CAP);
        assert_eq!(held[0].row, row(3), "the two read first are gone");
        assert!(model.proposal_context(row(1)).is_none());
        // An unread proposal is the store's to bound: it is never evicted.
        model.card_received(proposal(10_000, P, "Ada", context()));
        model.card_received(proposal(10_001, P, "Ada", context()));
        model.read(row(10_001));
        assert!(model.proposal_context(row(10_000)).is_some());
        // A resolved one leaves the order, so it does not count twice.
        model.card_gone(row(10_001));
        assert_eq!(
            model.proposals(&direct(P, "human")).len(),
            READ_PROPOSAL_CAP
        );
    }

    fn contact(id: &str, name: &str, p: &str) -> ContactView {
        ContactView {
            contact_id: id.to_owned(),
            name: name.to_owned(),
            name_from_card: true,
            name_at: None,
            photo_from_card: None,
            photo_at: None,
            photo: None,
            routes: vec![RouteView {
                peer: peer(p),
                endpoint: endpoint("human"),
                fingerprint: Some("0123456789abcdef".to_owned()),
            }],
        }
    }

    #[test]
    fn a_proposal_shows_the_full_peer_id_beside_the_name_and_offers_accept_and_decline_and_no_keep()
    {
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        let shown = model.proposals(&direct(P, "human"));
        assert_eq!(shown.len(), 1);
        assert_eq!(
            shown[0].peer.as_str(),
            P,
            "the full PeerId, never the short one"
        );
        assert_eq!(
            shown[0].name, "Ada Lovelace",
            "the name, a space, the surname"
        );
        assert_eq!(shown[0].fingerprint, "0123456789abcdef");
        assert_eq!(
            model.card_actions(row(1)),
            vec![Intent::AcceptCard(row(1)), Intent::DeclineCard(row(1))],
            "Accept and Decline, and no Keep"
        );
        assert!(
            model.proposals(&direct(Q, "human")).is_empty(),
            "only its own conversation"
        );
    }

    #[test]
    fn a_card_never_reaches_the_conversation_list() {
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        assert!(
            model.conversations().is_empty(),
            "no summary carries its name"
        );
        assert_eq!(model.proposal_conversations(), vec![direct(P, "human")]);
        let titled = model.title(&direct(P, "human"));
        assert!(
            !titled.contains("Ada"),
            "the title is not the card's claim: {titled}"
        );
    }

    #[test]
    fn accepts_context_line_and_notes_say_what_it_will_do_before_it_runs() {
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        assert_eq!(
            model.proposals(&direct(P, "human"))[0].accept,
            AcceptLine::Creates
        );
        let updating = ProposalContext {
            updates: Some(ContactRef {
                contact_id: "c1".to_owned(),
                name: "Countess".to_owned(),
            }),
            same_name: vec![ContactRef {
                contact_id: "c2".to_owned(),
                name: "Ada Lovelace".to_owned(),
            }],
            shares_node: vec![ContactRef {
                contact_id: "c3".to_owned(),
                name: "Ada at work".to_owned(),
            }],
            key: KeyView::Changed {
                recorded: "fedcba9876543210".to_owned(),
            },
            ..context()
        };
        model.card_received(proposal(2, Q, "Ada", updating));
        let shown = &model.proposals(&direct(Q, "human"))[0];
        assert_eq!(shown.accept, AcceptLine::Updates("Countess".to_owned()));
        assert_eq!(
            shown.notes,
            vec![
                ProposalNote::SameName("Ada Lovelace".to_owned()),
                ProposalNote::SharesNode("Ada at work".to_owned())
            ]
        );
        assert_eq!(
            shown.key,
            KeyView::Changed {
                recorded: "fedcba9876543210".to_owned()
            },
            "a key change shown before Accept, with the old fingerprint beside the new"
        );
    }

    #[test]
    fn a_resolved_proposal_is_gone() {
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        model.card_gone(row(1));
        assert!(model.proposals(&direct(P, "human")).is_empty());
        assert!(model.card_actions(row(1)).is_empty());
    }

    #[test]
    fn share_is_offered_in_a_direct_conversation_on_a_route_and_never_in_a_channel() {
        let model = UiModel::new();
        let key = direct(P, "human");
        assert_eq!(model.share_card(&key), Some(Intent::ShareCard(key)));
        let channel = ConversationKey::Channel(
            interweave_transport_api::ChannelId::parse("room").expect("a channel"),
        );
        assert_eq!(model.share_card(&channel), None);
        let no_route = ConversationKey::Direct {
            peer: peer(P),
            endpoint: None,
        };
        assert_eq!(model.share_card(&no_route), None);
    }

    #[test]
    fn a_contacts_name_titles_its_conversation_told_apart_by_the_short_peer_id_when_shared() {
        let mut model = UiModel::new();
        model.contacts_listed(vec![contact("c1", "Ada", P)]);
        // Plain for a terminal or a script, isolated where a view draws it;
        // the list's summary carries both. Mutations: `title` isolated (the
        // CLI's conversations would print U+FFFD around the name);
        // `drawn_title` plain.
        assert_eq!(model.title(&direct(P, "human")), "Ada", "the name, plain");
        assert_eq!(
            model.drawn_title(&direct(P, "human")),
            "\u{2068}Ada\u{2069}",
            "the name, isolated where it is drawn"
        );
        model.contacts_listed(vec![contact("c1", "Ada", P), contact("c2", "Ada", Q)]);
        let p_title = model.drawn_title(&direct(P, "human"));
        let q_title = model.drawn_title(&direct(Q, "human"));
        assert_ne!(
            p_title, q_title,
            "two contacts of one name are told apart in the title"
        );
        assert!(p_title.contains("Ada") && q_title.contains("Ada"));
        assert_eq!(
            p_title,
            crate::labels::fill(
                placeholder_en::text(UiText::ContactTitle),
                &[
                    ("name", &crate::labels::isolated("Ada")),
                    ("peer", &crate::labels::short_peer(P))
                ]
            )
        );
    }

    #[test]
    fn delete_is_offered_for_a_contact_held_and_only_while_it_is() {
        let mut model = UiModel::new();
        assert_eq!(
            model.delete_contact("c1"),
            None,
            "nothing held, nothing offered"
        );
        model.contacts_listed(vec![ContactView {
            contact_id: "c1".to_owned(),
            name: "Ada".to_owned(),
            name_from_card: true,
            name_at: None,
            photo_from_card: None,
            photo_at: None,
            photo: None,
            routes: Vec::new(),
        }]);
        let delete = model.delete_contact("c1").expect("offered");
        assert_eq!(
            delete,
            Intent::DeleteContact {
                contact_id: "c1".to_owned()
            }
        );
        assert!(model.offers(&delete));
        model.contacts_listed(Vec::new());
        assert!(
            !model.offers(&delete),
            "a contact gone is not deleted twice"
        );
    }

    #[test]
    fn a_name_is_refused_on_save_for_the_reason_a_person_can_act_on() {
        assert_eq!(name_problem(""), Some(NameProblem::Needed));
        assert_eq!(
            name_problem("   "),
            Some(NameProblem::Needed),
            "spaces only"
        );
        assert_eq!(name_problem(&"a".repeat(64)), None, "64 is the bound");
        assert_eq!(name_problem(&"a".repeat(65)), Some(NameProblem::TooLong));
        assert_eq!(
            name_problem("Ada\nLovelace"),
            Some(NameProblem::BadCharacters)
        );
        assert_eq!(
            name_problem("Ada\u{202e}"),
            Some(NameProblem::BadCharacters),
            "a bidirectional control, which could close an isolate"
        );
        assert_eq!(name_problem("Ada Lovelace"), None);
        // Counted after NFC, as the rule counts: 64 U+0958 are 128 scalars
        // there, too long and nothing else. Mutation: counting raw chars.
        assert_eq!(
            name_problem(&"\u{0958}".repeat(64)),
            Some(NameProblem::TooLong)
        );
        // A bad character is the reason even in a long name, and past the
        // first 64. Mutations: length tested first; only the first piece
        // checked.
        assert_eq!(
            name_problem(&format!("{}\n", "a".repeat(70))),
            Some(NameProblem::BadCharacters)
        );
        assert_eq!(
            name_problem(&format!("{}\n{}", "a".repeat(10), "a".repeat(70))),
            Some(NameProblem::BadCharacters)
        );
        // What the facade's own rule says, for every case above.
        for (name, ok) in [("Ada", true), ("Ada\u{2069}", false), ("", false)] {
            let draft = interweave_human_chat_protocol::CardDraft {
                name,
                surname: None,
                photo: None,
            };
            assert_eq!(
                interweave_human_chat_protocol::check_draft(draft).is_ok(),
                ok,
                "{name:?}"
            );
            assert_eq!(name_problem(name).is_none(), ok, "{name:?}");
        }
    }

    // A proposal judged again replaces a held one in place, shown state
    // kept, and brings back none the model no longer holds. Mutations:
    // `card_judged` inserting; `card_judged` clearing the read mark.
    #[test]
    fn a_proposal_judged_again_replaces_only_a_held_one_and_keeps_it_shown() {
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        model.read(row(1));
        let changed = ProposalContext {
            changes_nothing: true,
            updates: Some(ContactRef {
                contact_id: "c1".to_owned(),
                name: "Ada".to_owned(),
            }),
            ..context()
        };
        model.card_judged(proposal(1, P, "Ada", changed));
        let shown = model.proposals(&direct(P, "human"));
        assert_eq!(shown[0].accept, AcceptLine::Matches("Ada".to_owned()));
        assert!(
            model.proposals_viewed(&direct(P, "human"), true).is_empty(),
            "still shown"
        );
        model.card_judged(proposal(2, P, "Ada", context()));
        assert_eq!(
            model.proposals(&direct(P, "human")).len(),
            1,
            "row 2 not brought back"
        );
    }

    #[test]
    fn accepts_label_says_whether_it_creates_beside_another_or_updates() {
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        let shown = |m: &UiModel, p: &str| m.proposals(&direct(p, "human"))[0].accept_text();
        assert_eq!(shown(&model, P), UiText::CardAccept, "case N");
        let near = |kind: &str| ContactRef {
            contact_id: kind.to_owned(),
            name: "Ada".to_owned(),
        };
        model.card_received(proposal(
            2,
            Q,
            "Ada",
            ProposalContext {
                same_name: vec![near("d")],
                ..context()
            },
        ));
        assert_eq!(shown(&model, Q), UiText::AcceptAsNew, "case D");
        model.card_gone(row(2));
        model.card_received(proposal(
            3,
            Q,
            "Ada",
            ProposalContext {
                shares_node: vec![near("p")],
                ..context()
            },
        ));
        assert_eq!(shown(&model, Q), UiText::AcceptAsNew, "case P");
        model.card_gone(row(3));
        model.card_received(proposal(
            4,
            Q,
            "Ada",
            ProposalContext {
                updates: Some(near("u")),
                same_name: vec![near("d")],
                ..context()
            },
        ));
        assert_eq!(
            shown(&model, Q),
            UiText::UpdateContact,
            "case U, whatever else"
        );
        model.card_gone(row(4));
        model.card_received(proposal(
            5,
            Q,
            "Ada",
            ProposalContext {
                updates: Some(near("u")),
                changes_nothing: true,
                ..context()
            },
        ));
        assert_eq!(shown(&model, Q), UiText::Close, "case S");
        assert_eq!(
            model.proposals(&direct(Q, "human"))[0].accept,
            AcceptLine::Matches("Ada".to_owned())
        );
        assert_eq!(
            model.card_actions(row(5)),
            vec![Intent::DeclineCard(row(5))],
            "Close alone, which declines"
        );
    }

    #[test]
    fn a_contact_page_names_only_the_other_contacts_on_the_same_peer() {
        let mut model = UiModel::new();
        let mut work = contact("c2", "Ada at work", P);
        work.routes[0].endpoint = endpoint("work");
        model.contacts_listed(vec![contact("c1", "Ada", P), work, contact("c3", "Bob", Q)]);
        let sharing: Vec<&str> = model
            .contacts_sharing("c1", &peer(P))
            .iter()
            .map(|c| c.contact_id.as_str())
            .collect();
        assert_eq!(sharing, vec!["c2"], "not itself, not another device");
        assert_eq!(
            model
                .contact_on(&direct(P, "work"))
                .map(|c| c.contact_id.as_str()),
            Some("c2")
        );
        assert!(model.contact_on(&direct(Q, "work")).is_none());
    }

    fn saved(contact_id: &str) -> CardOutcome {
        CardOutcome::Accepted {
            contact_id: contact_id.to_owned(),
            created: true,
            photo_dropped: false,
        }
    }

    /// Contact-cards design 5.4.2: an Accept leaves a confirmation row in
    /// the proposal's place, focused, with the snackbar "Contact saved"
    /// and its Undo; the Undo asks the facade for the card back, and the
    /// proposal returns read, its confirmation row gone, announced and
    /// focused.
    #[test]
    fn an_accept_leaves_a_confirmation_row_and_its_undo_brings_the_proposal_back_read() {
        use crate::{Announce, Confirmation, FocusTarget};
        let mut model = UiModel::new();
        let key = direct(P, "human");
        model.card_received(proposal(1, P, "Ada", context()));
        model.card_resolved(row(1), saved("c1"));
        assert!(model.proposals(&key).is_empty());
        let answers = model.card_answers(&key);
        assert_eq!(answers.len(), 1);
        assert!(answers[0].created);
        assert_eq!(answers[0].contact_id, "c1");
        let snack = model.snackbar().expect("a snackbar");
        assert_eq!(snack.confirms, Confirmation::ContactSaved);
        assert!(snack.undo);
        assert_eq!(
            model.outcome().and_then(|o| o.focus.clone()),
            Some(FocusTarget::Proposal(row(1)))
        );
        assert_eq!(
            model.outcome().and_then(|o| o.announce),
            None,
            "the focused confirmation row says it; the snackbar is not announced"
        );

        assert_eq!(model.undo(snack.id), Some(Intent::UndoCard(row(1))));
        assert!(model.snackbar().is_none());
        model.card_restored(proposal(1, P, "Ada", context()));
        assert!(
            model.card_answers(&key).is_empty(),
            "the confirmation row goes"
        );
        assert_eq!(model.proposals(&key).len(), 1, "the proposal is back");
        assert!(
            model.proposals_viewed(&key, true).is_empty(),
            "read: showing it again reads nothing"
        );
        let outcome = model.outcome().expect("an outcome");
        assert_eq!(outcome.announce, Some(Announce::Restored));
        assert_eq!(outcome.focus, Some(FocusTarget::Proposal(row(1))));
        assert!(
            model.take_released().cards.is_empty(),
            "an Undo taken releases nothing: the facade gave it back"
        );
    }

    /// 5.4.3: a Decline takes the proposal away, focus to the next one,
    /// with "Card declined" and its Undo; closing the snackbar lets go of
    /// what the facade side held for the Undo.
    #[test]
    fn a_decline_takes_the_row_away_and_its_closed_snackbar_releases_the_card() {
        use crate::{Confirmation, FocusTarget};
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        model.card_received(proposal(2, P, "Ada", context()));
        model.card_resolved(row(1), CardOutcome::Declined);
        let snack = model.snackbar().expect("a snackbar");
        assert_eq!(snack.confirms, Confirmation::CardDeclined);
        assert_eq!(
            model.outcome().and_then(|o| o.announce),
            Some(crate::Announce::Confirmed(Confirmation::CardDeclined)),
            "no row stays to say a Decline: the snackbar is announced"
        );
        assert_eq!(
            model.outcome().and_then(|o| o.focus.clone()),
            Some(FocusTarget::Proposal(row(2))),
            "the next proposal"
        );
        assert!(
            model.card_answers(&direct(P, "human")).is_empty(),
            "no confirmation row"
        );
        model.snackbar_closed(snack.id);
        assert_eq!(model.take_released().cards, vec![row(1)]);

        // The last one leaves for the conversation's header title.
        model.card_resolved(row(2), CardOutcome::Declined);
        assert_eq!(
            model.outcome().and_then(|o| o.focus.clone()),
            Some(FocusTarget::Header(direct(P, "human")))
        );
        // A newer answer replaces the snackbar, and the older Undo's card
        // is let go with it.
        model.card_received(proposal(3, P, "Grace", context()));
        model.card_resolved(row(3), saved("c3"));
        assert_eq!(model.take_released().cards, vec![row(2)]);
    }

    /// Case S's Close removes the row as Decline does, without a snackbar,
    /// and nothing is held for an Undo.
    #[test]
    fn case_s_close_takes_the_row_away_with_no_snackbar() {
        let mut model = UiModel::new();
        let mut same = context();
        same.updates = Some(crate::ContactRef {
            contact_id: "c1".to_owned(),
            name: "Ada".to_owned(),
        });
        same.changes_nothing = true;
        model.card_received(proposal(1, P, "Ada", same));
        model.card_resolved(row(1), CardOutcome::Declined);
        assert!(model.snackbar().is_none());
        assert_eq!(model.take_released().cards, vec![row(1)]);
    }

    /// A refused Undo leaves the answer as the snackbar said: focus where
    /// the answer sent it, and the card let go.
    #[test]
    fn a_refused_undo_leaves_the_answer_and_lets_the_card_go() {
        use crate::FocusTarget;
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        model.card_resolved(row(1), saved("c1"));
        let id = model.snackbar().expect("up").id;
        let undo = model.undo(id).expect("an Undo");
        model.intent_refused(&undo);
        assert_eq!(model.card_answers(&direct(P, "human")).len(), 1);
        assert_eq!(
            model.outcome().and_then(|o| o.focus.clone()),
            Some(FocusTarget::Proposal(row(1)))
        );
        assert_eq!(model.take_released().cards, vec![row(1)]);
    }

    /// #414 F1: an answer that opens no snackbar -- case S's Close here --
    /// still replaces the card the facade side held for the open
    /// snackbar's Undo, so that snackbar closes and offers no Undo the
    /// facade would refuse.
    #[test]
    fn an_answer_with_no_snackbar_closes_the_open_card_snackbar() {
        let mut model = UiModel::new();
        model.card_received(proposal(1, P, "Ada", context()));
        let mut same = context();
        same.updates = Some(crate::ContactRef {
            contact_id: "c1".to_owned(),
            name: "Ada".to_owned(),
        });
        same.changes_nothing = true;
        model.card_received(proposal(2, P, "Ada", same));
        model.card_resolved(row(1), saved("c1"));
        assert!(model.snackbar().is_some(), "Contact saved, with Undo");
        model.card_resolved(row(2), CardOutcome::Declined);
        assert!(model.snackbar().is_none(), "its Undo is gone: closed");
        let mut released = model.take_released().cards;
        released.sort();
        assert_eq!(released, vec![row(1), row(2)]);
    }
}

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

use super::{ConversationKey, Intent, RouteLabel, UiModel};

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactRef {
    /// The store's id for it.
    pub contact_id: String,
    /// Its name, as accepted or edited.
    pub name: String,
}

/// How a card's identity key stands against its route's record.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Where the photo came from, if there is one: from a card or not.
    pub photo_from_card: Option<bool>,
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
    /// Whether its own card has a photo.
    pub has_photo: bool,
    /// Its key's fingerprint.
    pub fingerprint: String,
    /// This node's `PeerId`, beside the fingerprint, once known.
    pub peer: Option<TransportIdentity>,
}

/// The model's card state.
#[derive(Debug, Clone, Default)]
pub(super) struct Cards {
    proposals: BTreeMap<RowId, Proposal>,
    /// The proposals shown and so read: their durable copy is gone, and
    /// they last until the session ends unless the person acts
    /// (RETENTION.md: unread until shown, gone at exit once shown).
    read: std::collections::BTreeSet<RowId>,
    contacts: Vec<ContactView>,
    identity: Option<IdentityView>,
    /// When each conversation last shared its identity's card, this
    /// session: what the conversation's details show.
    shared: BTreeMap<ConversationKey, u64>,
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
    pub(super) fn read(&mut self, row: RowId) {
        if self.proposals.contains_key(&row) {
            self.read.insert(row);
        }
    }

    /// The contact holding a direct route, if any.
    pub(super) fn contact_for(
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

    /// A proposal the person accepted or declined, or whose copy is gone: it is gone from the
    /// conversation, whatever the store did.
    pub fn card_resolved(&mut self, row: RowId) {
        self.cards.proposals.remove(&row);
        self.cards.read.remove(&row);
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
                    accept: c
                        .updates
                        .as_ref()
                        .map_or(AcceptLine::Creates, |u| AcceptLine::Updates(u.name.clone())),
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

    /// What a proposal offers: Accept and Decline, and no Keep.
    #[must_use]
    pub fn card_actions(&self, row: RowId) -> Vec<Intent> {
        if self.cards.proposals.contains_key(&row) {
            vec![Intent::AcceptCard(row), Intent::DeclineCard(row)]
        } else {
            Vec::new()
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

    /// The contacts, by name.
    #[must_use]
    pub fn contacts(&self) -> &[ContactView] {
        &self.cards.contacts
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

    fn contact(id: &str, name: &str, p: &str) -> ContactView {
        ContactView {
            contact_id: id.to_owned(),
            name: name.to_owned(),
            name_from_card: true,
            photo_from_card: None,
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
        model.card_resolved(row(1));
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
        assert_eq!(model.title(&direct(P, "human")), "Ada");
        model.contacts_listed(vec![contact("c1", "Ada", P), contact("c2", "Ada", Q)]);
        let p_title = model.title(&direct(P, "human"));
        let q_title = model.title(&direct(Q, "human"));
        assert_ne!(
            p_title, q_title,
            "two contacts of one name are told apart in the title"
        );
        assert!(p_title.contains("Ada") && q_title.contains("Ada"));
        assert_eq!(
            p_title,
            crate::labels::fill(
                placeholder_en::text(UiText::ContactTitle),
                &[("name", "Ada"), ("peer", &crate::labels::short_peer(P))]
            )
        );
    }
}

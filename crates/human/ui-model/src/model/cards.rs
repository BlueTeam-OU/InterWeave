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

/// The identity's own card as one share sent it, shown in its own card
/// row (contact-cards section 5.5): what went, never the card as it is
/// now -- an edit after the share does not rewrite the row (app-core's
/// `a_shared_card_is_its_own_card_row_as_it_went`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SentCard {
    /// The name and surname it carried, as one line.
    pub name: String,
    /// Its photo, decoded from the bytes it carried, if any.
    pub photo: Option<Photo>,
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
    /// The photo the facade side prepared from the last
    /// [`Intent::ChooseOwnPhoto`](crate::Intent::ChooseOwnPhoto), as the
    /// editor previewed it; the card stays as it was if there is none.
    /// The bytes are the facade side's own, never the view's.
    Prepared,
}

/// What a new photo choice falls back to, as the model shows it: the
/// facade side keeps a prepared photo as the fallback only when its number
/// is the one named here, found under any answer the model has not taken
/// yet, so a Save writes what the editor showed (#449 F4; app-core's
/// `a_choices_fallback_is_what_the_model_showed` and
/// `a_basis_finds_its_photo_under_an_answer_not_taken`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhotoBasis {
    /// The saved card's photo, or none.
    Card,
    /// Removed in the editor.
    Removed,
    /// The prepared photo of this choice.
    Prepared(u64),
}

/// The photo the own card's editor holds, chosen and not yet saved
/// (contact-cards design section 5.2, "The photo").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnPhotoStep {
    /// Being prepared: the circle shows progress, and Save waits for it.
    Preparing,
    /// Prepared: exactly what a share sends, decoded for the preview.
    Prepared(Photo),
    /// It could not be read or prepared: the card stays as it was
    /// (`PhotoUnusable`).
    Unusable,
    /// Removed in the editor, which Save writes (design 5.2 as #443
    /// proposes it: an unsaved change, with no snackbar and no Undo).
    Removed,
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
    /// Which of the contact's values Accept would change and which of the
    /// person's own it keeps (cases U and E); empty with no contact.
    pub values: ValueChanges,
}

/// What Accept would do to the contact holding the card's route, value by
/// value (contact-cards design section 5.4.1, cases U and E).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ValueChanges {
    /// The name it would take, when it changes.
    pub new_name: Option<String>,
    /// The card's name differs from one the person chose, which stays.
    pub kept_name: bool,
    /// What becomes of the contact's photo.
    pub photo: PhotoChange,
}

/// What Accept would do to the contact's photo (contact-cards design
/// section 5.4.1, cases U and E): one line, or none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PhotoChange {
    /// Nothing changes, and nothing is said.
    #[default]
    Unchanged,
    /// The card's photo would replace the contact's (`CardUpdatesPhoto`).
    New,
    /// The card carries none, so the contact's, which the person did not
    /// choose, would go (`CardUpdatesPhotoRemoved`).
    Removed,
    /// The person chose the contact's photo, which stays whatever the
    /// card carries (`CardKeepsPhoto`).
    Kept,
    /// The card carries a photo, and the person removed the contact's:
    /// it stays without one and the card's is not used
    /// (`CardKeepsNoPhoto`, contact-cards design 5.4.1).
    KeptNone,
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
        /// The contact's previous photo, which the person did not
        /// choose, went with this Accept (`CardPhotoNotUsedRemoved` when
        /// `photo_dropped` too).
        photo_removed: bool,
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
    /// A contact of the same name on another node, which the note links
    /// to (`ShowContact`, contact-cards design 5.4.1).
    SameName(ContactRef),
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
    /// Which values Accept changes and which edits it keeps.
    pub values: ValueChanges,
    /// When it arrived.
    pub received_at: u64,
    /// The contacts could not be read: the card was compared with none, so
    /// it shows `CardContactsUnread` in place of its case, with no notes,
    /// and offers no Accept (design/2026-10-27-start-window.md section 11).
    pub contacts_unread: bool,
    /// Case P: how many contacts on the card's `PeerId` there are besides
    /// the one its note names (`CardSharesDeviceMany`).
    pub shares_node_others: usize,
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

    /// Which ways the proposal's row swipes (contact-cards design section
    /// 5.4.4): toward the line's end Accept, toward its start Decline.
    /// Case S moves neither way, since Close is no swipe action. A row
    /// that shows `CardSameName` -- case D, and P, U or E when Accept
    /// would write a clashing name -- never accepts by swipe (the owner's
    /// O2, "whenever warned"): a swipe can happen without reading, and a
    /// stranger saved under a contact's name by a stray thumb is the
    /// impersonation ADR-0059 makes visible. Its Accept button, or its
    /// menu's Accept, saves it: each pressed on purpose, beside the
    /// warning (`a_proposal_swipes_both_ways_but_never_accepts_when_warned_or_moves_in_s`).
    /// Judged on what the row shows now, so a row judged again after a
    /// change (5.4.5) swipes as it then reads.
    #[must_use]
    pub fn swipe(&self) -> CardSwipe {
        let closes = matches!(self.accept, AcceptLine::Matches(_));
        let warned = self
            .notes
            .iter()
            .any(|n| matches!(n, ProposalNote::SameName(_)));
        CardSwipe {
            accept: !closes && !warned && !self.contacts_unread,
            decline: !closes,
        }
    }
}

/// Which ways a proposal's row swipes, named by the direction of reading
/// as [`super::swipe::SwipeOffer`] is: the view mirrors them right to left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardSwipe {
    /// Toward the line's end: Accept, under the label of its case.
    pub accept: bool,
    /// Toward the line's start: Decline.
    pub decline: bool,
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
    /// Its key: its fingerprint, or why it cannot be used.
    pub key: OwnKey,
    /// This node's `PeerId`, beside the fingerprint, once known.
    pub peer: Option<TransportIdentity>,
}

/// An identity's own key as the client can use it
/// (design/2026-10-24-identity-key.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnKey {
    /// It opened: its fingerprint, 16 lowercase hex characters.
    Ready {
        /// The fingerprint.
        fingerprint: String,
    },
    /// It was refused at open, for this reason.
    Unusable(KeyProblem),
}

impl OwnKey {
    /// The fingerprint, when the key opened.
    #[must_use]
    pub fn fingerprint(&self) -> Option<&str> {
        match self {
            Self::Ready { fingerprint } => Some(fingerprint),
            Self::Unusable(_) => None,
        }
    }
}

/// Why an identity's key cannot be used, in the person's terms (section 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyProblem {
    /// The platform no longer lets the client use it.
    LostOnDevice,
    /// Protected less than this device requires.
    BelowDevice,
    /// Saved on another device and usable only there.
    OtherDevice,
    /// A format this version does not know; a newer one may read it.
    NewerVersion,
    /// Not a valid key file for this identity.
    Damaged,
    /// Other users may read or change it.
    Exposed,
    /// The key folder is not one only this account owns.
    DirUnsafe {
        /// The folder's full path, shown under the reason.
        folder: String,
    },
    /// A problem that may be temporary.
    NotNow,
    /// Gone from this device after the card was shared.
    Missing,
}

impl KeyProblem {
    /// Whether a new key may be made: not while the key folder is not one
    /// this account owns, which the client never changes.
    #[must_use]
    pub const fn offers_new_key(&self) -> bool {
        !matches!(self, Self::DirUnsafe { .. })
    }

    /// Whether Try again is offered: for a problem that may pass, or a
    /// folder the person can fix.
    #[must_use]
    pub const fn offers_try_again(&self) -> bool {
        matches!(self, Self::NotNow | Self::DirUnsafe { .. })
    }

    /// Whether a new key deletes a file that is there: every kind but
    /// a missing one.
    #[must_use]
    pub const fn deletes_old_file(&self) -> bool {
        !matches!(self, Self::Missing)
    }
}

/// Where Make a new key and Try again stand on the card page (sections 5
/// and 6), this session: held until the page is left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStep {
    /// Make a new key was pressed: the facade is asked what it cancels.
    Asking,
    /// The confirmation is open; `pending` says own cards not yet
    /// accepted by the remote transport would be cancelled.
    Confirming {
        /// Own cards still on their way.
        pending: bool,
    },
    /// Confirmed: the key is being made.
    Making,
    /// Made: the conversations whose counterparts hold or may hold the
    /// old key, most recently shared first.
    Made {
        /// Where to share the card again.
        share_again: Vec<ConversationKey>,
    },
    /// Not made: `gone` says the old file was already removed.
    Failed {
        /// The saved key is gone.
        gone: bool,
    },
    /// Try again refused with the same kind.
    StillNotNow,
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
    /// Make a new key or Try again, on the card page.
    key_step: Option<KeyStep>,
    /// The editor's chosen photo, until it is saved or the editor closes.
    own_photo: Option<OwnPhotoStep>,
    /// What the editor showed before the choice being prepared, or found
    /// unusable: a prepared photo, a removal, or (`None`) the saved card's
    /// (design 5.2: the buttons stay as they were before the choice).
    own_photo_before: Option<OwnPhotoStep>,
    /// The last choice's number: an answer for an earlier one is not
    /// this one's.
    own_photo_seq: u64,
    /// The number of the choice whose prepared photo the editor shows,
    /// or showed before the choice now held.
    own_photo_shown_seq: u64,
    /// The read-proposal bound when a test build set one lower than
    /// [`READ_PROPOSAL_CAP`] (`UiModel::set_bounds_for_test`).
    cap: Option<usize>,
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
    /// The contact's previous photo went too (`CardPhotoNotUsedRemoved`
    /// in place of `CardPhotoNotUsed`).
    pub photo_removed: bool,
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

    /// Stop holding `conversation`'s unread proposals, which stay unread in
    /// the store for its next opening. A read one has no stored row and
    /// stays, under [`READ_PROPOSAL_CAP`].
    pub(super) fn let_go_unread(&mut self, conversation: &ConversationKey) {
        if let ConversationKey::Direct {
            peer,
            endpoint: Some(at),
        } = conversation
        {
            for row in self.unread_in(peer, at) {
                self.proposals.remove(&row);
            }
        }
    }

    /// How many read proposals are held: what [`READ_PROPOSAL_CAP`] bounds.
    pub(super) fn read_count(&self) -> usize {
        self.read.len()
    }

    /// The conversation of a proposal held unread: `None` once it was read,
    /// or for a row no proposal is.
    pub(super) fn unread_conversation(&self, row: RowId) -> Option<ConversationKey> {
        self.proposals
            .get(&row)
            .filter(|_| !self.read.contains(&row))
            .map(|p| ConversationKey::Direct {
                peer: p.peer.clone(),
                endpoint: Some(p.endpoint.clone()),
            })
    }

    /// A row was marked read: if it is a proposal's, it is shown now.
    /// Past [`READ_PROPOSAL_CAP`], the oldest read one goes: the store no
    /// longer holds it, and the facade no longer holds its copy either.
    pub(super) fn read(&mut self, row: RowId) {
        if self.proposals.contains_key(&row) && self.read.insert(row) {
            self.read_order.push_back(row);
        }
        while self.read_order.len() > self.read_cap() {
            if let Some(oldest) = self.read_order.pop_front() {
                self.read.remove(&oldest);
                self.proposals.remove(&oldest);
            }
        }
    }

    /// The read-proposal bound held to.
    fn read_cap(&self) -> usize {
        self.cap.unwrap_or(READ_PROPOSAL_CAP)
    }

    /// Hold to `cap` read proposals: a test build's lower bound.
    #[cfg(feature = "test-hooks")]
    pub(super) const fn set_read_cap(&mut self, cap: usize) {
        self.cap = Some(cap);
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
        while self.answers.len() > self.read_cap() {
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
    /// A card the store holds unread, with its context: listed at a start
    /// or a relist, or read with a conversation's page. In the windowed
    /// mode it is already among the store's count, so nothing is counted
    /// (`a_stored_card_handed_over_is_counted_once`); a card the drain just
    /// committed is [`UiModel::card_arrived`].
    pub fn card_received(&mut self, proposal: Proposal) {
        let conversation = ConversationKey::Direct {
            peer: proposal.peer.clone(),
            endpoint: Some(proposal.endpoint.clone()),
        };
        self.windows.holds_card(
            &conversation,
            proposal.row,
            super::window::PageMark {
                received_at: proposal.received_at,
                kept: false,
                row: proposal.row,
            },
        );
        self.cards.proposals.insert(proposal.row, proposal);
        // The marks of proposals answered, let go or evicted since go now:
        // each arrival keeps them to the proposals held.
        let held = &self.cards.proposals;
        self.windows.retain_card_marks(|row| held.contains_key(row));
    }

    /// A card the drain just committed unread, with its context: as
    /// [`UiModel::card_received`], and in the windowed mode new to the
    /// store's count too, so the conversation's badge takes it and it is
    /// the newest row -- once, a card already held being counted already
    /// (#498 review F1). The facade hands its drained cards here once it
    /// starts windowed, the next batch.
    pub fn card_arrived(&mut self, proposal: Proposal) {
        let conversation = ConversationKey::Direct {
            peer: proposal.peer.clone(),
            endpoint: Some(proposal.endpoint.clone()),
        };
        if let Some(windowed) = self.windowed.as_mut()
            && windowed.handed_over(proposal.row)
            && !self.cards.proposals.contains_key(&proposal.row)
        {
            windowed.arrived(
                &conversation,
                super::summaries::NewestRow::card(proposal.row, proposal.received_at),
            );
        }
        // As a message is: a closed conversation holds only its summary.
        if self.windowed.is_some() && self.windows.open_conversation() != Some(&conversation) {
            return;
        }
        self.windows.seen(super::window::PageMark {
            received_at: proposal.received_at,
            kept: false,
            row: proposal.row,
        });
        self.card_received(proposal);
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
        // Resolved while still unread, its unread row is gone from the
        // store: the count loses it.
        if let (Some(windowed), Some(conversation)) =
            (self.windowed.as_mut(), self.cards.unread_conversation(row))
        {
            windowed.left_unread(&conversation);
        }
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
                photo_removed,
            } => {
                self.cards.answered(CardAnswer {
                    row,
                    peer: proposal.peer,
                    endpoint: proposal.endpoint,
                    contact_id,
                    created,
                    photo_dropped,
                    photo_removed,
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

    /// An Undo of `row`'s answer was carried out but its card could not be
    /// judged again (#414 F2): the proposal does not return, its
    /// confirmation row goes -- the store no longer holds what it says --
    /// and the Undo is announced as done, focus on the conversation's
    /// header for an Accept's row, else where the Decline sent it.
    pub fn card_not_restored(&mut self, row: RowId) {
        let answered = self
            .cards
            .answers
            .remove(&row)
            .map(|a| ConversationKey::Direct {
                peer: a.peer,
                endpoint: Some(a.endpoint),
            });
        self.card_undone_without_row(row, answered.map(FocusTarget::Header));
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
    /// oldest first, each only while its contact is held.
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
            // A row that names a contact no longer held -- deleted since,
            // or an Undo the store wrote and then could not read back --
            // would say something no longer true.
            .filter(|a| self.contact(&a.contact_id).is_some())
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

    /// The contacts, as the facade listed them after a change, held in
    /// the client's one order ([`crate::contact_order`]) whatever order
    /// the store read them in.
    pub fn contacts_listed(&mut self, mut contacts: Vec<ContactView>) {
        contacts.sort_by(crate::contact_order);
        self.cards.contacts = contacts;
    }

    /// The identity in front, as the facade read it.
    pub fn identity_shown(&mut self, identity: IdentityView) {
        self.cards.identity = Some(identity);
    }

    /// The identity in front's key problem, if its key cannot be used.
    #[must_use]
    pub fn key_problem(&self) -> Option<&KeyProblem> {
        match &self.cards.identity.as_ref()?.key {
            OwnKey::Unusable(problem) => Some(problem),
            OwnKey::Ready { .. } => None,
        }
    }

    /// Where Make a new key and Try again stand.
    #[must_use]
    pub const fn key_step(&self) -> Option<&KeyStep> {
        self.cards.key_step.as_ref()
    }

    /// Make a new key, pressed: the facade is asked what it would cancel,
    /// when the problem offers a new key and nothing is under way.
    #[must_use]
    pub fn ask_new_key(&mut self) -> Option<Intent> {
        let offered = self.key_problem().is_some_and(KeyProblem::offers_new_key);
        let idle = !matches!(
            self.cards.key_step,
            Some(KeyStep::Asking | KeyStep::Confirming { .. } | KeyStep::Making)
        );
        (offered && idle).then(|| {
            self.cards.key_step = Some(KeyStep::Asking);
            Intent::AskNewKey
        })
    }

    /// The facade answered whether own cards are on their way: the
    /// confirmation opens.
    pub fn new_key_asked(&mut self, pending: bool) {
        if self.cards.key_step == Some(KeyStep::Asking) {
            self.cards.key_step = Some(KeyStep::Confirming { pending });
        }
    }

    /// The confirmation's Make a new key: the key is made from here.
    #[must_use]
    pub fn confirm_new_key(&mut self) -> Option<Intent> {
        if !matches!(self.cards.key_step, Some(KeyStep::Confirming { .. })) {
            return None;
        }
        self.close_own_card_undo();
        self.cards.key_step = Some(KeyStep::Making);
        Some(Intent::MakeNewKey)
    }

    /// The new key was made: the identity with its fingerprint -- or, when
    /// it could not be read, the fingerprint on the identity held -- and
    /// where to share the card again.
    pub fn key_made(
        &mut self,
        identity: Option<IdentityView>,
        fingerprint: String,
        share_again: Vec<ConversationKey>,
    ) {
        self.set_own_key(identity, OwnKey::Ready { fingerprint });
        self.cards.key_step = Some(KeyStep::Made { share_again });
    }

    /// No new key was made: the identity as the open now finds it -- or its
    /// problem on the identity held -- and whether the old file went first.
    pub fn key_not_made(
        &mut self,
        identity: Option<IdentityView>,
        problem: KeyProblem,
        gone: bool,
    ) {
        self.set_own_key(identity, OwnKey::Unusable(problem));
        self.cards.key_step = Some(KeyStep::Failed { gone });
    }

    /// The facade refused an Ask or a Make before it started: the step
    /// ends -- an Ask as if never pressed, a Make as not made, the old key
    /// as it was -- so the page never waits on it.
    pub fn key_command_refused(&mut self) {
        self.cards.key_step = match self.cards.key_step {
            Some(KeyStep::Making) => Some(KeyStep::Failed { gone: false }),
            Some(KeyStep::Asking) => None,
            ref other => other.clone(),
        };
    }

    fn set_own_key(&mut self, identity: Option<IdentityView>, key: OwnKey) {
        match identity {
            Some(identity) => self.cards.identity = Some(identity),
            None => {
                if let Some(held) = self.cards.identity.as_mut() {
                    held.key = key;
                }
            }
        }
    }

    /// Try again, when its problem offers it and nothing is under way.
    #[must_use]
    pub fn retry_key(&self) -> Option<Intent> {
        let offered = self.key_problem().is_some_and(KeyProblem::offers_try_again);
        (offered && !matches!(self.cards.key_step, Some(KeyStep::Making)))
            .then_some(Intent::RetryKey)
    }

    /// Try again's answer: the identity as the open found it; the same
    /// temporary kind again says so.
    pub fn key_retried(&mut self, identity: IdentityView) {
        let again = matches!(identity.key, OwnKey::Unusable(KeyProblem::NotNow))
            && matches!(self.key_problem(), Some(KeyProblem::NotNow));
        self.cards.identity = Some(identity);
        self.cards.key_step = again.then_some(KeyStep::StillNotNow);
    }

    /// A photo chosen in the own card's editor: being prepared, under the
    /// number its answer must carry. Any earlier choice not saved is
    /// replaced; what the editor showed before it is kept, to show while
    /// it is prepared and should it be unusable (design 5.2).
    pub fn own_photo_chosen(&mut self) -> u64 {
        let was = self.cards.own_photo.take();
        self.cards.own_photo_before = match was {
            Some(shown @ (OwnPhotoStep::Prepared(_) | OwnPhotoStep::Removed)) => Some(shown),
            Some(OwnPhotoStep::Preparing | OwnPhotoStep::Unusable) => {
                self.cards.own_photo_before.take()
            }
            None => None,
        };
        self.cards.own_photo = Some(OwnPhotoStep::Preparing);
        self.cards.own_photo_seq += 1;
        self.cards.own_photo_seq
    }

    /// What preparing choice `seq` came to: the photo as a share would
    /// send it, or `None` when it could not be read or prepared. Only the
    /// last choice, still being prepared, takes it: an earlier choice's
    /// answer, or one forgotten, removed or saved meanwhile, changes
    /// nothing.
    pub fn own_photo_prepared(&mut self, seq: u64, photo: Option<Photo>) {
        if seq != self.cards.own_photo_seq
            || !matches!(self.cards.own_photo, Some(OwnPhotoStep::Preparing))
        {
            return;
        }
        if let Some(photo) = photo {
            self.cards.own_photo = Some(OwnPhotoStep::Prepared(photo));
            self.cards.own_photo_before = None;
            self.cards.own_photo_shown_seq = seq;
        } else {
            self.cards.own_photo = Some(OwnPhotoStep::Unusable);
        }
    }

    /// Remove photo in the editor: no photo, once saved. A photo being
    /// prepared is dropped, and its answer changes nothing.
    pub fn own_photo_removed(&mut self) {
        self.cards.own_photo = Some(OwnPhotoStep::Removed);
        self.cards.own_photo_before = None;
    }

    /// The editor closed, or its card saved: the chosen photo is let go.
    pub fn own_photo_forgotten(&mut self) {
        self.cards.own_photo = None;
        self.cards.own_photo_before = None;
    }

    /// The editor's chosen photo, if any.
    #[must_use]
    pub const fn own_photo(&self) -> Option<&OwnPhotoStep> {
        self.cards.own_photo.as_ref()
    }

    /// What the last choice falls back to, for the facade side to keep:
    /// what the editor showed before it.
    #[must_use]
    pub const fn own_photo_basis(&self) -> PhotoBasis {
        match self.cards.own_photo_before {
            Some(OwnPhotoStep::Prepared(_)) => PhotoBasis::Prepared(self.cards.own_photo_shown_seq),
            Some(OwnPhotoStep::Removed) => PhotoBasis::Removed,
            Some(OwnPhotoStep::Preparing | OwnPhotoStep::Unusable) | None => PhotoBasis::Card,
        }
    }

    /// What the editor showed before the choice being prepared or found
    /// unusable: a prepared photo or a removal; `None` for the saved
    /// card's photo.
    #[must_use]
    pub const fn own_photo_before(&self) -> Option<&OwnPhotoStep> {
        self.cards.own_photo_before.as_ref()
    }

    /// Whether the editor's photo differs from the saved card's, as design
    /// 5.2 judges unsaved changes: a removal over a card with no photo, or
    /// an unusable choice over what the card already shows, is none.
    #[must_use]
    pub fn own_photo_unsaved(&self) -> bool {
        let saved = self.identity().is_some_and(|i| i.has_photo);
        let shows = |step: Option<&OwnPhotoStep>| match step {
            Some(OwnPhotoStep::Prepared(_)) => true,
            Some(OwnPhotoStep::Removed) => saved,
            Some(OwnPhotoStep::Preparing | OwnPhotoStep::Unusable) | None => false,
        };
        match self.own_photo() {
            None => false,
            // A photo on its way is a change whatever it comes to.
            Some(OwnPhotoStep::Preparing) => true,
            Some(OwnPhotoStep::Unusable) => shows(self.own_photo_before()),
            step => shows(step),
        }
    }

    /// What Save does to the photo: no photo after Remove; the facade
    /// side's own while a choice is held -- the prepared one, or, while it
    /// is prepared or when it is unusable, what the editor showed before
    /// it (the facade side keeps that too); else the card's as it is.
    #[must_use]
    pub const fn own_photo_edit(&self) -> PhotoEdit {
        match self.cards.own_photo {
            Some(OwnPhotoStep::Removed) => PhotoEdit::Remove,
            Some(OwnPhotoStep::Preparing | OwnPhotoStep::Prepared(_) | OwnPhotoStep::Unusable) => {
                PhotoEdit::Prepared
            }
            None => PhotoEdit::Keep,
        }
    }

    /// The confirmation cancelled, or the card page left: what it showed
    /// of a key step goes; one under way stays.
    pub fn key_step_closed(&mut self) {
        if !matches!(self.cards.key_step, Some(KeyStep::Making | KeyStep::Asking)) {
            self.cards.key_step = None;
        }
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
                // Each kind of note in the contacts' one order, so the
                // contact a note names first is the list's first; the
                // context itself stays as the store read it, since Accept
                // carries it back to be compared.
                let ordered = |refs: &[ContactRef]| {
                    let mut refs = refs.to_vec();
                    crate::order::sort_refs(&mut refs, &self.cards.contacts);
                    refs
                };
                // One contact named, the first in the Contacts list's order,
                // whichever the store gave first (design/2026-10-27-start-
                // window.md section 11, contact-cards 5.4.1): case P names
                // the first and counts the others.
                let mut notes = Vec::new();
                notes.extend(
                    ordered(&c.same_name)
                        .into_iter()
                        .take(1)
                        .map(ProposalNote::SameName),
                );
                let shares = ordered(&c.shares_node);
                let shares_node_others = shares.len().saturating_sub(1);
                notes.extend(
                    shares
                        .into_iter()
                        .take(1)
                        .map(|r| ProposalNote::SharesNode(r.name)),
                );
                notes.extend(
                    ordered(&c.key_matches)
                        .into_iter()
                        .map(|r| ProposalNote::KeyMatches(r.name)),
                );
                let contacts_unread = self.directory.state != super::directory::ContactsState::Read;
                if contacts_unread {
                    notes.clear();
                }
                ProposalItem {
                    contacts_unread,
                    shares_node_others: if contacts_unread {
                        0
                    } else {
                        shares_node_others
                    },
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
                    values: c.values.clone(),
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
            Some(_) if self.directory.state != super::directory::ContactsState::Read => {
                vec![Intent::DeclineCard(row)]
            }
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

    /// Delete one route of a contact the model holds, by the person's act
    /// alone: offered only while the contact holds the route and another,
    /// since the last route goes only with its contact
    /// (`a_route_is_deleted_only_while_its_contact_holds_another`).
    #[must_use]
    pub fn delete_route(
        &self,
        contact_id: &str,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
    ) -> Option<Intent> {
        let contact = self.contact(contact_id)?;
        let held = contact
            .routes
            .iter()
            .any(|r| &r.peer == peer && &r.endpoint == endpoint);
        (held && contact.routes.len() > 1).then(|| Intent::DeleteRoute {
            contact_id: contact.contact_id.clone(),
            peer: peer.clone(),
            endpoint: endpoint.clone(),
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

    /// The contacts in the client's one order: by name under the root's
    /// Unicode collation, then by the smallest route
    /// ([`crate::contact_order`]; human-client-ui.md section 3). Every
    /// list of contacts a person sees is this one, a view's and the
    /// command line's alike.
    #[must_use]
    pub fn contacts(&self) -> &[ContactView] {
        &self.cards.contacts
    }

    /// The names of every contact of the identity in front with a route
    /// on `peer`, for Peers' rows (add-a-peer design section 5.2): one
    /// per line, never joined, since a typed name may hold "and" or a
    /// comma. In the contacts' one order ([`UiModel::contacts`]), which
    /// they are held in.
    #[must_use]
    pub fn contact_names_on(&self, peer: &TransportIdentity) -> Vec<String> {
        self.cards
            .contacts
            .iter()
            .filter(|c| c.routes.iter().any(|r| &r.peer == peer))
            .map(|c| c.name.clone())
            .collect()
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
            values: ValueChanges::default(),
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
    fn the_contacts_a_peers_row_and_a_proposals_notes_take_one_order() {
        // The fixture every site's test orders (human-client-ui.md section
        // 3): the store reads "Zoe", "Ben", "Chloe", "Åke", "ben" in that
        // order. A proposal names one contact of a case P, the first in
        // that order, and counts the others (start-window section 11).
        // Mutations: `contacts_listed` keeping the store's order (the list
        // and the row read Zoe first); `proposals` sorting nothing (the
        // notes keep the context's order); the others counted with the
        // one named.
        let fixture = ["Zoe", "Ben", "Chloe", "Åke", "ben"];
        let order = ["Åke", "ben", "Ben", "Chloe", "Zoe"];
        let mut model = UiModel::new();
        model.contacts_listed(
            fixture
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    let mut c = contact(&format!("c{i}"), n, P);
                    c.routes[0].endpoint = endpoint(&format!("e{i}"));
                    c
                })
                .collect(),
        );
        let listed: Vec<&str> = model.contacts().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(listed, order, "the Contacts list");
        assert_eq!(
            model.contact_names_on(&peer(P)),
            order,
            "a Peers row's names"
        );
        let refs: Vec<ContactRef> = fixture
            .iter()
            .enumerate()
            .map(|(i, n)| ContactRef {
                contact_id: format!("c{i}"),
                name: (*n).to_owned(),
            })
            .collect();
        let mut standing = context();
        standing.shares_node = refs.clone();
        standing.key_matches = refs;
        model.card_received(proposal(1, P, "Ada", standing));
        let notes = &model.proposals(&direct(P, "human"))[0].notes;
        let named = |want: fn(&ProposalNote) -> Option<&str>| -> Vec<&str> {
            notes.iter().filter_map(want).collect()
        };
        let shares = named(|n| match n {
            ProposalNote::SharesNode(name) => Some(name.as_str()),
            _ => None,
        });
        assert_eq!(shares, ["Åke"], "case P names the first in order alone");
        assert_eq!(
            model.proposals(&direct(P, "human"))[0].shares_node_others,
            4
        );
        let keys = named(|n| match n {
            ProposalNote::KeyMatches(name) => Some(name.as_str()),
            _ => None,
        });
        assert_eq!(keys, order);
    }

    /// Of two contacts of one name, `CardSameName` names the one first in
    /// the Contacts list's order: same name, so the one whose route's
    /// `PeerId` sorts first by code point, whichever the store gave first
    /// (start-window section 11, contact-cards 6.2). Mutations: the note
    /// naming the store's first; every contact of the name named.
    #[test]
    fn two_contacts_of_one_name_the_note_names_the_one_first_in_order() {
        let mut model = UiModel::new();
        model.contacts_listed(vec![
            contact("on-k", "Anna Rossi", Q),
            contact("on-d", "Anna Rossi", P),
        ]);
        let mut standing = context();
        standing.same_name = ["on-k", "on-d"]
            .iter()
            .map(|id| ContactRef {
                contact_id: (*id).to_owned(),
                name: "Anna Rossi".to_owned(),
            })
            .collect();
        model.card_received(proposal(1, P, "Anna", standing));
        let same: Vec<String> = model.proposals(&direct(P, "human"))[0]
            .notes
            .iter()
            .filter_map(|n| match n {
                ProposalNote::SameName(r) => Some(r.contact_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(same, ["on-d"], "D sorts before K");
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
                ProposalNote::SameName(ContactRef {
                    contact_id: "c2".to_owned(),
                    name: "Ada Lovelace".to_owned(),
                }),
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
    fn a_route_is_deleted_only_while_its_contact_holds_another() {
        let route = |p: &str| RouteView {
            peer: peer(p),
            endpoint: endpoint("human"),
            fingerprint: None,
        };
        let contact = |routes: Vec<RouteView>| ContactView {
            contact_id: "c1".to_owned(),
            name: "Ada".to_owned(),
            name_from_card: true,
            name_at: None,
            photo_from_card: None,
            photo_at: None,
            photo: None,
            routes,
        };
        let mut model = UiModel::new();
        let human = endpoint("human");
        assert_eq!(model.delete_route("c1", &peer(P), &human), None);
        model.contacts_listed(vec![contact(vec![route(P), route(Q)])]);
        let delete = model.delete_route("c1", &peer(Q), &human).expect("offered");
        assert_eq!(
            delete,
            Intent::DeleteRoute {
                contact_id: "c1".to_owned(),
                peer: peer(Q),
                endpoint: human.clone(),
            }
        );
        assert!(model.offers(&delete));
        assert_eq!(
            model.delete_route("c1", &peer(Q), &endpoint("other")),
            None,
            "a route the contact does not hold"
        );
        assert_eq!(model.delete_route("c2", &peer(Q), &human), None);
        model.contacts_listed(vec![contact(vec![route(P)])]);
        assert!(!model.offers(&delete), "a route gone is not deleted twice");
        assert_eq!(
            model.delete_route("c1", &peer(P), &human),
            None,
            "the last route goes only with its contact"
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
            values: ValueChanges::default(),
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
                values: ValueChanges::default(),
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

    // Section 5.4.4's swipes by case, from the same contexts the labels
    // are judged on: no Accept swipe wherever CardSameName shows (O2), the
    // same cases without it keeping theirs. Mutations: `closes` not
    // withholding Decline (case S moves); `warned` dropped (a warned row
    // accepts by swipe).
    #[test]
    fn a_proposal_swipes_both_ways_but_never_accepts_when_warned_or_moves_in_s() {
        let near = |kind: &str| ContactRef {
            contact_id: kind.to_owned(),
            name: "Ada".to_owned(),
        };
        let both = CardSwipe {
            accept: true,
            decline: true,
        };
        let cases = [
            ("N", context(), both),
            (
                "D",
                ProposalContext {
                    same_name: vec![near("d")],
                    ..context()
                },
                CardSwipe {
                    accept: false,
                    decline: true,
                },
            ),
            (
                "P",
                ProposalContext {
                    shares_node: vec![near("p")],
                    ..context()
                },
                both,
            ),
            (
                "P, warned",
                ProposalContext {
                    shares_node: vec![near("p")],
                    same_name: vec![near("d")],
                    ..context()
                },
                CardSwipe {
                    accept: false,
                    decline: true,
                },
            ),
            (
                "U",
                ProposalContext {
                    updates: Some(near("u")),
                    ..context()
                },
                both,
            ),
            (
                "U, warned",
                ProposalContext {
                    updates: Some(near("u")),
                    same_name: vec![near("d")],
                    ..context()
                },
                CardSwipe {
                    accept: false,
                    decline: true,
                },
            ),
            (
                "S",
                ProposalContext {
                    updates: Some(near("u")),
                    changes_nothing: true,
                    values: ValueChanges::default(),
                    ..context()
                },
                CardSwipe::default(),
            ),
        ];
        for (n, (case, judged, swipe)) in cases.into_iter().enumerate() {
            let mut model = UiModel::new();
            let at = i64::try_from(n).expect("small");
            model.card_received(proposal(at + 1, Q, "Ada", judged));
            let shown = model.proposals(&direct(Q, "human"));
            assert_eq!(shown[0].swipe(), swipe, "case {case}");
        }
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
            photo_removed: false,
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
        model.contacts_listed(vec![contact("c1", "Ada", P)]);
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
        assert_eq!(outcome.focus, Some(FocusTarget::ProposalAccept(row(1))));
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
        model.contacts_listed(vec![contact("c1", "Ada", P)]);
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

    /// #414 F2: a confirmation row shows only while its contact is held,
    /// so a contact gone -- deleted, or an Undo written but not read back
    /// -- leaves no row saying it was saved.
    #[test]
    fn a_confirmation_row_shows_only_while_its_contact_is_held() {
        let mut model = UiModel::new();
        let key = direct(P, "human");
        model.card_received(proposal(1, P, "Ada", context()));
        model.card_resolved(row(1), saved("c1"));
        assert!(model.card_answers(&key).is_empty(), "not yet listed");
        model.contacts_listed(vec![contact("c1", "Ada", P)]);
        assert_eq!(model.card_answers(&key).len(), 1);
        model.contacts_listed(Vec::new());
        assert!(model.card_answers(&key).is_empty(), "gone with its contact");
    }

    /// #414 F2: an Undo carried out whose card could not be judged again
    /// drops the confirmation row the store no longer backs, and is
    /// announced as done, focus on the header, never refused.
    #[test]
    fn an_undo_whose_card_is_not_back_drops_the_row_and_says_it_was_done() {
        use crate::{Announce, FocusTarget};
        let mut model = UiModel::new();
        let key = direct(P, "human");
        model.card_received(proposal(1, P, "Ada", context()));
        model.card_resolved(row(1), saved("c1"));
        model.contacts_listed(vec![contact("c1", "Ada", P)]);
        let id = model.snackbar().expect("up").id;
        assert_eq!(model.undo(id), Some(Intent::UndoCard(row(1))));
        // The Undo updated the contact back; it is still held.
        model.card_not_restored(row(1));
        assert!(model.card_answers(&key).is_empty(), "the row goes");
        assert!(model.proposals(&key).is_empty(), "nothing returns");
        let outcome = model.outcome().expect("an outcome");
        assert_eq!(outcome.announce, Some(Announce::Restored));
        assert_eq!(outcome.focus, Some(FocusTarget::Header(key)));
    }

    fn own(key: OwnKey) -> IdentityView {
        IdentityView {
            endpoint: endpoint("human"),
            name: "Ada".to_owned(),
            given: "Ada".to_owned(),
            surname: None,
            has_photo: false,
            photo: None,
            key,
            peer: None,
        }
    }

    /// design/2026-10-24-identity-key.md sections 5 and 6: Make a new key
    /// asks first, opens the confirmation with what it cancels, makes the
    /// key only on its confirm, and ends made or failed; a folder this
    /// account does not own offers no new key, only Try again; Try again
    /// refused with the same temporary kind says so.
    #[test]
    fn a_new_key_is_asked_confirmed_and_made_and_try_again_says_a_repeat() {
        let mut model = UiModel::new();
        model.identity_shown(own(OwnKey::Ready {
            fingerprint: "0123456789abcdef".to_owned(),
        }));
        assert_eq!(model.ask_new_key(), None, "a key that opens offers none");

        model.identity_shown(own(OwnKey::Unusable(KeyProblem::Damaged)));
        assert_eq!(model.retry_key(), None, "damage is not tried again");
        assert_eq!(model.confirm_new_key(), None, "never before the ask");
        assert_eq!(model.ask_new_key(), Some(Intent::AskNewKey));
        assert_eq!(model.ask_new_key(), None, "one ask at a time");
        model.new_key_asked(true);
        assert_eq!(
            model.key_step(),
            Some(&KeyStep::Confirming { pending: true })
        );
        model.key_step_closed();
        assert_eq!(model.key_step(), None, "cancelled: nothing made");

        assert_eq!(model.ask_new_key(), Some(Intent::AskNewKey));
        model.new_key_asked(false);
        assert_eq!(model.confirm_new_key(), Some(Intent::MakeNewKey));
        assert_eq!(model.key_step(), Some(&KeyStep::Making));
        let back = vec![direct(P, "human")];
        model.key_made(
            Some(own(OwnKey::Ready {
                fingerprint: "fedcba9876543210".to_owned(),
            })),
            "fedcba9876543210".to_owned(),
            back.clone(),
        );
        assert_eq!(model.key_step(), Some(&KeyStep::Made { share_again: back }));
        assert_eq!(model.key_problem(), None);
        model.key_step_closed();
        assert_eq!(model.key_step(), None, "the list is not kept");

        model.identity_shown(own(OwnKey::Unusable(KeyProblem::DirUnsafe {
            folder: "/x".to_owned(),
        })));
        assert_eq!(model.ask_new_key(), None, "a folder not owned: no new key");
        assert_eq!(model.retry_key(), Some(Intent::RetryKey));

        model.identity_shown(own(OwnKey::Unusable(KeyProblem::NotNow)));
        model.key_retried(own(OwnKey::Unusable(KeyProblem::NotNow)));
        assert_eq!(model.key_step(), Some(&KeyStep::StillNotNow));
        model.key_retried(own(OwnKey::Ready {
            fingerprint: "0123456789abcdef".to_owned(),
        }));
        assert_eq!(model.key_step(), None, "opened");
    }

    /// Identity-key design 5.2 (#471 R2 F1): confirming Make a new key
    /// closes an open Cancel snackbar of one of the identity's own cards,
    /// its Undo with it -- that card carries the key being replaced -- and
    /// lets its record go. Mutation: the confirmation leaving the snackbar
    /// open.
    #[test]
    fn confirming_a_new_key_closes_an_own_cards_cancel_undo() {
        use interweave_human_client_api::{ClientEvent, OutboundStatus, OutboundUpdate};
        let cancel = |model: &mut UiModel, row: i64| {
            model.client_event(ClientEvent::Outbound(OutboundUpdate {
                row: RowId::from_stored(row),
                app_message_id: interweave_human_core::AppMessageId::parse(format!("{row:032x}"))
                    .expect("id"),
                status: OutboundStatus::Cancelled {
                    may_have_reached: false,
                },
                last_code: None,
            }));
            model.cancelled(RowId::from_stored(row), true);
        };
        let mut model = UiModel::new();
        model.identity_shown(own(OwnKey::Unusable(KeyProblem::Damaged)));
        model.card_queued(
            RowId::from_stored(1),
            direct(P, "human"),
            format!("{:032x}", 1),
            SentCard {
                name: "Ada".to_owned(),
                photo: None,
            },
            1,
        );
        cancel(&mut model, 1);
        assert!(
            model.snackbar().is_some_and(|s| s.undo),
            "control: the card's Cancel offers its Undo"
        );
        assert_eq!(model.ask_new_key(), Some(Intent::AskNewKey));
        model.new_key_asked(false);
        let _ = model.take_released();
        assert_eq!(model.confirm_new_key(), Some(Intent::MakeNewKey));
        assert!(model.snackbar().is_none(), "closed, its Undo with it");
        assert_eq!(
            model.take_released().cancelled,
            vec![RowId::from_stored(1)],
            "its record let go"
        );
    }

    /// #423 F1: a key step the facade refused, or one whose identity could
    /// not be read, still ends -- an Ask as never pressed, a Make as not
    /// made -- and a key made but not read back shows its fingerprint on
    /// the identity held.
    #[test]
    fn a_key_step_always_ends_made_or_not_made() {
        let mut model = UiModel::new();
        model.identity_shown(own(OwnKey::Unusable(KeyProblem::Damaged)));
        assert_eq!(model.ask_new_key(), Some(Intent::AskNewKey));
        model.key_command_refused();
        assert_eq!(model.key_step(), None);
        assert_eq!(
            model.ask_new_key(),
            Some(Intent::AskNewKey),
            "offered again"
        );
        model.new_key_asked(false);
        let _ = model.confirm_new_key();
        model.key_command_refused();
        assert_eq!(model.key_step(), Some(&KeyStep::Failed { gone: false }));

        model.key_made(None, "fedcba9876543210".to_owned(), Vec::new());
        let held = model.identity().expect("held");
        assert_eq!(held.name, "Ada", "the card as it was");
        assert_eq!(held.key.fingerprint(), Some("fedcba9876543210"));
    }

    // Contact-cards design section 6.3: Delete contact and Remove photo
    // each open a snackbar on the card pages with an Undo the facade side
    // carries out; its answer restores and is announced, a refusal or a
    // close lets go of what it held, and focus goes by the focus table.
    // Mutations: the snackbar not naming its contact; a close not
    // releasing it; a refused Undo leaving it awaited.
    #[test]
    fn a_contacts_delete_and_photo_removal_offer_an_undo_that_restores() {
        use super::super::swipe::ContactChange;
        let mut model = UiModel::new();
        model.contact_changed("c1".to_owned(), ContactChange::Deleted, 1);
        let snack = model.snackbar().expect("up");
        assert_eq!(snack.confirms, Confirmation::ContactDeleted);
        assert_eq!(snack.contact.as_deref(), Some("c1"));
        assert_eq!(snack.conversation, None);
        assert!(snack.undo);
        assert_eq!(
            model.outcome().and_then(|o| o.announce),
            Some(crate::Announce::Confirmed(Confirmation::ContactDeleted)),
            "announced (section 8)"
        );
        assert_eq!(
            model.undo(snack.id),
            Some(Intent::UndoContact {
                contact_id: "c1".to_owned(),
                change: ContactChange::Deleted,
                hold: 1,
            })
        );
        assert!(model.snackbar().is_none());
        model.contact_restored("c1", ContactChange::Deleted);
        let outcome = model.outcome().expect("restored");
        assert_eq!(outcome.announce, Some(crate::Announce::Restored));
        assert_eq!(outcome.focus, Some(FocusTarget::Contact("c1".to_owned())));
        assert!(model.take_released().is_empty(), "the Undo used it");

        // A refused Undo: the removal stands, its hold let go.
        model.contact_changed("c2".to_owned(), ContactChange::PhotoRemoved, 2);
        let snack = model.snackbar().expect("up");
        assert_eq!(snack.confirms, Confirmation::PhotoRemoved);
        let undo = model.undo(snack.id).expect("raised");
        model.intent_refused(&undo);
        assert_eq!(model.take_released().contacts, vec![2]);
        assert_eq!(
            model.outcome().and_then(|o| o.focus.clone()),
            Some(FocusTarget::ContactPage("c2".to_owned()))
        );
        model.contact_restored("c2", ContactChange::PhotoRemoved);
        assert_ne!(
            model.outcome().and_then(|o| o.announce),
            Some(crate::Announce::Restored),
            "nothing awaited any more"
        );

        // Closed with no Undo: let go, focus on the list if it held it.
        model.contact_changed("c3".to_owned(), ContactChange::Deleted, 3);
        let id = model.snackbar().expect("up").id;
        model.snackbar_closed(id);
        assert_eq!(model.take_released().contacts, vec![3]);
        assert_eq!(
            model.undo_closed().map(|u| u.focus.clone()),
            Some(FocusTarget::Contacts)
        );

        // Withdrawn: another hold's withdrawal closes nothing.
        model.contact_changed("c4".to_owned(), ContactChange::Deleted, 4);
        model.contact_undo_withdrawn(5);
        assert!(model.snackbar().is_some());
        model.contact_undo_withdrawn(4);
        assert!(model.snackbar().is_none());
        assert_eq!(model.take_released().contacts, vec![4]);
    }

    // #433 F1: two changes of one contact in a row -- Remove photo, then
    // Delete -- release the first change's hold alone, and the second's
    // Undo names its own. Mutation: a release by the contact's id, which
    // let the facade side drop the newer hold.
    #[test]
    fn a_contacts_second_change_releases_the_first_hold_and_keeps_its_own() {
        use super::super::swipe::ContactChange;
        let mut model = UiModel::new();
        model.contact_changed("c1".to_owned(), ContactChange::PhotoRemoved, 7);
        model.contact_changed("c1".to_owned(), ContactChange::Deleted, 8);
        assert_eq!(model.take_released().contacts, vec![7], "the replaced one");
        let id = model.snackbar().expect("up").id;
        assert_eq!(
            model.undo(id),
            Some(Intent::UndoContact {
                contact_id: "c1".to_owned(),
                change: ContactChange::Deleted,
                hold: 8,
            })
        );
    }

    /// Contact-cards design section 5.2, "The photo": a chosen photo is
    /// being prepared, and Save then takes the facade side's own; an
    /// answer counts only for the last choice still being prepared; while
    /// a choice is prepared or found unusable, what the editor showed
    /// before it stays known (a prepared photo, a removal, or the saved
    /// card's); Remove is an unsaved removal. Mutations: an earlier
    /// choice's answer taken for a later one; what was shown before
    /// forgotten at a second choice; Save keeping the card's photo while
    /// one is prepared.
    #[test]
    fn a_chosen_photo_is_prepared_before_save_takes_it() {
        let photo = |n: u8| Photo {
            width: 2,
            height: 2,
            rgba: vec![n; 16],
        };
        let mut model = UiModel::new();
        assert_eq!(model.own_photo(), None);
        assert_eq!(model.own_photo_edit(), PhotoEdit::Keep, "nothing chosen");

        let first = model.own_photo_chosen();
        assert_eq!(model.own_photo(), Some(&OwnPhotoStep::Preparing));
        assert_eq!(model.own_photo_before(), None, "the saved card's, before");
        assert_eq!(model.own_photo_basis(), PhotoBasis::Card);
        assert_eq!(
            model.own_photo_edit(),
            PhotoEdit::Prepared,
            "Save waits for it"
        );
        model.own_photo_prepared(first, Some(photo(1)));
        assert_eq!(model.own_photo(), Some(&OwnPhotoStep::Prepared(photo(1))));

        // A second choice: P1 is what the editor showed before it, through
        // an earlier answer arriving late, and through an unusable one.
        let second = model.own_photo_chosen();
        assert_eq!(
            model.own_photo_before(),
            Some(&OwnPhotoStep::Prepared(photo(1)))
        );
        assert_eq!(
            model.own_photo_basis(),
            PhotoBasis::Prepared(first),
            "by its number"
        );
        model.own_photo_prepared(first, Some(photo(9)));
        assert_eq!(
            model.own_photo(),
            Some(&OwnPhotoStep::Preparing),
            "not this choice's"
        );
        let third = model.own_photo_chosen();
        assert_eq!(
            model.own_photo_before(),
            Some(&OwnPhotoStep::Prepared(photo(1))),
            "a choice replacing one being prepared keeps what was shown before"
        );
        model.own_photo_prepared(second, Some(photo(2)));
        assert_eq!(model.own_photo(), Some(&OwnPhotoStep::Preparing));
        model.own_photo_prepared(third, None);
        assert_eq!(model.own_photo(), Some(&OwnPhotoStep::Unusable));
        assert_eq!(
            model.own_photo_before(),
            Some(&OwnPhotoStep::Prepared(photo(1)))
        );
        assert_eq!(
            model.own_photo_edit(),
            PhotoEdit::Prepared,
            "the facade side keeps P1 for Save"
        );

        // Remove, then a choice: the removal is what was shown before.
        model.own_photo_removed();
        assert_eq!(model.own_photo_edit(), PhotoEdit::Remove);
        let fourth = model.own_photo_chosen();
        assert_eq!(model.own_photo_before(), Some(&OwnPhotoStep::Removed));
        assert_eq!(model.own_photo_basis(), PhotoBasis::Removed);
        model.own_photo_removed();
        model.own_photo_prepared(fourth, Some(photo(4)));
        assert_eq!(model.own_photo(), Some(&OwnPhotoStep::Removed), "dropped");

        let fifth = model.own_photo_chosen();
        model.own_photo_forgotten();
        model.own_photo_prepared(fifth, Some(photo(5)));
        assert_eq!(model.own_photo(), None, "a late answer changes nothing");
        assert_eq!(model.own_photo_before(), None);
    }

    /// Design 5.2: unsaved changes are judged against the saved card. A
    /// photo prepared or on its way is one; a removal is one only over a
    /// card with a photo; an unusable choice is one only if what the editor
    /// showed before it was. Mutation: a removal counted whatever the card
    /// holds.
    #[test]
    fn the_editors_photo_is_unsaved_only_when_it_differs_from_the_card() {
        let photo = Photo {
            width: 2,
            height: 2,
            rgba: vec![1; 16],
        };
        let identity = |has_photo: bool| IdentityView {
            endpoint: EndpointId::parse("human").expect("an endpoint"),
            name: "Ada".to_owned(),
            given: "Ada".to_owned(),
            surname: None,
            has_photo,
            photo: has_photo.then(|| photo.clone()),
            key: OwnKey::Ready {
                fingerprint: "0123456789abcdef".to_owned(),
            },
            peer: None,
        };
        let mut model = UiModel::new();
        model.identity_shown(identity(false));
        assert!(!model.own_photo_unsaved(), "nothing chosen");
        model.own_photo_removed();
        assert!(!model.own_photo_unsaved(), "no photo removed from no photo");
        let seq = model.own_photo_chosen();
        assert!(model.own_photo_unsaved(), "one on its way");
        model.own_photo_prepared(seq, None);
        assert!(!model.own_photo_unsaved(), "unusable over the card's none");
        let seq = model.own_photo_chosen();
        model.own_photo_prepared(seq, Some(photo.clone()));
        assert!(model.own_photo_unsaved(), "a prepared one");

        model.own_photo_forgotten();
        model.identity_shown(identity(true));
        model.own_photo_removed();
        assert!(model.own_photo_unsaved(), "the card's photo removed");
        let seq = model.own_photo_chosen();
        model.own_photo_prepared(seq, None);
        assert!(model.own_photo_unsaved(), "unusable over a removal");
    }
}

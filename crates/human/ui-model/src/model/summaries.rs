// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Chats from the store's summaries (design/2026-10-27-start-window.md
//! sections 3 and 6.1): a conversation is listed with the store's count of
//! its unread rows and a preview of its newest shown row, whether or not
//! any of its rows is held. A model started from summaries is in the
//! windowed mode; one given a whole listing is not, and lists only what it
//! holds, as it always has.

use std::collections::BTreeMap;

use interweave_human_chat_protocol::{Block, Inline, Rendered, render};
use interweave_human_core::RowId;

use super::{ConversationKey, Direction, ItemKey, LastMessage, ListedOutbound, Table, UiModel};
use crate::labels::LabelKey;
use crate::preview::Preview;

/// What the store says of one conversation at the start, or when it is
/// asked again: its unread rows of a kind this build shows -- messages and
/// cards, one per row -- and its newest such row, unread or kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSummary {
    /// The conversation.
    pub conversation: ConversationKey,
    /// Its unread rows the store holds, held or not.
    pub unread: u64,
    /// Its newest shown row, `None` when it holds none (a conversation of
    /// pending rows alone, which are held).
    pub newest: Option<NewestRow>,
}

/// A conversation's newest shown row, as Chats previews it without holding
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewestRow {
    /// Its table: unread or kept.
    pub table: Table,
    /// Its row.
    pub row: RowId,
    /// When it reached this app.
    pub received_at: u64,
    /// Its first line, cut ([`crate::preview`]).
    pub preview: Preview,
    /// A contact card someone sent, which a view previews as that.
    pub card: bool,
}

/// The windowed mode's state: each conversation's summary, kept current
/// as rows arrive and are read, and re-based whenever the store is asked
/// again.
#[derive(Debug, Default)]
pub(super) struct Summaries {
    by: BTreeMap<ConversationKey, StoredSummary>,
    /// The unread table's high-water mark when the counts were read: a row
    /// at or below it is in them already, though handed over after them.
    counted_through: Option<RowId>,
}

impl Summaries {
    fn of(summaries: Vec<StoredSummary>, counted_through: Option<RowId>) -> Self {
        Self {
            by: summaries
                .into_iter()
                .map(|s| (s.conversation.clone(), s))
                .collect(),
            counted_through,
        }
    }

    /// Whether `row`, handed over now, is new to the counts: committed
    /// after they were read. The facade hands over rows it committed before
    /// a start's read -- held when a session closed -- after it.
    pub(super) fn is_new(&self, row: RowId) -> bool {
        self.counted_through.is_none_or(|mark| row > mark)
    }

    /// A row the store committed unread: counted, and the newest if it is.
    pub(super) fn arrived(&mut self, conversation: &ConversationKey, newest: NewestRow) {
        let summary = self
            .by
            .entry(conversation.clone())
            .or_insert_with(|| StoredSummary {
                conversation: conversation.clone(),
                unread: 0,
                newest: None,
            });
        summary.unread += 1;
        if summary
            .newest
            .as_ref()
            .is_none_or(|n| (n.received_at, n.row) <= (newest.received_at, newest.row))
        {
            summary.newest = Some(newest);
        }
    }

    /// An unread row of `conversation` left the unread table: read, or
    /// resolved.
    pub(super) fn left_unread(&mut self, conversation: &ConversationKey) {
        if let Some(summary) = self.by.get_mut(conversation) {
            summary.unread = summary.unread.saturating_sub(1);
        }
    }

    /// The store's count of `conversation`'s unread rows, as kept current.
    pub(super) fn unread_of(&self, conversation: &ConversationKey) -> u64 {
        self.by.get(conversation).map_or(0, |s| s.unread)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &StoredSummary> {
        self.by.values()
    }
}

impl NewestRow {
    /// The row as the list's [`LastMessage`]: received, its preview its
    /// body, as a single paragraph of the plain text the cut kept -- which
    /// the first line of gives back unchanged.
    pub(super) fn last(&self) -> LastMessage {
        LastMessage {
            key: ItemKey(self.table, self.row),
            direction: Direction::Inbound,
            label: match self.table {
                Table::Kept => LabelKey::Kept,
                _ => LabelKey::Unread,
            },
            body: Rendered::Markdown(vec![Block::Paragraph(vec![Inline::Text(
                self.preview.text.clone(),
            )])]),
            local_at: self.received_at,
            card: self.card,
            preview: self.preview.clone(),
        }
    }

    /// The newest row a message received now is, from its text.
    pub(super) fn received(row: RowId, received_at: u64, text: &str) -> Self {
        Self {
            table: Table::Unread,
            row,
            received_at,
            preview: crate::preview::preview(&render(text), &crate::body::image_text),
            card: false,
        }
    }

    /// The newest row a card received now is: previewed as a card, never
    /// by the name it claims.
    pub(super) const fn card(row: RowId, received_at: u64) -> Self {
        Self {
            table: Table::Unread,
            row,
            received_at,
            preview: Preview {
                text: String::new(),
                cut: false,
            },
            card: true,
        }
    }
}

impl UiModel {
    /// The start from the store's summaries rather than every row: the
    /// pending rows, held as at any start, and each conversation's count
    /// and newest row, with the unread table's high-water mark read beside
    /// them (`HumanStore::unread_high_water`): a row handed over later whose
    /// id is at or below it is in the counts already. The model is in the
    /// windowed mode from here on.
    pub fn started(
        &mut self,
        pending: Vec<ListedOutbound>,
        summaries: Vec<StoredSummary>,
        counted_through: Option<RowId>,
    ) {
        self.pending_listed(pending);
        self.windowed = Some(Summaries::of(summaries, counted_through));
    }

    /// The summaries asked again -- the store said rows are waiting there:
    /// every count and newest row re-based on the store's, and on its mark.
    pub fn summaries_listed(
        &mut self,
        summaries: Vec<StoredSummary>,
        counted_through: Option<RowId>,
    ) {
        if self.windowed.is_some() {
            self.windowed = Some(Summaries::of(summaries, counted_through));
        }
    }

    /// Whether the model started from summaries.
    #[must_use]
    pub const fn windowed(&self) -> bool {
        self.windowed.is_some()
    }
}

/// A summary-only preview's [`Preview`], so held and stored rows agree.
pub(super) fn held_preview(body: &Rendered) -> Preview {
    crate::preview::preview(body, &crate::body::image_text)
}

#[cfg(test)]
mod tests {
    use interweave_human_chat_protocol::{Card, HumanChatV2, MessageKind};
    use interweave_human_client_api::{Destination, Origin, Received};
    use interweave_human_core::RowId;
    use interweave_transport_api::{EndpointId, TransportIdentity};

    use super::super::cards::{KeyView, Proposal, ProposalContext, ValueChanges};
    use super::{NewestRow, StoredSummary};
    use crate::model::{ConversationKey, Direction, Table, UiModel};
    use crate::preview::cut;

    const P: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
    const Q: &str = "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5";

    fn peer(s: &str) -> TransportIdentity {
        TransportIdentity::parse(s).expect("a PeerId")
    }

    fn human() -> EndpointId {
        EndpointId::parse("human").expect("an endpoint")
    }

    fn direct(p: &str) -> ConversationKey {
        ConversationKey::Direct {
            peer: peer(p),
            endpoint: Some(human()),
        }
    }

    fn summary(p: &str, unread: u64, table: Table, row: i64, at: u64, text: &str) -> StoredSummary {
        StoredSummary {
            conversation: direct(p),
            unread,
            newest: Some(NewestRow {
                table,
                row: RowId::from_stored(row),
                received_at: at,
                preview: cut(text),
                card: false,
            }),
        }
    }

    fn envelope(n: u32, text: &str) -> HumanChatV2 {
        HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{n:032x}"),
            text: text.to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        }
    }

    fn received(row: i64, p: &str, at: u64, text: &str) -> Received {
        Received {
            row: RowId::from_stored(row),
            origin: Origin::Direct {
                peer: peer(p),
                endpoint: human(),
            },
            envelope: envelope(u32::try_from(row).expect("small"), text),
            received_at: at,
        }
    }

    fn card(row: i64, p: &str, at: u64) -> Proposal {
        Proposal {
            row: RowId::from_stored(row),
            peer: peer(p),
            endpoint: human(),
            card: Card {
                app_message_id: "0".repeat(32),
                name: "Ada".to_owned(),
                surname: None,
                photo: None,
                identity_key: [1; 32],
                sent_at_ms: None,
                from_endpoint: None,
            },
            received_at: at,
            context: ProposalContext {
                updates: None,
                shares_node: Vec::new(),
                same_name: Vec::new(),
                key_matches: Vec::new(),
                key: KeyView::New,
                fingerprint: "0123456789abcdef".to_owned(),
                changes_nothing: false,
                values: ValueChanges::default(),
            },
        }
    }

    fn listed(model: &UiModel, p: &str) -> crate::model::ConversationSummary {
        model
            .conversations()
            .into_iter()
            .find(|s| s.key == direct(p))
            .expect("listed")
    }

    /// Every conversation is listed with the store's count of its unread
    /// rows, none of them held: a flood of 5,000 shows 5,000 and its
    /// newest row's preview; a kept-only conversation is listed too
    /// (design §3, §10 items 1 and 2). Mutations: the badge counting held
    /// rows alone; a summary with nothing unread left out.
    #[test]
    fn every_conversation_is_listed_with_the_stores_count_under_a_flood() {
        let mut model = UiModel::new();
        model.started(
            Vec::new(),
            vec![
                summary(P, 5_000, Table::Unread, 9_000, 50, "the newest of many"),
                summary(Q, 0, Table::Kept, 7, 10, "kept long ago"),
            ],
            None,
        );
        assert!(model.windowed());
        assert!(model.is_empty(), "no row is held");
        let flood = listed(&model, P);
        assert_eq!(flood.unread, 5_000);
        assert_eq!(flood.last.preview.shown(), "the newest of many");
        let kept = listed(&model, Q);
        assert_eq!(kept.unread, 0);
        assert_eq!(kept.last.preview.shown(), "kept long ago");
        assert_eq!(model.conversations()[0].key, direct(P), "newest first");
    }

    /// A row that arrives is counted once and is the preview; read, it
    /// leaves the count -- the store's count, kept current. Mutations: the
    /// arrival not counted; the read not taken off; a held unread row
    /// counted again on top of the store's count.
    #[test]
    fn an_arrival_counts_once_and_a_read_leaves_the_count() {
        let mut model = UiModel::new();
        model.started(
            Vec::new(),
            vec![summary(P, 2, Table::Unread, 1, 10, "older")],
            None,
        );
        // Open, so the arrival is held and can be read.
        model.window_input(crate::model::window::WindowInput::Opened(direct(P)));
        model.received(received(5, P, 20, "just now"));
        let now = listed(&model, P);
        assert_eq!(now.unread, 3);
        assert_eq!(now.last.preview.shown(), "just now");
        model.received(received(5, P, 20, "just now"));
        assert_eq!(listed(&model, P).unread, 3, "the same row once");

        model.read(RowId::from_stored(5));
        assert_eq!(listed(&model, P).unread, 2);
        assert_eq!(
            listed(&model, P).last.preview.shown(),
            "just now",
            "still the newest, read"
        );
    }

    /// A row of this session newer than the store's newest -- one's own
    /// sent message -- is the preview, as a held row newer than the
    /// summary's always is (design §10 item 3a). Mutation: the summary's
    /// newest always winning.
    #[test]
    fn a_session_row_newer_than_the_stored_newest_is_the_preview() {
        let mut model = UiModel::new();
        model.started(
            Vec::new(),
            vec![summary(P, 1, Table::Unread, 1, 10, "theirs")],
            None,
        );
        model.sent(
            RowId::from_stored(2),
            &Destination::Direct {
                peer: peer(P),
                endpoint: Some(human()),
            },
            envelope(2, "mine, later"),
            30,
        );
        let now = listed(&model, P);
        assert_eq!(now.last.direction, Direction::Outbound);
        assert_eq!(now.last.preview.shown(), "mine, later");
        assert_eq!(now.unread, 1, "an own row is never unread");
    }

    /// In the windowed mode a card someone sent counts in the badge and is
    /// the preview, as a card (contact-cards §5.4.5); resolved unread, it
    /// leaves the count. A model given a whole listing keeps cards out of
    /// both, as it always has. Mutation: the card's arrival not counted.
    #[test]
    fn a_card_counts_in_the_badge_in_the_windowed_mode_only() {
        let mut model = UiModel::new();
        model.started(Vec::new(), Vec::new(), None);
        model.window_input(crate::model::window::WindowInput::Opened(direct(P)));
        model.card_arrived(card(3, P, 40));
        let now = listed(&model, P);
        assert_eq!(now.unread, 1);
        assert!(now.last.card);
        assert_eq!(now.last.direction, Direction::Inbound);
        assert!(
            !now.last.preview.shown().contains("Ada"),
            "never by the name it claims"
        );
        model.read(RowId::from_stored(3));
        assert_eq!(listed(&model, P).unread, 0, "read, it leaves the count");

        let mut listed_mode = UiModel::new();
        listed_mode.card_arrived(card(3, P, 40));
        assert!(
            listed_mode.conversations().is_empty(),
            "the listed mode lists no card"
        );
    }

    /// A card the store already counts, handed over with a page or a
    /// listing, is not counted again; read, it leaves the count at zero
    /// (#498 review F1). Mutation: `card_received` counting as an arrival.
    #[test]
    fn a_stored_card_handed_over_is_counted_once() {
        let mut model = UiModel::new();
        model.started(
            Vec::new(),
            vec![StoredSummary {
                conversation: direct(P),
                unread: 1,
                newest: Some(NewestRow::card(RowId::from_stored(3), 40)),
            }],
            None,
        );
        model.card_received(card(3, P, 40));
        assert_eq!(listed(&model, P).unread, 1);
        model.read(RowId::from_stored(3));
        assert_eq!(listed(&model, P).unread, 0);
    }

    /// A row the facade committed before the start's read and hands over
    /// after it -- held when a session closed -- is in the store's count
    /// already: a message or a card at or below the mark counts nothing, a
    /// row above it once (design section 3). Mutation: the mark ignored.
    #[test]
    fn a_row_the_starts_count_already_holds_is_not_counted_again() {
        let mut model = UiModel::new();
        model.started(
            Vec::new(),
            vec![summary(P, 2, Table::Unread, 4, 10, "counted")],
            Some(RowId::from_stored(4)),
        );
        model.window_input(crate::model::window::WindowInput::Opened(direct(P)));
        model.received(received(3, P, 9, "held at the close"));
        model.card_arrived(card(4, P, 10));
        assert_eq!(listed(&model, P).unread, 2, "both in the count already");
        model.received(received(5, P, 20, "after the read"));
        assert_eq!(listed(&model, P).unread, 3);
    }

    /// Asked again, the store's summaries replace the counts the session
    /// kept. Mutation: the re-base ignored.
    #[test]
    fn summaries_asked_again_rebase_every_count() {
        let mut model = UiModel::new();
        model.started(
            Vec::new(),
            vec![summary(P, 2, Table::Unread, 1, 10, "older")],
            None,
        );
        model.received(received(5, P, 20, "just now"));
        model.summaries_listed(
            vec![summary(P, 7, Table::Unread, 9, 60, "after a while")],
            None,
        );
        let now = listed(&model, P);
        assert_eq!(now.unread, 7);
        assert_eq!(now.last.preview.shown(), "after a while");
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A row's actions as swipes and buttons, Hide, and the snackbar's Undo
//! (`design/2026-10-16-message-swipe.md`; `human-client-ui.md` §6).
//!
//! A swipe is a shortcut for a row button: both name a [`RowAction`], and
//! [`UiModel::intent_for`] answers it from what [`UiModel::actions`]
//! offers at that moment, so the swipe never decides legality itself and
//! a row whose state changed under the finger raises nothing (§8).
//!
//! Every action runs when it is committed. An Undo is the REVERSE action,
//! never a cancelled delay: if the app dies while a snackbar is up, the
//! message is as the snackbar said (§5). So nothing here waits for a
//! snackbar to close before raising an intent, and closing one raises
//! none -- it only lets go of what the Undo needed
//! ([`UiModel::take_released`]).

use interweave_human_core::RowId;

use super::{ConversationKey, Direction, Intent, ItemKey, Table, UiModel};

/// What a row's swipe or button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowAction {
    /// Keep a read message.
    Keep,
    /// Drop a read, unkept message from this session (Hide for now).
    Hide,
    /// Remove Keep from a kept message.
    Unkeep,
    /// Retry a pending message now.
    Retry,
    /// Cancel a pending message.
    Cancel,
}

/// Which action a row's swipe offers in each direction, named by the
/// direction of reading rather than the screen: `toward_end` is a swipe
/// toward the end of a line -- right in a left-to-right layout, left in a
/// right-to-left one, which the view mirrors (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SwipeOffer {
    /// Keep or Retry.
    pub toward_end: Option<RowAction>,
    /// Hide, Unkeep or Cancel.
    pub toward_start: Option<RowAction>,
}

/// What a snackbar confirms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    /// A message was kept.
    Kept,
    /// A message was hidden for this session.
    Hidden,
    /// Keep was removed.
    Unkept,
    /// A send was cancelled: in the words of the row's own cancelled
    /// label, since the cancelled attempt may have reached the receiver.
    Cancelled {
        /// Whether an attempt may have reached the remote.
        may_have_reached: bool,
    },
    /// A conversation was hidden from the list.
    ConversationHidden,
    /// Details' Copy put the conversation's whole identifier on the
    /// clipboard: a direct conversation's `PeerId`, or a channel's id.
    /// Shown only where the platform confirms nothing itself (#304 F11).
    IdentifierCopied {
        /// A channel's id rather than a `PeerId`.
        channel: bool,
    },
    /// A card's Accept created a contact (contact-cards design 5.4.2,
    /// cases N, D and P).
    ContactSaved,
    /// A card's Accept updated a contact (cases U and E).
    ContactUpdated,
    /// A card was declined (5.4.3).
    CardDeclined,
}

/// The one snackbar, while it is up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snackbar {
    /// Its identity: the view closes or undoes THIS one by it, so an
    /// answer to a snackbar already replaced does nothing.
    pub id: u64,
    /// What it confirms.
    pub confirms: Confirmation,
    /// Whether it offers Undo now. Never an Undo that would fail: with no
    /// content left to keep, or no cancelled record to restore, it has
    /// none (§8).
    pub undo: bool,
    /// The conversation it is shown in, or `None` on the conversation
    /// list (a conversation hide).
    pub conversation: Option<ConversationKey>,
    /// The card whose Accept or Decline it confirms, by its row.
    pub card: Option<RowId>,
}

/// Where focus goes after an action took a row away, brought it back or
/// changed its buttons (§7, §10): never to the top of the list, never lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusTarget {
    /// A message row's first control.
    Item(ItemKey),
    /// A conversation's row on the conversation list.
    Conversation(ConversationKey),
    /// A conversation's header title, once no row of it is left: never
    /// the composer, which would open the phone's keyboard unasked.
    Header(ConversationKey),
    /// The conversation list's heading, once no conversation is left on it.
    ConversationsHeading,
    /// A card's proposal, or the confirmation row its Accept left in its
    /// place, by the card's row: the row's first control.
    Proposal(RowId),
    /// A card's proposal's Accept, or Close where the card changes nothing:
    /// where an Undo of its answer sends focus (contact-cards design
    /// section 9's focus table).
    ProposalAccept(RowId),
}

/// What the view says once, through its polite live region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Announce {
    /// A snackbar opened: say what it confirms.
    Confirmed(Confirmation),
    /// An Undo brought the state back.
    Restored,
}

/// The latest outcome of a swipe, a button or an Undo, for the view to
/// act on once: each has a new `id`, and the view acts on an id it has
/// not acted on yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Its identity.
    pub id: u64,
    /// What to announce, if anything. A refusal says nothing here: the
    /// existing storage notice and refusal announcement say it.
    pub announce: Option<Announce>,
    /// Where focus goes, or `None` to leave it where it is.
    pub focus: Option<FocusTarget>,
    /// The row this outcome took away, or whose buttons it changed -- a
    /// Keep or Unkeep: `focus` is moved only if focus was inside that row
    /// (§7, §10), or, for a Keep or Unkeep, if the row's own button made
    /// it, which the view knows and the model does not. A touch that
    /// swiped a row leaves focus where it was, so the list does not jump.
    /// `None`: `focus` is moved wherever focus is.
    pub moves_from: Option<FocusTarget>,
}

/// A snackbar closed with no Undo taken -- its timeout, a swipe away, a
/// newer snackbar, leaving, an arrival (§7, §8, §10): where focus goes if
/// the closed snackbar's Undo held it, since that control is gone. The
/// view knows where focus was, and acts on each `id` once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoClosed {
    /// Its identity.
    pub id: u64,
    /// The conversation the snackbar was shown in, or `None` on the
    /// conversation list. Once the person has left it, focus goes to its
    /// row on the list instead of `focus` (§7).
    pub conversation: Option<ConversationKey>,
    /// The row the closed snackbar's action was about, if it is still
    /// shown, else where focus goes after its removal: the next row, the
    /// previous one, the header title, or on the list the next
    /// conversation, the previous one, the list's heading.
    pub focus: FocusTarget,
}

/// What a closed snackbar's Undo no longer needs, for the root to let go
/// of on the facade side ([`UiModel::take_released`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Released {
    /// The read copies held for Keep of messages now hidden: by the row
    /// whose release handed each over, as [`Intent::Keep`]'s `from`.
    pub copies: Vec<(Table, RowId)>,
    /// The cancelled pending records held for their Undo, by the rows the
    /// cancels ended: one per snackbar closed or replaced, however many
    /// that was since the last take.
    pub cancelled: Vec<RowId>,
    /// The cards held for the Undo of their Accept or Decline, by row.
    pub cards: Vec<RowId>,
}

impl Released {
    /// Whether there is nothing to let go of.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.copies.is_empty() && self.cancelled.is_empty() && self.cards.is_empty()
    }
}

/// What an open snackbar's Undo does.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Undo {
    /// Undo of Keep: Unkeep the row the Keep made.
    Unkeep { item: ItemKey, kept_row: RowId },
    /// Undo of Unkeep: the Keep `actions` offers on it then.
    Keep { item: ItemKey },
    /// Undo of Cancel: restore the record the facade side holds, if it
    /// holds one.
    Restore {
        item: ItemKey,
        row: RowId,
        restorable: bool,
    },
    /// Undo of Hide, of one message or of a conversation's.
    Unhide {
        items: Vec<ItemKey>,
        conversation: Option<ConversationKey>,
    },
    /// Undo of a card's Accept or Decline: the facade side holds the
    /// card and what the Accept changed.
    Card { row: RowId },
    /// Nothing to take back: a copy changes nothing here.
    None,
}

impl Undo {
    /// The items the Undo needs held: not evicted while it is offered.
    fn items(&self) -> Vec<ItemKey> {
        match self {
            Self::Unkeep { item, .. } | Self::Keep { item } | Self::Restore { item, .. } => {
                vec![*item]
            }
            Self::Unhide { items, .. } => items.clone(),
            Self::Card { .. } | Self::None => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct Open {
    id: u64,
    confirms: Confirmation,
    conversation: Option<ConversationKey>,
    undo: Undo,
    /// Where the action sent focus: for a conversation hide, the hidden
    /// row's place on the list, which the list no longer holds.
    sent_focus: Option<FocusTarget>,
}

/// An Undo raised and not answered yet: its result restores the state
/// and is announced as such, never confirmed with a snackbar of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Awaiting {
    Unkept(ItemKey),
    Kept(ItemKey),
    Restored(ItemKey),
    /// The Undo of a card's answer.
    Card(RowId),
}

/// The swipe state the model holds.
#[derive(Debug, Default)]
pub(super) struct SwipeState {
    seq: u64,
    open: Option<Open>,
    awaiting: Option<Awaiting>,
    /// Items whose Keep is on its way: one entry per Keep issued, each gone
    /// at its answer or its refusal, so bounded by the Keeps in flight.
    keeping: std::collections::HashSet<ItemKey>,
    /// Cancels on their way, by the pending row each cancels, with the
    /// sequence number when it was issued: a snackbar with a later id was
    /// opened by an action taken after the Cancel, and the Cancel's answer
    /// never takes it away (§6). Each gone at its answer or its refusal,
    /// so bounded by the Cancels in flight.
    cancelling: std::collections::HashMap<RowId, u64>,
    outcome: Option<Outcome>,
    undo_closed: Option<UndoClosed>,
    released: Released,
    /// Where the answer a card Undo takes back sent focus: where a
    /// refused Undo sends it again.
    card_focus: Option<FocusTarget>,
}

impl SwipeState {
    /// Whether `key` must stay held: an open snackbar's Undo needs it, or
    /// a Keep of it is on its way and its answer attaches a row the store
    /// holds -- so the session cap never evicts it (review of #289, round
    /// 3).
    pub(super) fn protects(&self, key: &ItemKey) -> bool {
        self.keeping.contains(key)
            || self
                .open
                .as_ref()
                .is_some_and(|o| o.undo.items().contains(key))
    }
}

impl UiModel {
    /// The actions a row offers as buttons, in order (§7): Keep and Hide
    /// on a read, unkept message, Unkeep on a kept one, Retry and Cancel
    /// on a pending one, none on anything else.
    #[must_use]
    pub fn row_actions(&self, key: ItemKey) -> Vec<RowAction> {
        let mut list: Vec<RowAction> = self
            .actions(key)
            .iter()
            .filter_map(|intent| match intent {
                Intent::Keep { .. } => Some(RowAction::Keep),
                Intent::Unkeep(_) => Some(RowAction::Unkeep),
                Intent::Retry(_) => Some(RowAction::Retry),
                Intent::Cancel(_) => Some(RowAction::Cancel),
                _ => None,
            })
            .collect();
        if self.hide_offered(key) {
            list.push(RowAction::Hide);
        }
        list
    }

    /// Which way the row may be swiped (§3): a direction exactly when its
    /// action is legal now. Keep and Retry toward the end, Hide, Unkeep
    /// and Cancel toward the start.
    #[must_use]
    pub fn swipe_offer(&self, key: ItemKey) -> SwipeOffer {
        let actions = self.row_actions(key);
        let has = |a| actions.contains(&a);
        SwipeOffer {
            toward_end: [RowAction::Keep, RowAction::Retry]
                .into_iter()
                .find(|a| has(*a)),
            toward_start: [RowAction::Hide, RowAction::Unkeep, RowAction::Cancel]
                .into_iter()
                .find(|a| has(*a)),
        }
    }

    /// The intent `action` on `key` raises NOW, or `None` if it is no
    /// longer legal: a swipe is re-checked at commit, and a row whose
    /// state changed under the finger springs back (§8).
    #[must_use]
    pub fn intent_for(&self, key: ItemKey, action: RowAction) -> Option<Intent> {
        if action == RowAction::Hide {
            return self.hide_offered(key).then_some(Intent::Hide(key));
        }
        self.actions(key).into_iter().find(|intent| {
            matches!(
                (action, intent),
                (RowAction::Keep, Intent::Keep { .. })
                    | (RowAction::Unkeep, Intent::Unkeep(_))
                    | (RowAction::Retry, Intent::Retry(_))
                    | (RowAction::Cancel, Intent::Cancel(_))
            )
        })
    }

    /// Hide is offered on every read, unkept received message, with or
    /// without the content Keep would need: hiding needs no copy (§3).
    fn hide_offered(&self, key: ItemKey) -> bool {
        // Not while a Keep is on its way: its answer attaches a row the
        // store holds, and the item must still be here to take it (review
        // F1 on #289, round 2).
        !self.swipe.keeping.contains(&key)
            && self.items.get(&key).is_some_and(|item| {
                !item.hidden && item.direction == Direction::Inbound && item.rows.is_empty()
            })
    }

    /// The root issued `intent`: a Keep is on its way to the store until it
    /// is answered ([`Self::kept`]) or refused ([`Self::intent_refused`]).
    pub fn keep_requested(&mut self, intent: &Intent) {
        if let Intent::Keep { item, .. } = intent {
            self.swipe.keeping.insert(*item);
        }
    }

    /// The root issued the Cancel of pending `row`: until it is answered
    /// ([`Self::cancelled`]) or refused ([`Self::intent_refused`]), the
    /// snackbar up now stays, with its Undo (§6).
    pub fn cancel_requested(&mut self, row: RowId) {
        // A second Cancel of the same row while the first waits keeps the
        // first's mark: the snackbars after it are still the newer ones.
        self.swipe.cancelling.entry(row).or_insert(self.swipe.seq);
    }

    /// Whether conversation `key` may be hidden from the list (§10): it
    /// shows something, and every message it shows is a read, unkept
    /// received one or a sent one the transport is done with. Nothing
    /// unread, kept or pending.
    #[must_use]
    pub fn conversation_hide_offered(&self, key: &ConversationKey) -> bool {
        let mut shown = self
            .items
            .values()
            .filter(|i| &i.conversation == key && !i.hidden)
            .peekable();
        shown.peek().is_some()
            && shown.all(|item| match &item.outbound {
                Some(status) => status.is_terminal(),
                None => item.rows.is_empty(),
            })
            // Nor while a Keep of one of them is on its way: its answer
            // attaches a row the store holds (review of #289, round 3).
            && !self
                .items
                .iter()
                .any(|(k, i)| &i.conversation == key && self.swipe.keeping.contains(k))
    }

    /// The intent hiding conversation `key` raises now, if it is offered.
    #[must_use]
    pub fn hide_conversation_intent(&self, key: &ConversationKey) -> Option<Intent> {
        self.conversation_hide_offered(key)
            .then(|| Intent::HideConversation(key.clone()))
    }

    // --- results -----------------------------------------------------------

    /// Hide `key` for the rest of the session: it leaves the list now and
    /// the session's memory once its snackbar closes. Writes nothing --
    /// its durable copy went when it was read (`human-client-ui.md` §6).
    pub fn hide(&mut self, key: ItemKey) {
        if !self.hide_offered(key) {
            return;
        }
        let Some(conversation) = self.items.get(&key).map(|i| i.conversation.clone()) else {
            return;
        };
        let focus = self.focus_after_leaving(&conversation, key);
        if let Some(item) = self.items.get_mut(&key) {
            item.hidden = true;
        }
        self.open(
            Confirmation::Hidden,
            Some(conversation),
            Undo::Unhide {
                items: vec![key],
                conversation: None,
            },
            focus,
            Some(FocusTarget::Item(key)),
        );
    }

    /// Hide conversation `key` from the list, every message in it as
    /// [`Self::hide`] hides one (§10).
    pub fn hide_conversation(&mut self, key: &ConversationKey) {
        if !self.conversation_hide_offered(key) {
            return;
        }
        let list = self.conversations();
        let at = list.iter().position(|c| &c.key == key);
        let focus = at
            .and_then(|i| {
                list.get(i + 1)
                    .or_else(|| i.checked_sub(1).and_then(|p| list.get(p)))
            })
            .map_or(FocusTarget::ConversationsHeading, |c| {
                FocusTarget::Conversation(c.key.clone())
            });
        let items: Vec<ItemKey> = self
            .items
            .iter()
            .filter(|(_, i)| &i.conversation == key && !i.hidden)
            .map(|(k, _)| *k)
            .collect();
        for k in &items {
            if let Some(item) = self.items.get_mut(k) {
                item.hidden = true;
            }
        }
        self.open(
            Confirmation::ConversationHidden,
            None,
            Undo::Unhide {
                items,
                conversation: Some(key.clone()),
            },
            Some(focus),
            Some(FocusTarget::Conversation(key.clone())),
        );
    }

    /// The facade cancelled pending `row` as the person asked: the row
    /// leaves the list, and `restorable` says whether the facade side
    /// holds the record an Undo would restore (§8).
    ///
    /// A snackbar opened after this Cancel was issued -- the person acted
    /// again while it waited -- stays, with its Undo: the cancel is then
    /// only announced, and offers no Undo of its own (§6).
    pub fn cancelled(&mut self, row: RowId, restorable: bool) {
        let issued = self.swipe.cancelling.remove(&row);
        // The terminal update released the row, so it is found by the
        // pending row it last held: its first, or the one a restore gave it.
        let key = self
            .items
            .iter()
            .find(|(_, i)| i.last_pending == Some(row))
            .map(|(k, _)| *k);
        let Some(key) = key else {
            // No item to show a snackbar for: no Undo can be offered, so
            // the record the facade side holds for one is let go now.
            if restorable {
                self.swipe.released.cancelled.push(row);
            }
            return;
        };
        let Some((conversation, may_have_reached)) = self.items.get(&key).map(|i| {
            (
                i.conversation.clone(),
                matches!(
                    i.outbound,
                    Some(interweave_human_client_api::OutboundStatus::Cancelled {
                        may_have_reached: true
                    })
                ),
            )
        }) else {
            return;
        };
        let focus = self.focus_after_leaving(&conversation, key);
        if let Some(item) = self.items.get_mut(&key) {
            item.hidden = true;
        }
        let confirms = Confirmation::Cancelled { may_have_reached };
        let newer_up = issued.is_some_and(|at| self.swipe.open.as_ref().is_some_and(|o| o.id > at));
        if newer_up {
            // No snackbar, so no Undo: what an Undo would have needed goes
            // now, as when a snackbar closes.
            self.drop_items(&[key]);
            if restorable {
                self.swipe.released.cancelled.push(row);
            }
            self.set_outcome(
                Some(Announce::Confirmed(confirms)),
                focus,
                Some(FocusTarget::Item(key)),
            );
            return;
        }
        self.open(
            confirms,
            Some(conversation),
            Undo::Restore {
                item: key,
                row,
                restorable,
            },
            focus,
            Some(FocusTarget::Item(key)),
        );
    }

    /// A Keep of `key` was answered: it is no longer on its way, whether or
    /// not the item is still held.
    pub(super) fn keep_answered(&mut self, key: ItemKey) {
        self.swipe.keeping.remove(&key);
    }

    /// The root kept `key` as `kept_row`: confirmed with a snackbar, or,
    /// when it was an Undo of Unkeep, announced as restored.
    pub(super) fn swipe_kept(&mut self, key: ItemKey, kept_row: RowId) {
        if self.swipe.awaiting == Some(Awaiting::Kept(key)) {
            self.swipe.awaiting = None;
            self.set_outcome(Some(Announce::Restored), Some(FocusTarget::Item(key)), None);
            return;
        }
        let conversation = self.items.get(&key).map(|i| i.conversation.clone());
        // The row's buttons change, so a pressed one is gone: focus goes
        // to the row's first control (§7).
        self.open(
            Confirmation::Kept,
            conversation,
            Undo::Unkeep {
                item: key,
                kept_row,
            },
            Some(FocusTarget::Item(key)),
            Some(FocusTarget::Item(key)),
        );
    }

    /// The root removed Keep from `key`'s kept row.
    pub(super) fn swipe_unkept(&mut self, key: ItemKey) {
        if self.swipe.awaiting == Some(Awaiting::Unkept(key)) {
            self.swipe.awaiting = None;
            self.set_outcome(Some(Announce::Restored), Some(FocusTarget::Item(key)), None);
            return;
        }
        let conversation = self.items.get(&key).map(|i| i.conversation.clone());
        // The row's buttons change, so a pressed one is gone: focus goes
        // to the row's first control (§7).
        self.open(
            Confirmation::Unkept,
            conversation,
            Undo::Keep { item: key },
            Some(FocusTarget::Item(key)),
            Some(FocusTarget::Item(key)),
        );
    }

    /// The facade side restored the record cancelled as `cancelled` as the
    /// pending row `row`: the message is back in its place, pending, under
    /// the key it was first shown with.
    pub fn restored(&mut self, cancelled: RowId, row: RowId) {
        let Some(Awaiting::Restored(key)) = self.swipe.awaiting else {
            return;
        };
        if self.items.get(&key).and_then(|i| i.last_pending) != Some(cancelled) {
            return;
        }
        self.swipe.awaiting = None;
        if let Some(item) = self.items.get_mut(&key) {
            item.hidden = false;
            item.last_pending = Some(row);
            item.rows.push((Table::Pending, row));
            item.outbound = Some(interweave_human_client_api::OutboundStatus::Sending {
                attempts: 0,
                next_retry_at: None,
                last_problem: None,
            });
        }
        self.by_row.insert((Table::Pending, row), key);
        if self.ephemeral_set.remove(&key) {
            self.ephemeral.retain(|k| *k != key);
        }
        // A status the facade reported before this answer arrived.
        if let Some(update) = self.held.remove(&row) {
            self.held_order.retain(|r| *r != row);
            self.apply(update);
        }
        self.set_outcome(Some(Announce::Restored), Some(FocusTarget::Item(key)), None);
    }

    /// An intent was refused (storage degraded or full, or nothing left
    /// to act on). If it was an Undo, the snackbar is already closed and
    /// focus goes where the action itself sent it (§7): a refused Undo of
    /// Cancel to the next row, as the cancel did -- the message stays
    /// cancelled, and its record is let go -- and a refused Undo of
    /// Unkeep or Keep to that row, still in place.
    ///
    /// Any other refusal is an outcome with nothing to say and nowhere to
    /// move focus: a row a swipe slid out for it -- a Cancel the store
    /// refused -- springs back on it, and focus stays (§8).
    pub fn intent_refused(&mut self, intent: &Intent) {
        match intent {
            Intent::Keep { item, .. } => {
                self.swipe.keeping.remove(item);
            }
            Intent::Cancel(row) => {
                self.swipe.cancelling.remove(row);
            }
            _ => {}
        }
        let Some(awaiting) = self.swipe.awaiting else {
            self.set_outcome(None, None, None);
            return;
        };
        let matches = match (awaiting, intent) {
            (Awaiting::Kept(key), Intent::Keep { item, .. }) => key == *item,
            (Awaiting::Unkept(key), Intent::Unkeep(row)) => self
                .items
                .get(&key)
                .is_some_and(|i| i.rows.contains(&(Table::Kept, *row))),
            (Awaiting::Restored(_), Intent::Restore(_)) => true,
            (Awaiting::Card(awaited), Intent::UndoCard(row)) => awaited == *row,
            _ => false,
        };
        if !matches {
            self.set_outcome(None, None, None);
            return;
        }
        self.swipe.awaiting = None;
        match awaiting {
            Awaiting::Kept(key) | Awaiting::Unkept(key) => {
                self.set_outcome(None, Some(FocusTarget::Item(key)), None);
            }
            Awaiting::Restored(key) => {
                let focus = self
                    .items
                    .get(&key)
                    .map(|i| i.conversation.clone())
                    .and_then(|c| self.focus_after_leaving(&c, key));
                self.drop_items(&[key]);
                if let Intent::Restore(row) = intent {
                    self.swipe.released.cancelled.push(*row);
                }
                self.set_outcome(None, focus, None);
            }
            Awaiting::Card(row) => {
                // The answer stands, as the snackbar said; what its Undo
                // needed is let go.
                self.swipe.released.cards.push(row);
                let focus = self.swipe.card_focus.take();
                self.set_outcome(None, focus, None);
            }
        }
    }

    // --- the snackbar -------------------------------------------------------

    /// The snackbar, if one is up.
    #[must_use]
    pub fn snackbar(&self) -> Option<Snackbar> {
        let open = self.swipe.open.as_ref()?;
        Some(Snackbar {
            id: open.id,
            confirms: open.confirms,
            undo: self.undo_offered(&open.undo),
            conversation: open.conversation.clone(),
            card: match open.undo {
                Undo::Card { row } => Some(row),
                _ => None,
            },
        })
    }

    fn undo_offered(&self, undo: &Undo) -> bool {
        match undo {
            Undo::Unkeep { item, kept_row } => self
                .items
                .get(item)
                .is_some_and(|i| i.rows.contains(&(Table::Kept, *kept_row))),
            Undo::Keep { item } => self.intent_for(*item, RowAction::Keep).is_some(),
            Undo::Restore {
                item, restorable, ..
            } => *restorable && self.items.contains_key(item),
            Undo::Unhide { items, .. } => items.iter().any(|k| self.items.contains_key(k)),
            // The facade side holds it until this snackbar closes; one it
            // no longer holds answers the Undo with a refusal.
            Undo::Card { .. } => true,
            Undo::None => false,
        }
    }

    /// The person pressed the Undo of snackbar `id`. The snackbar closes,
    /// and the reverse action is raised for the root to carry out -- or,
    /// for a Hide, done here, since a Hide wrote nothing. `None` when the
    /// snackbar is gone, replaced, or offers no Undo now.
    pub fn undo(&mut self, id: u64) -> Option<Intent> {
        if self.swipe.open.as_ref().map(|o| o.id) != Some(id) {
            return None;
        }
        let open = self.swipe.open.take()?;
        if !self.undo_offered(&open.undo) {
            self.closed_under(&open);
            self.finish(open);
            return None;
        }
        match open.undo {
            Undo::Unkeep { item, kept_row } => {
                self.swipe.awaiting = Some(Awaiting::Unkept(item));
                Some(Intent::Unkeep(kept_row))
            }
            Undo::Keep { item } => {
                self.swipe.awaiting = Some(Awaiting::Kept(item));
                self.intent_for(item, RowAction::Keep)
            }
            Undo::Restore { item, row, .. } => {
                self.swipe.awaiting = Some(Awaiting::Restored(item));
                Some(Intent::Restore(row))
            }
            Undo::Unhide {
                items,
                conversation,
            } => {
                self.unhide(&items);
                let focus = match conversation {
                    Some(c) => Some(FocusTarget::Conversation(c)),
                    None => items.first().map(|k| FocusTarget::Item(*k)),
                };
                self.set_outcome(Some(Announce::Restored), focus, None);
                None
            }
            Undo::Card { row } => {
                self.swipe.awaiting = Some(Awaiting::Card(row));
                self.swipe.card_focus = open.sent_focus;
                Some(Intent::UndoCard(row))
            }
            Undo::None => None,
        }
    }

    /// Snackbar `id` closed -- its timeout, a swipe away, leaving the
    /// conversation, the app leaving the foreground -- with no Undo. The
    /// action already happened; only what its Undo needed is let go.
    pub fn snackbar_closed(&mut self, id: u64) {
        if self.swipe.open.as_ref().map(|o| o.id) == Some(id)
            && let Some(open) = self.swipe.open.take()
        {
            self.closed_under(&open);
            self.finish(open);
        }
    }

    /// The latest outcome, for the view to act on once by its id.
    #[must_use]
    pub const fn outcome(&self) -> Option<&Outcome> {
        self.swipe.outcome.as_ref()
    }

    /// The latest snackbar closed with no Undo taken, for the view to act
    /// on once by its id: where focus goes if that Undo held it.
    #[must_use]
    pub const fn undo_closed(&self) -> Option<&UndoClosed> {
        self.swipe.undo_closed.as_ref()
    }

    /// Details' Copy put the identifier of `conversation` on the clipboard,
    /// on a platform that confirms nothing itself: the one snackbar says
    /// so, with no Undo. Focus is the view's: back on the overflow button.
    pub fn identifier_copied(&mut self, conversation: ConversationKey) {
        let channel = matches!(conversation, ConversationKey::Channel(_));
        self.open(
            Confirmation::IdentifierCopied { channel },
            Some(conversation),
            Undo::None,
            None,
            None,
        );
    }

    /// What closed snackbars no longer need, for the root to let go of.
    pub fn take_released(&mut self) -> Released {
        std::mem::take(&mut self.swipe.released)
    }

    // --- cards ---------------------------------------------------------------

    /// Open the snackbar of a card's answer, its Undo the card's. An
    /// unannounced one says nothing through the live region: Accept's,
    /// whose confirmation row, focused, already says it (contact-cards
    /// design section 8, ux-designer's #416).
    pub(super) fn open_card_snackbar(
        &mut self,
        confirms: Confirmation,
        conversation: ConversationKey,
        row: RowId,
        focus: Option<FocusTarget>,
        announced: bool,
    ) {
        self.open(
            confirms,
            Some(conversation),
            Undo::Card { row },
            focus,
            Some(FocusTarget::Proposal(row)),
        );
        if !announced && let Some(outcome) = self.swipe.outcome.as_mut() {
            outcome.announce = None;
        }
    }

    /// Close the open snackbar if its Undo is a card's: the facade side
    /// holds one card answer, and every answer replaces it, so an answer
    /// that opens no snackbar of its own -- case S's Close, a proposal
    /// the model let go -- leaves no Undo the facade can no longer carry
    /// out (#414 F1).
    pub(super) fn close_card_snackbar(&mut self) {
        let card = self
            .swipe
            .open
            .as_ref()
            .is_some_and(|o| matches!(o.undo, Undo::Card { .. }));
        if card && let Some(open) = self.swipe.open.take() {
            self.closed_under(&open);
            self.finish(open);
        }
    }

    /// Let go of what the facade side holds for the Undo of `row`'s answer.
    pub(super) fn swipe_release_card(&mut self, row: RowId) {
        self.swipe.released.cards.push(row);
    }

    /// An outcome of a card's answer that opens no snackbar.
    pub(super) fn card_outcome(&mut self, announce: Option<Announce>, focus: Option<FocusTarget>) {
        self.set_outcome(announce, focus, None);
    }

    /// The Undo of `row`'s answer was carried out: announced as restored,
    /// focus on the proposal. A restore the model did not ask for -- none
    /// awaited -- is shown and says nothing.
    pub(super) fn card_undone(&mut self, row: RowId) {
        if self.swipe.awaiting != Some(Awaiting::Card(row)) {
            return;
        }
        self.swipe.awaiting = None;
        self.swipe.card_focus = None;
        self.set_outcome(
            Some(Announce::Restored),
            Some(FocusTarget::ProposalAccept(row)),
            None,
        );
    }

    // --- internals ----------------------------------------------------------

    fn open(
        &mut self,
        confirms: Confirmation,
        conversation: Option<ConversationKey>,
        undo: Undo,
        focus: Option<FocusTarget>,
        moves_from: Option<FocusTarget>,
    ) {
        // One at a time: a new one replaces the old, and the old one's
        // Undo goes with it (§6).
        if let Some(old) = self.swipe.open.take() {
            self.closed_under(&old);
            self.finish(old);
        }
        self.swipe.seq += 1;
        self.swipe.open = Some(Open {
            id: self.swipe.seq,
            confirms,
            conversation,
            undo,
            sent_focus: focus.clone(),
        });
        self.set_outcome(Some(Announce::Confirmed(confirms)), focus, moves_from);
    }

    fn set_outcome(
        &mut self,
        announce: Option<Announce>,
        focus: Option<FocusTarget>,
        moves_from: Option<FocusTarget>,
    ) {
        self.swipe.seq += 1;
        self.swipe.outcome = Some(Outcome {
            id: self.swipe.seq,
            announce,
            focus,
            moves_from,
        });
    }

    /// Let go of what a closed snackbar's Undo needed: a hidden or
    /// cancelled message leaves the session's memory, and the copy or
    /// record the facade side held for it is released. Anything else the
    /// Undo held goes back under the session cap.
    fn finish(&mut self, open: Open) {
        match open.undo {
            Undo::Unhide { items, .. } => {
                // A row attached closes this snackbar first
                // (`row_attached`), and a hidden item offers no Keep
                // (`actions`), so none holds a row. One whose Keep was
                // issued before it was hidden is shown again, never
                // dropped: its answer attaches a row the store holds.
                let (kept, gone): (Vec<ItemKey>, Vec<ItemKey>) = items
                    .into_iter()
                    .partition(|k| self.swipe.keeping.contains(k));
                self.unhide(&kept);
                for key in &gone {
                    if let Some(from) = self.items.get(key).and_then(|i| i.released_from) {
                        self.swipe.released.copies.push(from);
                    }
                }
                self.drop_items(&gone);
            }
            Undo::Restore { item, row, .. } => {
                self.drop_items(&[item]);
                self.swipe.released.cancelled.push(row);
            }
            Undo::Unkeep { .. } | Undo::Keep { .. } => self.evict(),
            Undo::Card { row } => self.swipe.released.cards.push(row),
            Undo::None => {}
        }
    }

    /// Snackbar `open` closed with no Undo taken: say where focus goes if
    /// its Undo held it (§7). Called before [`Self::finish`] drops what the
    /// Undo held, since a removed row's place is found by it.
    fn closed_under(&mut self, open: &Open) {
        // A copy's snackbar held nothing in place: focus stays where the
        // view put it, on the overflow button.
        if matches!(open.undo, Undo::None) {
            return;
        }
        let shown = |key: &ItemKey| self.items.get(key).is_some_and(|i| !i.hidden);
        let about = |key: ItemKey| {
            if shown(&key) {
                return Some(FocusTarget::Item(key));
            }
            open.conversation
                .as_ref()
                .and_then(|c| self.focus_after_leaving(c, key))
        };
        let focus = match &open.undo {
            Undo::Unkeep { item, .. } | Undo::Keep { item } | Undo::Restore { item, .. } => {
                about(*item)
            }
            Undo::Unhide {
                conversation: Some(c),
                ..
            } => {
                if self.conversations().iter().any(|row| &row.key == c) {
                    Some(FocusTarget::Conversation(c.clone()))
                } else {
                    open.sent_focus.clone()
                }
            }
            Undo::Unhide { items, .. } => items.first().and_then(|k| about(*k)),
            // The confirmation row an Accept left, else where the answer
            // sent focus.
            Undo::Card { row } => {
                if self.cards.shows(*row) {
                    Some(FocusTarget::Proposal(*row))
                } else {
                    open.sent_focus.clone()
                }
            }
            Undo::None => None,
        };
        // No row left to name: the conversation's header title, or the
        // list's heading -- never lost.
        let focus = focus.unwrap_or_else(|| {
            open.conversation
                .clone()
                .map_or(FocusTarget::ConversationsHeading, FocusTarget::Header)
        });
        self.swipe.seq += 1;
        self.swipe.undo_closed = Some(UndoClosed {
            id: self.swipe.seq,
            conversation: open.conversation.clone(),
            focus,
        });
    }

    fn unhide(&mut self, items: &[ItemKey]) {
        for key in items {
            if let Some(item) = self.items.get_mut(key) {
                item.hidden = false;
            }
        }
    }

    /// A store row was attached to `key` -- a Keep answered, a copy of the
    /// message received: a message the store holds is never hidden. The
    /// snackbar that hid it closes with nothing dropped, the others it hid
    /// shown again, as an Undo would (review F1 on #289).
    pub(super) fn row_attached(&mut self, key: ItemKey) {
        let was_hidden = self.items.get(&key).is_some_and(|i| i.hidden);
        if let Some(item) = self.items.get_mut(&key) {
            item.hidden = false;
        }
        let hid_it =
            self.swipe.open.as_ref().is_some_and(
                |o| matches!(&o.undo, Undo::Unhide { items, .. } if items.contains(&key)),
            );
        if hid_it && let Some(open) = self.swipe.open.take() {
            if let Undo::Unhide { items, .. } = &open.undo {
                self.unhide(items);
            }
            self.closed_under(&open);
            self.evict();
        } else if was_hidden {
            self.evict();
        }
    }

    /// A message arrived in `conversation`: if the open snackbar hid it,
    /// the arrival wins and nothing is lost -- the conversation comes back
    /// whole, as an Undo would bring it, and the snackbar closes (§10).
    pub(super) fn arrived_in(&mut self, conversation: &ConversationKey) {
        let hid_it = self.swipe.open.as_ref().is_some_and(
            |o| matches!(&o.undo, Undo::Unhide { conversation: Some(c), .. } if c == conversation),
        );
        if hid_it && let Some(open) = self.swipe.open.take() {
            if let Undo::Unhide { items, .. } = &open.undo {
                self.unhide(items);
            }
            self.closed_under(&open);
            self.evict();
        }
    }

    /// Where focus goes when `key` leaves `conversation`'s list: the next
    /// row, else the previous one. Placed by the list's own order, so a
    /// row already hidden -- a refused restore -- still has a place to
    /// leave from. With no row left, the conversation's header title
    /// (§7): never the composer, which would open the phone's keyboard
    /// unasked (seen on the Pixel). `None` only for an item not held.
    fn focus_after_leaving(
        &self,
        conversation: &ConversationKey,
        key: ItemKey,
    ) -> Option<FocusTarget> {
        let place = self.items.get(&key).map(|i| (i.local_at, key))?;
        let shown: Vec<(u64, ItemKey)> = self
            .messages(conversation)
            .iter()
            .map(|m| (m.local_at, m.key))
            .filter(|p| p.1 != key)
            .collect();
        shown
            .iter()
            .find(|p| **p > place)
            .or_else(|| shown.iter().rev().find(|p| **p < place))
            .map_or_else(
                || FocusTarget::Header(conversation.clone()),
                |p| FocusTarget::Item(p.1),
            )
            .into()
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Swipe actions, Hide and the snackbar's Undo, through the model's
//! public surface (`design/2026-10-16-message-swipe.md` §3, §5, §7, §8,
//! §10, and its acceptance item 18).

#![allow(clippy::expect_used, clippy::panic)]

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{
    ClientEvent, Destination, Origin, OutboundStatus, OutboundUpdate,
};
use interweave_human_core::{AppMessageId, RowId};
use interweave_human_ui_model::{
    Announce, Confirmation, ConversationKey, FocusTarget, Intent, ItemKey, ListedInbound, Reply,
    RowAction, SESSION_ITEM_CAP, SwipeOffer, Table, UiModel,
};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn endpoint(name: &str) -> EndpointId {
    EndpointId::parse(name).expect("valid")
}

fn envelope(id: u32, text: &str) -> HumanChatV2 {
    HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: format!("{id:032x}"),
        text: text.to_owned(),
        reply_to: None,
        sent_at_ms: None,
        from_endpoint: None,
    }
}

fn row(n: i64) -> RowId {
    RowId::from_stored(n)
}

fn key(from: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: from.clone(),
        endpoint: Some(endpoint("human")),
    }
}

fn destination(to: &TransportIdentity) -> Destination {
    Destination::Direct {
        peer: to.clone(),
        endpoint: Some(endpoint("human")),
    }
}

/// An unread message from `from` as unread row `n`, shown at `at`.
fn unread(model: &mut UiModel, from: &TransportIdentity, n: i64, at: u64) -> ItemKey {
    model.unread_listed(vec![ListedInbound {
        row: row(n),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: endpoint("human"),
        },
        envelope: envelope(u32::try_from(n).expect("small"), "hello"),
        received_at: at,
    }]);
    item_at(model, &key(from), at)
}

/// The item `conversation` shows at local time `at`.
fn item_at(model: &UiModel, conversation: &ConversationKey, at: u64) -> ItemKey {
    model
        .messages(conversation)
        .iter()
        .find(|m| m.local_at == at)
        .expect("shown")
        .key
}

/// A read, unkept message: kind A.
fn read(model: &mut UiModel, from: &TransportIdentity, n: i64, at: u64) -> ItemKey {
    let item = unread(model, from, n, at);
    model.read(row(n));
    item
}

/// A pending message to `to` as pending row `n`: kind C.
fn pending(model: &mut UiModel, to: &TransportIdentity, n: i64, at: u64) -> ItemKey {
    model.sent(
        row(n),
        &destination(to),
        envelope(1000 + u32::try_from(n).expect("small"), "out"),
        at,
    );
    item_at(model, &key(to), at)
}

fn status(model: &mut UiModel, n: i64, status: OutboundStatus) {
    model.client_event(ClientEvent::Outbound(OutboundUpdate {
        row: row(n),
        app_message_id: AppMessageId::parse(format!("{:032x}", 1000 + n)).expect("id"),
        status,
        last_code: None,
    }));
}

/// The person cancelled pending row `n`, and the facade did it.
fn cancel(model: &mut UiModel, n: i64, restorable: bool) {
    status(
        model,
        n,
        OutboundStatus::Cancelled {
            may_have_reached: false,
        },
    );
    model.cancelled(row(n), restorable);
}

fn shown(model: &UiModel, conversation: &ConversationKey) -> Vec<ItemKey> {
    model.messages(conversation).iter().map(|m| m.key).collect()
}

fn snack_id(model: &UiModel) -> u64 {
    model.snackbar().expect("a snackbar").id
}

// --- which swipes are offered (§3) -----------------------------------------

#[test]
fn the_offer_follows_actions_and_the_hide_rule_for_kinds_a_to_d() {
    let p = peer();
    let mut model = UiModel::new();
    let kind_a = read(&mut model, &p, 1, 1);
    let kind_b = read(&mut model, &p, 2, 2);
    model.kept(kind_b, row(20));
    let kind_c = pending(&mut model, &p, 3, 3);
    let kind_d = pending(&mut model, &p, 4, 4);
    status(
        &mut model,
        4,
        OutboundStatus::Accepted {
            endpoint: endpoint("human"),
        },
    );
    let still_unread = unread(&mut model, &p, 5, 5);

    assert_eq!(
        model.swipe_offer(kind_a),
        SwipeOffer {
            toward_end: Some(RowAction::Keep),
            toward_start: Some(RowAction::Hide),
        },
        "A: Keep one way, Hide the other"
    );
    assert_eq!(
        model.row_actions(kind_a),
        [RowAction::Keep, RowAction::Hide]
    );
    assert_eq!(
        model.swipe_offer(kind_b),
        SwipeOffer {
            toward_end: None,
            toward_start: Some(RowAction::Unkeep),
        },
        "B: Unkeep only"
    );
    assert_eq!(
        model.swipe_offer(kind_c),
        SwipeOffer {
            toward_end: Some(RowAction::Retry),
            toward_start: Some(RowAction::Cancel),
        },
        "C: Retry one way, Cancel the other"
    );
    assert_eq!(
        model.row_actions(kind_c),
        [RowAction::Retry, RowAction::Cancel]
    );
    assert_eq!(
        model.swipe_offer(kind_d),
        SwipeOffer::default(),
        "D: nothing"
    );
    assert!(model.row_actions(kind_d).is_empty());
    assert_eq!(
        model.swipe_offer(still_unread),
        SwipeOffer::default(),
        "an unread row has no action: reading it is the view's"
    );
}

#[test]
fn a_row_whose_copy_is_gone_offers_hide_and_no_keep() {
    let p = peer();
    let mut model = UiModel::new();
    let a = read(&mut model, &p, 1, 1);
    model.copy_gone(a);
    assert!(model.actions(a).is_empty());
    assert_eq!(
        model.swipe_offer(a),
        SwipeOffer {
            toward_end: None,
            toward_start: Some(RowAction::Hide),
        },
        "hiding needs no copy"
    );
}

#[test]
fn a_conversation_is_offered_for_hiding_only_holding_read_unkept_or_finished_messages() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    assert!(!model.conversation_hide_offered(&k), "nothing shown");
    read(&mut model, &p, 1, 1);
    pending(&mut model, &p, 2, 2);
    assert!(!model.conversation_hide_offered(&k), "a pending message");
    status(
        &mut model,
        2,
        OutboundStatus::Accepted {
            endpoint: endpoint("human"),
        },
    );
    assert!(model.conversation_hide_offered(&k), "A and D only");
    assert_eq!(
        model.hide_conversation_intent(&k),
        Some(Intent::HideConversation(k.clone()))
    );
    let u = unread(&mut model, &p, 3, 3);
    assert!(!model.conversation_hide_offered(&k), "an unread message");
    model.read(row(3));
    model.kept(u, row(30));
    assert!(!model.conversation_hide_offered(&k), "a kept message");
    assert_eq!(model.hide_conversation_intent(&k), None);
}

// --- commit raises the intent at once (§5, §8) -----------------------------

#[test]
fn unkeep_and_cancel_raise_their_intent_at_commit_and_closing_raises_none() {
    let p = peer();
    let mut model = UiModel::new();
    let b = read(&mut model, &p, 1, 1);
    model.kept(b, row(10));
    let c = pending(&mut model, &p, 2, 2);

    assert_eq!(
        model.intent_for(b, RowAction::Unkeep),
        Some(Intent::Unkeep(row(10)))
    );
    assert_eq!(
        model.intent_for(c, RowAction::Cancel),
        Some(Intent::Cancel(row(2)))
    );

    // The facade carried both out; closing their snackbars raises
    // nothing -- there is nothing left to raise -- and lets go only of
    // what the Undo needed.
    model.unkept(row(10));
    let id = snack_id(&model);
    model.snackbar_closed(id);
    assert!(model.take_released().is_empty());
    cancel(&mut model, 2, true);
    let id = snack_id(&model);
    model.snackbar_closed(id);
    let released = model.take_released();
    assert_eq!(released.cancelled, Some(row(2)));
    assert!(released.copies.is_empty());
}

#[test]
fn a_swipe_is_rechecked_at_commit() {
    let p = peer();
    let mut model = UiModel::new();
    let c = pending(&mut model, &p, 1, 1);
    assert!(model.swipe_offer(c).toward_start.is_some());
    // Accepted under the finger: the row springs back, nothing is raised.
    status(
        &mut model,
        1,
        OutboundStatus::Accepted {
            endpoint: endpoint("human"),
        },
    );
    assert_eq!(model.intent_for(c, RowAction::Cancel), None);
    assert_eq!(model.intent_for(c, RowAction::Retry), None);
    assert_eq!(model.intent_for(c, RowAction::Hide), None);
}

// --- each Undo is the reverse action (§5, item 18) -------------------------

#[test]
fn the_undo_of_keep_raises_unkeep_and_its_result_is_a_restore_not_a_snackbar() {
    let p = peer();
    let mut model = UiModel::new();
    let a = read(&mut model, &p, 1, 1);
    let keep = model.intent_for(a, RowAction::Keep).expect("offered");
    assert_eq!(
        keep,
        Intent::Keep {
            item: a,
            from: (Table::Unread, row(1))
        }
    );
    model.kept(a, row(10));
    let snack = model.snackbar().expect("Kept is confirmed");
    assert_eq!(snack.confirms, Confirmation::Kept);
    assert!(snack.undo);
    assert_eq!(
        model.outcome().and_then(|o| o.announce),
        Some(Announce::Confirmed(Confirmation::Kept))
    );

    assert_eq!(model.undo(snack.id), Some(Intent::Unkeep(row(10))));
    assert!(model.snackbar().is_none(), "one Undo per snackbar");
    assert_eq!(
        model.undo(snack.id),
        None,
        "two undos in a row are not possible"
    );
    model.unkept(row(10));
    assert!(model.snackbar().is_none(), "an Undo's result opens none");
    let outcome = model.outcome().expect("outcome");
    assert_eq!(outcome.announce, Some(Announce::Restored));
    assert_eq!(outcome.focus, Some(FocusTarget::Item(a)));
}

#[test]
fn the_undo_of_unkeep_raises_keep_from_the_released_kept_row() {
    let p = peer();
    let mut model = UiModel::new();
    let b = read(&mut model, &p, 1, 1);
    model.kept(b, row(10));
    let first = snack_id(&model);
    model.snackbar_closed(first);
    model.unkept(row(10));
    let snack = model.snackbar().expect("Unkeep is confirmed");
    assert_eq!(snack.confirms, Confirmation::Unkept);
    assert!(snack.undo);
    assert_eq!(
        model.undo(snack.id),
        Some(Intent::Keep {
            item: b,
            from: (Table::Kept, row(10))
        })
    );
    model.kept(b, row(11));
    assert!(model.snackbar().is_none());
    assert_eq!(
        model.outcome().and_then(|o| o.announce),
        Some(Announce::Restored)
    );
}

#[test]
fn with_its_copy_gone_the_unkeep_snackbar_offers_no_undo() {
    let p = peer();
    let mut model = UiModel::new();
    let b = read(&mut model, &p, 1, 1);
    model.kept(b, row(10));
    model.unkept(row(10));
    model.copy_gone(b);
    let snack = model.snackbar().expect("still confirmed");
    assert_eq!(snack.confirms, Confirmation::Unkept);
    assert!(!snack.undo, "never an Undo that would fail");
    assert_eq!(model.undo(snack.id), None);
}

#[test]
fn the_undo_of_cancel_raises_restore_and_brings_the_row_back_in_its_place() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    let before = read(&mut model, &p, 1, 1);
    let c = pending(&mut model, &p, 2, 2);
    let after = read(&mut model, &p, 3, 3);
    assert_eq!(shown(&model, &k), [before, c, after]);

    cancel(&mut model, 2, true);
    assert_eq!(shown(&model, &k), [before, after], "the row leaves");
    let snack = model.snackbar().expect("Cancel is confirmed");
    assert_eq!(
        snack.confirms,
        Confirmation::Cancelled {
            may_have_reached: false
        }
    );
    assert_eq!(
        model.outcome().and_then(|o| o.focus.clone()),
        Some(FocusTarget::Item(after)),
        "focus to the next row"
    );
    assert_eq!(model.undo(snack.id), Some(Intent::Restore(row(2))));
    model.restored(row(2), row(9));
    assert_eq!(shown(&model, &k), [before, c, after], "in its old place");
    assert_eq!(
        model.actions(c),
        [Intent::Retry(row(9)), Intent::Cancel(row(9))],
        "pending again, as the restored row"
    );
    let outcome = model.outcome().expect("outcome");
    assert_eq!(outcome.announce, Some(Announce::Restored));
    assert_eq!(outcome.focus, Some(FocusTarget::Item(c)));

    // And it can be cancelled again, under the row it now holds.
    cancel(&mut model, 9, true);
    assert_eq!(shown(&model, &k), [before, after]);
}

#[test]
fn a_cancel_says_may_have_been_received_in_the_rows_own_words() {
    let p = peer();
    let mut model = UiModel::new();
    pending(&mut model, &p, 1, 1);
    status(
        &mut model,
        1,
        OutboundStatus::Cancelled {
            may_have_reached: true,
        },
    );
    model.cancelled(row(1), true);
    assert_eq!(
        model.snackbar().expect("confirmed").confirms,
        Confirmation::Cancelled {
            may_have_reached: true
        }
    );
}

#[test]
fn a_cancel_with_no_record_to_restore_offers_no_undo() {
    let p = peer();
    let mut model = UiModel::new();
    pending(&mut model, &p, 1, 1);
    cancel(&mut model, 1, false);
    let snack = model.snackbar().expect("still confirmed");
    assert!(!snack.undo);
    assert_eq!(model.undo(snack.id), None);
}

#[test]
fn a_refused_restore_leaves_the_message_cancelled_and_focus_on_the_next_row() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    let c = pending(&mut model, &p, 1, 1);
    let after = read(&mut model, &p, 2, 2);
    cancel(&mut model, 1, true);
    let id = snack_id(&model);
    let restore = model.undo(id).expect("offered");
    model.intent_refused(&restore);
    assert_eq!(shown(&model, &k), [after], "still cancelled");
    assert!(model.actions(c).is_empty(), "and gone from the session");
    assert_eq!(
        model.outcome().and_then(|o| o.focus.clone()),
        Some(FocusTarget::Item(after))
    );
    assert_eq!(model.take_released().cancelled, Some(row(1)));
}

#[test]
fn a_refused_undo_of_unkeep_leaves_focus_on_that_row() {
    let p = peer();
    let mut model = UiModel::new();
    let b = read(&mut model, &p, 1, 1);
    model.kept(b, row(10));
    model.unkept(row(10));
    let id = snack_id(&model);
    let keep = model.undo(id).expect("offered");
    model.intent_refused(&keep);
    assert!(model.snackbar().is_none());
    let outcome = model.outcome().expect("outcome");
    assert_eq!(outcome.announce, None, "the refusal path says it");
    assert_eq!(outcome.focus, Some(FocusTarget::Item(b)));
    assert_eq!(model.swipe_offer(b).toward_end, Some(RowAction::Keep));
}

// --- Hide for now (§5, §8) ------------------------------------------------

#[test]
fn the_undo_of_hide_raises_nothing_and_puts_the_row_back_in_its_place() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    let first = read(&mut model, &p, 1, 1);
    let a = read(&mut model, &p, 2, 2);
    let last = read(&mut model, &p, 3, 3);
    assert_eq!(model.intent_for(a, RowAction::Hide), Some(Intent::Hide(a)));
    model.hide(a);
    assert_eq!(shown(&model, &k), [first, last]);
    let snack = model.snackbar().expect("Hide is confirmed");
    assert_eq!(snack.confirms, Confirmation::Hidden);
    assert_eq!(model.undo(snack.id), None, "a Hide wrote nothing to undo");
    assert_eq!(shown(&model, &k), [first, a, last]);
    assert_eq!(
        model.outcome().and_then(|o| o.focus.clone()),
        Some(FocusTarget::Item(a))
    );
    assert!(model.take_released().is_empty());
}

#[test]
fn a_hidden_row_leaves_the_session_when_its_snackbar_closes() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    let earlier = read(&mut model, &p, 1, 1);
    let a = read(&mut model, &p, 2, 2);
    model.hide(a);
    assert_eq!(
        model.outcome().and_then(|o| o.focus.clone()),
        Some(FocusTarget::Item(earlier)),
        "the last row's focus goes to the previous one"
    );
    let id = snack_id(&model);
    model.snackbar_closed(id);
    assert_eq!(shown(&model, &k), [earlier]);
    assert!(model.actions(a).is_empty(), "it cannot be kept any more");
    assert_eq!(model.swipe_offer(a), SwipeOffer::default());
    assert_eq!(
        model.take_released().copies,
        [(Table::Unread, row(2))],
        "the facade side lets go of the copy Keep would have used"
    );

    // A reply to it reads as one to a message not held here.
    model.unread_listed(vec![ListedInbound {
        row: row(5),
        origin: Origin::Direct {
            peer: p.clone(),
            endpoint: endpoint("human"),
        },
        envelope: HumanChatV2 {
            reply_to: Some(format!("{:032x}", 2)),
            ..envelope(5, "re")
        },
        received_at: 5,
    }]);
    let reply = model
        .messages(&k)
        .into_iter()
        .find(|m| m.local_at == 5)
        .and_then(|m| m.reply);
    assert!(matches!(reply, Some(Reply::Unavailable(_))), "{reply:?}");
}

#[test]
fn a_reply_to_a_hidden_row_is_unavailable_while_its_snackbar_is_up() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    let a = read(&mut model, &p, 2, 2);
    model.hide(a);
    model.unread_listed(vec![ListedInbound {
        row: row(5),
        origin: Origin::Direct {
            peer: p.clone(),
            endpoint: endpoint("human"),
        },
        envelope: HumanChatV2 {
            reply_to: Some(format!("{:032x}", 2)),
            ..envelope(5, "re")
        },
        received_at: 5,
    }]);
    let reply = model.messages(&k).into_iter().find_map(|m| m.reply);
    assert!(matches!(reply, Some(Reply::Unavailable(_))), "{reply:?}");
}

#[test]
fn a_new_snackbar_replaces_the_old_and_its_undo_goes_with_it() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    let a = read(&mut model, &p, 1, 1);
    let other = read(&mut model, &p, 2, 2);
    model.hide(a);
    let old = snack_id(&model);
    model.hide(other);
    assert_ne!(snack_id(&model), old);
    assert_eq!(model.undo(old), None, "the replaced Undo is gone");
    assert!(shown(&model, &k).is_empty());
    assert!(model.actions(a).is_empty(), "the first hide is final");
    assert_eq!(model.take_released().copies, [(Table::Unread, row(1))]);
}

#[test]
fn an_open_snackbar_keeps_its_row_from_the_session_cap() {
    let p = peer();
    let mut model = UiModel::new();
    let a = read(&mut model, &p, 1, 1);
    model.hide(a);
    let id = snack_id(&model);
    // Past the cap, the oldest session-only item is evicted -- but not the
    // one the open Undo needs.
    for n in 0..=i64::try_from(SESSION_ITEM_CAP).expect("small") {
        read(&mut model, &p, 100 + n, 100);
    }
    assert!(model.snackbar().expect("open").undo);
    assert_eq!(model.undo(id), None);
    assert!(
        shown(&model, &key(&p)).contains(&a),
        "the Undo brought it back"
    );
}

// --- the conversation hide (§10) ------------------------------------------

#[test]
fn hiding_a_conversation_and_its_undo() {
    let p = peer();
    let q = peer();
    let mut model = UiModel::new();
    read(&mut model, &q, 1, 10);
    read(&mut model, &p, 2, 5);
    read(&mut model, &p, 3, 6);
    let list: Vec<_> = model.conversations().into_iter().map(|c| c.key).collect();
    assert_eq!(list, [key(&q), key(&p)]);

    model.hide_conversation(&key(&q));
    assert_eq!(
        model
            .conversations()
            .into_iter()
            .map(|c| c.key)
            .collect::<Vec<_>>(),
        [key(&p)]
    );
    let snack = model.snackbar().expect("confirmed");
    assert_eq!(snack.confirms, Confirmation::ConversationHidden);
    assert_eq!(snack.conversation, None, "shown on the list");
    assert_eq!(
        model.outcome().and_then(|o| o.focus.clone()),
        Some(FocusTarget::Conversation(key(&p)))
    );
    assert_eq!(model.undo(snack.id), None);
    assert_eq!(
        model
            .conversations()
            .into_iter()
            .map(|c| c.key)
            .collect::<Vec<_>>(),
        [key(&q), key(&p)]
    );
    assert_eq!(
        model.outcome().and_then(|o| o.focus.clone()),
        Some(FocusTarget::Conversation(key(&q)))
    );
}

#[test]
fn a_message_into_a_conversation_hidden_under_an_open_snackbar_brings_it_all_back() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    let first = read(&mut model, &p, 1, 1);
    model.hide_conversation(&k);
    let arrived = unread(&mut model, &p, 2, 2);
    assert_eq!(shown(&model, &k), [first, arrived], "nothing lost");
    assert!(
        model.snackbar().is_none(),
        "the arrival closes the snackbar"
    );
    assert!(model.take_released().is_empty());
}

#[test]
fn a_message_into_a_conversation_hidden_earlier_shows_only_that_message() {
    let p = peer();
    let mut model = UiModel::new();
    let k = key(&p);
    read(&mut model, &p, 1, 1);
    model.hide_conversation(&k);
    let id = snack_id(&model);
    model.snackbar_closed(id);
    assert!(model.conversations().is_empty());
    let arrived = unread(&mut model, &p, 2, 2);
    assert_eq!(shown(&model, &k), [arrived]);
}

#[test]
fn a_refused_cancel_is_an_outcome_that_moves_nothing() {
    let p = peer();
    let mut model = UiModel::new();
    let c = pending(&mut model, &p, 1, 1);
    let before = model.outcome().map(|o| o.id);
    let cancel = model.intent_for(c, RowAction::Cancel).expect("offered");
    model.intent_refused(&cancel);
    let outcome = model.outcome().expect("an outcome");
    assert_ne!(
        Some(outcome.id),
        before,
        "a new one, for the view to act on"
    );
    assert_eq!(outcome.announce, None, "the refusal path says it");
    assert_eq!(outcome.focus, None, "focus stays");
    assert!(model.snackbar().is_none());
    assert_eq!(shown(&model, &key(&p)), [c], "still pending, in place");
}

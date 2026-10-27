// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The verbs that go through the session: each one scripts what a person
//! would do in the window -- view a conversation, type and press Send,
//! press Cancel on a row, confirm a trust change -- and the model resolves
//! it into the commands the facade carries out. Nothing here calls the
//! facade or the store: what a verb can do is what the model offers.

use std::fmt::Write as _;
use std::future::Future;
use std::time::Instant;

use interweave_human_app_core::{Command, Failure as Why, Problem, Update};
use interweave_human_client_api::{ClientEvent, OutboundStatus, SessionState, TrustProblem};
use interweave_human_ui_model::{
    ConversationKey, Direction, EntryProblem, Intent, ItemKey, ItemStatus, ProposalItem, RowId,
    TrustInput, TrustOutcome, ViewEvent,
};
use interweave_local_client_api::{AdminBinding, DataSessionBinding};
use interweave_transport_api::{EndpointId, TransportIdentity};
use serde_json::json;

use crate::Failure;
use crate::cli::{Verb, conversation_text};
use crate::output;
use crate::session::{Opening, STEP, Session};

/// What every verb is given.
pub(crate) struct Run<'a> {
    /// The profile's name, for what is said about it.
    pub(crate) profile: &'a str,
    /// Print JSON lines.
    pub(crate) json: bool,
    /// When waiting ends.
    pub(crate) until: Instant,
}

/// Carry out `verb`, writing what it prints to `out`. `listen` is not
/// one of these: it prints as it goes ([`listen`]).
///
/// # Errors
/// The verb's [`Failure`]; `out` then holds what a script still needs,
/// the id of a message that did not finish.
pub(crate) async fn run<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    verb: &Verb,
    r: &Run<'_>,
    out: &mut String,
) -> Result<(), Failure> {
    match verb {
        Verb::Status => status(s, r, out).await,
        Verb::Conversations => {
            look(s, r).await;
            for c in s.model().conversations() {
                out.push_str(&output::conversation(&c, r.json));
            }
            Ok(())
        }
        Verb::Messages(key) => {
            look(s, r).await;
            print_messages(s, key, r.json, out);
            Ok(())
        }
        Verb::Read { conversation, keep } => read(s, r, conversation, keep, out).await,
        Verb::Send {
            conversation,
            text: Some(text),
            pending,
        } => send(s, r, conversation, text, *pending, out).await,
        Verb::Cancel(id) => row_action(s, r, RowVerb::Cancel, id, out).await,
        Verb::Retry(id) => row_action(s, r, RowVerb::Retry, id, out).await,
        Verb::Unkeep(id) => row_action(s, r, RowVerb::Unkeep, id, out).await,
        Verb::TrustList => trust_list(s, r, out).await,
        Verb::SetTrust(peer, allowed) => set_trust(s, r, peer, *allowed, out).await,
        Verb::CardShow => crate::cards::card_show(s, r, out).await,
        // Main reads another endpoint's key file; the identity in front,
        // named or not, is the model's.
        Verb::IdentityShow(_) => crate::identity::show(s, r, out).await,
        Verb::IdentityNewKey { confirm } => crate::identity::new_key(s, r, *confirm, out).await,
        Verb::CardSet {
            name,
            surname,
            photo,
        } => crate::cards::card_set(s, name, surname.as_deref(), photo, out).await,
        Verb::CardShare(key) => crate::cards::card_share(s, r, key, out).await,
        Verb::Proposals => crate::cards::proposals(s, r, out).await,
        Verb::ProposalsRead {
            conversation,
            accept,
            accept_key_change,
            decline,
        } => {
            let decided = crate::cards::Decided {
                accept,
                accept_key_change,
                decline,
            };
            crate::cards::proposals_read(s, r, conversation, &decided, out).await
        }
        Verb::Contacts => crate::cards::contacts(s, r, out).await,
        Verb::ContactShow(id) => crate::cards::contact_show(s, r, id, out).await,
        Verb::ContactRename(id, name) => {
            crate::cards::contact_edit(s, id, Some(name), None, out).await
        }
        Verb::ContactDelete(id) => crate::cards::contact_delete(s, id, out).await,
        Verb::RouteDelete {
            contact,
            peer,
            endpoint,
        } => crate::cards::route_delete(s, contact, peer, endpoint, out).await,
        Verb::ContactPhoto(id, photo) => {
            crate::cards::contact_edit(s, id, None, Some(photo), out).await
        }
        Verb::Send { text: None, .. }
        | Verb::Listen { .. }
        | Verb::PeersList
        | Verb::PeerAdd(_)
        | Verb::PeerAddList(_)
        | Verb::PeerRemove(_)
        | Verb::IdentitiesList
        | Verb::SetEnabled(..)
        | Verb::SetDefault(_)
        | Verb::StoreHealth
        | Verb::StoreDiscard(_)
        | Verb::PeersShow => Err(Failure::Refused(
            "this verb does not run through the session".to_owned(),
        )),
    }
}

/// Open the session for a verb that reads: what the store holds is shown
/// whether or not it opens, and what the daemon held for this client is
/// handed over first when it does. Why it did not open goes to stderr, for
/// a person; a script reads `status`.
pub(crate) async fn look<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
) {
    let opening = s.open(r.until).await;
    if opening == Opening::Ready {
        s.settle(r.until).await;
    } else if let Some(failure) = opening.failure(r.profile) {
        eprintln!(
            "interweave-human: showing what the message store holds: {}",
            failure.message()
        );
    }
}

/// Open the session for a verb that needs it, or fail as it did.
pub(crate) async fn ready<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
) -> Result<(), Failure> {
    s.open(r.until).await.failure(r.profile).map_or(Ok(()), Err)
}

fn print_messages<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
    key: &ConversationKey,
    json: bool,
    out: &mut String,
) {
    for m in s.model().messages(key) {
        out.push_str(&output::message(&m, json));
    }
}

/// A command the model side reports as not carried out, as a failure.
pub(crate) fn problems<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
) -> Result<(), Failure> {
    s.side()
        .take_problems()
        .into_iter()
        .next()
        .map_or(Ok(()), |problem| Err(problem_failure(problem)))
}

/// A problem the model side reported, as the failure it is said as.
fn problem_failure(problem: Problem) -> Failure {
    let (what, why) = match problem {
        Problem::Command { command, why } => (command_name(&command), why),
        Problem::UnreadNotListed(why) => ("listing the unread messages", why),
        Problem::CardsNotRead(why) => ("reading the cards and contacts", why),
        Problem::ShareNotRecorded(why) => ("recording a card's share", why),
    };
    match why {
        Why::StorageUnavailable => {
            Failure::Store(format!("{what}: the message store cannot record it now"))
        }
        Why::CopyGone => Failure::Refused(format!("{what}: its content is no longer held")),
        Why::NoSuchRow => Failure::Refused(format!("{what}: no such message any more")),
        Why::Refused => Failure::Refused(format!("{what}: the message store refused it")),
        Why::Corrupt => Failure::Store(format!("{what}: a stored row could not be read back")),
        Why::TooMany => Failure::Refused(format!("{what}: too many on their way")),
        Why::NotACard => Failure::Refused(format!("{what}: no card this build verifies")),
        Why::NoIdentityKey => Failure::Refused(format!("{what}: the identity has no key")),
        Why::CardNotSent => Failure::Refused(format!("{what}: the card could not be sent")),
        Why::OutOfBounds => Failure::Refused(format!("{what}: outside a card's bounds")),
        Why::ProposalChanged => {
            Failure::Refused(format!("{what}: the proposal changed since it was shown"))
        }
        Why::ServiceStopped => Failure::Refused(format!("{what}: the service was not running")),
    }
}

const fn command_name(command: &Command) -> &'static str {
    match command {
        Command::Send { .. } => "send",
        Command::MarkRead(_) => "marking read",
        Command::Keep { .. } => "keep",
        Command::Unkeep(_) => "unkeep",
        Command::Retry(_) => "retry",
        Command::Cancel(_) => "cancel",
        Command::Restore(_) => "restore",
        Command::ForgetCancelled(_)
        | Command::ForgetCopy(_)
        | Command::ForgetCard(_)
        | Command::ForgetContact(_)
        | Command::ForgetOwnPhoto => "releasing a held copy",
        Command::PrepareOwnPhoto { .. } => "preparing the card's photo",
        Command::RemoveOwnPhoto => "removing the card's photo",
        Command::Reopen => "reopen",
        Command::RecheckStorage => "re-checking storage",
        Command::ReadTrust => "reading trust",
        Command::SetTrust(_) => "changing trust",
        Command::ReadAddresses => "reading the addresses",
        Command::ReadOwnAddresses => "reading this device's addresses",
        Command::AddAddresses(_) => "adding addresses",
        Command::RemoveAddress(_) => "removing an address",
        Command::AcceptCard { .. } => "accepting a card",
        Command::DeclineCard(_) => "declining a card",
        Command::UndoCard(_) => "undoing a card's answer",
        Command::AskNewKey => "asking what a new key cancels",
        Command::MakeNewKey => "making a new key",
        Command::RetryKey => "opening the key again",
        Command::ShareCard(_) => "sharing the card",
        Command::SetOwnCard { .. } => "editing the own card",
        Command::RenameContact { .. } => "renaming a contact",
        Command::DeleteContact(_) => "deleting a contact",
        Command::UndoContact { .. } => "undoing a contact's change",
        Command::DeleteRoute { .. } => "deleting a route",
        Command::SetContactPhoto { .. } => "setting a contact's photo",
    }
}

async fn status<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    out: &mut String,
) -> Result<(), Failure> {
    if s.open(r.until).await == Opening::Ready {
        s.settle(r.until).await;
    }
    let model = s.model();
    let no_daemon =
        model.session_notice() == Some(interweave_human_ui_model::SessionNotice::NoDaemon);
    let session = output::session(s.state(), no_daemon);
    let endpoint = match s.state() {
        Some(SessionState::Ready {
            endpoint: Some(endpoint),
        }) => Some(endpoint.as_str().to_owned()),
        _ => None,
    };
    let problem = match s.state() {
        Some(SessionState::Refused { problem }) => Some(output::session_problem(*problem)),
        _ => None,
    };
    let connectivity = output::connectivity(model.connectivity());
    let (mut unread, mut pending, mut kept) = (0, 0, 0);
    for c in model.conversations() {
        for m in model.messages(&c.key) {
            match output::status(&m.status) {
                "unread" => unread += 1,
                "sending" | "not-confirmed" | "needs-attention" => pending += 1,
                _ => {}
            }
            kept += usize::from(m.kept);
        }
    }
    let hidden = s.side().hidden_rows();
    if r.json {
        out.push_str(&output::json_line(&json!({
            "session": session,
            "problem": problem,
            "endpoint": endpoint,
            "connectivity": connectivity,
            "unread": unread,
            "pending": pending,
            "kept": kept,
            "undisplayable": hidden,
        })));
        return Ok(());
    }
    let _ = write!(out, "session       {session}");
    if let Some(endpoint) = &endpoint {
        let _ = write!(out, " (endpoint {endpoint})");
    }
    if let Some(problem) = problem {
        let _ = write!(out, " ({problem})");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "connectivity  {connectivity}");
    let _ = writeln!(out, "unread        {unread}");
    let _ = writeln!(out, "pending       {pending}");
    let _ = writeln!(out, "kept          {kept}");
    if hidden > 0 {
        let _ = writeln!(out, "undisplayable {hidden}");
    }
    Ok(())
}

async fn read<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    key: &ConversationKey,
    keep: &[String],
    out: &mut String,
) -> Result<(), Failure> {
    look(s, r).await;
    // Every --keep names a message received in this conversation, checked
    // before anything is marked read: a mistyped id must not cost the read
    // (`a_keep_naming_an_outbound_message_refuses_before_anything_is_read`).
    let shown = s.model().messages(key);
    let mut to_keep: Vec<ItemKey> = Vec::new();
    for (n, id) in keep.iter().enumerate() {
        let item = s
            .model()
            .item_named(id)
            .filter(|k| {
                shown
                    .iter()
                    .any(|m| m.key == *k && m.direction == Direction::Inbound)
            })
            .ok_or_else(|| {
                Failure::Refused(format!(
                    "--keep number {}: no message received in this conversation has that id",
                    n + 1
                ))
            })?;
        to_keep.push(item);
    }
    let viewed = key.clone();
    s.will(move |m| {
        m.conversation_viewed(&viewed, true)
            .into_iter()
            .map(ViewEvent::Intent)
            .collect()
    });
    s.step().await;
    // From here a message may be read and not kept, which this run alone
    // holds (RETENTION.md): whatever fails after, what was read is printed
    // first, or the person never sees it
    // (`a_keep_that_fails_after_the_read_still_prints_what_was_read`).
    let kept = keep_after_read(s, &to_keep).await;
    print_messages(s, key, r.json, out);
    kept
}

/// Keep `to_keep`, read in this run: a message kept already is what was
/// asked.
async fn keep_after_read<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    to_keep: &[ItemKey],
) -> Result<(), Failure> {
    problems(s)?;
    let mut intents = Vec::new();
    for (n, item) in to_keep.iter().enumerate() {
        let actions = s.model().actions(*item);
        if let Some(keep) = actions.iter().find(|i| matches!(i, Intent::Keep { .. })) {
            intents.push(keep.clone());
        } else if !actions.iter().any(|i| matches!(i, Intent::Unkeep(_))) {
            return Err(Failure::Refused(format!(
                "--keep number {}: that message cannot be kept",
                n + 1
            )));
        }
    }
    if !intents.is_empty() {
        s.will(move |_| intents.into_iter().map(ViewEvent::Intent).collect());
        s.step().await;
        problems(s)?;
    }
    Ok(())
}

async fn send<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    key: &ConversationKey,
    text: &str,
    pending: bool,
    out: &mut String,
) -> Result<(), Failure> {
    if text.is_empty() {
        return Err(Failure::Refused("an empty message is not sent".to_owned()));
    }
    ready(s, r).await?;
    let (typed, pressed, draft) = (key.clone(), key.clone(), text.to_owned());
    s.will(move |_| vec![ViewEvent::DraftChanged { key: typed, draft }]);
    s.will(move |m| {
        m.send_draft(&pressed)
            .map(ViewEvent::Intent)
            .into_iter()
            .collect()
    });
    s.step().await;
    problems(s)?;
    let mut row = None;
    for update in s.take_updates() {
        match update {
            Update::SendRefused { error, .. } => {
                return Err(Failure::Refused(format!(
                    "the message was not sent: {}",
                    output::send_error(&error)
                )));
            }
            Update::Sent { row: sent, .. } => row = Some(sent),
            _ => {}
        }
    }
    let row = row.ok_or_else(|| Failure::Refused("the message was not sent".to_owned()))?;
    let item = s
        .model()
        .item_named(&format!("p{}", row.get()))
        .ok_or_else(|| Failure::Refused("the message sent is not shown".to_owned()))?;
    until_done(s, r, key, item, pending, out).await
}

/// Turn until `item`, an outbound message, is accepted or published, needs
/// the person, or `--wait` ends: its id and status are printed whichever.
async fn until_done<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    key: &ConversationKey,
    item: ItemKey,
    pending: bool,
    out: &mut String,
) -> Result<(), Failure> {
    loop {
        let status = s
            .model()
            .messages(key)
            .into_iter()
            .find(|m| m.key == item)
            .map(|m| m.status);
        let Some(status) = status else {
            return Err(Failure::Refused(format!("{item} is no longer shown")));
        };
        let token = output::status(&status);
        let line = |out: &mut String| {
            if r.json {
                out.push_str(&output::json_line(
                    &json!({ "id": item.to_string(), "status": token }),
                ));
            } else {
                let _ = writeln!(out, "{item} {token}");
            }
        };
        match &status {
            ItemStatus::Outbound(OutboundStatus::Accepted { .. } | OutboundStatus::Published) => {
                line(out);
                return Ok(());
            }
            // `--no-pending` (G4): a send that will not be accepted by the
            // end of `--wait` is cancelled -- RETENTION.md's explicit cancel,
            // with no Undo -- so no later run sends it. One held for
            // attention is among them: the next run's facade attempts every
            // pending row again, attention or not.
            ItemStatus::Outbound(OutboundStatus::NeedsAttention { .. }) if !pending => {
                return cancel_unaccepted(s, r, key, item, out).await;
            }
            ItemStatus::Outbound(OutboundStatus::NeedsAttention { problem, .. }) => {
                line(out);
                return Err(Failure::Refused(format!(
                    "{item} needs attention ({}): retry or cancel it",
                    output::send_problem(*problem)
                )));
            }
            ItemStatus::Outbound(OutboundStatus::Cancelled { .. }) => {
                line(out);
                return Err(Failure::Refused(format!("{item} was cancelled")));
            }
            _ if Instant::now() >= r.until && !pending => {
                return cancel_unaccepted(s, r, key, item, out).await;
            }
            _ if Instant::now() >= r.until => {
                line(out);
                return Err(Failure::Pending(format!(
                    "{item} is still on its way: it stays pending in the message store, and \
                     the next client to open the session sends it again"
                )));
            }
            _ => {}
        }
        s.step().await;
        tokio::time::sleep(STEP).await;
    }
}

/// `--no-pending`'s end: `item`, still on its way, is cancelled. One the
/// transport accepted while the cancel was asked is said as accepted; one
/// the cancel did not reach is said as it stands, pending.
async fn cancel_unaccepted<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    key: &ConversationKey,
    item: ItemKey,
    out: &mut String,
) -> Result<(), Failure> {
    // The cancel carried out is the row gone from the store (the `cancel`
    // verb's rule); its `Cancelled` event may follow a step later.
    let mut cancelled = false;
    if let Some(cancel) = s
        .model()
        .actions(item)
        .into_iter()
        .find(|i| matches!(i, Intent::Cancel(_)))
    {
        s.will(move |_| vec![ViewEvent::Intent(cancel)]);
        s.step().await;
        problems(s)?;
        cancelled = true;
    }
    let status = s
        .model()
        .messages(key)
        .into_iter()
        .find(|m| m.key == item)
        .map(|m| m.status);
    let (token, pending) = match &status {
        Some(ItemStatus::Outbound(OutboundStatus::Accepted { .. } | OutboundStatus::Published)) => {
            ("accepted", false)
        }
        Some(ItemStatus::Outbound(OutboundStatus::Cancelled { .. })) => ("cancelled", false),
        _ if cancelled => ("cancelled", false),
        Some(other) => (output::status(other), true),
        None => ("unknown", true),
    };
    if r.json {
        out.push_str(&output::json_line(
            &json!({ "id": item.to_string(), "status": token, "pending": pending }),
        ));
    } else {
        let _ = writeln!(out, "{item} {token}");
    }
    match (token, pending) {
        ("accepted", _) => Ok(()),
        (_, false) => Err(Failure::Refused(format!(
            "{item} was not accepted within --wait and is cancelled: no later run sends it"
        ))),
        // Never said as cancelled while it is not: a later run sends it.
        (_, true) => Err(Failure::Pending(format!(
            "{item} could not be cancelled: it stays pending in the message store, and the \
             next client to open the session sends it again"
        ))),
    }
}

#[derive(Clone, Copy)]
enum RowVerb {
    Cancel,
    Retry,
    Unkeep,
}

async fn row_action<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    verb: RowVerb,
    id: &str,
    out: &mut String,
) -> Result<(), Failure> {
    if matches!(verb, RowVerb::Retry) {
        ready(s, r).await?;
    }
    let item = s
        .model()
        .item_named(id)
        .ok_or_else(|| Failure::Refused("no message held has that id".to_owned()))?;
    let offered = s.model().actions(item).into_iter().find(|i| {
        matches!(
            (verb, i),
            (RowVerb::Cancel, Intent::Cancel(_))
                | (RowVerb::Retry, Intent::Retry(_))
                | (RowVerb::Unkeep, Intent::Unkeep(_))
        )
    });
    // Opening the session attempts every pending message: one it sent
    // before a retry was asked is what the retry was for.
    if offered.is_none()
        && matches!(verb, RowVerb::Retry)
        && let Some(key) = conversation_of(s, item)
        && s.model().messages(&key).iter().any(|m| {
            m.key == item
                && matches!(
                    m.status,
                    ItemStatus::Outbound(
                        OutboundStatus::Accepted { .. } | OutboundStatus::Published
                    )
                )
        })
    {
        return until_done(s, r, &key, item, true, out).await;
    }
    let intent = offered.ok_or_else(|| {
        Failure::Refused(format!(
            "{item} cannot be {} now",
            match verb {
                RowVerb::Cancel => "cancelled",
                RowVerb::Retry => "retried",
                RowVerb::Unkeep => "unkept",
            }
        ))
    })?;
    s.will(move |_| vec![ViewEvent::Intent(intent)]);
    s.step().await;
    problems(s)?;
    let done = match verb {
        RowVerb::Retry => {
            let key = conversation_of(s, item)
                .ok_or_else(|| Failure::Refused(format!("{item} is no longer shown")))?;
            return until_done(s, r, &key, item, true, out).await;
        }
        RowVerb::Cancel => "cancelled",
        RowVerb::Unkeep => "unkept",
    };
    if r.json {
        out.push_str(&output::json_line(
            &json!({ "id": item.to_string(), "status": done }),
        ));
    } else {
        let _ = writeln!(out, "{item} {done}");
    }
    Ok(())
}

/// The conversation `item` is shown in.
fn conversation_of<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
    item: ItemKey,
) -> Option<ConversationKey> {
    s.model()
        .conversations()
        .into_iter()
        .map(|c| c.key)
        .find(|k| s.model().messages(k).iter().any(|m| m.key == item))
}

fn trust_failure(problem: TrustProblem) -> Failure {
    match problem {
        TrustProblem::Unavailable => {
            Failure::Unreachable("the daemon's admin socket did not answer".to_owned())
        }
        TrustProblem::NotPermitted => {
            Failure::Refused("this client may not administer trust on this daemon".to_owned())
        }
        TrustProblem::Refused => Failure::Refused("the daemon refused the change".to_owned()),
        TrustProblem::Incompatible => Failure::Refused(
            "the daemon and this client share no version of the trust methods: \
             is the daemon another release?"
                .to_owned(),
        ),
        TrustProblem::Internal => Failure::Refused("the daemon failed to answer".to_owned()),
    }
}

/// Turn until the trust settings wait for nothing, or `--wait` ends.
async fn trust_settled<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
) -> Result<(), Failure> {
    loop {
        s.step().await;
        let trust = s.model().trust_settings();
        if !trust.reading() && trust.in_flight().is_none() {
            return Ok(());
        }
        if Instant::now() >= r.until {
            return Err(Failure::Unreachable(
                "the daemon did not answer in time".to_owned(),
            ));
        }
        tokio::time::sleep(STEP).await;
    }
}

/// Read the allowlist into the trust settings, as opening them does.
async fn trust_opened<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
) -> Result<(), Failure> {
    s.will(|_| vec![ViewEvent::Trust(TrustInput::Opened)]);
    trust_settled(s, r).await?;
    let trust = s.model().trust_settings();
    if trust.list().is_some() {
        return Ok(());
    }
    Err(match trust.outcome().0 {
        Some(TrustOutcome::Problem(p) | TrustOutcome::NotReadAgain(p)) => trust_failure(*p),
        _ => Failure::Refused("the allowlist was not read".to_owned()),
    })
}

async fn trust_list<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    out: &mut String,
) -> Result<(), Failure> {
    trust_opened(s, r).await?;
    let Some(list) = s.model().trust_settings().list() else {
        return Err(Failure::Refused("the allowlist was not read".to_owned()));
    };
    for row in &list.allowed {
        let origin = match row.origin {
            interweave_human_client_api::TrustOrigin::Configured => "configured",
            interweave_human_client_api::TrustOrigin::AddedHere => "added",
        };
        if r.json {
            out.push_str(&output::json_line(
                &json!({ "peer": row.peer.as_str(), "origin": origin }),
            ));
        } else {
            let _ = writeln!(out, "{}  {origin}", row.peer.as_str());
        }
    }
    Ok(())
}

async fn set_trust<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    peer: &TransportIdentity,
    allowed: bool,
    out: &mut String,
) -> Result<(), Failure> {
    trust_opened(s, r).await?;
    let said = |out: &mut String, already: bool| {
        let _ = writeln!(
            out,
            "{} {}{}",
            if allowed { "allowed" } else { "revoked" },
            peer.as_str(),
            if already { " (already)" } else { "" }
        );
    };
    let listed = s
        .model()
        .trust_settings()
        .list()
        .is_some_and(|l| l.allows(peer));
    if listed == allowed {
        said(out, true);
        return Ok(());
    }
    let before = s.model().trust_settings().outcome().1;
    let shown = peer.clone();
    s.will(move |_| {
        if allowed {
            vec![
                ViewEvent::Trust(TrustInput::EntryChanged(shown.as_str().to_owned())),
                ViewEvent::Trust(TrustInput::ProposeAllow),
            ]
        } else {
            vec![ViewEvent::Trust(TrustInput::ProposeRevoke(shown))]
        }
    });
    // Confirmed only as the model shows it: the change the person would
    // see on the confirmation, never one this verb built.
    s.will(|m| {
        m.trust_settings()
            .pending()
            .cloned()
            .map(|change| vec![ViewEvent::Trust(TrustInput::Confirm(change))])
            .unwrap_or_default()
    });
    trust_settled(s, r).await?;
    let trust = s.model().trust_settings();
    if trust.list().is_some_and(|l| l.allows(peer) == allowed) {
        said(out, false);
        return Ok(());
    }
    let (outcome, count) = trust.outcome();
    Err(match outcome.filter(|_| count != before) {
        Some(TrustOutcome::Entry(EntryProblem::OwnIdentity)) => {
            Failure::Refused("that is this profile's own identity".to_owned())
        }
        Some(TrustOutcome::Unconfirmed(_)) => Failure::Refused(
            "the daemon did not confirm the change, which may have been made: \
             read the list again"
                .to_owned(),
        ),
        Some(TrustOutcome::Superseded(_)) => Failure::Refused(
            "the change was not made: a later change of this peer from another \
             admin connection stands"
                .to_owned(),
        ),
        Some(TrustOutcome::Problem(p) | TrustOutcome::NotReadAgain(p)) => trust_failure(*p),
        _ => Failure::Refused("the change was not made".to_owned()),
    })
}

/// The endpoint this session's lease holds, while it holds one.
fn leased<B: DataSessionBinding, A: AdminBinding>(s: &Session<B, A>) -> Option<&EndpointId> {
    match s.state() {
        Some(SessionState::Ready { endpoint }) => endpoint.as_ref(),
        _ => None,
    }
}

/// One event `listen` prints, or `None` for what it does not.
fn event_line<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
    update: &Update,
    json: bool,
) -> Option<String> {
    let model = s.model();
    let (event, fields) = match update {
        // The local endpoint that received it: the one this session's lease
        // holds, beside the author and the peer's endpoint (G6).
        Update::Received(received) => {
            return message_line(s, received.row, json, leased(s).map(EndpointId::as_str));
        }
        Update::ReceivedCard(card) => {
            // Said as the proposals list says it, read by no one: the card
            // stays unread for `proposals read`.
            let key = ConversationKey::Direct {
                peer: card.peer.clone(),
                endpoint: Some(card.endpoint.clone()),
            };
            let item = model
                .proposals(&key)
                .into_iter()
                .find(|p| p.row == card.row)?;
            return Some(crate::cards::proposal_line(&item, &key, "unread", json));
        }
        Update::Client(ClientEvent::Outbound(update)) => (
            "outbound",
            outbound_fields(update.row, &ItemStatus::Outbound(update.status.clone())),
        ),
        Update::Client(ClientEvent::Session(state)) => {
            let no_daemon =
                model.session_notice() == Some(interweave_human_ui_model::SessionNotice::NoDaemon);
            (
                "session",
                json!({ "session": output::session(Some(state), no_daemon) }),
            )
        }
        Update::Client(ClientEvent::Connectivity(c)) => (
            "connectivity",
            json!({ "connectivity": output::connectivity(*c) }),
        ),
        Update::Client(ClientEvent::PeerPath { peer, path }) => (
            "path",
            json!({ "peer": peer.as_str(), "path": path.label() }),
        ),
        Update::Client(ClientEvent::PeerDisconnected { peer }) => {
            ("disconnected", json!({ "peer": peer.as_str() }))
        }
        _ => return None,
    };
    if json {
        let mut value = fields;
        value["event"] = json!(event);
        return Some(output::json_line(&value));
    }
    let mut line = event.to_owned();
    if let Some(map) = fields.as_object() {
        // A field with no value (an outbound `problem` while there is
        // none) is left out of the line, not printed empty.
        for value in map.values().filter_map(serde_json::Value::as_str) {
            line.push(' ');
            line.push_str(&output::one_line(value));
        }
    }
    line.push('\n');
    Some(line)
}

/// An outbound event's fields: the row's id, its status, and the send
/// problem it carries (G5) -- `null` when it carries none, so a script
/// reads one shape.
fn outbound_fields(row: RowId, status: &ItemStatus) -> serde_json::Value {
    json!({
        "id": format!("p{}", row.get()),
        "status": output::status(status),
        "problem": output::outbound_problem(status),
    })
}

/// The `message` event of unread row `row`, naming `local_endpoint` as
/// the endpoint that received it, or `None` when the model does not
/// show the row.
fn message_line<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
    row: RowId,
    json: bool,
    local_endpoint: Option<&str>,
) -> Option<String> {
    let model = s.model();
    let item = model.item_named(&format!("u{}", row.get()))?;
    let (key, message) = model.conversations().into_iter().find_map(|c| {
        model
            .messages(&c.key)
            .into_iter()
            .find(|m| m.key == item)
            .map(|m| (c.key, m))
    })?;
    if !json {
        return Some(format!(
            "message {}\n{}",
            conversation_text(&key),
            output::message(&message, false)
        ));
    }
    let mut value = output::message_value(&message);
    value["event"] = json!("message");
    value["conversation"] = json!(conversation_text(&key));
    value["local_endpoint"] = json!(local_endpoint);
    Some(output::json_line(&value))
}

/// How `listen` runs: its output form, whether it consumes what it
/// prints, and the input lines it sends from.
pub(crate) struct ListenMode {
    /// One JSON object per line.
    pub(crate) json: bool,
    /// Each message marked read, not kept, once its line is written and
    /// flushed (G3). A line that cannot be written leaves its message
    /// unread.
    pub(crate) consume: bool,
    /// JSON lines to send over the lease listen holds (G1); the channel
    /// closing at EOF ends the input, never listen.
    pub(crate) input: Option<tokio::sync::mpsc::UnboundedReceiver<InputLine>>,
}

/// One line `listen --input` read: its text, or why it could not be read
/// as text, which is answered `input-refused` like any line it cannot read.
pub(crate) type InputLine = Result<String, String>;

/// Read `reader`'s lines into `lines` until EOF, or until a read fails or
/// the receiver is gone. A line that is not UTF-8 is passed on as a
/// refusal and the reading goes on: one bad line never silences those
/// after it (`a_line_that_is_not_utf8_is_refused_and_the_next_is_read`).
pub(crate) fn read_input(
    mut reader: impl std::io::BufRead,
    lines: &tokio::sync::mpsc::UnboundedSender<InputLine>,
) {
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) => return,
            Ok(_) => {}
            Err(e) => {
                eprintln!("interweave-human: stdin: {e}; no more input is read");
                return;
            }
        }
        // As `BufRead::lines` ends a line: "\n" or "\r\n".
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
        }
        let line =
            String::from_utf8(std::mem::take(&mut bytes)).map_err(|_| "not UTF-8 text".to_owned());
        if lines.send(line).is_err() {
            return;
        }
    }
}

/// One input line `listen --input` sends, as read.
struct InputSend {
    conversation: ConversationKey,
    text: String,
    reference: serde_json::Value,
}

/// An input line read, or why it is refused with the `ref` it carried.
fn input_send<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
    line: &str,
) -> Result<InputSend, (serde_json::Value, String)> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|_| (serde_json::Value::Null, "not a JSON object".to_owned()))?;
    let send = value.get("send").filter(|v| v.is_object());
    let reference = send
        .and_then(|v| v.get("ref"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let refused = |why: &str| (reference.clone(), why.to_owned());
    let send = send.ok_or_else(|| refused("not a send: {\"send\": {...}}"))?;
    let conversation = send
        .get("conversation")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| refused("no conversation"))?;
    let conversation = crate::cli::conversation(conversation).map_err(|e| refused(&e))?;
    let text = send
        .get("text")
        .and_then(serde_json::Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| refused("no text, or an empty one"))?;
    // One lease, one source endpoint: a line may name it, never another
    // (G2), since this session sends as the endpoint it holds.
    if let Some(named) = send.get("endpoint") {
        let held = leased(s).map(EndpointId::as_str);
        if named.as_str() != held {
            return Err(refused("this listen holds another endpoint"));
        }
    }
    Ok(InputSend {
        conversation,
        text: text.to_owned(),
        reference,
    })
}

/// Write `line` and flush it: `Ok(false)` when the reader went away,
/// which ends `listen` as a stop does.
fn put(out: &mut dyn std::io::Write, line: &str) -> Result<bool, Failure> {
    match out.write_all(line.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(false),
        Err(e) => Err(Failure::Refused(format!("stdout: {e}"))),
    }
}

/// What `listen --consume` has done with the store's unread rows, so
/// that each is printed once and decided once.
#[derive(Default)]
struct Consumed {
    /// Unread message rows printed: each is marked read, and never printed
    /// again by a later drain of the store.
    printed: std::collections::HashSet<RowId>,
    /// Proposals asked to be declined, by row, with the line that says so:
    /// printed once the decline is carried out.
    declining: std::collections::HashMap<RowId, String>,
    /// Proposals whose decline was answered, done or failed: not asked again
    /// this run.
    decided: std::collections::HashSet<RowId>,
}

/// The rows `--consume` has yet to take from the model: each unread
/// message not yet printed, by row, and each proposal not yet declined.
/// The store keeps no record of which endpoint received a row, so a row
/// found here is printed with `local_endpoint` `null`
/// (`listen_consume_drains_what_the_store_held_unread_at_open`).
fn undrained<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
    consumed: &Consumed,
) -> (Vec<RowId>, Vec<(ConversationKey, ProposalItem)>) {
    let model = s.model();
    let mut messages = Vec::new();
    for c in model.conversations() {
        for intent in model.conversation_viewed(&c.key, true) {
            if let Intent::MarkRead(row) = intent
                && !consumed.printed.contains(&row)
                && !messages.contains(&row)
            {
                messages.push(row);
            }
        }
    }
    let proposals = crate::cards::every_proposal(s)
        .into_iter()
        .filter(|(_, p)| {
            !consumed.declining.contains_key(&p.row) && !consumed.decided.contains(&p.row)
        })
        .collect();
    (messages, proposals)
}

/// Hold the session open and print one line (one JSON object with
/// `--json`) per event, until `stop`. Received messages stay unread unless
/// `--consume`; `--input` sends each line it reads.
///
/// # Errors
/// [`Failure::Refused`] when stdout cannot be written; a reader that went
/// away ends it as `stop` does.
#[allow(
    clippy::too_many_lines,
    reason = "one loop: what is printed, consumed and sent each step, in order"
)]
pub(crate) async fn listen<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    mode: ListenMode,
    out: &mut dyn std::io::Write,
    stop: impl Future<Output = ()>,
) -> Result<(), Failure> {
    let ListenMode {
        json,
        consume,
        mut input,
    } = mode;
    let mut stop = std::pin::pin!(stop);
    let mut waiting: std::collections::VecDeque<InputLine> = std::collections::VecDeque::new();
    // The `ref` each input line carried, by the pending row its send made:
    // every outbound event of that row names it.
    let mut refs: std::collections::HashMap<RowId, serde_json::Value> =
        std::collections::HashMap::new();
    // The input line whose send was issued and is not yet answered.
    let mut issued: Option<serde_json::Value> = None;
    let mut consumed = Consumed::default();
    // What the store held unread when the session opened is the first
    // drain; a relist the facade asks for (`UnreadInStore`) is the next.
    let mut drain = consume;
    loop {
        s.step().await;
        // A command the model side could not carry out -- a read or a
        // decline `--consume` asked -- is said, never dropped: its row
        // stays unread in the store.
        // Each one, by the row it was for: a drain's reads and declines run
        // in one step, and every one refused leaves its row unread
        // (`every_read_the_store_refuses_is_said_by_its_row`).
        for problem in s.side().take_problems() {
            let row = match &problem {
                Problem::Command {
                    command: Command::MarkRead(row) | Command::DeclineCard(row),
                    ..
                } => Some(format!("u{}", row.get())),
                _ => None,
            };
            let failure = problem_failure(problem);
            let line = if json {
                output::json_line(
                    &json!({ "event": "failed", "id": row, "reason": failure.message() }),
                )
            } else {
                format!(
                    "failed {}{}\n",
                    row.map(|row| format!("{row} ")).unwrap_or_default(),
                    output::one_line(failure.message())
                )
            };
            if !put(out, &line)? {
                return Ok(());
            }
        }
        let mut read: Vec<RowId> = Vec::new();
        let mut decline: Vec<RowId> = Vec::new();
        // Set once the reader is gone or stdout fails: what was written
        // before is still marked read, then listen ends.
        let mut ended: Option<Result<(), Failure>> = None;
        let mut lines: Vec<(String, Option<RowId>)> = Vec::new();
        for update in s.take_updates() {
            let line = match &update {
                Update::Client(ClientEvent::UnreadInStore { .. }) | Update::UnreadListed { .. }
                    if consume =>
                {
                    drain = true;
                    event_line(s, &update, json)
                }
                Update::Received(received) if consume => {
                    if consumed.printed.contains(&received.row) {
                        None
                    } else {
                        event_line(s, &update, json)
                    }
                }
                // Declined as it arrives, and said once the decline is done.
                Update::ReceivedCard(_) if consume => {
                    drain = true;
                    None
                }
                Update::CardResolved { row, .. } if consumed.declining.contains_key(row) => {
                    consumed.decided.insert(*row);
                    consumed.declining.remove(row)
                }
                Update::Sent { row, .. } if issued.is_some() => {
                    let reference = issued.take().unwrap_or(serde_json::Value::Null);
                    refs.insert(*row, reference.clone());
                    let status = s
                        .model()
                        .item_named(&format!("p{}", row.get()))
                        .and_then(|item| {
                            s.model().conversations().into_iter().find_map(|c| {
                                s.model()
                                    .messages(&c.key)
                                    .into_iter()
                                    .find(|m| m.key == item)
                                    .map(|m| m.status)
                            })
                        })
                        .unwrap_or(ItemStatus::Outbound(OutboundStatus::Sending {
                            attempts: 0,
                            next_retry_at: None,
                            last_problem: None,
                        }));
                    let mut value = outbound_fields(*row, &status);
                    value["event"] = json!("outbound");
                    value["ref"] = reference;
                    Some(output::json_line(&value))
                }
                Update::SendRefused { error, .. } if issued.is_some() => {
                    let reference = issued.take().unwrap_or(serde_json::Value::Null);
                    Some(output::json_line(&json!({
                        "event": "input-refused",
                        "ref": reference,
                        "reason": output::send_error(error),
                    })))
                }
                Update::Client(ClientEvent::Outbound(o)) if refs.contains_key(&o.row) => {
                    let reference = if o.status.is_terminal() {
                        refs.remove(&o.row)
                    } else {
                        refs.get(&o.row).cloned()
                    };
                    event_line(s, &update, json).map(|line| {
                        let mut value: serde_json::Value =
                            serde_json::from_str(&line).unwrap_or_default();
                        value["ref"] = reference.unwrap_or_default();
                        output::json_line(&value)
                    })
                }
                _ => event_line(s, &update, json),
            };
            if let Some(line) = line {
                let row = match &update {
                    Update::Received(received) if consume => Some(received.row),
                    _ => None,
                };
                lines.push((line, row));
            }
        }
        if consume && drain && ended.is_none() {
            drain = false;
            let (messages, proposals) = undrained(s, &consumed);
            // A row this step already holds a line for -- one received in
            // the same turn as a card or a relist -- is that line's: never
            // printed twice
            // (`a_message_received_with_a_card_is_printed_once`).
            let queued: std::collections::HashSet<RowId> =
                lines.iter().filter_map(|(_, row)| *row).collect();
            for row in messages.into_iter().filter(|row| !queued.contains(row)) {
                if let Some(line) = message_line(s, row, json, None) {
                    lines.push((line, Some(row)));
                }
            }
            for (key, p) in proposals {
                if s.model()
                    .card_actions(p.row)
                    .contains(&Intent::DeclineCard(p.row))
                {
                    let said = crate::cards::proposal_line(&p, &key, "declined", json);
                    consumed.declining.insert(p.row, said);
                    decline.push(p.row);
                }
            }
        }
        for (line, row) in lines {
            match put(out, &line) {
                Ok(true) => {}
                Ok(false) => {
                    ended = Some(Ok(()));
                    break;
                }
                Err(failure) => {
                    ended = Some(Err(failure));
                    break;
                }
            }
            // Read once its line is written and flushed, never before:
            // the line is its presentation to the program reading (G3).
            if let Some(row) = row {
                consumed.printed.insert(row);
                read.push(row);
            }
        }
        if !read.is_empty() || !decline.is_empty() {
            let intents: Vec<ViewEvent> = read
                .into_iter()
                .map(Intent::MarkRead)
                .chain(decline.into_iter().map(Intent::DeclineCard))
                .map(ViewEvent::Intent)
                .collect();
            s.will(move |_| intents);
            // Carried out in the step that follows at once, before a stop
            // or the end is looked at: a line written is a row read
            // (`a_printed_message_is_read_before_listen_ends`).
            if let Some(ended) = ended {
                s.step().await;
                return ended;
            }
            continue;
        }
        if let Some(ended) = ended {
            return ended;
        }
        // One input send at a time: the next is issued once this one is
        // answered, sent or refused, so each answer names its own line.
        if issued.is_none()
            && let Some(line) = waiting.pop_front()
        {
            let read = line.map_err(|reason| (serde_json::Value::Null, reason));
            match read.and_then(|line| input_send(s, &line)) {
                Err((reference, reason)) => {
                    let refused = output::json_line(&json!({
                        "event": "input-refused",
                        "ref": reference,
                        "reason": reason,
                    }));
                    if !put(out, &refused)? {
                        return Ok(());
                    }
                }
                Ok(send) => {
                    let (typed, pressed) = (send.conversation.clone(), send.conversation);
                    let draft = send.text;
                    s.will(move |_| vec![ViewEvent::DraftChanged { key: typed, draft }]);
                    s.will(move |m| {
                        m.send_draft(&pressed)
                            .map(ViewEvent::Intent)
                            .into_iter()
                            .collect()
                    });
                    issued = Some(send.reference);
                }
            }
            continue;
        }
        tokio::select! {
            () = &mut stop => return Ok(()),
            line = async {
                match input.as_mut() {
                    Some(lines) => lines.recv().await,
                    None => std::future::pending().await,
                }
            } => match line {
                Some(line) => waiting.push_back(line),
                // EOF: no more input; listen goes on.
                None => input = None,
            },
            () = tokio::time::sleep(STEP) => {}
        }
    }
}

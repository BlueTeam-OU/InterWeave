// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Batch 2's verbs: the own card, sharing it, card proposals and the
//! contacts (plan section 29 step 4; ADR-0059). As the batch-1 verbs do,
//! each scripts what a person does in the window, and the model resolves
//! it: a proposal is accepted only as the model shows it, with the context
//! it was shown with, and only in the run that showed it -- a card shown
//! and left undecided is read, and gone when the run ends (RETENTION.md:
//! a card is unread content until shown).

use std::fmt::Write as _;
use std::io::Read as _;
use std::time::Instant;

use interweave_human_app_core::Update;
use interweave_human_client_api::{ClientEvent, OutboundStatus};
use interweave_human_store::RowId;
use interweave_human_ui_model::{
    AcceptLine, ContactView, ConversationKey, IdentityView, Intent, ItemStatus, KeyView, PhotoEdit,
    ProposalItem, ProposalNote, ViewEvent,
};
use interweave_local_client_api::{AdminBinding, DataSessionBinding};
use serde_json::{Value, json};

use crate::Failure;
use crate::cli::{PhotoArg, conversation_text};
use crate::output::{json_line, one_line, utc};
use crate::session::{STEP, Session};
use crate::verbs::{Run, look, problems, ready};

/// The largest photo file read: a chosen image is re-encoded to the card's
/// bound by the facade, and this keeps what is read before that bounded
/// too (`a_photo_file_past_the_bound_is_refused_unread`).
pub(crate) const MAX_PHOTO_FILE: u64 = 32 * 1024 * 1024;

/// Read a photo argument into what the model takes.
///
/// # Errors
/// [`Failure::Refused`] when the file cannot be read or is past
/// [`MAX_PHOTO_FILE`].
pub(crate) fn photo(arg: &PhotoArg) -> Result<PhotoEdit, Failure> {
    match arg {
        PhotoArg::Keep => Ok(PhotoEdit::Keep),
        PhotoArg::Remove => Ok(PhotoEdit::Remove),
        PhotoArg::File(path) => {
            let file = std::fs::File::open(path)
                .map_err(|e| Failure::Refused(format!("the photo file: {e}")))?;
            let mut bytes = Vec::new();
            file.take(MAX_PHOTO_FILE + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| Failure::Refused(format!("the photo file: {e}")))?;
            if bytes.len() as u64 > MAX_PHOTO_FILE {
                return Err(Failure::Refused(format!(
                    "the photo file is larger than {} MiB",
                    MAX_PHOTO_FILE / 1024 / 1024
                )));
            }
            Ok(PhotoEdit::Chosen(bytes))
        }
    }
}

/// A proposal's id: its card's unread row, as a message's is.
fn proposal_id(row: RowId) -> String {
    format!("u{}", row.get())
}

fn key_value(key: &KeyView) -> Value {
    match key {
        KeyView::New => json!({ "state": "new" }),
        KeyView::Same => json!({ "state": "same" }),
        KeyView::Changed { recorded } => json!({ "state": "changed", "recorded": recorded }),
    }
}

fn proposal_value(p: &ProposalItem, key: &ConversationKey, status: &str) -> Value {
    let notes: Vec<Value> = p
        .notes
        .iter()
        .map(|n| match n {
            ProposalNote::SameName(name) => json!({ "same_name": name }),
            ProposalNote::SharesNode(name) => json!({ "shares_node": name }),
            ProposalNote::KeyMatches(name) => json!({ "key_matches": name }),
        })
        .collect();
    json!({
        "id": proposal_id(p.row),
        "conversation": conversation_text(key),
        "status": status,
        "peer": p.peer.as_str(),
        "route": p.route.as_str(),
        "name": p.name,
        "has_photo": p.has_photo,
        "fingerprint": p.fingerprint,
        "key": key_value(&p.key),
        "accept": match &p.accept {
            AcceptLine::Creates => json!({ "creates": true }),
            AcceptLine::Updates(name) => json!({ "updates": name }),
            AcceptLine::Matches(name) => json!({ "matches": name }),
        },
        "notes": notes,
        "received_at_ms": p.received_at,
    })
}

/// One proposal as `proposals` prints it. Everything in it but the
/// `PeerId`, the route label and the fingerprint is what the card claims.
fn proposal(p: &ProposalItem, key: &ConversationKey, status: &str, json: bool) -> String {
    if json {
        return json_line(&proposal_value(p, key, status));
    }
    let mut out = format!(
        "{}  {:<8} {}  {}\n    card from {}/{}, key {}",
        proposal_id(p.row),
        status,
        utc(p.received_at),
        one_line(&p.name),
        p.peer.as_str(),
        one_line(p.route.as_str()),
        p.fingerprint,
    );
    match &p.key {
        KeyView::New => out.push_str(" (new on this route)"),
        KeyView::Same => out.push_str(" (as recorded)"),
        KeyView::Changed { recorded } => {
            let _ = write!(out, " (CHANGED: recorded {recorded})");
        }
    }
    out.push('\n');
    match &p.accept {
        AcceptLine::Creates => out.push_str("    accept creates a contact\n"),
        AcceptLine::Updates(name) => {
            let _ = writeln!(out, "    accept updates {}", one_line(name));
        }
        AcceptLine::Matches(name) => {
            let _ = writeln!(
                out,
                "    matches {}; nothing to accept, decline closes it",
                one_line(name)
            );
        }
    }
    for note in &p.notes {
        let (what, name) = match note {
            ProposalNote::SameName(n) => ("a contact of the same name on another node", n),
            ProposalNote::SharesNode(n) => ("a contact on the same node", n),
            ProposalNote::KeyMatches(n) => ("a contact holding the same key", n),
        };
        let _ = writeln!(out, "    note: {what}: {}", one_line(name));
    }
    out
}

fn contact_value(c: &ContactView) -> Value {
    let routes: Vec<Value> = c
        .routes
        .iter()
        .map(|r| {
            json!({
                "peer": r.peer.as_str(),
                "endpoint": r.endpoint.as_str(),
                "fingerprint": r.fingerprint,
            })
        })
        .collect();
    json!({
        "id": c.contact_id,
        "name": c.name,
        "name_from": if c.name_from_card { "card" } else { "own" },
        "photo_from": c.photo_from_card.map(|card| if card { "card" } else { "own" }),
        "routes": routes,
    })
}

fn contact(c: &ContactView, routes: bool, json: bool) -> String {
    if json {
        return json_line(&contact_value(c));
    }
    let mut out = format!(
        "{}  {}{}\n",
        c.contact_id,
        one_line(&c.name),
        if c.name_from_card { "" } else { " (own name)" }
    );
    if routes {
        for r in &c.routes {
            let _ = writeln!(
                out,
                "    {}/{}  {}",
                r.peer.as_str(),
                r.endpoint.as_str(),
                r.fingerprint.as_deref().unwrap_or("no key recorded")
            );
        }
    }
    out
}

fn identity(i: &IdentityView, json: bool) -> String {
    if json {
        return json_line(&json!({
            "endpoint": i.endpoint.as_str(),
            "name": i.name,
            "has_photo": i.has_photo,
            "fingerprint": i.fingerprint,
            "peer": i.peer.as_ref().map(interweave_transport_api::TransportIdentity::as_str),
        }));
    }
    format!(
        "identity     {}\nname         {}\nphoto        {}\nfingerprint  {}\npeer         {}\n",
        i.endpoint.as_str(),
        one_line(&i.name),
        if i.has_photo { "yes" } else { "none" },
        i.fingerprint,
        i.peer
            .as_ref()
            .map_or("not known until the session opens", |p| p.as_str()),
    )
}

/// A proposal as `listen` prints it: an event, unread.
pub(crate) fn proposal_line(p: &ProposalItem, key: &ConversationKey, json: bool) -> String {
    if json {
        let mut value = proposal_value(p, key, "unread");
        value["event"] = json!("proposal");
        return json_line(&value);
    }
    format!(
        "proposal {}\n{}",
        conversation_text(key),
        proposal(p, key, "unread", false)
    )
}

/// `card show`.
pub(crate) async fn card_show<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    out: &mut String,
) -> Result<(), Failure> {
    look(s, r).await;
    let i = s.model().identity().ok_or_else(|| {
        Failure::Refused("the identity has no key: its card cannot be shown or shared".to_owned())
    })?;
    out.push_str(&identity(i, r.json));
    Ok(())
}

/// `card set`.
pub(crate) async fn card_set<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    name: &str,
    surname: Option<&str>,
    photo_arg: &PhotoArg,
    out: &mut String,
) -> Result<(), Failure> {
    let photo = photo(photo_arg)?;
    let intent = Intent::SetOwnCard {
        name: name.to_owned(),
        surname: surname.map(str::to_owned),
        photo,
    };
    s.will(move |_| vec![ViewEvent::Intent(intent)]);
    s.step().await;
    problems(s)?;
    let i = s
        .model()
        .identity()
        .ok_or_else(|| Failure::Refused("the identity has no key".to_owned()))?;
    out.push_str(&identity(i, false));
    Ok(())
}

/// `card share`: the card goes as a pending message, followed as `send`
/// follows one.
pub(crate) async fn card_share<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    key: &ConversationKey,
    out: &mut String,
) -> Result<(), Failure> {
    let Some(intent) = s.model().share_card(key) else {
        return Err(Failure::Refused(
            "a card is shared in a direct conversation naming its endpoint: <peer>/<endpoint>"
                .to_owned(),
        ));
    };
    ready(s, r).await?;
    s.will(move |_| vec![ViewEvent::Intent(intent)]);
    s.step().await;
    problems(s)?;
    // The card's row and, often in the same step, its first outcome.
    let first = s.take_updates();
    let row = first
        .iter()
        .find_map(|u| match u {
            Update::CardShared { row, .. } => Some(*row),
            _ => None,
        })
        .ok_or_else(|| Failure::Refused("the card was not shared".to_owned()))?;
    let outcome = |updates: Vec<Update>, status: &mut Option<OutboundStatus>| {
        for u in updates {
            if let Update::Client(ClientEvent::Outbound(update)) = u
                && update.row == row
            {
                *status = Some(update.status);
            }
        }
    };
    let id = format!("p{}", row.get());
    let line = |out: &mut String, token: &str| {
        if r.json {
            out.push_str(&json_line(&json!({ "id": id, "status": token })));
        } else {
            let _ = writeln!(out, "{id} {token}");
        }
    };
    let mut status: Option<OutboundStatus> = None;
    outcome(first, &mut status);
    loop {
        if let Some(st) = &status {
            let token = crate::output::status(&ItemStatus::Outbound(st.clone()));
            match st {
                OutboundStatus::Accepted { .. } | OutboundStatus::Published => {
                    line(out, token);
                    return Ok(());
                }
                OutboundStatus::NeedsAttention { problem, .. } => {
                    line(out, token);
                    return Err(Failure::Refused(format!(
                        "{id} needs attention ({})",
                        crate::output::send_problem(*problem)
                    )));
                }
                OutboundStatus::Cancelled { .. } => {
                    line(out, token);
                    return Err(Failure::Refused(format!("{id} was cancelled")));
                }
                _ => {}
            }
        }
        if Instant::now() >= r.until {
            line(out, "sending");
            return Err(Failure::Pending(format!(
                "{id} is still on its way: it stays pending in the message store"
            )));
        }
        tokio::time::sleep(STEP).await;
        s.step().await;
        outcome(s.take_updates(), &mut status);
    }
}

/// Every proposal held, read or not, in conversation order.
fn every_proposal<B: DataSessionBinding, A: AdminBinding>(
    s: &Session<B, A>,
) -> Vec<(ConversationKey, ProposalItem)> {
    let model = s.model();
    model
        .proposal_conversations()
        .into_iter()
        .flat_map(|key| {
            model
                .proposals(&key)
                .into_iter()
                .map(move |p| (key.clone(), p))
        })
        .collect()
}

/// `proposals`: every proposal held, none of them shown read.
pub(crate) async fn proposals<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    out: &mut String,
) -> Result<(), Failure> {
    look(s, r).await;
    for (key, p) in every_proposal(s) {
        out.push_str(&proposal(&p, &key, "unread", r.json));
    }
    Ok(())
}

/// `proposals read`: the conversation's proposals shown, and so read, and
/// the named ones accepted or declined in the same run. Every id is
/// checked before anything is read; whatever fails after, what was shown
/// is printed (`a_decision_that_fails_after_the_read_still_prints_the_proposals`).
pub(crate) async fn proposals_read<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    key: &ConversationKey,
    accept: &[String],
    decline: &[String],
    out: &mut String,
) -> Result<(), Failure> {
    if !matches!(
        key,
        ConversationKey::Direct {
            endpoint: Some(_),
            ..
        }
    ) {
        return Err(Failure::Refused(
            "a card's proposals are read on its route: <peer>/<endpoint>, as proposals prints it"
                .to_owned(),
        ));
    }
    look(s, r).await;
    let shown = s.model().proposals(key);
    let find = |id: &str, n: usize, what: &str| {
        shown
            .iter()
            .find(|p| proposal_id(p.row) == id)
            .map(|p| p.row)
            .ok_or_else(|| {
                Failure::Refused(format!(
                    "--{what} number {}: no proposal of this conversation has that id",
                    n + 1
                ))
            })
    };
    let mut decisions: Vec<(RowId, bool)> = Vec::new();
    for (n, id) in accept.iter().enumerate() {
        let row = find(id, n, "accept")?;
        if shown
            .iter()
            .any(|p| p.row == row && matches!(p.accept, AcceptLine::Matches(_)))
        {
            return Err(Failure::Refused(format!(
                "--accept number {}: {id} matches its contact and changes nothing; --decline closes it",
                n + 1
            )));
        }
        decisions.push((row, true));
    }
    for (n, id) in decline.iter().enumerate() {
        decisions.push((find(id, n, "decline")?, false));
    }
    let mut rows: Vec<RowId> = decisions.iter().map(|(row, _)| *row).collect();
    rows.sort();
    if rows.windows(2).any(|w| w[0] == w[1]) {
        return Err(Failure::Refused(
            "a proposal is accepted or declined, not both, and once".to_owned(),
        ));
    }
    // What is read is what is printed: the proposals shown, and not one
    // that arrives in the step that reads them, which stays unread for
    // the next run.
    let viewed = key.clone();
    let printed: Vec<RowId> = shown.iter().map(|p| p.row).collect();
    s.will(move |m| {
        printed_only(m.proposals_viewed(&viewed, true), &printed)
            .into_iter()
            .map(ViewEvent::Intent)
            .collect()
    });
    s.step().await;
    // From here every proposal shown is read, and lasts this run only: it
    // is printed whatever fails after, or the person never sees it, each
    // with what was done to it
    // (`an_accept_made_before_another_fails_is_printed_accepted`).
    let (done, decided) = decide(s, &decisions).await;
    for p in &shown {
        let status = match decisions.iter().find(|(row, _)| *row == p.row) {
            Some((row, true)) if done.contains(row) => "accepted",
            Some((row, false)) if done.contains(row) => "declined",
            _ => "read",
        };
        out.push_str(&proposal(p, key, status, r.json));
    }
    decided
}

/// The reads `viewed` asks for, of the rows `printed` alone: a proposal
/// the model gained after the list was printed is not read
/// (`only_what_was_printed_is_marked_read`).
fn printed_only(viewed: Vec<Intent>, printed: &[RowId]) -> Vec<Intent> {
    viewed
        .into_iter()
        .filter(|i| matches!(i, Intent::MarkRead(row) if printed.contains(row)))
        .collect()
}

/// Carry out `decisions`, as the model offers them: the rows decided,
/// whatever else failed, and whether everything was.
async fn decide<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    decisions: &[(RowId, bool)],
) -> (Vec<RowId>, Result<(), Failure>) {
    if let Err(failure) = problems(s) {
        return (Vec::new(), Err(failure));
    }
    let mut intents = Vec::new();
    for (row, accept) in decisions {
        let wanted = if *accept {
            Intent::AcceptCard(*row)
        } else {
            Intent::DeclineCard(*row)
        };
        if !s.model().card_actions(*row).contains(&wanted) {
            return (
                Vec::new(),
                Err(Failure::Refused(format!(
                    "{} cannot be decided now",
                    proposal_id(*row)
                ))),
            );
        }
        intents.push(wanted);
    }
    if intents.is_empty() {
        return (Vec::new(), Ok(()));
    }
    s.will(move |_| intents.into_iter().map(ViewEvent::Intent).collect());
    s.step().await;
    let resolved: Vec<RowId> = s
        .take_updates()
        .into_iter()
        .filter_map(|u| match u {
            Update::CardResolved(row) => Some(row),
            _ => None,
        })
        .collect();
    (resolved, problems(s))
}

/// `contacts`.
pub(crate) async fn contacts<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    out: &mut String,
) -> Result<(), Failure> {
    look(s, r).await;
    for c in s.model().contacts() {
        out.push_str(&contact(c, false, r.json));
    }
    Ok(())
}

/// `contacts show`.
pub(crate) async fn contact_show<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    id: &str,
    out: &mut String,
) -> Result<(), Failure> {
    look(s, r).await;
    let c = s
        .model()
        .contact(id)
        .ok_or_else(|| Failure::Refused("no contact has that id".to_owned()))?;
    out.push_str(&contact(c, true, r.json));
    Ok(())
}

/// `contacts rename` and `contacts photo`.
pub(crate) async fn contact_edit<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    id: &str,
    name: Option<&str>,
    photo_arg: Option<&PhotoArg>,
    out: &mut String,
) -> Result<(), Failure> {
    if s.model().contact(id).is_none() {
        return Err(Failure::Refused("no contact has that id".to_owned()));
    }
    let intent = match (name, photo_arg) {
        (Some(name), _) => Intent::RenameContact {
            contact_id: id.to_owned(),
            name: name.to_owned(),
        },
        (None, Some(arg)) => Intent::SetContactPhoto {
            contact_id: id.to_owned(),
            photo: photo(arg)?,
        },
        (None, None) => return Err(Failure::Refused("nothing to change".to_owned())),
    };
    s.will(move |_| vec![ViewEvent::Intent(intent)]);
    s.step().await;
    problems(s)?;
    let c = s
        .model()
        .contact(id)
        .ok_or_else(|| Failure::Refused("the contact is gone".to_owned()))?;
    out.push_str(&contact(c, false, false));
    Ok(())
}

/// `contacts delete`: as the model offers it, for a contact it holds.
pub(crate) async fn contact_delete<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    id: &str,
    out: &mut String,
) -> Result<(), Failure> {
    let intent = s
        .model()
        .delete_contact(id)
        .ok_or_else(|| Failure::Refused("no contact has that id".to_owned()))?;
    s.will(move |_| vec![ViewEvent::Intent(intent)]);
    s.step().await;
    problems(s)?;
    let _ = writeln!(out, "{id} deleted");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_what_was_printed_is_marked_read() {
        let row = RowId::from_stored;
        let viewed = vec![Intent::MarkRead(row(3)), Intent::MarkRead(row(9))];
        assert_eq!(
            printed_only(viewed, &[row(3)]),
            [Intent::MarkRead(row(3))],
            "the card that arrived after the list, row 9, stays unread"
        );
        assert!(printed_only(vec![Intent::MarkRead(row(9))], &[]).is_empty());
    }
}

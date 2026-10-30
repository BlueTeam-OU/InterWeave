// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the verbs print. Every value a script reads is a fixed token
//! (`sending`, `accepted`, `ready`), never the person-facing label a view
//! would show, so a script does not change when the language does; the
//! README lists them.
//!
//! Text from a peer reaches a terminal, so nothing a peer wrote is
//! printed raw: in text output a control character other than a newline
//! or a tab -- or a line separator, or a bidirectional control -- becomes
//! U+FFFD, and in JSON each of those is escaped, C1 included, which
//! `serde_json` leaves raw. Either way a peer cannot move the cursor,
//! retitle the window, break a line a reader splits on, reorder what is
//! shown, or rewrite what was printed before
//! (`text_from_a_peer_reaches_no_terminal_as_a_control`).

use interweave_human_client_api::{
    Connectivity, OutboundStatus, SendError, SendProblem, SessionProblem, SessionState,
};
use std::fmt::Write as _;

use interweave_human_ui_model::{
    ConversationSummary, Direction, ItemStatus, MessageItem, Reply, Retention, RouteLabel,
};
use interweave_transport_api::TransportIdentity;
use serde_json::{Value, json};

use crate::cli::conversation_text;

/// A session problem's token.
pub(crate) const fn session_problem(problem: SessionProblem) -> &'static str {
    match problem {
        SessionProblem::EndpointInUse => "endpoint-in-use",
        SessionProblem::NotAvailableToThisClient => "not-available-to-this-client",
        SessionProblem::Internal => "internal",
    }
}

/// The session's token: `opening` before the facade has said anything,
/// `no-daemon` when it is re-opening and no daemon serves the profile.
pub(crate) const fn session(state: Option<&SessionState>, no_daemon: bool) -> &'static str {
    match state {
        None | Some(SessionState::Reconnecting { .. }) if no_daemon => "no-daemon",
        None => "opening",
        Some(SessionState::Ready { .. }) => "ready",
        Some(SessionState::Reconnecting { .. }) => "reconnecting",
        Some(SessionState::Refused { .. }) => "refused",
        Some(SessionState::StorageDegraded) => "storage-degraded",
        Some(SessionState::Closed) => "closed",
    }
}

/// Connectivity's token. `unknown` is never `offline` (human-client-ui.md
/// section 7).
pub(crate) const fn connectivity(c: Connectivity) -> &'static str {
    match c {
        Connectivity::OnlineDirect => "online-direct",
        Connectivity::OnlineRelay => "online-relay",
        Connectivity::OnlinePartial => "online-partial",
        Connectivity::Offline => "offline",
        Connectivity::Unknown => "unknown",
    }
}

/// A send problem's token.
pub(crate) const fn send_problem(problem: SendProblem) -> &'static str {
    match problem {
        SendProblem::PeerUntrusted => "peer-untrusted",
        SendProblem::RouteUnavailable => "route-unavailable",
        SendProblem::NoNetworkPath => "no-network-path",
        SendProblem::Busy => "busy",
        SendProblem::ServiceUnavailable => "service-unavailable",
        SendProblem::Incompatible => "incompatible",
        SendProblem::TooLarge => "too-large",
        SendProblem::NotConfigured => "not-configured",
        SendProblem::Internal => "internal",
    }
}

/// The send problem an outbound status carries, as its token: the one
/// that holds it for attention, or the last attempt's while it is
/// retried; none once it is terminal.
pub(crate) const fn outbound_problem(status: &ItemStatus) -> Option<&'static str> {
    match status {
        ItemStatus::Outbound(
            OutboundStatus::Sending {
                last_problem: Some(problem),
                ..
            }
            | OutboundStatus::Unconfirmed {
                last_problem: Some(problem),
                ..
            }
            | OutboundStatus::NeedsAttention { problem, .. },
        ) => Some(send_problem(*problem)),
        _ => None,
    }
}

/// A refused send's token.
pub(crate) const fn send_error(error: &SendError) -> &'static str {
    match error {
        SendError::TooLarge => "too-large",
        SendError::InvalidEnvelope => "invalid-envelope",
        SendError::NotConfigured => "not-configured",
        SendError::StorageUnavailable => "storage-unavailable",
        SendError::AlreadyPending => "already-pending",
        SendError::EndpointBound => "endpoint-bound",
    }
}

/// A message's status token.
pub(crate) const fn status(status: &ItemStatus) -> &'static str {
    match status {
        ItemStatus::Outbound(OutboundStatus::Sending { .. }) => "sending",
        ItemStatus::Outbound(OutboundStatus::Unconfirmed { .. }) => "not-confirmed",
        ItemStatus::Outbound(OutboundStatus::NeedsAttention { .. }) => "needs-attention",
        ItemStatus::Outbound(OutboundStatus::Accepted { .. }) => "accepted",
        ItemStatus::Outbound(OutboundStatus::Published) => "published",
        ItemStatus::Outbound(OutboundStatus::Cancelled { .. }) => "cancelled",
        ItemStatus::Inbound(Retention::Unread) => "unread",
        ItemStatus::Inbound(Retention::ReadEphemeral) => "read",
        ItemStatus::Inbound(Retention::Kept) => "kept",
    }
}

/// A character no peer's text may carry raw to a terminal or a reader
/// that splits lines: a control (Cc), a line or paragraph separator, or a
/// bidirectional control that reorders what is shown.
const fn unsafe_char(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{2028}'
                | '\u{2029}'
                | '\u{061C}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
        )
}

/// One field of a line -- a name, an address, a path, an error -- safe to
/// print: every [`unsafe_char`], a newline and a tab included, replaced, so
/// a value never splits the line it is printed on
/// (`a_field_never_splits_its_line`). [`printable`] keeps the newlines a
/// message's text holds.
pub(crate) fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if unsafe_char(c) { '\u{FFFD}' } else { c })
        .collect()
}

/// `text` safe to print to a terminal: every [`unsafe_char`] other than a
/// newline or a tab replaced. For a message's text alone; a field of a
/// line is [`one_line`].
pub(crate) fn printable(text: &str) -> String {
    text.chars()
        .map(|c| {
            if unsafe_char(c) && c != '\n' && c != '\t' {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .collect()
}

/// One JSON line: `value` with every [`unsafe_char`] escaped, C1 and DEL
/// included, then a newline.
pub(crate) fn json_line(value: &Value) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + 1);
    for c in raw.chars() {
        // `serde_json` escapes below U+0020 already; what is left can only
        // be inside a string, where an escape means the same character.
        if unsafe_char(c) {
            let _ = write!(out, "\\u{:04x}", u32::from(c));
        } else {
            out.push(c);
        }
    }
    out.push('\n');
    out
}

/// A UTC time from Unix milliseconds: `2026-10-22 07:03:23Z`.
pub(crate) fn utc(ms: u64) -> String {
    let secs = ms / 1_000;
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Civil from days (the proleptic Gregorian calendar), for 1970 on.
    let z = i64::try_from(days).unwrap_or(i64::MAX / 2) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

fn direction(d: Direction) -> &'static str {
    match d {
        Direction::Outbound => "out",
        Direction::Inbound => "in",
    }
}

/// One message's JSON object, as `messages`, `read` and `listen` print it.
pub(crate) fn message_value(item: &MessageItem) -> Value {
    let reply = item.reply.as_ref().map(|r| match r {
        Reply::Present(key) => json!({ "id": key.to_string() }),
        Reply::Unavailable(_) => json!({ "unavailable": true }),
    });
    json!({
        "id": item.key.to_string(),
        "direction": direction(item.direction),
        "status": status(&item.status),
        "kept": item.kept,
        "author": item.author.as_ref().map(TransportIdentity::as_str),
        "route_label": item.route_label.as_ref().map(RouteLabel::as_str),
        "sent_at_ms": item.sent_at_ms,
        "local_at_ms": item.local_at,
        "reply_to": reply,
        "text": item.source,
        // One's own card row (contact-cards 5.5): the card as it went, and
        // no text; `null` for a message.
        "card": item.card.as_ref().map(|c| json!({
            "name": c.name,
            "has_photo": c.photo.is_some(),
        })),
    })
}

/// One message as `messages` and `read` print it.
pub(crate) fn message(item: &MessageItem, json: bool) -> String {
    if json {
        return json_line(&message_value(item));
    }
    let from = match (&item.author, &item.route_label) {
        (Some(a), Some(r)) => format!("{}/{}", a.as_str(), r.as_str()),
        (Some(a), None) => a.as_str().to_owned(),
        (None, _) => "me".to_owned(),
    };
    let mut out = format!(
        "{}  {:<3} {:<15} {}  {}\n",
        item.key,
        direction(item.direction),
        status(&item.status),
        utc(item.local_at),
        one_line(&from),
    );
    if let Some(card) = &item.card {
        out.push_str("    card: ");
        out.push_str(&one_line(&card.name));
        out.push('\n');
    }
    for line in printable(&item.source).lines() {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// One conversation as `conversations` prints it.
pub(crate) fn conversation(summary: &ConversationSummary, json: bool) -> String {
    let key = conversation_text(&summary.key);
    if json {
        return json_line(&json!({
            "conversation": key,
            "title": summary.title,
            "unread": summary.unread,
            "last_activity_ms": summary.last_activity,
            "last_id": summary.last.key.to_string(),
        }));
    }
    format!(
        "{key}  {} unread  {}  {}\n",
        summary.unread,
        utc(summary.last_activity),
        one_line(&summary.title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_never_splits_its_line() {
        let field = one_line("/ip4/1.2.3.4/tcp/1\n/ip4/5.6.7.8/tcp/2\tx\u{2028}y\u{1b}");
        assert!(
            !field.contains(['\n', '\t', '\u{2028}', '\u{1b}']),
            "{field:?}"
        );
        assert!(field.starts_with("/ip4/1.2.3.4/tcp/1"), "{field:?}");
    }

    #[test]
    fn text_from_a_peer_reaches_no_terminal_as_a_control() {
        let hostile = "a\u{1b}]0;owned\u{7}b\u{9b}2Jc\u{7f}\u{202e}rev\u{2066}i\u{2028}x\nd\te";
        let text = printable(hostile);
        for c in [
            '\u{1b}', '\u{7}', '\u{9b}', '\u{7f}', '\u{202e}', '\u{2066}', '\u{2028}',
        ] {
            assert!(!text.contains(c), "{c:?} printed raw");
        }
        assert!(text.contains("\nd\te"), "a newline and a tab stay");
        let line = json_line(&json!({ "text": hostile }));
        for c in [
            '\u{1b}', '\u{9b}', '\u{7f}', '\u{202e}', '\u{2066}', '\u{2028}',
        ] {
            assert!(!line.contains(c), "{c:?} raw in {line:?}");
        }
        assert_eq!(line.matches('\n').count(), 1, "one line: {line:?}");
        let back: Value = serde_json::from_str(&line).expect("still JSON");
        assert_eq!(back["text"], hostile, "the escapes mean the same text");
    }

    /// One's own card row prints its card, in JSON and as text, and no
    /// text of its own. Mutation: the card left out of either form.
    #[test]
    fn an_own_card_row_prints_its_card() {
        use interweave_human_ui_model::{ConversationKey, SentCard, UiModel};
        let peer = interweave_transport_api::TransportIdentity::parse(
            "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
        )
        .expect("a PeerId");
        let key = ConversationKey::Direct {
            peer,
            endpoint: Some(interweave_transport_api::EndpointId::parse("human").expect("valid")),
        };
        let mut model = UiModel::new();
        model.card_queued(
            interweave_human_ui_model::RowId::from_stored(1),
            key.clone(),
            "0".repeat(32),
            SentCard {
                name: "Ada Lovelace".to_owned(),
                photo: None,
            },
            1,
        );
        let item = &model.messages(&key)[0];
        let value = message_value(item);
        assert_eq!(value["card"]["name"], "Ada Lovelace");
        assert_eq!(value["card"]["has_photo"], false);
        assert_eq!(value["text"], "");
        assert!(message(item, false).contains("    card: Ada Lovelace\n"));
    }

    #[test]
    fn unknown_connectivity_is_never_offline() {
        assert_eq!(connectivity(Connectivity::Unknown), "unknown");
        assert_eq!(connectivity(Connectivity::Offline), "offline");
    }

    #[test]
    fn utc_is_the_civil_date() {
        assert_eq!(utc(0), "1970-01-01 00:00:00Z");
        assert_eq!(utc(951_782_400_000), "2000-02-29 00:00:00Z");
        assert_eq!(utc(1_792_652_603_000), "2026-10-22 07:03:23Z");
    }
}

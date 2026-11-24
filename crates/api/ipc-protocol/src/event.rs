// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The closed event catalogue (`ipc/event.schema.json`; `LOCAL-IPC.md`
//! §Event catalogue): `event_type` binds `data` to its shape.
//!
//! As with requests, the envelope ([`EventFrame`]) carries the type as
//! text and the data uninterpreted, and [`Event::decode`] binds them: an
//! unknown type from a server is the server's protocol violation, which a
//! client should be able to NAME rather than lose inside a parse error.

use interweave_local_client_api::{
    Generation, LocalSessionEvent, ReceivedBroadcast, ReceivedDirect, SessionEvent,
};
use interweave_transport_api::{
    ChannelId, EndpointId, MessageId, Payload, PeerPath, TransportError, TransportIdentity,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::version::{IpcVersion, Since};

/// Maximum CHARACTERS in a `peer.disconnected` reason class
/// (`ipc/event.schema.json` `maxLength`, which counts code points).
pub const MAX_REASON_CLASS_CHARS: usize = 128;

/// One event of the closed catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// `message.direct`: to exactly the connection holding the
    /// destination endpoint's lease.
    MessageDirect(DirectReceived),
    /// `message.broadcast`: to every connection with `events` holding a
    /// join reference for the channel.
    MessageBroadcast(BroadcastReceived),
    /// `endpoint.lease_changed`: revocation only; a grant is learned from
    /// `hello_response`.
    LeaseChanged(LeaseChanged),
    /// `peer.disconnected`: to every connection with `events`.
    PeerDisconnected(PeerDisconnected),
    /// `peer.path_changed`: to every connection with `events` that has a
    /// route to the peer, with or without `previous` (on major 3 the shape
    /// 2.4 brought is the only one).
    PathChanged(PathChanged),
}

/// Whether a connection at `version` may be sent `event` at all: its
/// catalogue type's version ([`EventType::available_at`]). ONE predicate,
/// judged before the shape, so a skipped event takes no sequence number
/// (the server's send loop). A path notice without `previous` -- a route's
/// begin, a routed peer's reconnect -- is no longer judged apart: 2.4
/// brought it, and every 3.0 connection is past 2.4 (`LOCAL-IPC.md`
/// §Version negotiation and phases).
#[must_use]
pub fn available_to(event: &SessionEvent, version: IpcVersion) -> bool {
    // The runtime's state is the `server_state` frame's, on every version.
    EventType::of_session(event).is_none_or(|kind| kind.available_at(version))
}

/// The event types, as the catalogue names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// `message.direct`.
    #[serde(rename = "message.direct")]
    MessageDirect,
    /// `message.broadcast`.
    #[serde(rename = "message.broadcast")]
    MessageBroadcast,
    /// `endpoint.lease_changed`.
    #[serde(rename = "endpoint.lease_changed")]
    LeaseChanged,
    /// `peer.disconnected`.
    #[serde(rename = "peer.disconnected")]
    PeerDisconnected,
    /// `peer.path_changed`.
    #[serde(rename = "peer.path_changed")]
    PathChanged,
}

impl EventType {
    /// Every event type, in catalogue order; `tests/schema_agreement.rs`
    /// holds it to the enum's variants.
    pub const ALL: [Self; 5] = [
        Self::MessageDirect,
        Self::MessageBroadcast,
        Self::LeaseChanged,
        Self::PeerDisconnected,
        Self::PathChanged,
    ];

    /// The type's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MessageDirect => "message.direct",
            Self::MessageBroadcast => "message.broadcast",
            Self::LeaseChanged => "endpoint.lease_changed",
            Self::PeerDisconnected => "peer.disconnected",
            Self::PathChanged => "peer.path_changed",
        }
    }

    /// The type a wire name names, if it is in the catalogue.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == name)
    }

    /// The IPC version that introduced the type: it is emitted only on a
    /// connection that negotiated this pair or a later one. Every type is
    /// [`Since::THREE_ZERO`]; `peer.path_changed`'s 2.1 is its history.
    #[must_use]
    pub const fn since(self) -> Since {
        match self {
            Self::MessageDirect
            | Self::MessageBroadcast
            | Self::LeaseChanged
            | Self::PeerDisconnected
            | Self::PathChanged => Since::THREE_ZERO,
        }
    }

    /// The catalogue type a session's event becomes on the wire, judged
    /// BEFORE its shape is: `None` for the runtime's state, which is the
    /// `server_state` frame's, never an event.
    #[must_use]
    pub const fn of_session(event: &SessionEvent) -> Option<Self> {
        match event {
            SessionEvent::Direct(_) => Some(Self::MessageDirect),
            SessionEvent::Broadcast(_) => Some(Self::MessageBroadcast),
            SessionEvent::Local(LocalSessionEvent::EndpointLeaseChanged { .. }) => {
                Some(Self::LeaseChanged)
            }
            SessionEvent::Local(LocalSessionEvent::PeerDisconnected { .. }) => {
                Some(Self::PeerDisconnected)
            }
            SessionEvent::Local(LocalSessionEvent::PeerPathChanged { .. }) => {
                Some(Self::PathChanged)
            }
            SessionEvent::Local(LocalSessionEvent::ServerState { .. }) => None,
        }
    }

    /// Whether a connection at `version` may be sent this type, the pair
    /// compared as a pair ([`Since::admits`]).
    #[must_use]
    pub const fn available_at(self, version: IpcVersion) -> bool {
        self.since().admits(version)
    }
}

/// The literal `"direct"` of a direct message's `mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectMode {
    /// The only legal value.
    #[serde(rename = "direct")]
    Direct,
}

/// The literal `"broadcast"` of a broadcast's `mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BroadcastMode {
    /// The only legal value.
    #[serde(rename = "broadcast")]
    Broadcast,
}

/// `endpoints:message-received`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectReceived {
    /// The sender's idempotency key.
    pub message_id: MessageId,
    /// Always `"direct"`.
    pub mode: DirectMode,
    /// The authenticated remote peer.
    pub source_peer: TransportIdentity,
    /// PEER-ASSERTED: proves only that the peer claimed this endpoint.
    #[serde(deserialize_with = "crate::derived::endpoint")]
    pub source_endpoint: EndpointId,
    /// The local endpoint that accepted it. Never absent.
    #[serde(deserialize_with = "crate::derived::endpoint")]
    pub destination_endpoint: EndpointId,
    /// The application bytes and their advisory media type.
    pub payload: Payload,
    /// Local receipt time in Unix milliseconds.
    pub received_at: u64,
}

impl From<ReceivedDirect> for DirectReceived {
    fn from(message: ReceivedDirect) -> Self {
        Self {
            message_id: message.message_id,
            mode: DirectMode::Direct,
            source_peer: message.source_peer,
            source_endpoint: message.source_endpoint,
            destination_endpoint: message.destination_endpoint,
            payload: message.payload,
            received_at: message.received_at_ms,
        }
    }
}

/// `ipc:broadcast-received`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastReceived {
    /// The publisher's message id.
    pub message_id: MessageId,
    /// Always `"broadcast"`.
    pub mode: BroadcastMode,
    /// The authenticated original publisher.
    pub source_peer: TransportIdentity,
    /// The channel, from the topic.
    pub channel: ChannelId,
    /// The application bytes and their advisory media type.
    pub payload: Payload,
    /// Unix milliseconds at ingest, the daemon's clock.
    pub received_at: u64,
}

impl From<ReceivedBroadcast> for BroadcastReceived {
    fn from(message: ReceivedBroadcast) -> Self {
        Self {
            message_id: message.message_id,
            mode: BroadcastMode::Broadcast,
            source_peer: message.source_peer,
            channel: message.channel,
            payload: message.payload,
            received_at: message.received_at_ms,
        }
    }
}

/// `ipc:lease-changed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseChanged {
    /// The endpoint whose lease ended.
    #[serde(deserialize_with = "crate::derived::endpoint")]
    pub endpoint: EndpointId,
    /// The epoch that is no longer valid.
    pub revoked_epoch: Generation,
}

/// `ipc:path-changed`, `peer.path_changed`'s data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathChanged {
    /// Which peer.
    pub peer: TransportIdentity,
    /// The path before: the pending notice's, when one was replaced.
    /// ABSENT, never `null`, when the route began or the routed peer
    /// connected again.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::frame::absent_or"
    )]
    pub previous: Option<PeerPath>,
    /// The path now.
    pub current: PeerPath,
    /// The runtime's class for the change, 1..=128 characters.
    #[serde(deserialize_with = "reason_class")]
    pub reason_class: String,
    /// Local wall-clock milliseconds of the newest change it carries.
    pub observed_at: u64,
}

/// `peer.disconnected`'s data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerDisconnected {
    /// Which peer.
    pub peer: TransportIdentity,
    /// Coarse class, 1..=128 characters; `policy` for a trust revocation.
    #[serde(deserialize_with = "reason_class")]
    pub reason_class: String,
}

impl Event {
    /// The event's type.
    #[must_use]
    pub const fn event_type(&self) -> EventType {
        match self {
            Self::MessageDirect(_) => EventType::MessageDirect,
            Self::MessageBroadcast(_) => EventType::MessageBroadcast,
            Self::LeaseChanged(_) => EventType::LeaseChanged,
            Self::PeerDisconnected(_) => EventType::PeerDisconnected,
            Self::PathChanged(_) => EventType::PathChanged,
        }
    }

    /// The event a session's event becomes on the wire; `None` for the
    /// runtime's state, which is not in the catalogue: it travels as the
    /// `server_state` frame, which the server sends from its own view.
    ///
    /// # Errors
    /// [`TransportError::Internal`] for a reason class outside its
    /// bounds: the runtime named it, and a server that truncated it would
    /// send something the runtime did not say.
    pub fn from_session(event: SessionEvent) -> Result<Option<Self>, TransportError> {
        Ok(Some(match event {
            SessionEvent::Direct(message) => Self::MessageDirect(message.into()),
            SessionEvent::Broadcast(message) => Self::MessageBroadcast(message.into()),
            SessionEvent::Local(LocalSessionEvent::EndpointLeaseChanged {
                endpoint,
                revoked_epoch,
            }) => Self::LeaseChanged(LeaseChanged {
                endpoint,
                revoked_epoch,
            }),
            SessionEvent::Local(LocalSessionEvent::PeerDisconnected { peer, reason_class }) => {
                if reason_class.is_empty() || reason_class.chars().count() > MAX_REASON_CLASS_CHARS
                {
                    return Err(TransportError::Internal);
                }
                Self::PeerDisconnected(PeerDisconnected { peer, reason_class })
            }
            SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                peer,
                previous,
                current,
                reason_class,
                observed_at,
            }) => {
                if reason_class.is_empty() || reason_class.chars().count() > MAX_REASON_CLASS_CHARS
                {
                    return Err(TransportError::Internal);
                }
                Self::PathChanged(PathChanged {
                    peer,
                    previous,
                    current,
                    reason_class,
                    observed_at,
                })
            }
            SessionEvent::Local(LocalSessionEvent::ServerState { .. }) => return Ok(None),
        }))
    }

    /// Bind `data` to `event_type`'s shape, on a connection that
    /// negotiated `version`.
    ///
    /// # Errors
    /// [`TransportError::ProtocolViolation`] for a type outside the
    /// catalogue, one introduced after `version` (LOCAL-IPC.md §Version
    /// negotiation: a type is accepted only at or after the version that
    /// introduced it), a missing `data`, or data not of the type's shape:
    /// the catalogue is closed, so each is the server's fault
    /// (`an_event_before_its_version_is_the_servers_violation`).
    pub fn decode(
        event_type: &str,
        data: Option<&RawValue>,
        version: IpcVersion,
    ) -> Result<Self, TransportError> {
        fn typed<T: serde::de::DeserializeOwned>(data: &RawValue) -> Result<T, TransportError> {
            crate::strict::from_str(data.get()).map_err(|_| TransportError::ProtocolViolation)
        }
        let kind = EventType::parse(event_type)
            .filter(|kind| kind.available_at(version))
            .ok_or(TransportError::ProtocolViolation)?;
        let data = data.ok_or(TransportError::ProtocolViolation)?;
        Ok(match kind {
            EventType::MessageDirect => Self::MessageDirect(typed(data)?),
            EventType::MessageBroadcast => Self::MessageBroadcast(typed(data)?),
            EventType::LeaseChanged => Self::LeaseChanged(typed(data)?),
            EventType::PeerDisconnected => Self::PeerDisconnected(typed(data)?),
            EventType::PathChanged => Self::PathChanged(typed(data)?),
        })
    }

    /// The session's event this wire event is: the inverse of
    /// [`Event::from_session`], for a client binding (`ipc-client`).
    #[must_use]
    pub fn into_session(self) -> SessionEvent {
        match self {
            Self::MessageDirect(message) => SessionEvent::Direct(ReceivedDirect {
                source_peer: message.source_peer,
                source_endpoint: message.source_endpoint,
                destination_endpoint: message.destination_endpoint,
                message_id: message.message_id,
                payload: message.payload,
                received_at_ms: message.received_at,
            }),
            Self::MessageBroadcast(message) => SessionEvent::Broadcast(ReceivedBroadcast {
                source_peer: message.source_peer,
                channel: message.channel,
                message_id: message.message_id,
                payload: message.payload,
                received_at_ms: message.received_at,
            }),
            Self::LeaseChanged(changed) => {
                SessionEvent::Local(LocalSessionEvent::EndpointLeaseChanged {
                    endpoint: changed.endpoint,
                    revoked_epoch: changed.revoked_epoch,
                })
            }
            Self::PeerDisconnected(gone) => {
                SessionEvent::Local(LocalSessionEvent::PeerDisconnected {
                    peer: gone.peer,
                    reason_class: gone.reason_class,
                })
            }
            Self::PathChanged(changed) => SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                peer: changed.peer,
                previous: changed.previous,
                current: changed.current,
                reason_class: changed.reason_class,
                observed_at: changed.observed_at,
            }),
        }
    }

    /// The `event` frame carrying this event at `sequence`.
    ///
    /// # Panics
    /// Never: every body serializes.
    #[must_use]
    pub fn into_frame(self, sequence: u64) -> EventFrame {
        let data = match &self {
            Self::MessageDirect(d) => serde_json::value::to_raw_value(d),
            Self::MessageBroadcast(d) => serde_json::value::to_raw_value(d),
            Self::LeaseChanged(d) => serde_json::value::to_raw_value(d),
            Self::PeerDisconnected(d) => serde_json::value::to_raw_value(d),
            Self::PathChanged(d) => serde_json::value::to_raw_value(d),
        }
        .unwrap_or_else(|_| unreachable!("an event body serializes"));
        EventFrame {
            frame_type: EventTag::Event,
            sequence,
            event_type: self.event_type().as_str().to_owned(),
            data: Some(data),
        }
    }
}

/// The literal `"event"` discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventTag {
    /// The only legal value.
    #[serde(rename = "event")]
    Event,
}

/// An `event` frame as it crosses the wire, before its body is bound.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventFrame {
    /// Always `"event"`.
    #[serde(rename = "type")]
    pub frame_type: EventTag,
    /// Per connection, for gap detection only: not a replay cursor.
    pub sequence: u64,
    /// The type's wire name, judged by [`Event::decode`].
    pub event_type: String,
    /// The body as it arrived, judged against the type's shape.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::raw::absent_or_object"
    )]
    pub data: Option<Box<RawValue>>,
}

impl EventFrame {
    /// The event this frame carries, on a connection that negotiated
    /// `version`.
    ///
    /// # Errors
    /// As [`Event::decode`].
    pub fn event(&self, version: IpcVersion) -> Result<Event, TransportError> {
        Event::decode(&self.event_type, self.data.as_deref(), version)
    }
}

fn reason_class<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let class = String::deserialize(d)?;
    if class.is_empty() || class.chars().count() > MAX_REASON_CLASS_CHARS {
        return Err(serde::de::Error::custom(format!(
            "a reason class is 1..={MAX_REASON_CLASS_CHARS} characters"
        )));
    }
    Ok(class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

    fn peer() -> TransportIdentity {
        TransportIdentity::parse(PEER).expect("peer")
    }

    fn ep(name: &str) -> EndpointId {
        interweave_test_support::endpoint_keys::endpoint_id(name)
    }

    fn payload() -> Payload {
        Payload::at_ceiling(None, b"hi".to_vec()).expect("payload")
    }

    fn every_session_event() -> Vec<SessionEvent> {
        vec![
            SessionEvent::Direct(ReceivedDirect {
                source_peer: peer(),
                source_endpoint: ep("bot"),
                destination_endpoint: ep("human"),
                message_id: MessageId::from_bytes([1; 16]),
                payload: payload(),
                received_at_ms: 7,
            }),
            SessionEvent::Broadcast(ReceivedBroadcast {
                source_peer: peer(),
                channel: ChannelId::parse("ops").expect("channel"),
                message_id: MessageId::from_bytes([2; 16]),
                payload: payload(),
                received_at_ms: 8,
            }),
            SessionEvent::Local(LocalSessionEvent::EndpointLeaseChanged {
                endpoint: ep("human"),
                revoked_epoch: Generation::parse("AAAAAAAAAAAAAAAA").expect("epoch"),
            }),
            SessionEvent::Local(LocalSessionEvent::PeerDisconnected {
                peer: peer(),
                reason_class: "policy".into(),
            }),
            SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                peer: peer(),
                previous: Some(interweave_transport_api::PeerPath::Relayed),
                current: interweave_transport_api::PeerPath::Direct,
                reason_class: "dcutr".into(),
                observed_at: 9,
            }),
        ]
    }

    /// What a client reads back is what the session gave the server, for
    /// every type: `into_session` inverts `from_session`.
    #[test]
    fn into_session_inverts_from_session_for_every_type() {
        let events = every_session_event();
        let kinds: std::collections::BTreeSet<EventType> = events
            .iter()
            .map(|e| {
                Event::from_session(e.clone())
                    .expect("encodes")
                    .expect("a catalogue event")
                    .event_type()
            })
            .collect();
        assert_eq!(kinds.len(), EventType::ALL.len(), "every type is exercised");
        for event in events {
            let wire = Event::from_session(event.clone())
                .expect("encodes")
                .expect("a catalogue event");
            assert_eq!(wire.into_session(), event);
        }
    }

    #[test]
    fn every_session_event_round_trips_through_its_frame() {
        let events: Vec<Event> = every_session_event()
            .into_iter()
            .map(|e| Event::from_session(e).expect("maps").expect("an event"))
            .collect();
        assert_eq!(
            events.iter().map(Event::event_type).collect::<Vec<_>>(),
            EventType::ALL,
            "one event per type"
        );
        for (sequence, event) in (0_u64..).zip(events) {
            let wire = serde_json::to_string(&event.clone().into_frame(sequence)).expect("ser");
            let back: EventFrame = serde_json::from_str(&wire).expect("de");
            assert_eq!(back.sequence, sequence);
            assert_eq!(back.event(crate::supported()[0]), Ok(event));
        }
    }

    #[test]
    fn a_direct_message_names_its_mode_and_its_receipt_time() {
        let [SessionEvent::Direct(message), ..] = &every_session_event()[..] else {
            unreachable!()
        };
        let data = serde_json::to_value(DirectReceived::from(message.clone())).expect("ser");
        assert_eq!(data["mode"], json!("direct"));
        assert_eq!(data["received_at"], json!(7));
        assert!(data.get("received_at_ms").is_none());
    }

    #[test]
    fn an_unknown_type_or_a_malformed_body_is_the_servers_violation() {
        let frame = |value: serde_json::Value| {
            serde_json::from_str::<EventFrame>(&value.to_string()).expect("envelope")
        };
        for value in [
            json!({"type": "event", "sequence": 0, "event_type": "peer.connected", "data": {}}),
            json!({"type": "event", "sequence": 0, "event_type": "peer.disconnected"}),
            json!({"type": "event", "sequence": 0, "event_type": "peer.disconnected",
                   "data": {"peer": PEER, "reason_class": ""}}),
            json!({"type": "event", "sequence": 0, "event_type": "peer.disconnected",
                   "data": {"peer": PEER, "reason_class": "x".repeat(129)}}),
        ] {
            assert_eq!(
                frame(value.clone()).event(crate::supported()[0]),
                Err(TransportError::ProtocolViolation),
                "{value}"
            );
        }
    }

    /// Versions are additive in both directions: a type the server may
    /// not emit before its version, the client may not accept before it
    /// either. Every type is 3.0's, so each is read at 3.0 and refused at
    /// 2.13 -- whose minor is the higher, the case a comparison of the
    /// minor alone reads.
    #[test]
    fn an_event_before_its_version_is_the_servers_violation() {
        let at = |major, minor| IpcVersion { major, minor };
        for session in every_session_event() {
            let event = Event::from_session(session.clone())
                .expect("maps")
                .expect("an event");
            assert_eq!(event.event_type().since(), Since::THREE_ZERO);
            let frame = event.clone().into_frame(0);
            assert_eq!(frame.event(at(3, 0)), Ok(event), "read at 3.0");
            assert!(available_to(&session, at(3, 0)));
            for below in [at(2, 13), at(2, 0)] {
                assert_eq!(
                    frame.event(below),
                    Err(TransportError::ProtocolViolation),
                    "{} refused at {below:?}",
                    frame.event_type
                );
                assert!(!available_to(&session, below));
            }
        }
    }

    /// A route's begin, `previous` absent, is read and sent at 3.0: the
    /// shape 2.4 brought is the one 3.0 holds.
    #[test]
    fn a_route_notice_without_previous_is_a_three_zero_event() {
        let begun = SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
            peer: peer(),
            previous: None,
            current: interweave_transport_api::PeerPath::Relayed,
            reason_class: "route_established".into(),
            observed_at: 9,
        });
        let version = crate::supported()[0];
        assert!(available_to(&begun, version));
        let event = Event::from_session(begun).expect("maps").expect("an event");
        assert_eq!(event.clone().into_frame(0).event(version), Ok(event));
    }

    /// The runtime's state is no catalogue event: it is the
    /// `server_state` frame's, so it maps to nothing rather than to an
    /// error a server would count as a numbered gap.
    #[test]
    fn the_runtimes_state_is_not_a_catalogue_event() {
        assert_eq!(
            Event::from_session(SessionEvent::Local(LocalSessionEvent::ServerState {
                health: interweave_transport_api::Health::Healthy,
                connectivity: None,
            })),
            Ok(None)
        );
    }

    #[test]
    fn a_reason_class_out_of_bounds_is_refused_before_the_wire() {
        for class in [String::new(), "x".repeat(MAX_REASON_CLASS_CHARS + 1)] {
            assert_eq!(
                Event::from_session(SessionEvent::Local(LocalSessionEvent::PeerDisconnected {
                    peer: peer(),
                    reason_class: class,
                })),
                Err(TransportError::Internal)
            );
        }
    }
}

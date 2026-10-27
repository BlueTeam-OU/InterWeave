// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A generic automated TEST client: what ADR-0055 decision 1 makes an
//! automated client, and enough of what decision 3 says such a client owes
//! itself to prove decision 2 against real daemons. It carries no protocol:
//! the endpoint, the client kind and the channel are the case's
//! configuration ([`ClientConfig`]), and the payload is opaque bytes.
//! Nothing a production build may depend on.
//!
//! What it does:
//! - holds the lease of its configured endpoint as its configured client
//!   kind and joins its channel, and when its daemon goes away opens a
//!   fresh session and joins again;
//! - sends a direct message under ONE `message_id` for every attempt,
//!   retrying the refusals that a later attempt can mend, and gives up a
//!   message only at the caller's deadline;
//! - keeps every inbound message before anything reads it, and collapses
//!   a second copy of a `message_id` it already holds, keeping the first
//!   and counting the other: the transport suppresses duplicates only
//!   within its bounded, in-memory window (ADR-0019), so past it, or past
//!   a receiving daemon's restart, a same-id retry is a second delivery
//!   that only the receiving client can fold (decision 3).
//!
//! What it does not do: its record is in memory, not durable, and its
//! retry pause is a test's (milliseconds); a real client's are its own.

#![allow(clippy::expect_used)]

use std::collections::HashSet;
use std::time::Duration;

use interweave_ipc_client::{IpcBinding, IpcSession};
use interweave_local_client_api::{
    DataCapability, DataSessionBinding as _, DataSessionPort as _, SessionEvent, SessionRequest,
};
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, DirectDestination, EndpointId, MediaType, MessageId, Payload,
    TransportError, TransportIdentity,
};

/// What a client claims: the endpoint it leases, the client kind it
/// presents, and the channel it joins. Any names a deployment chooses;
/// the transport gives none of them a meaning.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// The endpoint leased, and the one a direct send addresses at the
    /// peer: a deployment renders the same endpoint on every profile.
    pub endpoint: EndpointId,
    /// The client kind presented; the profile allows it on the endpoint.
    pub client_kind: String,
    /// The channel joined and published on.
    pub channel: ChannelId,
}

impl ClientConfig {
    /// A configuration from its three names.
    ///
    /// # Panics
    /// On a name the transport's grammar refuses: a case's own mistake.
    #[must_use]
    pub fn new(endpoint: &str, client_kind: &str, channel: &str) -> Self {
        Self {
            endpoint: EndpointId::parse(endpoint).expect("an endpoint id"),
            client_kind: client_kind.to_owned(),
            channel: ChannelId::parse(channel).expect("a channel id"),
        }
    }
}

/// `text` as an opaque UTF-8 payload.
///
/// # Panics
/// On text past the payload ceiling: a client refuses it before any send.
#[must_use]
pub fn payload(text: &str) -> Payload {
    Payload::at_ceiling(
        Some(MediaType::parse("text/plain; charset=utf-8").expect("a media type")),
        text.as_bytes().to_vec(),
    )
    .expect("within the ceiling")
}

/// Whether a later attempt under the same `message_id` can mend `e`
/// (ADR-0055 decision 2). `UnauthorizedPeer` and `InvalidArgument` are
/// the sender's to see, never retried.
#[must_use]
pub fn retryable(e: TransportError) -> bool {
    matches!(
        e,
        TransportError::RemoteEndpointUnavailable
            | TransportError::PeerUnreachable
            | TransportError::PeerUnknown
            | TransportError::Overloaded
            | TransportError::Timeout
    )
}

/// Whether `e` says the SESSION is gone, not the send: the socket closed
/// (`BackendUnavailable`), or the daemon closed it on its way down -- a
/// clean stop sends `close` with `ShuttingDown`, and every later call on
/// that session answers the same. Either way the client opens a fresh
/// session (lease and joins) before the next attempt.
#[must_use]
pub fn session_lost(e: TransportError) -> bool {
    matches!(
        e,
        TransportError::BackendUnavailable | TransportError::ShuttingDown
    )
}

/// One inbound message, as the client's record holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inbound {
    /// The authenticated peer it came from.
    pub source: TransportIdentity,
    /// Its `message_id`.
    pub message_id: MessageId,
    /// Whether it was a broadcast, not a direct message.
    pub broadcast: bool,
    /// The payload's bytes.
    pub bytes: Vec<u8>,
}

/// What the client has taken from its daemon, in order.
#[derive(Debug, Default)]
pub struct Received {
    /// Every first copy, in arrival order.
    pub inbound: Vec<Inbound>,
    /// Second copies of a `message_id` already held, counted and
    /// never offered again.
    pub duplicates: usize,
    seen: HashSet<(bool, MessageId)>,
}

impl Received {
    fn append(&mut self, message: Inbound) {
        if self.seen.insert((message.broadcast, message.message_id)) {
            self.inbound.push(message);
        } else {
            self.duplicates += 1;
        }
    }
}

/// How a send ended: the attempts it took and their refusals, in order.
#[derive(Debug)]
pub struct Sent {
    /// The refusals before the last attempt, each retried.
    pub retried: Vec<TransportError>,
    /// The last attempt's answer.
    pub outcome: Result<EndpointId, TransportError>,
}

/// The automated client of one profile.
pub struct Client {
    config: ClientConfig,
    binding: IpcBinding,
    session: Option<IpcSession>,
    /// What it has taken from its daemon.
    pub received: Received,
    /// Sessions opened: 1 at start, one more per reconnect.
    pub sessions_opened: usize,
}

/// The pause between two attempts or reconnects: a test's, not a
/// production schedule.
const STEP: Duration = Duration::from_millis(100);

impl Client {
    /// A client of the daemon behind `binding`, not yet connected.
    #[must_use]
    pub fn new(config: ClientConfig, binding: IpcBinding) -> Self {
        Self {
            config,
            binding,
            session: None,
            received: Received::default(),
            sessions_opened: 0,
        }
    }

    /// Lease the endpoint and join the channel, retrying until `patience`
    /// runs out.
    ///
    /// # Errors
    /// The last refusal, when `patience` ran out first.
    pub async fn connect(&mut self, patience: Duration) -> Result<(), TransportError> {
        let deadline = tokio::time::Instant::now() + patience;
        loop {
            match self.open().await {
                Ok(session) => {
                    self.session = Some(session);
                    self.sessions_opened += 1;
                    return Ok(());
                }
                Err(e) if tokio::time::Instant::now() >= deadline => return Err(e),
                Err(_) => tokio::time::sleep(STEP).await,
            }
        }
    }

    async fn open(&self) -> Result<IpcSession, TransportError> {
        let request = SessionRequest::new(
            self.config.client_kind.as_str(),
            Some(self.config.endpoint.clone()),
            [DataCapability::Events, DataCapability::Commands],
        )
        .map_err(|_| TransportError::InvalidArgument)?;
        let session = self.binding.open(request).await?;
        session.join(self.config.channel.clone()).await?;
        Ok(session)
    }

    /// Give the lease up, as a client that stops does: the session closed.
    pub fn disconnect(&mut self) {
        self.session = None;
    }

    /// Whether a session is held.
    #[must_use]
    pub fn connected(&self) -> bool {
        self.session.is_some()
    }

    fn session(&self) -> Result<&IpcSession, TransportError> {
        self.session
            .as_ref()
            .ok_or(TransportError::EndpointNotRegistered)
    }

    /// Take what the daemon holds for this client into its record. A
    /// session that failed is dropped and a fresh one opened -- a fresh
    /// lease and a fresh join -- within `patience`; what the failed one
    /// held is lost with it (ADR-0055 decision 2).
    ///
    /// # Errors
    /// When no session could be opened again within `patience`.
    pub async fn pump(&mut self, patience: Duration) -> Result<(), TransportError> {
        let taken = match &self.session {
            Some(session) => session.events(usize::MAX).await,
            None => return Ok(()),
        };
        let Ok(events) = taken else {
            self.session = None;
            return self.connect(patience).await;
        };
        for event in events {
            match event {
                SessionEvent::Direct(m) => self.received.append(Inbound {
                    source: m.source_peer,
                    message_id: m.message_id,
                    broadcast: false,
                    bytes: m.payload.bytes().to_vec(),
                }),
                SessionEvent::Broadcast(m) => self.received.append(Inbound {
                    source: m.source_peer,
                    message_id: m.message_id,
                    broadcast: true,
                    bytes: m.payload.bytes().to_vec(),
                }),
                SessionEvent::Local(_) => {}
            }
        }
        Ok(())
    }

    /// Pump until `done` holds of the record or `patience` runs out;
    /// whether it held.
    pub async fn pump_until(
        &mut self,
        patience: Duration,
        done: impl Fn(&Received) -> bool,
    ) -> bool {
        let deadline = tokio::time::Instant::now() + patience;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            let _ = self.pump(left.max(STEP)).await;
            if done(&self.received) {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    /// One attempt of a direct send to `peer`'s configured endpoint.
    ///
    /// # Errors
    /// The daemon's refusal, or `EndpointNotRegistered` with no session.
    pub async fn send_once(
        &self,
        peer: &TransportIdentity,
        message_id: MessageId,
        text: &str,
    ) -> Result<EndpointId, TransportError> {
        self.session()?
            .send_direct(
                DirectDestination {
                    peer: peer.clone(),
                    endpoint: Some(self.config.endpoint.clone()),
                },
                message_id,
                payload(text),
            )
            .await
    }

    /// One attempt of a direct send of `payload` as given -- its own media
    /// type and bytes -- to `endpoint` at `peer`, for a case that needs a
    /// frame another kind of client would read.
    ///
    /// # Errors
    /// The daemon's refusal, or `EndpointNotRegistered` with no session.
    pub async fn send_payload_once(
        &self,
        peer: &TransportIdentity,
        endpoint: EndpointId,
        message_id: MessageId,
        payload: Payload,
    ) -> Result<EndpointId, TransportError> {
        self.session()?
            .send_direct(
                DirectDestination {
                    peer: peer.clone(),
                    endpoint: Some(endpoint),
                },
                message_id,
                payload,
            )
            .await
    }

    /// A direct send under one `message_id`, every retryable refusal
    /// retried until it is accepted or `patience` runs out; a refusal no
    /// retry mends ends it at once. A session lost on the way is opened
    /// again before the next attempt.
    pub async fn send(
        &mut self,
        peer: &TransportIdentity,
        message_id: MessageId,
        text: &str,
        patience: Duration,
    ) -> Sent {
        let deadline = tokio::time::Instant::now() + patience;
        let mut retried = Vec::new();
        loop {
            // A reconnect that fails is the attempt's refusal: retried
            // like any other, rather than falling through to a send on no
            // session, whose `EndpointNotRegistered` no retry mends.
            let reconnect = match self.session {
                None => self.connect(STEP).await.err(),
                Some(_) => None,
            };
            let outcome = match reconnect {
                Some(e) => Err(e),
                None => self.send_once(peer, message_id, text).await,
            };
            match outcome {
                Err(e)
                    if (retryable(e) || session_lost(e))
                        && tokio::time::Instant::now() < deadline =>
                {
                    if session_lost(e) {
                        self.session = None;
                    }
                    retried.push(e);
                    tokio::time::sleep(STEP).await;
                }
                outcome => return Sent { retried, outcome },
            }
        }
    }

    /// Publish `text` on the channel under `message_id`.
    ///
    /// # Errors
    /// The daemon's refusal.
    pub async fn publish(&self, message_id: MessageId, text: &str) -> Result<(), TransportError> {
        self.session()?
            .broadcast(
                self.config.channel.clone(),
                BroadcastMessageV1 {
                    message_id,
                    sent_at_ms: 0,
                    payload: payload(text),
                },
            )
            .await
    }
}

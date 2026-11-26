// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Why a session re-opens on its timer, as a root reads it
//! (`TransportClient::retrying_for_room`): the daemon's `Overloaded` is
//! "no room at the daemon" (design/2026-11-04-identities.md §5.2), any
//! other cause is not. Over the fake node, with a data binding that
//! answers each open as scripted.
//!
//! What this does NOT prove: the assertion's `Overloaded`, which is
//! proved over the scripted admin port in `tests/hiding.rs`
//! (`no_daemon_to_answer_is_reconnecting_and_the_next_reconnect_asserts`);
//! the accessor reads whichever answer put the session on the timer.
//!
//! Every key here is TEST-ONLY (`interweave_test_support::endpoint_keys`).

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex};

use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, SessionState, TransportClient};
use interweave_local_client_api::{DataSessionBinding, SessionRequest};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::{EndpointId, TransportError};

fn human() -> EndpointId {
    endpoint_id("human")
}

/// The fake node's opens, each answered by the next scripted error
/// first, then by the node.
#[derive(Clone)]
struct Scripted {
    node: FakeNode,
    errors: Arc<Mutex<VecDeque<TransportError>>>,
}

impl DataSessionBinding for Scripted {
    type Session = <FakeNode as DataSessionBinding>::Session;

    fn open(
        &self,
        request: SessionRequest,
    ) -> impl Future<Output = Result<Self::Session, TransportError>> + Send {
        let error = self.errors.lock().unwrap().pop_front();
        let node = self.node.clone();
        async move {
            match error {
                Some(error) => Err(error),
                None => node.open(request).await,
            }
        }
    }
}

fn node() -> FakeNode {
    let config = || FakeConfig {
        peer: ProfileIdentity::generate()
            .transport_identity()
            .expect("peer id"),
        endpoints: vec![FakeEndpoint::open(human(), false)],
        default_endpoint: Some(human()),
        queue_bound: 16,
    };
    FakeNetwork::pair(config(), config()).0
}

fn next_at(client: &TransportClient<Scripted, FakeNode>) -> u64 {
    match client.session_state() {
        SessionState::Reconnecting { next_at, .. } => *next_at,
        other => panic!("re-opening: {other:?}"),
    }
}

/// An open answered `Overloaded` re-opens on the timer and reads as
/// retrying for room; one answered `Timeout` re-opens too and does not; a
/// refusal after `Overloaded`, left by Try again, re-opens for the
/// person and does not; a lease taken reads as neither. Mutations:
/// `open_failed` not keeping the cause -- the first assertion fails;
/// `set_state` not clearing it -- the Try again's re-open still reads
/// `Overloaded`'s.
#[tokio::test]
async fn retrying_for_room_names_an_overloaded_retry_alone() {
    let node = node();
    let errors = Arc::new(Mutex::new(VecDeque::from([
        TransportError::Overloaded,
        TransportError::Timeout,
        TransportError::Overloaded,
        TransportError::EndpointInUse,
    ])));
    let mut client = TransportClient::new(
        Scripted {
            node: node.clone(),
            errors,
        },
        node,
        HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store"),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![human()],
            channels: Vec::new(),
            max_payload_bytes: 49_152,
        },
        Box::new(|| 1_786_600_000_000),
        0,
    )
    .expect("an empty store")
    .without_status();
    assert!(!client.retrying_for_room(), "nothing asked yet");

    client.tick(0).await;
    let due = next_at(&client);
    assert!(client.retrying_for_room(), "{:?}", client.session_state());

    client.tick(due).await;
    let due = next_at(&client);
    assert!(
        !client.retrying_for_room(),
        "a retry for another cause: {:?}",
        client.session_state()
    );

    client.tick(due).await;
    let due = next_at(&client);
    assert!(client.retrying_for_room(), "{:?}", client.session_state());
    client.tick(due).await;
    assert!(
        matches!(client.session_state(), SessionState::Refused { .. }),
        "{:?}",
        client.session_state()
    );
    assert!(!client.retrying_for_room(), "refused");
    client.reopen(due);
    assert!(
        matches!(client.session_state(), SessionState::Reconnecting { .. }),
        "{:?}",
        client.session_state()
    );
    assert!(!client.retrying_for_room(), "Try again's re-open");

    client.tick(due).await;
    assert!(
        matches!(client.session_state(), SessionState::Ready { .. }),
        "{:?}",
        client.session_state()
    );
    assert!(!client.retrying_for_room());
}

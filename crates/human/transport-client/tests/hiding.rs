// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The advertise assertion made before every lease (ADR-0061 decision 5;
//! architect-cto's ruling 01a216b6, 2026-11-26), over a scripted admin
//! port that writes one log with the data binding beside it: what the
//! facade asked, on which connection, and in which order. Isolation
//! only: what `set_advertise` does in the runtime is proved beside it
//! (`crates/transport/composition/tests/endpoint_set.rs`), not here.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::{BTreeSet, VecDeque};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;

use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{
    ClientConfig, SessionProblem, SessionState, TransportClient,
};
use interweave_local_client_api::{
    AdminBinding, AdminCapability, AdminPort, AdminStatus, DataSessionBinding, EndpointAdminView,
    Generation, LocalAdminPort, SessionRequest, TrustAdminView, TrustChange,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};

fn human() -> EndpointId {
    endpoint_id("human")
}

/// One thing the facade did, in the order it did it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    /// An admin port opened, its number and what it holds.
    Opened(u64, BTreeSet<AdminCapability>),
    /// `set_advertise` asked on port `.0`.
    Asserted(u64, EndpointId, bool),
    /// The assertion's answer handed back on port `.0`.
    Answered(u64),
    /// Port `.0` dropped: its connection closed.
    Closed(u64),
    /// A data session opened, leasing this endpoint.
    Leased(Option<EndpointId>),
}

/// How the next assertion is answered.
#[derive(Debug, Clone, Copy)]
enum Answer {
    Ok,
    Fails(TransportError),
    /// Unanswered until [`Script::released`].
    Holds,
}

#[derive(Default)]
struct Script {
    steps: Vec<Step>,
    /// The port refuses to open with this.
    open_fails: Option<TransportError>,
    /// The next assertions' answers; once empty, `Ok`.
    answers: VecDeque<Answer>,
    /// A held assertion answers `Ok` once this is set.
    released: bool,
    ports: u64,
}

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Script>>);

impl Log {
    fn with<T>(&self, f: impl FnOnce(&mut Script) -> T) -> T {
        f(&mut self.0.lock().unwrap())
    }

    fn steps(&self) -> Vec<Step> {
        self.with(|s| s.steps.clone())
    }

    fn take(&self) -> Vec<Step> {
        self.with(|s| std::mem::take(&mut s.steps))
    }

    fn asserted(&self) -> usize {
        self.steps()
            .iter()
            .filter(|s| matches!(s, Step::Asserted(..)))
            .count()
    }

    fn leased(&self) -> usize {
        self.steps()
            .iter()
            .filter(|s| matches!(s, Step::Leased(_)))
            .count()
    }
}

/// The admin side: ports that answer `set_advertise` as scripted and
/// nothing else.
#[derive(Clone)]
struct Admins(Log);

struct ScriptedPort {
    number: u64,
    log: Log,
    port: LocalAdminPort,
}

impl Drop for ScriptedPort {
    fn drop(&mut self) {
        let number = self.number;
        self.log.with(|s| s.steps.push(Step::Closed(number)));
    }
}

impl AdminBinding for Admins {
    type Admin = ScriptedPort;

    fn admin(
        &self,
        capabilities: BTreeSet<AdminCapability>,
    ) -> impl Future<Output = Result<ScriptedPort, TransportError>> + Send {
        let log = self.0.clone();
        let opened = log.with(|s| {
            if let Some(error) = s.open_fails {
                return Err(error);
            }
            s.ports += 1;
            let number = s.ports;
            s.steps.push(Step::Opened(number, capabilities.clone()));
            Ok(number)
        });
        std::future::ready(opened.map(|number| ScriptedPort {
            number,
            log,
            port: LocalAdminPort::new(
                Generation::parse(format!("hiding-admin-port-{number}")).unwrap(),
                capabilities,
            ),
        }))
    }
}

impl AdminPort for ScriptedPort {
    fn port(&self) -> &LocalAdminPort {
        &self.port
    }
    fn set_endpoint_advertise(
        &self,
        endpoint: EndpointId,
        advertise: bool,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        let number = self.number;
        let log = self.log.clone();
        let held = !self.port.holds(AdminCapability::Endpoints);
        let answer = log.with(|s| {
            s.steps.push(Step::Asserted(number, endpoint, advertise));
            s.answers.pop_front().unwrap_or(Answer::Ok)
        });
        std::future::poll_fn(move |_| {
            // A port without the capability is refused as the runtime
            // refuses it, whatever the script says.
            let answer = if held {
                Err(TransportError::CapabilityDenied)
            } else {
                match answer {
                    Answer::Ok => Ok(()),
                    Answer::Fails(error) => Err(error),
                    Answer::Holds if log.with(|s| s.released) => Ok(()),
                    // No waker is kept: the facade polls at each tick.
                    Answer::Holds => return Poll::Pending,
                }
            };
            log.with(|s| s.steps.push(Step::Answered(number)));
            Poll::Ready(answer)
        })
    }
    fn status(&self) -> impl Future<Output = Result<AdminStatus, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn leases(
        &self,
    ) -> impl Future<Output = Result<Vec<EndpointAdminView>, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn revoke_endpoint(
        &self,
        _: EndpointId,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn set_endpoint_enabled(
        &self,
        _: EndpointId,
        _: bool,
    ) -> impl Future<Output = Result<Option<Generation>, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn set_default_endpoint(
        &self,
        _: Option<EndpointId>,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn trust(&self) -> impl Future<Output = Result<TrustAdminView, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn set_trust(
        &self,
        _: TransportIdentity,
        _: bool,
    ) -> impl Future<Output = Result<TrustChange, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn shutdown(&self, _: Duration) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
}

/// The data side: a fake node's, each open written to the same log.
#[derive(Clone)]
struct Leases {
    node: FakeNode,
    log: Log,
}

impl DataSessionBinding for Leases {
    type Session = <FakeNode as DataSessionBinding>::Session;

    fn open(
        &self,
        request: SessionRequest,
    ) -> impl Future<Output = Result<Self::Session, TransportError>> + Send {
        self.log
            .with(|s| s.steps.push(Step::Leased(request.endpoint().cloned())));
        let node = self.node.clone();
        async move { node.open(request).await }
    }
}

type Client = TransportClient<Leases, Admins>;

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

fn config() -> ClientConfig {
    ClientConfig {
        client_kind: "human-client".to_owned(),
        endpoints: vec![human()],
        channels: Vec::new(),
        max_payload_bytes: 49_152,
    }
}

fn store() -> HumanStore {
    HumanStore::open_in_memory(&human(), StoreOptions::default()).expect("store")
}

/// A graphical client's facade, reading no status so that the log holds
/// the assertion's connections alone.
fn graphical(node: &FakeNode, log: &Log) -> Client {
    TransportClient::new(
        Leases {
            node: node.clone(),
            log: log.clone(),
        },
        Admins(log.clone()),
        store(),
        config(),
        Box::new(|| 1_786_600_000_000),
        0,
    )
    .expect("an empty store")
    .without_status()
}

/// The order the ruling asks for, on one port: opened holding
/// `admin.endpoints` alone, asked `false` for the store's identity,
/// answered, closed, and only then the lease.
fn asserted_then_leased(number: u64) -> Vec<Step> {
    vec![
        Step::Opened(number, BTreeSet::from([AdminCapability::Endpoints])),
        Step::Asserted(number, human(), false),
        Step::Answered(number),
        Step::Closed(number),
        Step::Leased(Some(human())),
    ]
}

/// The lease is taken after the assertion's answer and not before, on a
/// short connection of its own that holds `admin.endpoints` alone and is
/// closed before the lease is asked. Mutations: the open moved ahead of
/// the assertion (a lease before the answer); the assertion made on a
/// port holding `admin.status`; the port kept past the answer.
#[tokio::test]
async fn the_assertion_answers_on_its_own_connection_before_the_lease() {
    let node = node();
    let log = Log::default();
    let mut client = graphical(&node, &log);
    client.tick(0).await;
    assert_eq!(log.steps(), asserted_then_leased(1));
    assert_eq!(
        client.session_state(),
        &SessionState::Ready {
            endpoint: Some(human())
        }
    );
}

/// A daemon that answered with anything but success refuses the lease as
/// `NotHidden`, and no timer asks again; Try again asserts again and,
/// on `Ok`, leases. A port the daemon refused to open -- it answered the
/// hello -- is the same refusal. Mutations: an answered error classified
/// as a retry; the open taken whatever the answer; `reopen` leaving the
/// assertion out.
#[tokio::test]
async fn an_answered_error_refuses_the_lease_and_try_again_asserts_again() {
    for (open_fails, answer) in [
        (None, Some(TransportError::Internal)),
        (None, Some(TransportError::CapabilityDenied)),
        (None, Some(TransportError::ProtocolUnsupported)),
        (None, Some(TransportError::InvalidArgument)),
        (None, Some(TransportError::EndpointUnknown)),
        (None, Some(TransportError::Overloaded)),
        (Some(TransportError::CapabilityDenied), None),
        (Some(TransportError::ProtocolUnsupported), None),
    ] {
        let node = node();
        let log = Log::default();
        log.with(|s| {
            s.open_fails = open_fails;
            s.answers.extend(answer.map(Answer::Fails));
        });
        let mut client = graphical(&node, &log);
        client.tick(0).await;
        let case = format!("{open_fails:?} {answer:?}");
        assert_eq!(
            client.session_state(),
            &SessionState::Refused {
                problem: SessionProblem::NotHidden
            },
            "{case}"
        );
        assert_eq!(log.leased(), 0, "no lease: {case}");
        // No timer leaves a refusal: an hour of ticks asks nothing.
        log.with(|s| s.open_fails = None);
        log.take();
        for now in (1..=3_600).step_by(60) {
            client.tick(now * 1_000).await;
        }
        assert!(log.steps().is_empty(), "nothing asked: {case}");
        // Try again: asserted again, and the lease follows its `Ok`. The
        // refused open made no port; a refused answer was on port 1.
        let next = u64::from(open_fails.is_none()) + 1;
        client.reopen(3_600_001);
        client.tick(3_600_001).await;
        assert_eq!(log.steps(), asserted_then_leased(next), "{case}");
        assert!(
            matches!(client.session_state(), SessionState::Ready { .. }),
            "{case}: {:?}",
            client.session_state()
        );
    }
}

/// No daemon to answer -- the admin connection cannot be made, or ends
/// before the answer -- is today's `Reconnecting`, never a refusal: no
/// lease is asked, and the next reconnect asserts again. Mutations: an
/// unreachable daemon classified `NotHidden`; the lease asked after a
/// failed assertion.
#[tokio::test]
async fn no_daemon_to_answer_is_reconnecting_and_the_next_reconnect_asserts() {
    for (open_fails, answer) in [
        (Some(TransportError::BackendUnavailable), Answer::Ok),
        (Some(TransportError::ShuttingDown), Answer::Ok),
        (None, Answer::Fails(TransportError::BackendUnavailable)),
        (None, Answer::Fails(TransportError::ShuttingDown)),
        (None, Answer::Fails(TransportError::Timeout)),
    ] {
        let node = node();
        let log = Log::default();
        log.with(|s| {
            s.open_fails = open_fails;
            s.answers.push_back(answer);
        });
        let mut client = graphical(&node, &log);
        client.tick(0).await;
        let case = format!("{open_fails:?} {answer:?}");
        let SessionState::Reconnecting { attempt, next_at } = client.session_state().clone() else {
            panic!("{case}: {:?}", client.session_state());
        };
        assert_eq!(attempt, 1, "{case}");
        assert_eq!(log.leased(), 0, "no lease: {case}");
        log.with(|s| {
            s.open_fails = None;
            s.answers.clear();
        });
        log.take();
        client.tick(next_at).await;
        let next = u64::from(open_fails.is_none()) + 1;
        assert_eq!(log.steps(), asserted_then_leased(next), "{case}");
    }
}

/// An assertion the daemon does not answer holds the lease, tick after
/// tick, and holds no tick: another client's turns, made between this
/// one's, open their own session meanwhile. Released, the next tick
/// leases. Mutations: the assertion awaited inside the tick (the tick
/// never returns, and the time-out says so); the lease asked while it is
/// pending.
#[tokio::test]
async fn an_assertion_that_blocks_takes_no_lease_until_it_answers() {
    let held_node = node();
    let held_log = Log::default();
    held_log.with(|s| s.answers.push_back(Answer::Holds));
    let mut held = graphical(&held_node, &held_log);
    let other_node = node();
    let other_log = Log::default();
    let mut other = graphical(&other_node, &other_log);
    for now in 0..20 {
        tokio::time::timeout(Duration::from_secs(5), held.tick(now * 1_000))
            .await
            .expect("a tick never waits for the assertion's answer");
        assert!(
            matches!(held.session_state(), SessionState::Reconnecting { .. }),
            "{:?}",
            held.session_state()
        );
        other.tick(now * 1_000).await;
    }
    assert_eq!(held_log.leased(), 0, "no lease while unanswered");
    assert_eq!(held_log.asserted(), 1, "one assertion, still waited for");
    assert!(
        matches!(other.session_state(), SessionState::Ready { .. }),
        "the other client went on: {:?}",
        other.session_state()
    );
    held_log.with(|s| s.released = true);
    held.tick(20_000).await;
    assert_eq!(held_log.steps(), asserted_then_leased(1));
    assert!(matches!(held.session_state(), SessionState::Ready { .. }));
}

/// A daemon restart ends the session; the re-lease asserts again before
/// it is taken. Mutation: the assertion made once per client rather
/// than per acquisition.
#[tokio::test]
async fn a_restart_asserts_again_before_the_new_lease() {
    let node = node();
    let log = Log::default();
    let mut client = graphical(&node, &log);
    client.tick(0).await;
    assert_eq!(log.take(), asserted_then_leased(1));
    node.restart();
    let _ = client.drain(16, 1).await;
    let SessionState::Reconnecting { next_at, .. } = client.session_state().clone() else {
        panic!("the session was lost: {:?}", client.session_state());
    };
    client.tick(next_at).await;
    assert_eq!(log.steps(), asserted_then_leased(2));
    assert!(matches!(client.session_state(), SessionState::Ready { .. }));
}

/// The command-line client leases with no assertion, on its first open
/// and after a restart alike, and opens no admin connection for one
/// (plan §30's note). Mutation: `for_command_line` built with the
/// assertion.
#[tokio::test]
async fn the_command_line_client_never_asserts() {
    let node = node();
    let log = Log::default();
    let mut client = TransportClient::for_command_line(
        Leases {
            node: node.clone(),
            log: log.clone(),
        },
        Admins(log.clone()),
        store(),
        config(),
        Box::new(|| 1_786_600_000_000),
        0,
    )
    .expect("an empty store")
    .without_status();
    client.tick(0).await;
    node.restart();
    let _ = client.drain(16, 1).await;
    let SessionState::Reconnecting { next_at, .. } = client.session_state().clone() else {
        panic!("the session was lost: {:?}", client.session_state());
    };
    client.tick(next_at).await;
    assert_eq!(
        log.steps(),
        [Step::Leased(Some(human())), Step::Leased(Some(human()))]
    );
}

/// Over the conformance-proven fake, whose directory lists an enabled,
/// advertised, leased endpoint: an identity authored advertised and
/// leased by a graphical client is listed to no trusted peer, since its
/// lease came after the assertion; the command-line client's leaves the
/// entry as authored, listed. Mutations: the assertion asked `true`; the
/// command-line client built asserting.
#[tokio::test]
async fn a_graphical_lease_is_unlisted_and_a_command_line_lease_as_authored() {
    for command_line in [false, true] {
        let authored = || FakeConfig {
            peer: ProfileIdentity::generate()
                .transport_identity()
                .expect("peer id"),
            endpoints: vec![FakeEndpoint::open(human(), true)],
            default_endpoint: Some(human()),
            queue_bound: 16,
        };
        let (a, b) = FakeNetwork::pair(authored(), authored());
        let build = if command_line {
            TransportClient::for_command_line
        } else {
            TransportClient::new
        };
        let mut client = build(
            a.clone(),
            a.clone(),
            store(),
            config(),
            Box::new(|| 1_786_600_000_000),
            0,
        )
        .expect("an empty store");
        client.tick(0).await;
        assert!(
            matches!(client.session_state(), SessionState::Ready { .. }),
            "{:?}",
            client.session_state()
        );
        let asker = b
            .open(
                SessionRequest::new(
                    "human-client",
                    None,
                    [interweave_local_client_api::DataCapability::EndpointsQuery],
                )
                .expect("a request"),
            )
            .await
            .expect("a session");
        let listed =
            interweave_local_client_api::DataSessionPort::query_endpoints(&asker, a.peer().clone())
                .await
                .expect("the directory")
                .endpoints;
        if command_line {
            assert_eq!(listed, [human()], "as authored");
            assert!(a.advertise_sets().is_empty(), "nothing asserted");
        } else {
            assert!(listed.is_empty(), "unlisted: {listed:?}");
            assert_eq!(a.advertise_sets(), [(human(), false)]);
        }
    }
}

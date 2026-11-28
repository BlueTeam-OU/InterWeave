// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The server: accept, admit, and one task per connection until asked to
//! stop (plan §16 (1)-(8)).
//!
//! Stopping here is the server's share of the daemon's shutdown order
//! (plan §16 (6), `LOCAL-IPC.md`'s shutdown order): stop accepting, hand
//! each data session what its queue holds, within the stop's grace, then
//! close every connection -- each gets `close{ShuttingDown}` and its
//! session is closed, which releases its lease. Refusing new inbound
//! direct work before any of it (the runtime's drain), settling the
//! network, closing the Swarm, unlinking the sockets and dropping the
//! lock are the daemon's, before this starts and after it returns.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use interweave_ipc_protocol::{Close, Frame, ServerState};
use interweave_local_client_api::{AdminBinding, AdminCapability, DataSessionBinding};
use interweave_transport_api::TransportError;
use tokio::io::AsyncWriteExt as _;
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::admission::{Refusal, Slots};
use crate::connection::{self, CLOSE_GRACE, Shared};

/// How long stopping waits, past the hand-over's grace, for every
/// connection to end before aborting what remains: one close grace, and
/// as long again.
const STOP_GRACE: Duration = CLOSE_GRACE.saturating_mul(2);
use crate::counters::Counters;
use crate::hello::ServerConfig;
use crate::listen::Listeners;
use crate::state;

/// Serve `binding` on the bound sockets until `stop` resolves, then hand
/// each data session its queue within the profile's shutdown grace
/// ([`ServerConfig::shutdown_grace`]), close every connection and return.
///
/// The server holds no lease table and no queue: every lease, join and
/// event lives in `binding`, which the composition root built.
pub async fn serve<B>(
    listeners: Listeners,
    binding: B,
    config: ServerConfig,
    stop: impl Future<Output = ()> + Send,
) -> Arc<Counters>
where
    B: DataSessionBinding + AdminBinding + Clone + Send + Sync + 'static,
    B::Session: Send + Sync + 'static,
    B::Admin: Send + Sync + 'static,
{
    let grace = config.shutdown_grace;
    serve_handing_over(listeners, binding, config, async move {
        stop.await;
        grace
    })
    .await
}

/// [`serve`], the hand-over bounded by the grace `stop` resolves with:
/// the stop's own, which an `admin.shutdown` may have named.
pub async fn serve_handing_over<B>(
    listeners: Listeners,
    binding: B,
    config: ServerConfig,
    stop: impl Future<Output = Duration> + Send,
) -> Arc<Counters>
where
    B: DataSessionBinding + AdminBinding + Clone + Send + Sync + 'static,
    B::Session: Send + Sync + 'static,
    B::Admin: Send + Sync + 'static,
{
    let counters = Arc::new(Counters::default());
    let slots = Slots::new(config.limits, Arc::clone(&counters));
    let (state_tx, state_rx) = watch::channel::<Option<ServerState>>(None);
    let mut background = JoinSet::new();
    // The server's own status holding: `admin.status` only, granting no
    // client anything. Without it data clients get no `server_state`.
    if let Ok(port) = binding.admin([AdminCapability::Status].into()).await {
        if let Some(view) = state::read(&port).await {
            state::publish(&state_tx, view);
        }
        background.spawn(state::refresh(
            port,
            state_tx.clone(),
            config.keepalive.interval,
        ));
    }
    let shared = Arc::new(Shared {
        config,
        counters: Arc::clone(&counters),
        state: state_rx,
    });
    let (stop_tx, stop_rx) = watch::channel(None);
    let mut connections = JoinSet::new();
    tokio::pin!(stop);
    let grace = loop {
        tokio::select! {
            grace = &mut stop => break grace,
            accepted = listeners.accept() => {
                let Ok((stream, domain)) = accepted else {
                    // An accept error (a descriptor limit, say) must not
                    // spin the loop.
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                };
                let peer_uid = stream.peer_cred().ok().map(|c| c.uid());
                match slots.admit(peer_uid, listeners.owner_uid(), domain) {
                    // Nothing is written to a process of another uid.
                    Err(Refusal::PeerCredential) => drop(stream),
                    Err(Refusal::Full) => {
                        connections.spawn(async move {
                            let mut stream = stream;
                            if let Ok(bytes) = Frame::Close(Close::new(TransportError::Overloaded)).encode() {
                                let _ = stream.write_all(&bytes).await;
                            }
                            let _ = stream.shutdown().await;
                        });
                    }
                    Ok(slot) => {
                        connections.spawn(connection::run(
                            stream,
                            domain,
                            binding.clone(),
                            Arc::clone(&shared),
                            stop_rx.clone(),
                            slot,
                        ));
                    }
                }
            }
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
        }
    };
    // One deadline for every connection's hand-over, from the stop.
    let _ = stop_tx.send(Some(tokio::time::Instant::now() + grace));
    // Every connection sees `stop` at once (its loop never waits on the
    // client), hands its session over within `grace` and bounds its own
    // end by CLOSE_GRACE; STOP_GRACE past the grace is the backstop,
    // after which what is left is aborted (#151 review, F1).
    let drained = tokio::time::timeout(grace.saturating_add(STOP_GRACE), async {
        while connections.join_next().await.is_some() {}
    })
    .await;
    if drained.is_err() {
        connections.shutdown().await;
    }
    drop(state_tx);
    background.shutdown().await;
    counters
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::admission::Limits;
    use crate::fake::{Fake, PEER, peer};
    use crate::frames::FrameReader;
    use crate::hello::KeepalivePolicy;
    use crate::listen::{SocketPaths, bind};
    use interweave_ipc_protocol::encode_frame;
    use interweave_local_client_api::{LocalSessionEvent, SessionEvent};
    use interweave_transport_api::Health;
    use tokio::net::UnixStream;
    use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;

    const PATIENCE: Duration = Duration::from_secs(5);

    struct Harness {
        _root: tempfile::TempDir,
        paths: SocketPaths,
        stop: Option<oneshot::Sender<()>>,
        server: JoinHandle<Arc<Counters>>,
    }

    impl Harness {
        fn start(fake: &Fake, config: ServerConfig) -> Self {
            Self::start_with(fake, config, |_| {})
        }

        fn start_with(
            fake: &Fake,
            config: ServerConfig,
            tweak: impl FnOnce(&mut Listeners),
        ) -> Self {
            let root = tempfile::tempdir().expect("tempdir");
            let run_dir = root.path().join("interweave");
            let paths = SocketPaths {
                data: run_dir.join("data.sock"),
                admin: run_dir.join("admin.sock"),
                run_dir,
            };
            let mut listeners = bind(&paths).expect("binds");
            tweak(&mut listeners);
            let (stop, stopped) = oneshot::channel();
            let server = tokio::spawn(serve(listeners, fake.clone(), config, async {
                let _ = stopped.await;
            }));
            Self {
                _root: root,
                paths,
                stop: Some(stop),
                server,
            }
        }

        async fn stop(mut self) -> Arc<Counters> {
            let _ = self.stop.take().expect("once").send(());
            self.server.await.expect("the server stops")
        }
    }

    fn config() -> ServerConfig {
        ServerConfig {
            peer: peer(),
            limits: Limits::default(),
            keepalive: KeepalivePolicy::default(),
            shutdown_grace: Duration::from_secs(5),
            command_deadline: Duration::from_secs(10),
            write_stall: crate::hello::WRITE_STALL,
        }
    }

    struct Client {
        reader: FrameReader<OwnedReadHalf>,
        write: OwnedWriteHalf,
    }

    impl Client {
        async fn connect(path: &std::path::Path) -> Self {
            let (read, write) = UnixStream::connect(path)
                .await
                .expect("connects")
                .into_split();
            Self {
                reader: FrameReader::new(read),
                write,
            }
        }

        async fn send(&mut self, body: &str) {
            use tokio::io::AsyncWriteExt as _;
            let frame = encode_frame(body).expect("a frame");
            self.write.write_all(&frame).await.expect("sent");
        }

        /// The next frame, or `None` when the server closed the stream.
        async fn next(&mut self) -> Option<Frame> {
            let body = tokio::time::timeout(PATIENCE, self.reader.next())
                .await
                .expect("the server answers in time")
                .ok()??;
            Some(Frame::parse(&body).expect("the server writes frames"))
        }

        /// The next frame that is not a `server_state` push.
        async fn next_reply(&mut self) -> Option<Frame> {
            loop {
                match self.next().await? {
                    Frame::ServerState(_) => {}
                    frame => return Some(frame),
                }
            }
        }

        async fn response(&mut self) -> ResponseFrameView {
            match self.next_reply().await {
                Some(Frame::Response(r)) => ResponseFrameView {
                    id: r.id.as_str().to_owned(),
                    body: Frame::Response(r).to_body(),
                },
                other => panic!("a response, got {other:?}"),
            }
        }

        async fn hello(&mut self, hello: &str) {
            self.send(hello).await;
            match self.next_reply().await {
                Some(Frame::HelloResponse(_)) => {}
                other => panic!("a hello_response, got {other:?}"),
            }
        }
    }

    struct ResponseFrameView {
        id: String,
        body: String,
    }

    /// A data hello claiming the fake's `human`, the TEST-ONLY derived id
    /// `fake::ep("human")` (`the_data_hello_claims_the_fakes_human`).
    const DATA: &str = r#"{"type":"hello","ipc_version":{"major":3,"minor":0},
        "client":{"kind":"human-client"},"endpoint":{"id":"12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr"},
        "requested_capabilities":["events","commands"],"features":["keepalive"]}"#;
    const ADMIN: &str = r#"{"type":"hello","ipc_version":{"major":3,"minor":0},
        "client":{"kind":"transportctl"},"requested_capabilities":["admin.status"]}"#;
    /// A read-only diagnostics data session -- no endpoint, so no
    /// keepalive, and no `events`: nothing wakes its loop on a timer, so it
    /// shows what the client's own traffic alone does.
    const DATA_QUIET: &str = r#"{"type":"hello","ipc_version":{"major":3,"minor":0},
        "client":{"kind":"diagnostics"},"requested_capabilities":[]}"#;

    /// Each hello a backpressure test runs under, and whether it is an
    /// admin one: the data connection whose event tick wakes its loop, and
    /// the two that nothing wakes (#151 re-review, 1).
    const EVERY_QUIET_AND_BUSY_CONNECTION: [(&str, bool, &str); 3] = [
        ("data with events", false, DATA),
        ("data without events or keepalive", false, DATA_QUIET),
        ("admin", true, ADMIN),
    ];

    fn join(id: &str) -> String {
        format!(
            r#"{{"type":"request","id":"{id}","method":"channel.join","params":{{"channel":"ops"}}}}"#
        )
    }

    fn send(id: &str) -> String {
        format!(
            r#"{{"type":"request","id":"{id}","method":"direct.send","params":{{"peer":"{PEER}",
            "message_id":"00000000000000000000000000000001","payload":{{"bytes":""}}}}}}"#
        )
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_request_is_answered_and_an_unknown_method_leaves_the_connection_open() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        client
            .send(r#"{"type":"request","id":"x","method":"admin.trust.add"}"#)
            .await;
        let unknown = client.response().await;
        assert!(
            unknown.body.contains("ProtocolUnsupported"),
            "{}",
            unknown.body
        );
        client.send(&join("1")).await;
        let joined = client.response().await;
        assert_eq!(joined.id, "1");
        assert!(joined.body.contains(r#""ok":true"#), "{}", joined.body);
        assert!(fake.script().calls.contains(&"join ops".to_owned()));
        drop(client);
        harness.stop().await;
    }

    #[test]
    fn the_data_hello_claims_the_fakes_human() {
        assert!(DATA.contains(crate::fake::ep("human").as_str()));
    }

    /// `admin.peers.list` (2.2 on major 2, under `admin.status`) is a 3.0
    /// method: a 3.0 connection holding `admin.status` is answered from
    /// the port.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_peers_read_is_answered_at_three_zero() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let request = r#"{"type":"request","id":"p","method":"admin.peers.list","params":{}}"#;
        {
            let (minor, answered) = (0, true);
            let mut client = Client::connect(&harness.paths.admin).await;
            client
                .hello(&ADMIN.replace(r#""minor":0"#, &format!(r#""minor":{minor}"#)))
                .await;
            client.send(request).await;
            let response = client.response().await;
            if answered {
                assert!(response.body.contains(r#""ok":true"#), "{}", response.body);
            } else {
                assert!(
                    response.body.contains("ProtocolUnsupported"),
                    "{}",
                    response.body
                );
            }
            drop(client);
        }
        assert_eq!(
            fake.script().calls.iter().filter(|c| *c == "peers").count(),
            1,
            "the 3.0 request reached the port"
        );
        harness.stop().await;
    }

    /// The `admin.peers` address methods (2.7 on major 2) are 3.0's:
    /// answered from the port on a 3.0 connection granted `admin.peers`,
    /// and refused `CapabilityDenied` on one that was not -- the grant
    /// admits it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_address_methods_are_admitted_at_three_zero_under_admin_peers() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let request = r#"{"type":"request","id":"a","method":"admin.peers.add_address","params":{"address":"x"}}"#;
        let peers_hello = ADMIN.replace(r#""admin.status""#, r#""admin.status","admin.peers""#);
        for (hello, minor, answer) in [
            (ADMIN, 0, "CapabilityDenied"),
            (peers_hello.as_str(), 0, r#""ok":true"#),
        ] {
            let mut client = Client::connect(&harness.paths.admin).await;
            client
                .hello(&hello.replace(r#""minor":0"#, &format!(r#""minor":{minor}"#)))
                .await;
            client.send(request).await;
            let response = client.response().await;
            assert!(
                response.body.contains(answer),
                "3.{minor}: {}",
                response.body
            );
            drop(client);
        }
        assert_eq!(
            fake.script()
                .calls
                .iter()
                .filter(|c| c.starts_with("add_address"))
                .count(),
            1,
            "only the granted request reached the port"
        );
        harness.stop().await;
    }

    /// `admin.trust.list` answers the 2.3 row (`persisted` true with its
    /// `source`), the one row shape 3.0 serves.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_trust_row_carries_its_source_at_three_zero() {
        let fake = Fake::default();
        fake.script().trusted = vec![
            interweave_transport_api::TransportIdentity::parse(
                "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5",
            )
            .expect("peer"),
        ];
        let harness = Harness::start(&fake, config());
        let request = r#"{"type":"request","id":"t","method":"admin.trust.list","params":{}}"#;
        let trust_hello = ADMIN.replace(r#""admin.status""#, r#""admin.status","admin.trust""#);
        {
            let (minor, persisted) = (0, true);
            let mut client = Client::connect(&harness.paths.admin).await;
            client
                .hello(&trust_hello.replace(r#""minor":0"#, &format!(r#""minor":{minor}"#)))
                .await;
            client.send(request).await;
            let response = client.response().await;
            let body: serde_json::Value = serde_json::from_str(&response.body).expect("json");
            let row = &body["result"]["allowed"][0];
            assert_eq!(row["persisted"], persisted, "3.{minor}: {}", response.body);
            assert_eq!(
                row.get("source").and_then(serde_json::Value::as_str),
                persisted.then_some("configured"),
                "3.{minor}: {}",
                response.body
            );
            drop(client);
        }
        harness.stop().await;
    }

    /// An admin method on the data socket is refused before dispatch and
    /// COUNTED; the admin socket reports the count.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_other_domains_method_is_refused_and_counted() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut data = Client::connect(&harness.paths.data).await;
        data.hello(DATA).await;
        data.send(r#"{"type":"request","id":"1","method":"admin.shutdown","params":{}}"#)
            .await;
        let refused = data.response().await;
        assert!(
            refused.body.contains("CapabilityDenied"),
            "{}",
            refused.body
        );
        assert!(
            !fake
                .script()
                .calls
                .iter()
                .any(|c| c.starts_with("shutdown"))
        );
        let mut admin = Client::connect(&harness.paths.admin).await;
        admin.hello(ADMIN).await;
        admin
            .send(r#"{"type":"request","id":"s","method":"admin.status"}"#)
            .await;
        let status = admin.response().await;
        assert!(
            status
                .body
                .contains(r#""cross_domain_capability_denied_total":1"#),
            "{}",
            status.body
        );
        assert!(
            status.body.contains(r#""data_connections":1"#),
            "{}",
            status.body
        );
        assert!(
            status.body.contains(r#""admin_connections":1"#),
            "{}",
            status.body
        );
        drop((data, admin));
        harness.stop().await;
    }

    /// 16 in flight, 48 waiting, the 65th `Overloaded`; a waiting one
    /// cancelled is `CancelledBeforeDispatch`, one in flight
    /// `CancellationRaced` -- and its outcome is not answered again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn requests_are_bounded_and_a_cancel_answers_once() {
        let fake = Fake::default();
        let hold = Arc::new(tokio::sync::Notify::new());
        fake.script().hold_send = Some(Arc::clone(&hold));
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        for i in 0..=(crate::MAX_IN_FLIGHT + crate::MAX_PENDING) {
            client.send(&send(&format!("r{i}"))).await;
        }
        let overloaded = client.response().await;
        assert_eq!(
            overloaded.id,
            format!("r{}", crate::MAX_IN_FLIGHT + crate::MAX_PENDING)
        );
        assert!(
            overloaded.body.contains("Overloaded"),
            "{}",
            overloaded.body
        );
        client.send(r#"{"type":"cancel","id":"r20"}"#).await;
        let waiting = client.response().await;
        assert_eq!(waiting.id, "r20");
        assert!(
            waiting.body.contains("CancelledBeforeDispatch"),
            "{}",
            waiting.body
        );
        client.send(r#"{"type":"cancel","id":"r0"}"#).await;
        let raced = client.response().await;
        assert_eq!(raced.id, "r0");
        assert!(raced.body.contains("CancellationRaced"), "{}", raced.body);
        // Release every send: each remaining request is answered once,
        // r0 and r20 not at all.
        fake.script().hold_send = None;
        let mut answered = std::collections::BTreeSet::new();
        while answered.len() < crate::MAX_IN_FLIGHT + crate::MAX_PENDING - 2 {
            hold.notify_waiters();
            let response = client.response().await;
            assert!(
                answered.insert(response.id.clone()),
                "{} answered twice",
                response.id
            );
        }
        assert!(!answered.contains("r0") && !answered.contains("r20"));
        drop(client);
        harness.stop().await;
    }

    /// An id whose response the client already has -- here a raced
    /// cancel answered it while its task still runs -- is not outstanding
    /// to the client: its reuse is refused with a response, and the
    /// connection stays open.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_answered_id_reused_while_its_task_runs_is_refused_not_closed() {
        let fake = Fake::default();
        let hold = Arc::new(tokio::sync::Notify::new());
        fake.script().hold_send = Some(Arc::clone(&hold));
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        client.send(&send("r0")).await;
        client.send(r#"{"type":"cancel","id":"r0"}"#).await;
        let raced = client.response().await;
        assert!(raced.body.contains("CancellationRaced"), "{}", raced.body);
        client.send(&send("r0")).await;
        let refused = client.response().await;
        assert_eq!(refused.id, "r0");
        assert!(refused.body.contains("InvalidArgument"), "{}", refused.body);
        hold.notify_waiters();
        drop(client);
        harness.stop().await;
    }

    /// A request id reused while the first still awaits its response --
    /// in flight or waiting for a slot -- closes the connection with
    /// `ProtocolViolation`; distinct ids beside it are the control.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_request_id_reused_while_outstanding_closes_the_connection() {
        for (reused, label) in [("r0", "in flight"), ("r16", "waiting")] {
            let fake = Fake::default();
            let hold = Arc::new(tokio::sync::Notify::new());
            fake.script().hold_send = Some(Arc::clone(&hold));
            let harness = Harness::start(&fake, config());
            let mut client = Client::connect(&harness.paths.data).await;
            client.hello(DATA).await;
            // r16 waits: the first MAX_IN_FLIGHT fill every slot.
            for i in 0..=crate::MAX_IN_FLIGHT {
                client.send(&send(&format!("r{i}"))).await;
            }
            // The control: a cancel for a distinct id is answered and the
            // connection stays open.
            client.send(r#"{"type":"cancel","id":"r1"}"#).await;
            assert_eq!(client.response().await.id, "r1", "{label}");
            client.send(&send(reused)).await;
            loop {
                match client.next_reply().await {
                    Some(Frame::Close(close)) => {
                        assert_eq!(close.code, TransportError::ProtocolViolation, "{label}");
                        break;
                    }
                    Some(Frame::Response(response)) => {
                        panic!("{label}: answered rather than closed: {response:?}")
                    }
                    Some(_) => {}
                    None => panic!("{label}: closed without a close frame"),
                }
            }
            hold.notify_waiters();
            drop(client);
            harness.stop().await;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_sessions_events_are_written_in_sequence() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        for _ in 0..2 {
            fake.script().events.push_back(SessionEvent::Local(
                LocalSessionEvent::PeerDisconnected {
                    peer: peer(),
                    reason_class: "policy".into(),
                },
            ));
        }
        let mut sequences = Vec::new();
        while sequences.len() < 2 {
            if let Some(Frame::Event(event)) = client.next_reply().await {
                assert_eq!(event.event_type, "peer.disconnected");
                sequences.push(event.sequence);
            }
        }
        assert_eq!(sequences, [0, 1]);
        drop(client);
        harness.stop().await;
    }

    /// A client that does not read fills the socket and then the event
    /// lane; the pump then takes nothing, so the rest stays with the
    /// binding -- and once the client reads, every event arrives, in
    /// sequence, none lost (#151 review, F3).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn events_past_the_lane_stay_with_the_binding_until_there_is_room() {
        // Past what the socket buffer and the lane hold together.
        const QUEUED: usize = 1000;
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        for _ in 0..QUEUED {
            fake.script().events.push_back(SessionEvent::Local(
                LocalSessionEvent::PeerDisconnected {
                    peer: peer(),
                    reason_class: "policy".into(),
                },
            ));
        }
        // Wait for the pump to stop taking: the socket and the lane are full.
        let mut held = QUEUED;
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let now = fake.script().events.len();
            if now == held {
                break;
            }
            held = now;
        }
        assert!(held > 0, "the pump took what the lane had no room for");
        let mut next = 0;
        while next < QUEUED {
            match client.next_reply().await {
                Some(Frame::Event(event)) => {
                    assert_eq!(event.sequence, next as u64, "no event lost or reordered");
                    next += 1;
                }
                Some(_) => {}
                None => panic!("the connection closed after {next} of {QUEUED} events"),
            }
        }
        drop(client);
        harness.stop().await;
    }

    /// Unanswered probes close the connection, and its lease is released
    /// as for any disconnect.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unanswered_probes_close_the_connection_and_release_its_lease() {
        let fake = Fake::default();
        // A slow release: a close sent without waiting for it would reach
        // the client while the lease is still held.
        fake.script().close_delay = Duration::from_millis(300);
        let mut config = config();
        config.keepalive.interval = Duration::from_millis(50);
        config.keepalive.response_timeout = Duration::from_millis(50);
        config.keepalive.max_missed = 2;
        let harness = Harness::start(&fake, config);
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let mut pings = 0;
        let close = loop {
            match client.next_reply().await {
                Some(Frame::Ping(_)) => pings += 1,
                Some(Frame::Close(close)) => break close,
                other => panic!("pings then a close, got {other:?}"),
            }
        };
        // Read the moment the `close` arrives, with no wait: the lease is
        // released BEFORE the client is told, so one that reconnects on
        // the `close` finds it free (#151 re-review, 3).
        assert!(
            fake.script().leased.is_empty(),
            "the lease is released before the close is sent"
        );
        assert_eq!(fake.script().closed, 1);
        assert_eq!(close.code, TransportError::Timeout);
        assert_eq!(pings, 2);
        assert!(client.next().await.is_none(), "closed");
        harness.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_answered_probe_keeps_the_connection() {
        let fake = Fake::default();
        let mut config = config();
        config.keepalive.interval = Duration::from_millis(100);
        // A WINDOW A LOADED HOST CAN MEET, IN A RUN LONGER THAN IT: 100 ms
        // closed the connection once before the client's pong was read
        // (BrokenPipe, rust-ui-dev's observation 01a162d8), and a window
        // longer than the whole run would pass on a server that ignored
        // every pong. Answering for three windows keeps the claim: a pong
        // not taken closes the connection inside the run.
        config.keepalive.response_timeout = Duration::from_secs(1);
        config.keepalive.max_missed = 1;
        let harness = Harness::start(&fake, config);
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let until = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < until {
            let Some(Frame::Ping(ping)) = client.next_reply().await else {
                panic!("a ping")
            };
            client.send(&Frame::Pong(ping.echo()).to_body()).await;
        }
        client.send(&join("after")).await;
        assert_eq!(client.response().await.id, "after", "still open");
        drop(client);
        harness.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_disconnect_closes_the_session_and_frees_the_lease_and_the_slot() {
        let fake = Fake::default();
        let mut config = config();
        config.limits = Limits {
            max_clients: 1,
            max_admin_clients: 1,
        };
        let harness = Harness::start(&fake, config);
        let mut first = Client::connect(&harness.paths.data).await;
        first.hello(DATA).await;
        let mut second = Client::connect(&harness.paths.data).await;
        match second.next_reply().await {
            Some(Frame::Close(close)) => assert_eq!(close.code, TransportError::Overloaded),
            other => panic!("the one slot is held: {other:?}"),
        }
        drop(first);
        let deadline = tokio::time::Instant::now() + PATIENCE;
        while fake.script().closed == 0 {
            assert!(tokio::time::Instant::now() < deadline, "the session closes");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(fake.script().leased.is_empty());
        let mut third = Client::connect(&harness.paths.data).await;
        third.hello(DATA).await;
        drop(third);
        harness.stop().await;
    }

    /// Another uid cannot connect unprivileged, so the run directory's
    /// owner is made someone else: the connection is closed without a
    /// word, and counted.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_peer_of_another_uid_is_closed_before_hello_and_counted() {
        let fake = Fake::default();
        let harness = Harness::start_with(&fake, config(), |l| {
            l.owner_uid = l.owner_uid.wrapping_add(1);
        });
        let mut client = Client::connect(&harness.paths.data).await;
        // Nothing is sent: the server may close before a write would land,
        // and a closed socket is what is being asserted.
        assert!(client.next().await.is_none(), "closed, nothing written");
        let counters = harness.stop().await;
        assert_eq!(counters.snapshot().peer_credential_refused_total, 1);
        assert!(
            fake.script().calls.is_empty(),
            "the binding was never asked"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_data_client_gets_the_view_on_connect_and_on_change() {
        let fake = Fake::default();
        let mut config = config();
        config.keepalive.interval = Duration::from_millis(50);
        let harness = Harness::start(&fake, config);
        let mut client = Client::connect(&harness.paths.data).await;
        client.send(DATA).await;
        assert!(matches!(client.next().await, Some(Frame::HelloResponse(_))));
        let Some(Frame::ServerState(first)) = client.next().await else {
            panic!("the view on connect")
        };
        assert_eq!(first.health, Health::Healthy);
        fake.script().health = Some(Health::Degraded);
        loop {
            match client.next().await {
                Some(Frame::ServerState(view)) if view.health == Health::Degraded => break,
                Some(Frame::ServerState(_) | Frame::Ping(_)) => {}
                other => panic!("the changed view, got {other:?}"),
            }
        }
        drop(client);
        harness.stop().await;
    }

    /// A data session without `events` cannot read the view, so it is
    /// sent none -- on connect or on a change -- while the connection
    /// beside it, holding `events`, is sent both: the control.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_data_client_without_events_is_sent_no_view() {
        let fake = Fake::default();
        let mut config = config();
        config.keepalive.interval = Duration::from_millis(50);
        let harness = Harness::start(&fake, config);
        let mut quiet = Client::connect(&harness.paths.data).await;
        quiet.send(DATA_QUIET).await;
        assert!(matches!(quiet.next().await, Some(Frame::HelloResponse(_))));
        let mut control = Client::connect(&harness.paths.data).await;
        control.send(DATA).await;
        assert!(matches!(
            control.next().await,
            Some(Frame::HelloResponse(_))
        ));
        assert!(
            matches!(control.next().await, Some(Frame::ServerState(_))),
            "the control is sent the view on connect"
        );
        fake.script().health = Some(Health::Degraded);
        loop {
            match control.next().await {
                Some(Frame::ServerState(view)) if view.health == Health::Degraded => break,
                Some(Frame::ServerState(_) | Frame::Ping(_)) => {}
                other => panic!("the changed view, got {other:?}"),
            }
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(300), quiet.next())
                .await
                .is_err(),
            "nothing for the session without events"
        );
        drop((quiet, control));
        harness.stop().await;
    }

    /// Queue `n` events with the fake session and wait until the pump has
    /// stopped taking them -- the socket and the lane are full -- so some
    /// are still with the session; answers how many it still holds.
    async fn stuff(fake: &Fake, n: usize) -> usize {
        for _ in 0..n {
            fake.script().events.push_back(SessionEvent::Local(
                LocalSessionEvent::PeerDisconnected {
                    peer: peer(),
                    reason_class: "policy".into(),
                },
            ));
        }
        let mut held = n;
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let now = fake.script().events.len();
            if now == held {
                break;
            }
            held = now;
        }
        assert!(held > 0, "the pump took what the lane had no room for");
        held
    }

    /// The stop's hand-over (`LOCAL-IPC.md`'s shutdown order, A
    /// 2026-11-17): what the session still held when the server was asked
    /// to stop is written to a client that reads, in sequence, and THEN
    /// the `close` -- none dropped, none counted. As the gap was found the
    /// connection ended on the stop and the close overtook what was
    /// queued: the client read only what the socket and the lane held.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_stop_writes_what_the_session_held_before_the_close() {
        const QUEUED: usize = 1000;
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let _ = stuff(&fake, QUEUED).await;
        let stopping = tokio::spawn(harness.stop());
        let mut next = 0;
        loop {
            match client.next_reply().await {
                Some(Frame::Event(event)) => {
                    assert_eq!(event.sequence, next as u64, "in sequence, none lost");
                    next += 1;
                }
                Some(Frame::Close(close)) => {
                    assert_eq!(close.code, TransportError::ShuttingDown);
                    break;
                }
                Some(_) => {}
                None => panic!("the stream ended without a close after {next} events"),
            }
        }
        assert_eq!(
            next, QUEUED,
            "every event the session held, before the close"
        );
        let counters = stopping.await.expect("stopped");
        assert_eq!(
            counters.events_dropped_at_stop(),
            0,
            "nothing left to count"
        );
        assert_eq!(fake.script().closed, 1, "the session is closed after");
    }

    /// A client that does not read is closed when the stop's grace ends,
    /// and what it did not take is counted: the events still in its lane
    /// and what its session still held. Read after the server has gone,
    /// what the socket delivered and what was counted are the queue
    /// whole, but for the one frame the writer may have been part-way
    /// through when it was aborted, which nothing counts (a limit, said
    /// in `LOCAL-IPC.md`'s shutdown order).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_stop_counts_what_a_client_that_does_not_read_left() {
        const QUEUED: usize = 1000;
        let fake = Fake::default();
        let mut short = config();
        short.shutdown_grace = Duration::from_millis(300);
        let harness = Harness::start(&fake, short);
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let held = stuff(&fake, QUEUED).await;
        let counters = harness.stop().await;
        assert!(
            fake.script().events.is_empty(),
            "the session's rest was taken to count"
        );
        let dropped = usize::try_from(counters.events_dropped_at_stop()).expect("fits");
        assert!(
            dropped >= held,
            "at least what the session held: {dropped} < {held}"
        );
        let mut read = 0;
        while let Some(frame) = client.next().await {
            if matches!(frame, Frame::Event(_)) {
                read += 1;
            }
        }
        assert!(
            read + dropped == QUEUED || read + dropped + 1 == QUEUED,
            "read {read} + counted {dropped} is the queue of {QUEUED}, less one at most"
        );
    }

    /// A writer that gives up on a client that does not read inside the
    /// stop's grace (`write_stall` shorter than the grace) closes its
    /// lane, and the stop counts what it held: what the socket delivered and
    /// what was counted are the queue whole, the event the writer failed
    /// on included. As the gap was found, the lane's events went uncounted
    /// whenever the writer gave up before the grace ended (#709 review F1).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_stop_counts_what_a_writer_that_gave_up_left() {
        const QUEUED: usize = 1000;
        let fake = Fake::default();
        let mut stalling = config();
        // The stall past the time `stuff` takes to see the pump stop (two
        // 200 ms looks), so the writer gives up inside the stop's grace,
        // not before the stop.
        stalling.shutdown_grace = Duration::from_secs(5);
        stalling.write_stall = Duration::from_millis(1_500);
        let harness = Harness::start(&fake, stalling);
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let _ = stuff(&fake, QUEUED).await;
        let counters = harness.stop().await;
        let dropped = usize::try_from(counters.events_dropped_at_stop()).expect("fits");
        let mut read = 0;
        while let Some(frame) = client.next().await {
            if matches!(frame, Frame::Event(_)) {
                read += 1;
            }
        }
        assert_eq!(
            read + dropped,
            QUEUED,
            "read {read} + counted {dropped} is the queue of {QUEUED}"
        );
    }

    /// Shut the client's write half: a clean end of stream between frames.
    async fn leave(client: &mut Client) {
        use tokio::io::AsyncWriteExt as _;
        client.write.shutdown().await.expect("shut");
    }

    /// Every event frame the client reads until the server ends the
    /// stream, checked in sequence; and whether a `close` came.
    async fn read_to_the_end(client: &mut Client) -> (usize, bool) {
        let mut next = 0;
        while let Some(frame) = client.next().await {
            match frame {
                Frame::Event(event) => {
                    assert_eq!(event.sequence, next as u64, "in sequence, none lost");
                    next += 1;
                }
                Frame::Close(_) => return (next, true),
                _ => {}
            }
        }
        (next, false)
    }

    /// Wait until the fake's session is closed and its lease free.
    async fn until_released(fake: &Fake, patience: Duration) {
        let deadline = tokio::time::Instant::now() + patience;
        while fake.script().closed == 0 || !fake.script().leased.is_empty() {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the lease was released within {patience:?}"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// A client's close (`LOCAL-IPC.md` §Disconnect/reconnect, A
    /// 2026-11-28): a client that shuts its write half is written what
    /// its session still held, in sequence, and then the stream ends --
    /// no `close` frame -- with nothing dropped and nothing counted. The
    /// lease is released FIRST: it is free while the client has read
    /// nothing at all, its lane and socket full, long before the grace
    /// would have let a hand-over that drained before releasing finish.
    /// As the gap was found, the session was closed on the end of stream
    /// and what its queue held was dropped uncounted; the client read only
    /// what the socket and the lane held.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_clients_close_is_written_what_the_session_held_before_the_stream_ends() {
        const QUEUED: usize = 1000;
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let held = stuff(&fake, QUEUED).await;
        leave(&mut client).await;
        // The grace is 5 s: a release inside 2 s with nothing read did not
        // wait for the client.
        until_released(&fake, Duration::from_secs(2)).await;
        let (read, closed) = read_to_the_end(&mut client).await;
        assert!(!closed, "a leaving client is sent no close");
        assert_eq!(
            read, QUEUED,
            "every event the session held ({held} at the close), before the end"
        );
        let counters = harness.stop().await;
        assert_eq!(counters.events_dropped_at_close(), 0, "nothing to count");
        assert_eq!(counters.events_dropped_at_stop(), 0, "not a stop");
    }

    /// A leaving client that does not read is ended when the grace from
    /// its end of stream runs out, its slot freed by then and the writer's
    /// `CLOSE_GRACE`, and what it did not take is counted as dropped at a
    /// close -- not at a stop. What the socket delivered and what was
    /// counted are the queue whole, less the one frame the writer may have
    /// been part-way through when it was aborted. The queue ends in
    /// runtime states, which the connection never writes as events: they
    /// are no loss to the client and are not counted, so the count stays a
    /// floor. The `close{Timeout}` queued for the cut-short hand-over is
    /// never written, since the writer is stuck behind a socket the client
    /// does not read.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_clients_close_counts_what_a_client_that_does_not_read_left() {
        const QUEUED: usize = 1000;
        const STATES: usize = 5;
        let fake = Fake::default();
        let mut short = config();
        short.shutdown_grace = Duration::from_millis(300);
        short.limits = Limits {
            max_clients: 1,
            max_admin_clients: 1,
        };
        let harness = Harness::start(&fake, short);
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let held = stuff(&fake, QUEUED).await;
        for _ in 0..STATES {
            fake.script()
                .events
                .push_back(SessionEvent::Local(LocalSessionEvent::ServerState {
                    health: Health::Healthy,
                    connectivity: None,
                }));
        }
        leave(&mut client).await;
        let left = tokio::time::Instant::now();
        // The one slot comes back once the connection has ended.
        loop {
            use tokio::io::AsyncWriteExt as _;
            let mut next = Client::connect(&harness.paths.data).await;
            // Refused for the slot, the connection may be closed before
            // the hello lands: the write's failure is that refusal.
            let hello = encode_frame(DATA).expect("a frame");
            let _ = next.write.write_all(&hello).await;
            match next.next_reply().await {
                Some(Frame::HelloResponse(_)) => break,
                Some(Frame::Close(close)) if close.code == TransportError::Overloaded => {}
                None => {}
                other => panic!("a hello_response or the slot's refusal: {other:?}"),
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            left.elapsed() < Duration::from_millis(300) + crate::connection::CLOSE_GRACE * 2,
            "the slot is held for the grace and the writer's CLOSE_GRACE, not longer: {:?}",
            left.elapsed()
        );
        let counters = harness.stop().await;
        let dropped = usize::try_from(counters.events_dropped_at_close()).expect("fits");
        assert!(
            dropped >= held,
            "at least what the session held: {dropped} < {held}"
        );
        assert_eq!(counters.events_dropped_at_stop(), 0, "not a stop's");
        let (read, closed) = read_to_the_end(&mut client).await;
        assert!(!closed, "a writer stuck behind the socket writes no close");
        assert!(
            read + dropped == QUEUED || read + dropped + 1 == QUEUED,
            "read {read} + counted {dropped} is the queue of {QUEUED} events, less one at \
             most, and none of its {STATES} states"
        );
    }

    /// A leaving client that reads only once the hand-over's grace has run
    /// out is told the remainder is NOT complete: what the socket and the
    /// writer still carry, then `close{Timeout}` -- on the control lane, so
    /// ahead of what the event lane still held -- and the count is exact:
    /// the writer finished every frame it began, so what the client read
    /// and what was counted are the queue whole. The control is
    /// `a_clients_close_is_written_what_the_session_held_before_the_stream_ends`,
    /// whose complete hand-over ends with no close. As the gap was found,
    /// a cut-short hand-over ended with the same clean end of stream as a
    /// complete one, and the client could not tell them apart.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_clients_close_cut_short_ends_with_a_timeout_close() {
        const QUEUED: usize = 1000;
        const GRACE: Duration = Duration::from_millis(300);
        let fake = Fake::default();
        let mut short = config();
        short.shutdown_grace = GRACE;
        let harness = Harness::start(&fake, short);
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let held = stuff(&fake, QUEUED).await;
        leave(&mut client).await;
        // Past the grace, well inside the writer's CLOSE_GRACE after it.
        tokio::time::sleep(GRACE + Duration::from_millis(400)).await;
        let mut read = 0;
        let mut close = None;
        while let Some(frame) = client.next().await {
            match frame {
                Frame::Event(event) => {
                    assert_eq!(event.sequence, read as u64, "in sequence");
                    read += 1;
                }
                Frame::Close(frame) => {
                    close = Some(frame.code);
                    break;
                }
                _ => {}
            }
        }
        assert_eq!(close, Some(TransportError::Timeout), "after {read} events");
        assert!(read < QUEUED, "cut short: {read} of {QUEUED}, {held} held");
        let counters = harness.stop().await;
        let dropped = usize::try_from(counters.events_dropped_at_close()).expect("fits");
        assert_eq!(
            read + dropped,
            QUEUED,
            "read {read} + counted {dropped} is the queue of {QUEUED}"
        );
        assert_eq!(counters.events_dropped_at_stop(), 0, "not a stop's");
    }

    /// `admin.status` serves what a leaving client did not read as
    /// `ipc.events_dropped_at_close_total`: 0 before, then the queue it
    /// left, while the session-queue overflow count (the fake's 2) and
    /// the stop's count do not move (`LOCAL-IPC.md` A 2026-11-28).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_clients_close_is_served_in_admin_status_apart_from_the_other_drops() {
        const QUEUED: usize = 1000;
        let fake = Fake::default();
        fake.script().events_dropped = Some(2);
        let mut short = config();
        short.shutdown_grace = Duration::from_millis(300);
        short.limits = Limits {
            // The ceiling is across both sockets: the admin connection and
            // the one data client.
            max_clients: 2,
            max_admin_clients: 1,
        };
        let harness = Harness::start(&fake, short);
        let mut admin = Client::connect(&harness.paths.admin).await;
        admin.hello(ADMIN).await;
        let mut asked = 0_u32;
        let mut status = async |admin: &mut Client| -> serde_json::Value {
            asked += 1;
            admin
                .send(&format!(
                    r#"{{"type":"request","id":"s{asked}","method":"admin.status"}}"#
                ))
                .await;
            let answer = admin.response().await;
            let answer: serde_json::Value = serde_json::from_str(&answer.body).expect("json");
            answer["result"]["ipc"].clone()
        };
        let before = status(&mut admin).await;
        assert_eq!(before["events_dropped_at_close_total"], 0, "{before}");
        assert_eq!(before["events_dropped_total"], 2, "{before}");

        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let held = stuff(&fake, QUEUED).await;
        leave(&mut client).await;
        // The close is counted when the connection ends, after the grace
        // the server gives a client that does not read.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while status(&mut admin).await["events_dropped_at_close_total"] == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "a close is counted within the grace"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let after = status(&mut admin).await;
        let left = after["events_dropped_at_close_total"]
            .as_u64()
            .unwrap_or_else(|| panic!("a kept count is served: {after}"));
        assert!(
            left >= held as u64,
            "at least what the session held: {after}"
        );
        assert_eq!(
            after["events_dropped_total"], 2,
            "overflow count unmoved: {after}"
        );
        let counters = harness.stop().await;
        assert_eq!(
            counters.events_dropped_at_stop(),
            0,
            "the stop's is unmoved"
        );
        assert_eq!(
            counters.events_dropped_at_close(),
            left,
            "one count, served whole"
        );
    }

    /// Only a CLEAN end of stream is a client leaving: a stream cut inside
    /// a frame is a connection lost, and is handed nothing and counts
    /// nothing, as before A 2026-11-28 -- the client reads only what the
    /// socket and the lane already held. The control beside it: the same
    /// queue, ended cleanly, is read whole.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_connection_cut_inside_a_frame_is_handed_nothing() {
        const QUEUED: usize = 1000;
        for clean in [true, false] {
            let fake = Fake::default();
            let harness = Harness::start(&fake, config());
            let mut client = Client::connect(&harness.paths.data).await;
            client.hello(DATA).await;
            let _ = stuff(&fake, QUEUED).await;
            if !clean {
                use tokio::io::AsyncWriteExt as _;
                // A length prefix promising nine bytes, and one of them.
                client
                    .write
                    .write_all(&[0, 0, 0, 9, b'{'])
                    .await
                    .expect("written");
            }
            leave(&mut client).await;
            until_released(&fake, PATIENCE).await;
            let (read, _) = read_to_the_end(&mut client).await;
            let counters = harness.stop().await;
            if clean {
                assert_eq!(read, QUEUED, "the control: a clean end is handed it all");
            } else {
                assert!(read < QUEUED, "a cut stream is handed nothing more: {read}");
                assert!(
                    fake.script().events.is_empty(),
                    "the session's close took the rest, and it was dropped"
                );
            }
            assert_eq!(
                counters.events_dropped_at_close(),
                0,
                "neither counts: one read all, one was not a close"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stopping_closes_every_connection_with_shutting_down() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let paths = harness.paths.clone();
        let stopping = tokio::spawn(harness.stop());
        match client.next_reply().await {
            Some(Frame::Close(close)) => assert_eq!(close.code, TransportError::ShuttingDown),
            other => panic!("a close, got {other:?}"),
        }
        stopping.await.expect("stopped");
        assert_eq!(fake.script().closed, 1, "the session is closed");
        let _ = (paths, crate::fake::ep("human"));
    }

    /// `admin.shutdown` stops the server it is asked of, so its answer is
    /// ready when the stop arrives: it is written, then the `close`. On
    /// one thread the server's task runs the stop before the connection
    /// wakes for the answer, so both are ready together and a select that
    /// took the stop dropped the answer about half the time; the run is
    /// repeated until that would have shown with near certainty. (On a
    /// multi-thread runtime the woken connection takes its worker's LIFO
    /// slot, wins nearly always, and the test passed without the fix.)
    #[tokio::test(flavor = "current_thread")]
    async fn an_admin_shutdown_is_answered_before_the_close() {
        const SHUTDOWN: &str = r#"{"type":"hello","ipc_version":{"major":3,"minor":0},
            "client":{"kind":"transportctl"},"requested_capabilities":["admin.shutdown"]}"#;
        for _ in 0..32 {
            let fake = Fake::default();
            let mut harness = Harness::start(&fake, config());
            fake.script().stop_on_shutdown = harness.stop.take();
            let mut admin = Client::connect(&harness.paths.admin).await;
            admin.hello(SHUTDOWN).await;
            admin
                .send(r#"{"type":"request","id":"s","method":"admin.shutdown","params":{}}"#)
                .await;
            let answer = admin.response().await;
            assert_eq!(answer.id, "s");
            assert!(answer.body.contains(r#""ok":true"#), "{}", answer.body);
            match admin.next_reply().await {
                Some(Frame::Close(close)) => assert_eq!(close.code, TransportError::ShuttingDown),
                other => panic!("a close after the answer, got {other:?}"),
            }
            harness.server.await.expect("the server stops");
            assert_eq!(fake.script().calls, ["shutdown 5000"]);
        }
    }

    /// A port call that panicked before the stop is answered `Internal`
    /// before the `close`, as the running loop answers it: the same race
    /// as above, run the same way.
    #[tokio::test(flavor = "current_thread")]
    async fn a_call_that_panicked_before_the_stop_is_answered_internal() {
        const SHUTDOWN: &str = r#"{"type":"hello","ipc_version":{"major":3,"minor":0},
            "client":{"kind":"transportctl"},"requested_capabilities":["admin.shutdown"]}"#;
        for _ in 0..32 {
            let fake = Fake::default();
            let mut harness = Harness::start(&fake, config());
            {
                let mut script = fake.script();
                script.stop_on_shutdown = harness.stop.take();
                script.panic_shutdown = true;
            }
            let mut admin = Client::connect(&harness.paths.admin).await;
            admin.hello(SHUTDOWN).await;
            admin
                .send(r#"{"type":"request","id":"s","method":"admin.shutdown","params":{}}"#)
                .await;
            let answer = admin.response().await;
            assert_eq!(answer.id, "s");
            assert!(answer.body.contains("Internal"), "{}", answer.body);
            match admin.next_reply().await {
                Some(Frame::Close(close)) => assert_eq!(close.code, TransportError::ShuttingDown),
                other => panic!("a close after the answer, got {other:?}"),
            }
            harness.server.await.expect("the server stops");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_second_hello_or_a_server_class_after_hello_is_a_violation() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        for second in [DATA, r#"{"type":"server_state","health":"healthy"}"#] {
            let mut client = Client::connect(&harness.paths.data).await;
            client.hello(DATA).await;
            client.send(second).await;
            match client.next_reply().await {
                Some(Frame::Close(close)) => {
                    assert_eq!(close.code, TransportError::ProtocolViolation);
                }
                other => panic!("a close, got {other:?}"),
            }
            // Each connection releases its lease before the next claims it.
            let deadline = tokio::time::Instant::now() + PATIENCE;
            while !fake.script().leased.is_empty() {
                assert!(tokio::time::Instant::now() < deadline);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        harness.stop().await;
    }

    fn send_by(id: &str, deadline_ms: u64) -> String {
        format!(
            r#"{{"type":"request","id":"{id}","method":"direct.send","deadline_ms":{deadline_ms},
            "params":{{"peer":"{PEER}","message_id":"00000000000000000000000000000001",
            "payload":{{"bytes":""}}}}}}"#
        )
    }

    /// A request handed over and still unanswered at its deadline is
    /// answered `Timeout`; its outcome is discarded when it comes. A zero
    /// deadline is clamped to one second, not refused.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_request_past_its_deadline_is_answered_timeout_once() {
        let fake = Fake::default();
        let hold = Arc::new(tokio::sync::Notify::new());
        fake.script().hold_send = Some(Arc::clone(&hold));
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        let sent = tokio::time::Instant::now();
        client.send(&send_by("late", 0)).await;
        let timed_out = client.response().await;
        let waited = sent.elapsed();
        assert_eq!(timed_out.id, "late");
        assert!(timed_out.body.contains("Timeout"), "{}", timed_out.body);
        assert!(
            waited >= Duration::from_millis(900),
            "clamped to 1 s, answered after {waited:?}"
        );
        fake.script().hold_send = None;
        hold.notify_waiters();
        client.send(&join("next")).await;
        assert_eq!(
            client.response().await.id,
            "next",
            "the late outcome is not answered"
        );
        drop(client);
        harness.stop().await;
    }

    /// A request still waiting at its deadline is answered `Timeout` and
    /// never reaches the port.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_waiting_request_past_its_deadline_is_dropped_unsent() {
        let fake = Fake::default();
        let hold = Arc::new(tokio::sync::Notify::new());
        fake.script().hold_send = Some(Arc::clone(&hold));
        let mut config = config();
        config.command_deadline = Duration::from_secs(60);
        let harness = Harness::start(&fake, config);
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        for i in 0..crate::MAX_IN_FLIGHT {
            client.send(&send(&format!("held{i}"))).await;
        }
        client.send(&send_by("waiting", 1000)).await;
        let timed_out = client.response().await;
        assert_eq!(timed_out.id, "waiting");
        assert!(timed_out.body.contains("Timeout"), "{}", timed_out.body);
        let sends = fake
            .script()
            .calls
            .iter()
            .filter(|c| c.starts_with("send"))
            .count();
        assert_eq!(
            sends,
            crate::MAX_IN_FLIGHT,
            "the waiting one was never handed over"
        );
        drop(client);
        harness.stop().await;
    }

    /// A client that sends and never reads: its answers stall the writer,
    /// which gives up after `write_stall`, the connection closes and the
    /// slot is free again -- the loop never waited on it (#151 review,
    /// F1-F2).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_client_that_never_reads_is_closed_and_frees_its_slot() {
        for (label, admin, hello) in EVERY_QUIET_AND_BUSY_CONNECTION {
            let fake = Fake::default();
            let mut config = config();
            config.limits = Limits {
                max_clients: 1,
                max_admin_clients: 1,
            };
            config.write_stall = Duration::from_millis(300);
            let harness = Harness::start(&fake, config);
            let socket = if admin {
                &harness.paths.admin
            } else {
                &harness.paths.data
            };
            let mut deaf = Client::connect(socket).await;
            deaf.hello(hello).await;
            let unknown = encode_frame(r#"{"type":"request","id":"x","method":"no.such.method"}"#)
                .expect("frame");
            let flood: Vec<u8> = unknown
                .iter()
                .copied()
                .cycle()
                .take(unknown.len() * 40_000)
                .collect();
            // The server may close before the flood is written; that is the
            // outcome under test, not an error.
            let _ = tokio::time::timeout(
                PATIENCE,
                tokio::io::AsyncWriteExt::write_all(&mut deaf.write, &flood),
            )
            .await;
            let deadline = tokio::time::Instant::now() + PATIENCE;
            loop {
                let mut next = Client::connect(socket).await;
                // A connection past the limit is closed before its hello is
                // read, so the write may meet a closed socket; the reply is
                // what is judged.
                let _ = tokio::io::AsyncWriteExt::write_all(
                    &mut next.write,
                    &encode_frame(hello).expect("frame"),
                )
                .await;
                match next.next_reply().await {
                    Some(Frame::HelloResponse(_)) => break,
                    Some(Frame::Close(close)) if close.code == TransportError::Overloaded => {}
                    other => panic!("{label}: the slot frees, got {other:?}"),
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "{label}: the stalled connection was never closed"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            drop(deaf);
            harness.stop().await;
        }
    }

    /// A port call that panics is answered `Internal`, once, and the
    /// connection goes on.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_panicking_port_call_is_answered_internal() {
        let fake = Fake::default();
        fake.script().panic_join = true;
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        client.send(&join("boom")).await;
        let answer = client.response().await;
        assert_eq!(answer.id, "boom");
        assert!(answer.body.contains("Internal"), "{}", answer.body);
        fake.script().panic_join = false;
        client.send(&join("after")).await;
        assert_eq!(client.response().await.id, "after");
        drop(client);
        harness.stop().await;
    }

    /// `peer.path_changed` (2.1 on major 2) is a 3.0 type: a 3.0
    /// connection is sent it, numbered, before the next event.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_path_change_goes_to_a_three_zero_connection_numbered() {
        let path = || {
            SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                peer: peer(),
                previous: Some(interweave_transport_api::PeerPath::Relayed),
                current: interweave_transport_api::PeerPath::Direct,
                reason_class: "dcutr".into(),
                observed_at: 1,
            })
        };
        let gone = || {
            SessionEvent::Local(LocalSessionEvent::PeerDisconnected {
                peer: peer(),
                reason_class: "policy".into(),
            })
        };
        {
            let (hello, expected) = (
                DATA,
                vec![("peer.path_changed", 0), ("peer.disconnected", 1)],
            );
            let fake = Fake::default();
            let harness = Harness::start(&fake, config());
            let mut client = Client::connect(&harness.paths.data).await;
            client.hello(hello).await;
            fake.script().events.extend([path(), gone()]);
            let mut seen = Vec::new();
            while !seen.iter().any(|(t, _)| *t == "peer.disconnected") {
                if let Some(Frame::Event(event)) = client.next_reply().await {
                    seen.push((
                        if event.event_type == "peer.path_changed" {
                            "peer.path_changed"
                        } else {
                            "peer.disconnected"
                        },
                        event.sequence,
                    ));
                }
            }
            assert_eq!(seen, expected, "{hello}");
            drop(client);
            harness.stop().await;
        }
    }

    /// A path notice with no `previous` -- a route's begin, a routed
    /// peer's reconnect -- is the 2.4 shape, the one 3.0 holds: a 3.0
    /// connection is sent it, numbered, `previous` absent from the data
    /// rather than `null`, and a change WITH `previous` after it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_route_notice_goes_to_a_three_zero_connection_numbered() {
        let notice = |previous| {
            SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                peer: peer(),
                previous,
                current: interweave_transport_api::PeerPath::Relayed,
                reason_class: interweave_local_client_api::ROUTE_ESTABLISHED.into(),
                observed_at: 1,
            })
        };
        let gone = || {
            SessionEvent::Local(LocalSessionEvent::PeerDisconnected {
                peer: peer(),
                reason_class: "policy".into(),
            })
        };
        {
            let (hello, expected) = (
                DATA,
                vec![("begun", 0), ("changed", 1), ("peer.disconnected", 2)],
            );
            let fake = Fake::default();
            let harness = Harness::start(&fake, config());
            let mut client = Client::connect(&harness.paths.data).await;
            client.hello(hello).await;
            fake.script().events.extend([
                notice(None),
                notice(Some(interweave_transport_api::PeerPath::Direct)),
                gone(),
            ]);
            let mut seen = Vec::new();
            while !seen.iter().any(|(t, _)| *t == "peer.disconnected") {
                if let Some(Frame::Event(event)) = client.next_reply().await {
                    let data = event.data.as_deref().map(|d| d.get().to_owned());
                    let kind = match (event.event_type.as_str(), &data) {
                        ("peer.path_changed", Some(d)) if d.contains(r#""previous""#) => "changed",
                        ("peer.path_changed", Some(d)) => {
                            assert!(!d.contains("null"), "absent, never null: {d}");
                            "begun"
                        }
                        _ => "peer.disconnected",
                    };
                    seen.push((kind, event.sequence));
                }
            }
            assert_eq!(seen, expected, "{hello}");
            drop(client);
            harness.stop().await;
        }
    }

    /// A path change the protocol refuses -- its class out of bounds --
    /// takes its number on a 3.0 connection, whose version admits the
    /// type, so the client sees the gap.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_refused_path_change_leaves_its_gap_on_a_three_zero_connection() {
        let refused = || {
            SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                peer: peer(),
                previous: Some(interweave_transport_api::PeerPath::Relayed),
                current: interweave_transport_api::PeerPath::Direct,
                reason_class: String::new(),
                observed_at: 1,
            })
        };
        let gone = || {
            SessionEvent::Local(LocalSessionEvent::PeerDisconnected {
                peer: peer(),
                reason_class: "policy".into(),
            })
        };
        {
            let (hello, expected) = (DATA, 1);
            let fake = Fake::default();
            let harness = Harness::start(&fake, config());
            let mut client = Client::connect(&harness.paths.data).await;
            client.hello(hello).await;
            fake.script().events.extend([refused(), gone()]);
            let sequence = loop {
                if let Some(Frame::Event(event)) = client.next_reply().await {
                    assert_eq!(event.event_type, "peer.disconnected");
                    break event.sequence;
                }
            };
            assert_eq!(sequence, expected, "{hello}");
            drop(client);
            harness.stop().await;
        }
    }

    /// An event the protocol refuses takes its sequence number, so the
    /// client sees a gap instead of nothing.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_refused_event_leaves_a_sequence_gap() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut client = Client::connect(&harness.paths.data).await;
        client.hello(DATA).await;
        for reason_class in [String::new(), "policy".to_owned()] {
            fake.script().events.push_back(SessionEvent::Local(
                LocalSessionEvent::PeerDisconnected {
                    peer: peer(),
                    reason_class,
                },
            ));
        }
        let sequence = loop {
            if let Some(Frame::Event(event)) = client.next_reply().await {
                break event.sequence;
            }
        };
        assert_eq!(sequence, 1, "the refused event took 0");
        drop(client);
        harness.stop().await;
    }

    /// A client that pipelines faster than the writer drains, but reads,
    /// is slowed by backpressure and never closed: every request is
    /// answered, once.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_pipelining_client_that_reads_is_answered_in_full() {
        // Past what the socket buffer and the control lane hold together,
        // so the outbox and the paused read are reached.
        const BURST: usize = 5000;
        for (label, admin, hello) in EVERY_QUIET_AND_BUSY_CONNECTION {
            let fake = Fake::default();
            let harness = Harness::start(&fake, config());
            let socket = if admin {
                &harness.paths.admin
            } else {
                &harness.paths.data
            };
            let mut client = Client::connect(socket).await;
            client.hello(hello).await;
            let burst: Vec<u8> = (0..BURST)
                .flat_map(|i| {
                    encode_frame(&format!(
                        r#"{{"type":"request","id":"u{i}","method":"no.such.method"}}"#
                    ))
                    .expect("frame")
                })
                .collect();
            // The client writes and reads at once, as a pipelining client
            // must: one that finished writing before it read would
            // deadlock against the server's paused read by construction.
            let Client { reader, write } = &mut client;
            let send = async {
                if let Err(error) = tokio::io::AsyncWriteExt::write_all(write, &burst).await {
                    panic!("{label}: the burst was refused: {error}");
                }
            };
            let receive = async {
                let mut answered = std::collections::BTreeSet::new();
                while answered.len() < BURST {
                    let body = tokio::time::timeout(PATIENCE, reader.next())
                        .await
                        .unwrap_or_else(|_| {
                            panic!("{label}: stalled after {} answers", answered.len())
                        })
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| panic!("{label}: closed after {}", answered.len()));
                    match Frame::parse(&body).expect("a frame") {
                        Frame::Response(response) => {
                            assert!(body.contains("ProtocolUnsupported"), "{label}: {body}");
                            assert!(
                                answered.insert(response.id.as_str().to_owned()),
                                "{label}: {} twice",
                                response.id.as_str()
                            );
                        }
                        Frame::ServerState(_) | Frame::Ping(_) => {}
                        other => panic!("{label}: {other:?}"),
                    }
                }
            };
            tokio::join!(send, receive);
            drop(client);
            harness.stop().await;
        }
    }

    /// A client that stops reading AND sending while everything it is owed
    /// still fits -- the socket full, a few answers in the control lane,
    /// nothing in the outbox -- is closed once the writer gives up, on a
    /// connection with no timer of its own (#151 re-review 2, F1). The
    /// flood test cannot reach this: its overflow always lands in the
    /// outbox, whose wake sees the writer go.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_silent_client_that_stopped_reading_is_closed_and_frees_its_slot() {
        // Answers left in the lane past the socket: inside 0..CONTROL_LANE
        // with room either side for the measurement to be off.
        const IN_THE_LANE: usize = 30;
        let fake = Fake::default();
        let mut config = config();
        config.limits = Limits {
            max_clients: 1,
            max_admin_clients: 1,
        };
        config.write_stall = Duration::from_millis(300);
        let harness = Harness::start(&fake, config);
        let mut silent = Client::connect(&harness.paths.data).await;
        silent.hello(DATA_QUIET).await;
        let request = |i: usize| {
            encode_frame(&format!(
                r#"{{"type":"request","id":"u{i:06}","method":"no.such.method"}}"#
            ))
            .expect("frame")
        };
        // One answer read, to learn exactly what the server writes per
        // request.
        tokio::io::AsyncWriteExt::write_all(&mut silent.write, &request(0))
            .await
            .expect("sent");
        let answer = encode_frame(&silent.response().await.body).expect("frame");
        // How many such writes this kernel buffers on a fresh socket before
        // one would block: the server's socket is the same kind.
        let (mut server_end, _client_end) = std::os::unix::net::UnixStream::pair().expect("a pair");
        server_end.set_nonblocking(true).expect("nonblocking");
        let mut buffered = 0;
        while std::io::Write::write(&mut server_end, &answer).is_ok_and(|n| n == answer.len()) {
            buffered += 1;
        }
        assert!(buffered > 0, "the measurement wrote nothing");
        let burst: Vec<u8> = (1..=buffered + IN_THE_LANE).flat_map(request).collect();
        tokio::io::AsyncWriteExt::write_all(&mut silent.write, &burst)
            .await
            .expect("sent");
        // Now silent: no reads, no writes.
        let deadline = tokio::time::Instant::now() + PATIENCE;
        loop {
            let mut next = Client::connect(&harness.paths.data).await;
            let _ = tokio::io::AsyncWriteExt::write_all(
                &mut next.write,
                &encode_frame(DATA_QUIET).expect("frame"),
            )
            .await;
            match next.next_reply().await {
                Some(Frame::HelloResponse(_)) => break,
                Some(Frame::Close(close)) if close.code == TransportError::Overloaded => {}
                other => panic!("the slot frees, got {other:?}"),
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the silent connection was never closed ({buffered} buffered)"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        drop(silent);
        harness.stop().await;
    }

    /// Stopping does not wait on a client that never reads.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stopping_is_bounded_by_a_client_that_never_reads() {
        let fake = Fake::default();
        let harness = Harness::start(&fake, config());
        let mut deaf = Client::connect(&harness.paths.data).await;
        deaf.hello(DATA).await;
        let flood: Vec<u8> = (0..20_000)
            .flat_map(|_| {
                encode_frame(r#"{"type":"request","id":"x","method":"no.such.method"}"#)
                    .expect("frame")
            })
            .collect();
        let _ = tokio::time::timeout(
            Duration::from_millis(500),
            tokio::io::AsyncWriteExt::write_all(&mut deaf.write, &flood),
        )
        .await;
        let stopped = tokio::time::timeout(PATIENCE, harness.stop()).await;
        assert!(
            stopped.is_ok(),
            "stop returned with a deaf client connected"
        );
        drop(deaf);
    }
}

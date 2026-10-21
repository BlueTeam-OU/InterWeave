// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! S9, between two shipped daemons: a local caller cannot name its own
//! source endpoint (threat-model.md, "Endpoint source spoof by ordinary
//! local caller"), and a remote message cannot reach the admin authority
//! (threat-model.md, "Admin confused deputy").
//!
//! Both daemons run the shipped desktop example on the host's private
//! address, as `tests/desktop-e2e`'s exchange does: A trusts B and B
//! trusts A, each with the `human` (human-client) and `claude`
//! (claude-channel) endpoints.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::time::Duration;

use interweave_ipc_protocol::{
    DecodedFrame, Frame, FrameError, Request, RequestId, SendParams, decode_frame, encode_frame,
};
use interweave_local_client_api::{
    AdminBinding as _, AdminCapability, AdminPort as _, DataCapability, DataSessionBinding as _,
    DataSessionPort as _, SessionEvent, SessionRequest,
};
use interweave_test_support::e2e::{PATIENCE, human, stranger};
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, DirectDestination, EndpointId, MessageId, TransportError,
    TransportIdentity,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::UnixStream;

mod common;

use common::{Pair, arriving, payload};

fn claude() -> EndpointId {
    EndpointId::parse("claude").expect("endpoint")
}

/// One raw data connection, its hello written by the test.
struct Raw {
    stream: UnixStream,
    buf: Vec<u8>,
}

impl Raw {
    async fn hello(socket: &Path, hello: &str) -> (Self, Frame) {
        let mut raw = Self {
            stream: UnixStream::connect(socket).await.expect("connects"),
            buf: Vec::new(),
        };
        raw.send(hello).await;
        let first = raw.next().await.expect("an answer to the hello");
        (raw, first)
    }

    async fn send(&mut self, body: &str) {
        self.stream
            .write_all(&encode_frame(body).expect("a frame"))
            .await
            .expect("sent");
    }

    async fn next(&mut self) -> Option<Frame> {
        tokio::time::timeout(PATIENCE, async {
            loop {
                match decode_frame(&self.buf) {
                    Ok(DecodedFrame { body, consumed }) => {
                        let frame = Frame::parse(&body).expect("a frame");
                        self.buf.drain(..consumed);
                        return Some(frame);
                    }
                    Err(FrameError::Incomplete { .. }) => {}
                    Err(e) => panic!("a bad frame: {e:?}"),
                }
                let mut chunk = [0_u8; 4096];
                match self.stream.read(&mut chunk).await {
                    Ok(0) | Err(_) => return None,
                    Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                }
            }
        })
        .await
        .expect("answered in time")
    }

    /// The response to `id`: `Ok` or the error code. Pushed frames and
    /// keepalive probes are passed over -- a probe answered, since the
    /// lease requires keepalive.
    async fn response(&mut self, id: &str) -> Result<(), TransportError> {
        loop {
            match self.next().await {
                Some(Frame::Response(response)) if response.id.as_str() == id => {
                    return response.error.map_or(Ok(()), |e| Err(e.code));
                }
                Some(Frame::Ping(ping)) => {
                    let echo = serde_json::json!({"type": "pong", "nonce": ping.nonce});
                    self.send(&echo.to_string()).await;
                }
                Some(_) => {}
                None => panic!("closed before answering {id}"),
            }
        }
    }
}

/// A `direct.send` to `peer`'s `human`, as the body a client writes;
/// `extra` is added to its params.
fn direct_send(
    id: &str,
    peer: &TransportIdentity,
    n: u8,
    extra: Option<(&str, serde_json::Value)>,
) -> String {
    let body = Frame::Request(
        Request::DirectSend(SendParams {
            peer: peer.clone(),
            endpoint: Some(human()),
            message_id: MessageId::from_bytes([n; 16]),
            payload: payload(format!("from claude {n}").as_bytes()),
        })
        .into_frame(RequestId::new(id).expect("id"), None),
    )
    .to_body();
    let mut value: serde_json::Value = serde_json::from_str(&body).expect("json");
    if let Some((key, field)) = extra {
        value["params"][key] = field;
    }
    value.to_string()
}

/// A local caller cannot choose its source endpoint: the source is the
/// lease's (ADR-0030). On the shipped daemon, a session leased to
/// `claude` that adds a source of `human` to its `direct.send` -- under
/// each name a source could be given -- is refused `InvalidArgument`
/// over a route that exists, and a hello claiming `human` while another
/// session holds it is refused `EndpointInUse`. The controls are the same send without the field,
/// before the spoofs and after them, which B receives with `claude` as
/// their source; B receives nothing else.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_source_endpoint_named_by_the_caller_is_refused_and_the_receiver_reads_the_lease() {
    let pair = Pair::start().await;
    let receiver = pair
        .b
        .binding()
        .open(interweave_test_support::e2e::lease_request())
        .await
        .expect("B leases human");
    let holder = pair
        .a
        .binding()
        .open(interweave_test_support::e2e::lease_request())
        .await
        .expect("A's human is held");

    let (_, squatter) = Raw::hello(
        &pair.a.data_socket(),
        r#"{"type":"hello","ipc_version":{"major":2,"minor":0},
        "client":{"kind":"human-client"},"endpoint":{"id":"human"},
        "requested_capabilities":["events","commands"],"features":["keepalive"]}"#,
    )
    .await;
    match squatter {
        Frame::Close(close) => assert_eq!(close.code, TransportError::EndpointInUse),
        other => panic!("human is held, got {other:?}"),
    }

    let (mut spoofer, granted) = Raw::hello(
        &pair.a.data_socket(),
        r#"{"type":"hello","ipc_version":{"major":2,"minor":0},
        "client":{"kind":"claude-channel"},"endpoint":{"id":"claude"},
        "requested_capabilities":["events","commands"],"features":["keepalive"]}"#,
    )
    .await;
    assert!(
        matches!(granted, Frame::HelloResponse(_)),
        "claude leased: {granted:?}"
    );
    // The control first: the same send, no source named, retried until
    // the daemons have found each other -- so the route exists, and a
    // refusal below is the named source and nothing else.
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let mut attempt = 0_u32;
    loop {
        attempt += 1;
        let id = format!("plain-{attempt}");
        spoofer
            .send(&direct_send(&id, &pair.b_peer, 100, None))
            .await;
        if spoofer.response(&id).await.is_ok() {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no direct route:\n{}",
            pair.logs()
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let spoofs = [
        ("source_endpoint", serde_json::json!("human")),
        ("source", serde_json::json!({"endpoint": "human"})),
        ("from", serde_json::json!("human")),
    ];
    for (n, (key, field)) in spoofs.into_iter().enumerate() {
        let id = format!("spoof-{n}");
        let n = u8::try_from(n).expect("small");
        spoofer
            .send(&direct_send(&id, &pair.b_peer, n, Some((key, field))))
            .await;
        assert_eq!(
            spoofer.response(&id).await,
            Err(TransportError::InvalidArgument),
            "a send naming its source as {key}"
        );
    }
    // And the same send again once more: the refusals took nothing.
    spoofer
        .send(&direct_send("after", &pair.b_peer, 101, None))
        .await;
    assert_eq!(
        spoofer.response("after").await,
        Ok(()),
        "the second control"
    );
    let got = arriving(&receiver, Duration::from_secs(2)).await;
    let sources: Vec<(EndpointId, Vec<u8>)> = got
        .iter()
        .filter_map(|e| match e {
            SessionEvent::Direct(d) => {
                Some((d.source_endpoint.clone(), d.payload.bytes().to_vec()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        sources,
        vec![
            (claude(), b"from claude 100".to_vec()),
            (claude(), b"from claude 101".to_vec())
        ],
        "only the controls arrived, each from the lease"
    );
    drop((receiver, holder, spoofer));
    pair.stop().await;
}

/// Payloads that ARE admin requests -- as the JSON a client writes, and
/// as a whole length-prefixed IPC frame -- sent by a trusted remote peer
/// directly and on a channel.
fn admin_shaped(stranger: &TransportIdentity) -> Vec<Vec<u8>> {
    let grant = format!(
        r#"{{"type":"request","id":"x","method":"admin.trust.set","params":{{"peer":"{}","allowed":true}}}}"#,
        stranger.as_str()
    );
    let revoke = r#"{"type":"request","id":"y","method":"admin.endpoints.revoke","params":{"endpoint":"human"}}"#;
    let shutdown = r#"{"type":"request","id":"z","method":"admin.shutdown"}"#;
    vec![
        grant.clone().into_bytes(),
        encode_frame(&grant).expect("a frame"),
        revoke.as_bytes().to_vec(),
        encode_frame(shutdown).expect("a frame"),
    ]
}

/// A network message cannot act as the admin authority
/// (threat-model.md, "Admin confused deputy"): a trusted peer sends B
/// admin requests as payloads, directly and as broadcasts. Each reaches
/// B's session as the bytes it was -- the control that the deputy was
/// handed them -- and B's trust list, endpoints and leases are exactly
/// as before, and B still runs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remote_message_shaped_as_an_admin_request_changes_nothing() {
    let mut pair = Pair::start().await;
    let receiver = pair
        .b
        .binding()
        .open(interweave_test_support::e2e::lease_request())
        .await
        .expect("B leases human");
    let sender = pair
        .a
        .binding()
        .open(
            SessionRequest::new(
                "claude-channel",
                Some(claude()),
                [DataCapability::Events, DataCapability::Commands],
            )
            .expect("a request"),
        )
        .await
        .expect("A leases claude");
    let admin = pair
        .b
        .binding()
        .admin(
            [
                AdminCapability::Status,
                AdminCapability::Endpoints,
                AdminCapability::Trust,
            ]
            .into(),
        )
        .await
        .expect("B's admin port");
    let trust_before = admin.trust().await.expect("trust");
    assert!(
        trust_before.peers().any(|p| p == &pair.a_peer),
        "the control: the list read is B's, trusting A"
    );
    let endpoints_before = format!("{:?}", admin.endpoints_result().await.expect("endpoints"));
    let target = stranger();
    let payloads = admin_shaped(&target);

    let deadline = tokio::time::Instant::now() + PATIENCE;
    for (n, bytes) in payloads.iter().enumerate() {
        let n = u8::try_from(n).expect("small");
        loop {
            let sent = sender
                .send_direct(
                    DirectDestination {
                        peer: pair.b_peer.clone(),
                        endpoint: Some(human()),
                    },
                    MessageId::from_bytes([n; 16]),
                    payload(bytes),
                )
                .await;
            if sent.is_ok() {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "no direct route: {sent:?}\n{}",
                pair.logs()
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    let general = ChannelId::parse("general").expect("a channel");
    sender.join(general.clone()).await.expect("A joins");
    receiver.join(general.clone()).await.expect("B joins");
    let mut direct = Vec::new();
    let mut broadcast = Vec::new();
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let mut round = 0_u8;
    while broadcast.len() < payloads.len() {
        // Published until the mesh has formed; each round sends all four.
        round = round.wrapping_add(1);
        for (n, bytes) in payloads.iter().enumerate() {
            let n = u8::try_from(n).expect("small");
            sender
                .broadcast(
                    general.clone(),
                    BroadcastMessageV1 {
                        message_id: MessageId::from_bytes(
                            [round.wrapping_mul(16).wrapping_add(n); 16],
                        ),
                        sent_at_ms: 0,
                        payload: payload(bytes),
                    },
                )
                .await
                .expect("accepted locally");
        }
        for event in arriving(&receiver, Duration::from_millis(500)).await {
            match event {
                SessionEvent::Direct(d) => direct.push(d.payload.bytes().to_vec()),
                SessionEvent::Broadcast(b) => {
                    let bytes = b.payload.bytes().to_vec();
                    if !broadcast.contains(&bytes) {
                        broadcast.push(bytes);
                    }
                }
                SessionEvent::Local(_) => {}
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the broadcasts did not all arrive ({} of {}):\n{}",
            broadcast.len(),
            payloads.len(),
            pair.logs()
        );
    }
    assert_eq!(direct, payloads, "each direct payload arrived as its bytes");
    broadcast.sort();
    let mut want = payloads.clone();
    want.sort();
    assert_eq!(broadcast, want, "each broadcast arrived as its bytes");

    let trust_after = admin.trust().await.expect("trust");
    assert_eq!(
        trust_after.peers().cloned().collect::<Vec<_>>(),
        trust_before.peers().cloned().collect::<Vec<_>>(),
        "no trust granted"
    );
    assert!(!trust_after.peers().any(|p| p == &target));
    assert_eq!(
        format!("{:?}", admin.endpoints_result().await.expect("endpoints")),
        endpoints_before,
        "no endpoint revoked or changed"
    );
    assert!(
        pair.b_daemon.child.try_wait().is_ok_and(|s| s.is_none()),
        "B was not shut down:\n{}",
        pair.logs()
    );
    drop((receiver, sender, admin));
    pair.stop().await;
}

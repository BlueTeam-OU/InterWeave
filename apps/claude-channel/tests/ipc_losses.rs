// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `status.broadcast_loss` over the real IPC binding, against a scripted
//! daemon on a Unix socket that speaks the frames byte for byte
//! (TOOL-SURFACE.md §status, A 2026-10-25). On a 2.11 connection the
//! bridge sends `session.losses` with no params and drains what the
//! daemon pushes ahead of the answer, which waits behind them
//! (LOCAL-IPC.md §Disconnect/reconnect and optional keepalive); on a
//! connection below 2.11 nothing is sent and the status says why.
//!
//! What this does not prove: a real daemon's counting, which is
//! `tests/desktop-e2e`'s and the runtime's own tests'.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_claude_channel::serve::{Config, Env, serve};
use interweave_ipc_client::{IpcBinding, SocketPaths};
use interweave_ipc_protocol::{
    DecodedFrame, DirectReceived, Frame, FrameError, decode_frame, encode_frame,
};
use interweave_local_client_api::ReceivedDirect;
use interweave_transport_api::{
    EndpointId, MAX_PAYLOAD_BYTES, MessageId, Payload, TransportIdentity,
};
use serde_json::{Value, json};
use tokio::io::{
    AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader, DuplexStream, Lines,
};
use tokio::net::{UnixListener, UnixStream};

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const PATIENCE: Duration = Duration::from_secs(10);
/// The bridge's clock: the session's open, `since`.
const NOW_MS: u64 = 1_791_227_222_497;
/// The granted event queue, and the events pushed ahead of the answer:
/// five times the buffer, so a bridge that does not drain while it
/// reads cannot see the answer.
const QUEUE: u32 = 2;
const BURST: u64 = 10;

struct Daemon {
    stream: UnixStream,
    buf: Vec<u8>,
}

impl Daemon {
    async fn write(&mut self, body: &Value) {
        let frame = encode_frame(&body.to_string()).expect("a frame");
        self.stream.write_all(&frame).await.expect("written");
    }

    /// The client's next frame, or `None` if none comes within `wait`.
    async fn next(&mut self, wait: Duration) -> Option<Frame> {
        tokio::time::timeout(wait, async {
            loop {
                match decode_frame(&self.buf) {
                    Ok(DecodedFrame { body, consumed }) => {
                        self.buf.drain(..consumed);
                        return Frame::parse(&body).expect("the client writes frames");
                    }
                    Err(FrameError::Incomplete { .. }) => {}
                    Err(e) => panic!("a bad frame: {e:?}"),
                }
                let mut chunk = [0_u8; 4096];
                let n = self.stream.read(&mut chunk).await.expect("reads");
                assert!(n > 0, "the client closed");
                self.buf.extend_from_slice(&chunk[..n]);
            }
        })
        .await
        .ok()
    }
}

struct Host {
    input: DuplexStream,
    output: Lines<BufReader<DuplexStream>>,
}

impl Host {
    async fn call_status(&mut self, id: u64) {
        let call = json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
                          "params": {"name": "status", "arguments": {}}});
        self.input
            .write_all(format!("{call}\n").as_bytes())
            .await
            .expect("written");
    }

    async fn line(&mut self) -> Value {
        let line = tokio::time::timeout(PATIENCE, self.output.next_line())
            .await
            .expect("a line in time")
            .expect("readable")
            .expect("not closed");
        serde_json::from_str(&line).expect("json")
    }

    /// `status` answered under `id`, and the notifications seen before it.
    async fn status(&mut self, id: u64) -> (Value, u64) {
        let mut notified = 0;
        loop {
            let line = self.line().await;
            if line["id"] == json!(id) {
                let text = line["result"]["content"][0]["text"]
                    .as_str()
                    .expect("a tool result");
                return (
                    serde_json::from_str(text).expect("status is JSON"),
                    notified,
                );
            }
            if line["method"] == json!("notifications/claude/channel") {
                notified += 1;
            }
        }
    }
}

/// The bridge on `claude`, connected to a daemon that answers its hello
/// at `minor`.
async fn bridge(root: &std::path::Path, minor: u64) -> (Daemon, Host) {
    let data = root.join("data.sock");
    let listener = UnixListener::bind(&data).expect("binds");
    let binding = IpcBinding::new(
        SocketPaths {
            data,
            admin: root.join("admin.sock"),
        },
        "claude-channel",
    );
    let (input, bridge_in) = tokio::io::duplex(1 << 16);
    let (bridge_out, output) = tokio::io::duplex(1 << 16);
    let env = Env {
        now_ms: Box::new(|| NOW_MS),
        entropy: Box::new({
            let mut n = 0_u8;
            move || {
                n = n.wrapping_add(1);
                [n; 16]
            }
        }),
        log: interweave_claude_channel::log::Log::stderr_only(
            interweave_claude_channel::log::Level::Debug,
            "test",
        ),
    };
    let config = Config {
        endpoint: EndpointId::parse("claude").expect("endpoint"),
        desired_channels: Ok(Vec::new()),
        delivery: interweave_claude_channel_core::Delivery::Push,
    };
    tokio::spawn(serve(
        binding,
        config,
        env,
        BufReader::new(bridge_in),
        bridge_out,
    ));
    let (stream, _) = tokio::time::timeout(PATIENCE, listener.accept())
        .await
        .expect("the bridge connects")
        .expect("accepts");
    let mut daemon = Daemon {
        stream,
        buf: Vec::new(),
    };
    assert!(matches!(daemon.next(PATIENCE).await, Some(Frame::Hello(_))));
    daemon
        .write(&json!({
            "type": "hello_response",
            "ipc_version": {"major": 2, "minor": minor},
            "transport_contract_version": "2.0",
            "peer": PEER,
            "endpoint": "claude",
            "endpoint_lease_epoch": "AAAAAAAAAAAAAAAAAAAAAQ",
            "event_queue": QUEUE,
            "granted_capabilities": ["events", "commands"]
        }))
        .await;
    let host = Host {
        input,
        output: BufReader::new(output).lines(),
    };
    (daemon, host)
}

async fn wait_held(host: &mut Host, first_id: u64) -> u64 {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let mut id = first_id;
    loop {
        host.call_status(id).await;
        let (status, _) = host.status(id).await;
        id += 1;
        if status["endpoint_lease_state"] == json!("held") {
            return id;
        }
        assert!(tokio::time::Instant::now() < deadline, "never held");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// On 2.11 `status` sends `session.losses`, drains the events pushed
/// ahead of the answer -- ten behind a two-event queue -- and reports
/// the answer's two counts under their own names with `since`.
#[tokio::test]
async fn status_reads_session_losses_on_the_wire_behind_a_burst() {
    let root = tempfile::tempdir().expect("tempdir");
    let (mut daemon, mut host) = bridge(root.path(), 11).await;
    // The first status asks for the counts: answer them so the bridge is
    // held, with nothing lost yet.
    let id = 1;
    host.call_status(id).await;
    let Some(Frame::Request(request)) = daemon.next(PATIENCE).await else {
        panic!("the losses request")
    };
    assert_eq!(request.method, "session.losses");
    assert!(
        request.params.as_ref().is_none_or(|p| p.get() == "{}"),
        "no params: {:?}",
        request.params
    );
    daemon
        .write(
            &json!({"type": "response", "id": request.id.as_str(), "ok": true,
                       "result": {"queue_overflow": 0, "ingress_refused": 0}}),
        )
        .await;
    let (status, _) = host.status(id).await;
    assert_eq!(status["endpoint_lease_state"], json!("held"), "{status}");

    host.call_status(2).await;
    let Some(Frame::Request(request)) = daemon.next(PATIENCE).await else {
        panic!("the second losses request")
    };
    assert_eq!(request.method, "session.losses");
    for sequence in 0..BURST {
        let message = ReceivedDirect {
            source_peer: TransportIdentity::parse(PEER).expect("peer"),
            source_endpoint: EndpointId::parse("human").expect("endpoint"),
            destination_endpoint: EndpointId::parse("claude").expect("endpoint"),
            message_id: MessageId::from_bytes([u8::try_from(sequence).expect("small"); 16]),
            payload: Payload::new(None, format!("m{sequence}").into_bytes(), MAX_PAYLOAD_BYTES)
                .expect("payload"),
            received_at_ms: 0,
        };
        daemon
            .write(&json!({
                "type": "event", "sequence": sequence, "event_type": "message.direct",
                "data": serde_json::to_value(DirectReceived::from(message)).expect("data")
            }))
            .await;
    }
    daemon
        .write(
            &json!({"type": "response", "id": request.id.as_str(), "ok": true,
                       "result": {"queue_overflow": 7, "ingress_refused": 3}}),
        )
        .await;
    // The answer sits behind the burst: a bridge that awaited it without
    // draining would see it only once its two-event buffer was taken, so
    // never. All but the buffer's two were written before the answer.
    let (status, notified) = host.status(2).await;
    assert!(
        notified >= BURST - u64::from(QUEUE),
        "{notified} of {BURST} written before the answer"
    );
    let mut rest = notified;
    while rest < BURST {
        if host.line().await["method"] == json!("notifications/claude/channel") {
            rest += 1;
        }
    }
    assert_eq!(
        status["broadcast_loss"],
        json!({
            "queue_overflow": 7,
            "ingress_refused": 3,
            "since": interweave_claude_channel_core::rfc3339_utc(NOW_MS).expect("an instant"),
        })
    );
}

/// Below 2.11 the client answers `ProtocolUnsupported` itself: the
/// status says `protocol_unsupported`, never zeros, and no frame leaves
/// the bridge for it.
#[tokio::test]
async fn below_2_11_status_sends_nothing_and_says_why() {
    let root = tempfile::tempdir().expect("tempdir");
    let (mut daemon, mut host) = bridge(root.path(), 10).await;
    let id = wait_held(&mut host, 1).await;
    host.call_status(id).await;
    let (status, _) = host.status(id).await;
    assert_eq!(
        status["broadcast_loss"],
        json!({"unavailable": "protocol_unsupported"})
    );
    assert!(
        daemon.next(Duration::from_millis(300)).await.is_none(),
        "no frame for a method the connection does not have"
    );
}

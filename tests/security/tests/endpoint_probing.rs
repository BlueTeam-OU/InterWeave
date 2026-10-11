// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint-probing oracle, read off the wire.
//!
//! threat-model.md's row: a trusted peer destinations which endpoints exist
//! and which refuse it. The mitigation collapses unknown, offline,
//! disabled and policy-denied into one `no_route` answer of the same
//! code and shape. `direct_inbound.rs` unit-tests the encoder, and the
//! Stage 6 wire tests read back the DECODED error, which is equal by
//! construction once decoded. Neither shows what the bytes a prober
//! actually receives say. This test is the prober. It speaks the real
//! protocol id over the real stack, reads each answer raw, and requires
//! the four refusals to be identical byte for byte once the request id
//! each one echoes is masked.
//!
//! A positive control sits beside them: a frame to a leased endpoint is
//! `Accepted`, so the prober is reaching the routing layer and not
//! being refused by something in front of it.

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use futures::{AsyncReadExt as _, AsyncWriteExt as _, StreamExt as _};
use interweave_profile_config::{
    ChannelsConfig, DirectoryConfig, EndpointConfig, EndpointsConfig, ProfileConfig,
    RegistrationPolicy, TrustConfig, TrustPolicyKind,
};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{
    DirectMessageV2, DirectRejectReason, EndpointId, MediaType, MessageId, Payload,
    TransportIdentity,
};
use interweave_transport_libp2p::direct_codec::{DIRECT_PROTOCOL, DirectResponse, decode_response};
use interweave_transport_libp2p::runtime::{DirectEndpoints, SubstrateConfig, SwarmRuntime};
use interweave_transport_runtime::TrustSources;
use interweave_transport_runtime::ingress::DEFAULT_PER_PEER_BURST;
use interweave_trust_api::{EndpointTrustPolicy, InfrastructureSet, PeerTrustPolicy};
use libp2p::request_response::{self, Codec, ProtocolSupport};
use libp2p::swarm::SwarmEvent as Libp2pSwarmEvent;
use libp2p::{StreamProtocol, SwarmBuilder};

const PATIENCE: Duration = Duration::from_secs(20);

/// Bytes in, bytes out: the prober does no validation of its own.
#[derive(Clone, Default)]
struct RawCodec;

impl Codec for RawCodec {
    type Protocol = StreamProtocol;
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    async fn read_request<T>(&mut self, _: &StreamProtocol, io: &mut T) -> std::io::Result<Vec<u8>>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.take(64 * 1024).read_to_end(&mut buf).await?;
        Ok(buf)
    }

    async fn read_response<T>(&mut self, _: &StreamProtocol, io: &mut T) -> std::io::Result<Vec<u8>>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.take(64 * 1024).read_to_end(&mut buf).await?;
        Ok(buf)
    }

    async fn write_request<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        req: Vec<u8>,
    ) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        io.write_all(&req).await?;
        io.close().await
    }

    async fn write_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        res: Vec<u8>,
    ) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        io.write_all(&res).await?;
        io.close().await
    }
}

fn endpoint(name: &str) -> EndpointId {
    EndpointId::parse(name).expect("valid endpoint id")
}

fn entry(name: &str) -> EndpointConfig {
    EndpointConfig {
        id: endpoint(name),
        enabled: true,
        advertise: false,
        allowed_client_kinds: Vec::new(),
        inbound: EndpointTrustPolicy::default(),
        outbound: EndpointTrustPolicy::default(),
    }
}

fn profile_with(entries: Vec<EndpointConfig>) -> ProfileConfig {
    ProfileConfig {
        runtime: interweave_profile_config::runtime::RuntimeConfig::default(),
        ipc: interweave_profile_config::ipc::IpcConfig::default(),
        profile: None,
        identity: interweave_profile_config::sections::IdentityConfig::default(),
        observability: interweave_profile_config::sections::ObservabilityConfig::default(),
        transport: interweave_profile_config::connectivity::TransportConfig::default(),
        schema_version: 2,
        trust: TrustConfig {
            policy: TrustPolicyKind::default(),
            allowed_peers: std::collections::BTreeSet::new(),
        },
        endpoints: EndpointsConfig {
            registration_policy: RegistrationPolicy::default(),
            default_direct_endpoint: Some(endpoint("human")),
            directory: DirectoryConfig::default(),
            entries,
        },
        discovery: interweave_profile_config::DiscoveryConfig::default(),
        channels: ChannelsConfig::default(),
    }
}

/// A frame from the prober to `destination`, under its own request id.
fn probe(destination: &str, id: u8) -> Vec<u8> {
    DirectMessageV2 {
        message_id: MessageId::from_bytes([id; 16]),
        sent_at_ms: 1,
        source_endpoint: endpoint("human"),
        destination_endpoint: Some(endpoint(destination)),
        payload: Payload::at_ceiling(
            Some(MediaType::parse("text/plain").expect("valid media type")),
            b"is anyone there".to_vec(),
        )
        .expect("within the ceiling"),
    }
    .encode()
}

/// `response` with the request id it echoes zeroed, so two answers to
/// two requests can be compared for everything else. The id must
/// appear exactly once, or the mask would hide a difference.
fn masked(response: &[u8], id: u8) -> Vec<u8> {
    let needle = [id; 16];
    let at: Vec<usize> = response
        .windows(16)
        .enumerate()
        .filter_map(|(i, w)| (w == needle).then_some(i))
        .collect();
    assert_eq!(at.len(), 1, "the echoed id appears once in {response:?}");
    let mut out = response.to_vec();
    out[at[0]..at[0] + 16].fill(0);
    out
}

/// A receiver on the shipped config whose endpoints cover every cause,
/// and a raw prober it trusts, connected to it.
///
/// - `human` is configured, enabled and leased: the control.
/// - `claude` is configured and enabled, but nothing holds it: offline.
/// - `off` is configured and disabled.
/// - `narrow` is leased, but its inbound policy admits nobody, the
///   profile-trusted prober included: policy-denied.
/// - anything else is not configured at all: unknown.
struct Probing {
    receiver: SwarmRuntime,
    prober: libp2p::Swarm<request_response::Behaviour<RawCodec>>,
    receiver_peer: libp2p::PeerId,
}

async fn probing() -> Probing {
    let receiver_id = ProfileIdentity::generate();
    let receiver_peer = receiver_id.transport_identity().expect("peer id");
    let prober_keys = libp2p::identity::Keypair::generate_ed25519();
    let prober_peer = TransportIdentity::parse(prober_keys.public().to_peer_id().to_string())
        .expect("a valid peer id");

    let mut off = entry("off");
    off.enabled = false;
    let mut narrow = entry("narrow");
    narrow.inbound = EndpointTrustPolicy::StaticSubset {
        allowed_peers: std::collections::BTreeSet::new(),
    };
    let profile = profile_with(vec![entry("human"), entry("claude"), off, narrow]);

    let mut receiver = SwarmRuntime::start(
        &receiver_id,
        SubstrateConfig::default(),
        TrustSources::new(
            PeerTrustPolicy::new([prober_peer]).expect("a one-peer allowlist"),
            InfrastructureSet::default(),
        ),
    )
    .expect("the receiver starts");
    // A queue far above any burst here, so the queue is never the one
    // refusing.
    receiver
        .configure_direct(DirectEndpoints::from_profile(&profile, 512).expect("a valid profile"))
        .await
        .expect("endpoints install");
    for name in ["human", "narrow"] {
        receiver
            .claim_endpoint(name, endpoint(name), "in-process")
            .await
            .expect("the claim reaches the task")
            .expect("the endpoint is configured and free");
    }
    let address = receiver
        .listen("/ip4/127.0.0.1/tcp/0".parse().expect("loopback"))
        .await
        .expect("listens");

    let mut prober = SwarmBuilder::with_existing_identity(prober_keys)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .expect("the same transport stack the receiver uses")
        .with_behaviour(|_| {
            request_response::Behaviour::<RawCodec>::new(
                [(DIRECT_PROTOCOL, ProtocolSupport::Full)],
                request_response::Config::default(),
            )
        })
        .expect("behaviour")
        .build();
    prober.dial(address).expect("the dial starts");
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        tokio::select! {
            event = prober.select_next_some() => {
                if matches!(event, Libp2pSwarmEvent::ConnectionEstablished { .. }) {
                    break;
                }
            }
            _ = receiver.next_event() => {}
            () = tokio::time::sleep_until(deadline) => panic!("the prober never connected"),
        }
    }
    Probing {
        receiver,
        prober,
        receiver_peer: receiver_peer.as_str().parse().expect("a peer id"),
    }
}

/// Send every frame at once and return the raw answers in send order.
async fn ask(probing: &mut Probing, frames: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut order = std::collections::HashMap::new();
    for (index, frame) in frames.into_iter().enumerate() {
        let request = probing
            .prober
            .behaviour_mut()
            .send_request(&probing.receiver_peer, frame);
        order.insert(request, index);
    }
    let mut answers = vec![None; order.len()];
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while answers.iter().any(Option::is_none) {
        tokio::select! {
            event = probing.prober.select_next_some() => match event {
                Libp2pSwarmEvent::Behaviour(request_response::Event::Message {
                    message: request_response::Message::Response { request_id, response },
                    ..
                }) => answers[order[&request_id]] = Some(response),
                Libp2pSwarmEvent::Behaviour(request_response::Event::OutboundFailure {
                    error, ..
                }) => panic!("a probe was not answered: {error:?}"),
                _ => {}
            },
            _ = probing.receiver.next_event() => {}
            () = tokio::time::sleep_until(deadline) => panic!("not every probe was answered"),
        }
    }
    answers.into_iter().map(|a| a.expect("answered")).collect()
}

fn reason(answer: &[u8]) -> Option<DirectRejectReason> {
    match decode_response(answer).expect("the frozen shape") {
        DirectResponse::Accepted { .. } => None,
        DirectResponse::Rejected { reason, .. } => Some(reason),
    }
}

#[tokio::test]
async fn the_four_no_route_causes_are_byte_identical_on_the_wire() {
    let mut probing = probing().await;

    // One request per destination, each under its own id. The ids are
    // far from the response's one-byte tag and reason codes, so the mask
    // below finds each one exactly where it is echoed.
    let destinations = [
        ("human", 0xa1_u8),
        ("claude", 0xa2),
        ("off", 0xa3),
        ("narrow", 0xa4),
        ("nonexistent", 0xa5),
    ];
    let answers = ask(
        &mut probing,
        destinations.iter().map(|(d, id)| probe(d, *id)).collect(),
    )
    .await;

    // THE CONTROL: the leased endpoint accepts, so the prober reached
    // the routing layer.
    assert_eq!(reason(&answers[0]), None, "the control is accepted");

    // THE ORACLE: four causes, one answer.
    let refusals: Vec<(&str, Vec<u8>)> = destinations[1..]
        .iter()
        .zip(&answers[1..])
        .map(|((d, id), bytes)| (*d, masked(bytes, *id)))
        .collect();
    for (destination, bytes) in &refusals {
        assert_eq!(
            reason(bytes),
            Some(DirectRejectReason::NoRoute),
            "{destination} is refused as no_route"
        );
        assert_eq!(
            bytes, &refusals[0].1,
            "{destination}'s refusal must be byte-identical to {}'s",
            refusals[0].0
        );
    }

    probing.receiver.shutdown().await.expect("shuts down");
}

#[tokio::test]
async fn a_probe_sweep_spends_the_probers_direct_allowance() {
    // A refusal that cost nothing would make enumeration free: the
    // prober could ask after every name it can spell. The per-peer
    // bucket runs before the frame is parsed, so a probe spends a token
    // whether or not it routes. Twice the shipped burst of probes for
    // names that do not exist: the first burst is answered `no_route`,
    // and past it the prober is told `overloaded`, which says nothing
    // about the endpoint.
    let mut probing = probing().await;
    let burst = usize::try_from(DEFAULT_PER_PEER_BURST).expect("fits");
    let sweep: Vec<Vec<u8>> = (0..2 * burst)
        .map(|n| probe(&format!("guess-{n}"), u8::try_from(n).expect("small")))
        .collect();
    let answers = ask(&mut probing, sweep).await;
    let reasons: Vec<Option<DirectRejectReason>> = answers.iter().map(|a| reason(a)).collect();

    let no_route = reasons
        .iter()
        .filter(|r| **r == Some(DirectRejectReason::NoRoute))
        .count();
    let overloaded = reasons
        .iter()
        .filter(|r| **r == Some(DirectRejectReason::Overloaded))
        .count();
    assert_eq!(
        no_route + overloaded,
        reasons.len(),
        "nothing else: {reasons:?}"
    );
    assert!(
        no_route >= burst,
        "the whole burst is spendable on probes, got {no_route}"
    );
    // One refill boundary of tolerance: the sweep takes milliseconds,
    // and the shipped rate earns a token every half second.
    assert!(
        no_route <= burst + 1,
        "past the burst, probes are refused for the rate and learn nothing, got {no_route}"
    );

    // And the sweep spent real allowance: a legitimate frame from the
    // same peer, to the endpoint that would accept it, is refused too.
    let after = ask(&mut probing, vec![probe("human", 0xee)]).await;
    assert_eq!(
        reason(&after[0]),
        Some(DirectRejectReason::Overloaded),
        "the prober's own allowance is what the sweep spent"
    );

    probing.receiver.shutdown().await.expect("shuts down");
}

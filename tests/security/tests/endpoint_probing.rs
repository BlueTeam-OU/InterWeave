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

#[tokio::test]
async fn the_four_no_route_causes_are_byte_identical_on_the_wire() {
    let receiver_id = ProfileIdentity::generate();
    let receiver_peer = receiver_id.transport_identity().expect("peer id");
    let prober_keys = libp2p::identity::Keypair::generate_ed25519();
    let prober_peer = TransportIdentity::parse(prober_keys.public().to_peer_id().to_string())
        .expect("a valid peer id");

    // The receiver's endpoints, one per cause:
    // - `human` is configured, enabled and leased: the control.
    // - `claude` is configured and enabled, but nothing holds it: offline.
    // - `off` is configured and disabled.
    // - `narrow` is leased, but its inbound policy admits nobody, the
    //   profile-trusted prober included: policy-denied.
    // - `nonexistent` is not configured at all: unknown.
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
    receiver
        .configure_direct(DirectEndpoints::from_profile(&profile, 8).expect("a valid profile"))
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
    let receiver_peer_id: libp2p::PeerId = receiver_peer.as_str().parse().expect("a peer id");
    prober.dial(address).expect("the dial starts");

    // One request per destination, each under its own id.
    // Ids far from the response's one-byte tag and reason codes, so the
    // mask below finds each one exactly where it is echoed.
    let destinations = [
        ("human", 0xa1_u8),
        ("claude", 0xa2),
        ("off", 0xa3),
        ("narrow", 0xa4),
        ("nonexistent", 0xa5),
    ];
    let mut answers: std::collections::HashMap<request_response::OutboundRequestId, (&str, u8)> =
        std::collections::HashMap::new();
    let mut raw: std::collections::BTreeMap<&str, (u8, Vec<u8>)> =
        std::collections::BTreeMap::new();
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while raw.len() < destinations.len() {
        tokio::select! {
            event = prober.select_next_some() => match event {
                Libp2pSwarmEvent::ConnectionEstablished { .. } if answers.is_empty() => {
                    for (destination, id) in destinations {
                        let request = prober
                            .behaviour_mut()
                            .send_request(&receiver_peer_id, probe(destination, id));
                        answers.insert(request, (destination, id));
                    }
                }
                Libp2pSwarmEvent::Behaviour(request_response::Event::Message {
                    message: request_response::Message::Response { request_id, response },
                    ..
                }) => {
                    let (destination, id) = answers[&request_id];
                    raw.insert(destination, (id, response));
                }
                Libp2pSwarmEvent::Behaviour(request_response::Event::OutboundFailure {
                    error, ..
                }) => panic!("a probe was not answered: {error:?}"),
                _ => {}
            },
            _ = receiver.next_event() => {}
            () = tokio::time::sleep_until(deadline) => panic!("not every probe was answered: {raw:?}"),
        }
    }

    // THE CONTROL: the leased endpoint accepts, so the prober reached
    // the routing layer.
    let (_, accepted) = &raw["human"];
    assert!(
        matches!(
            decode_response(accepted).expect("the frozen shape"),
            DirectResponse::Accepted { .. }
        ),
        "the control is accepted"
    );

    // THE ORACLE: four causes, one answer.
    let refusals: Vec<(&str, Vec<u8>)> = ["claude", "off", "narrow", "nonexistent"]
        .into_iter()
        .map(|d| {
            let (id, bytes) = &raw[d];
            (d, masked(bytes, *id))
        })
        .collect();
    for (destination, bytes) in &refusals {
        assert!(
            matches!(
                decode_response(bytes).expect("the frozen shape"),
                DirectResponse::Rejected {
                    reason: DirectRejectReason::NoRoute,
                    ..
                }
            ),
            "{destination} is refused as no_route"
        );
        assert_eq!(
            bytes, &refusals[0].1,
            "{destination}'s refusal must be byte-identical to {}'s",
            refusals[0].0
        );
    }

    receiver.shutdown().await.expect("shuts down");
}

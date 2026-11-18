// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Service's stop hands its session what the runtime admitted
//! (`DIRECT.md` §Graceful shutdown, A 2026-11-17, binding the embedded
//! binding too): between the Service's runtime and a second embedded host
//! on this machine, over a real connection.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(unix)]

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use interweave_human_android_platform::{ServiceHost, ServiceLaunch, ToView, stand_in};
use interweave_human_app_core::Update;
use interweave_human_chat_protocol::{HumanChatV2, MessageKind, encode_outbound};
use interweave_human_client_api::{ClientEvent, SessionState};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_local_client_api::{
    AdminBinding as _, AdminCapability, AdminPort as _, DataCapability, DataSessionBinding as _,
    DataSessionPort as _, SessionRequest,
};
use interweave_profile_config::{
    ProfilePaths, TrustBoundary, create_private_dir_within, provision::embedded_document,
};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{
    DirectDestination, EndpointId, MAX_PAYLOAD_BYTES, MediaType, MessageId, Payload,
    TransportIdentity,
};
use interweave_transport_embedded::{EmbeddedHost, EmbeddedLaunch};

const PROFILE: &str = "human";
const PEER_PROFILE: &str = "peer";
const WAIT: Duration = Duration::from_secs(20);
const GRACE: Duration = Duration::from_secs(1);

/// An app data directory as the platform hands it over: private, under a
/// parent that is not.
struct App {
    _root: tempfile::TempDir,
    dir: PathBuf,
}

fn app() -> App {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let open = root.path().join("open");
    let dir = open.join("app");
    for (path, mode) in [(&open, 0o777), (&dir, 0o700)] {
        std::fs::create_dir(path).expect("mkdir");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
    }
    App { _root: root, dir }
}

fn endpoint(id: &str) -> EndpointId {
    EndpointId::parse(id).expect("valid")
}

/// The peer: an embedded host trusting the phone, leasing `agent`,
/// listening on the wildcard for the phone to dial.
fn peer(app: &App, phone: &TransportIdentity, identity: ProfileIdentity) -> EmbeddedHost {
    let document = format!(
        "schema_version: 2
profile:
  name: {PEER_PROFILE}
runtime:
  deployment: embedded-android
  android:
    endpoint: agent
ipc:
  enabled: false
transport:
  listen:
    addresses: [\"/ip4/0.0.0.0/tcp/0\"]
trust:
  policy: static-allowlist
  allowed_peers: [\"{}\"]
endpoints:
  default_direct_endpoint: agent
  entries:
    - id: agent
      enabled: true
      advertise: true
discovery:
  providers:
    - type: peer-cache
      enabled: true
      priority: 10
      config: {{}}
",
        phone.as_str()
    );
    let paths = ProfilePaths::resolve_embedded(
        PEER_PROFILE,
        TrustBoundary::new(&app.dir).expect("a boundary"),
    )
    .expect("paths");
    create_private_dir_within(paths.config_dir(), paths.boundary()).expect("config dir");
    std::fs::write(paths.config_file(), document).expect("write");
    EmbeddedHost::start(EmbeddedLaunch {
        app_data_dir: app.dir.clone(),
        profile: PEER_PROFILE.to_owned(),
        identity,
    })
    .expect("the peer starts")
}

/// The phone's profile: the app's own first-run document, trusting the
/// peer and bootstrapping from `address`.
fn provision_phone(app: &App, peer: &TransportIdentity, address: &str) {
    let document = embedded_document(PROFILE)
        .replace(
            "allowed_peers: []",
            &format!("allowed_peers: [\"{}\"]", peer.as_str()),
        )
        .replace(
            "    - type: peer-cache\n",
            &format!(
                "    - type: static-bootstrap\n      enabled: true\n      priority: 5\n      config:\n        peers: [\"{address}\"]\n    - type: peer-cache\n"
            ),
        );
    stand_in::provision_replacing(&app.dir, PROFILE, &document).expect("provisioned");
}

/// Until `host`'s admin view shows `peer` connected.
fn wait_connected(host: &EmbeddedHost, peer: &TransportIdentity) {
    host.runtime().block_on(async {
        let port = host
            .binding()
            .admin(BTreeSet::from([AdminCapability::Status]))
            .await
            .expect("an admin port");
        let deadline = tokio::time::Instant::now() + WAIT;
        loop {
            assert!(
                tokio::time::Instant::now() < deadline,
                "{} not connected within {WAIT:?}",
                peer.as_str()
            );
            let rows = port.peers().await.expect("peer rows");
            if rows.iter().any(|row| &row.peer == peer && row.connected) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    });
}

fn message(text: &str) -> Payload {
    let encoded = encode_outbound(
        &HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{:032x}", 7),
            text: text.to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        },
        MAX_PAYLOAD_BYTES,
    )
    .expect("fits");
    Payload::at_ceiling(
        Some(MediaType::parse(&encoded.media_type).expect("valid")),
        encoded.bytes,
    )
    .expect("fits")
}

/// A direct message the phone's runtime admitted (`AcceptedV2`) before
/// the stop, and the facade had not yet taken, is in the store unread
/// once `ServiceHost::stop` returns: the runtime refuses from the stop's
/// start, the facade's close reads its session's queue and commits it,
/// then the session closes. The facade's turns are held from before the
/// send, so the message is the close's to read, as one landing between
/// two turns is on a device. As the gap was found the Service closed the
/// session first and the close read nothing: the message went with it.
/// Mutations: the close not taking the rest (`TransportClient::close`).
#[test]
fn a_message_admitted_before_the_stop_is_unread_after_it() {
    let app = app();
    let phone_identity = ProfileIdentity::generate();
    let phone = phone_identity.transport_identity().expect("a PeerId");
    let peer_identity = ProfileIdentity::generate();
    let peer_id = peer_identity.transport_identity().expect("a PeerId");
    let peer_host = peer(&app, &phone, peer_identity);
    let ip = interweave_test_support::net::require_private_interface_v4();
    let port = peer_host.listening()[0]
        .rsplit_once("/tcp/")
        .map(|(_, port)| port.to_owned())
        .expect("a TCP listener");
    provision_phone(
        &app,
        &peer_id,
        &format!("/ip4/{ip}/tcp/{port}/p2p/{}", peer_id.as_str()),
    );

    let service = ServiceHost::new();
    service
        .start(ServiceLaunch {
            app_data_dir: app.dir.clone(),
            profile: PROFILE.to_owned(),
            identity: phone_identity,
            store_cipher: Box::new(interweave_human_store::key_custody::FilesystemCustody),
            unlock: None,
        })
        .expect("started");
    let mut link = service.hub().attach(Arc::new(|| {}));
    let deadline = Instant::now() + WAIT;
    while !link.take().iter().any(|m| {
        matches!(m, ToView::Update(u)
            if matches!(**u, Update::Client(ClientEvent::Session(SessionState::Ready { .. }))))
    }) {
        assert!(
            Instant::now() < deadline,
            "no ready session within {WAIT:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    wait_connected(&peer_host, &phone);

    service.hub().hold_turns_for_test();
    // A turn under way when the hold was set may still read the queue:
    // past two of the facade's 100 ms polls, none is.
    std::thread::sleep(Duration::from_millis(250));
    let sent = peer_host.runtime().block_on(async {
        let session = peer_host
            .binding()
            .open(
                SessionRequest::new(
                    "peer",
                    Some(endpoint("agent")),
                    [DataCapability::Commands, DataCapability::Events],
                )
                .expect("bounds"),
            )
            .await
            .expect("a session");
        session
            .send_direct(
                DirectDestination {
                    peer: phone.clone(),
                    endpoint: Some(endpoint("human")),
                },
                MessageId::from_bytes([7; 16]),
                message("before the stop"),
            )
            .await
    });
    assert!(sent.is_ok(), "admitted before the stop: {sent:?}");

    let stopped = service.stop(GRACE).expect("it ran");
    assert!(stopped.session_closed, "the facade closed in time");
    let paths = ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&app.dir).expect("b"))
        .expect("paths");
    let store = HumanStore::open_profile(
        &paths,
        &interweave_human_store::key_custody::FilesystemCustody,
        StoreOptions::default(),
    )
    .expect("the store");
    let unread = store.unread_inbound().expect("unread");
    assert_eq!(
        unread
            .iter()
            .map(|row| row.app_message_id.as_str().to_owned())
            .collect::<Vec<_>>(),
        [format!("{:032x}", 7)],
        "the admitted message is unread in the store"
    );
    peer_host.stop(GRACE).expect("the peer stops");
}

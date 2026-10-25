// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Plan §29 (e)'s hostile-code case against a real daemon, and what the
//! daemon holds afterwards read over its own admin port.
//!
//! **A hostile code adds its addresses and nothing else.** The text a
//! hostile party's code decodes to, for a peer nobody trusts, handed to
//! the client as both shells hand a scan's text over
//! (`PeersInput::Scanned`), through the client's own root halves --
//! `app-core`'s model side and facade side over the human client's
//! facade, on the daemon's IPC sockets: the daemon's address list gains
//! exactly the code's lines, its trust set is unchanged, and no trust
//! change was asked of it, by command or on the admin socket. The
//! controls that each read is live: the trust set read before holds B,
//! the list afterwards holds the code's lines, and the tap saw the add.
//! The window is not in this case: the desktop withholds Paste an image
//! (#409 F1) and Open an image… is the XDG portal's dialog, which this
//! display has no backend for; what an image decodes to is `ui-slint`'s
//! (`code.rs`).
//!
//! What this does not prove: the camera's path on Android (the same
//! model input takes its text; `ui-model`'s `peers.rs` holds a scan's
//! refusals).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use interweave_human_app_core::{Command, FacadeSide, ModelSide, Opener, Surface, Update};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{PeersInput, ScanControl, UiModel, ViewEvent};
use interweave_ipc_client::IpcBinding;
use interweave_local_client_api::{
    AddressSource, AdminBinding as _, AdminCapability, AdminPort as _, BootstrapAddress,
};
use interweave_transport_api::MAX_PAYLOAD_BYTES;

use crate::common::{Home, PATIENCE, human};
use crate::daemon_link::{Tap, asked};
use crate::world::two_daemons;

/// What the daemon's trust policy allows, as strings, in its order.
async fn allowed(home: &Home) -> Vec<String> {
    let port = home
        .binding()
        .admin([AdminCapability::Trust].into())
        .await
        .expect("an admin port");
    let view = port.trust().await.expect("the trust view");
    view.peers().map(|p| p.as_str().to_owned()).collect()
}

/// Every bootstrap address the daemon holds, authored then added.
async fn addresses(home: &Home) -> Vec<BootstrapAddress> {
    let port = home
        .binding()
        .admin([AdminCapability::Peers].into())
        .await
        .expect("an admin port");
    port.bootstrap_addresses().await.expect("the address list")
}

/// A surface whose person acts from a queue the case fills.
#[derive(Clone, Default)]
struct Scripted(Rc<RefCell<VecDeque<Vec<ViewEvent>>>>);

impl Surface for Scripted {
    fn render(&mut self, _model: &UiModel) {}

    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        self.0.borrow_mut().pop_front().unwrap_or_default()
    }
}

/// Asked to open nothing in this case.
struct Nothing;

impl Opener for Nothing {
    fn open(&mut self, destination: &str) {
        panic!("asked to open {destination}");
    }

    fn ask_network_access(&mut self) {}

    fn authenticate(&mut self) {}
}

fn wall_ms() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after the epoch")
            .as_millis(),
    )
    .expect("ms")
}

/// One pass of the root's loop, as the desktop root runs it: the
/// facade's turn, then each command the model side asks for, its updates
/// applied.
async fn pump(
    model: &mut ModelSide<Scripted, Nothing>,
    facade: &mut FacadeSide<IpcBinding, IpcBinding>,
    commands: &mut Vec<Command>,
) {
    for update in facade.turn(wall_ms()).await {
        model.apply(update);
    }
    for command in model.turn() {
        commands.push(command.clone());
        for update in facade.execute(command, wall_ms()).await {
            model.apply(update);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hostile_code_scanned_adds_its_addresses_and_nothing_else_the_trust_set_unchanged() {
    let world = two_daemons().await;
    let hostile = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    let lines = vec![
        format!("/ip4/192.168.1.20/tcp/4001/p2p/{hostile}"),
        format!("/ip4/10.0.0.7/tcp/4001/p2p/{hostile}"),
    ];

    let trusted = allowed(&world.a).await;
    assert!(
        trusted.iter().any(|p| p == world.b_peer.as_str()),
        "the control: the trust read is live and holds B: {trusted:?}"
    );
    let before = addresses(&world.a).await;
    assert!(
        !before.iter().any(|r| r.address.contains(&hostile)),
        "{before:?}"
    );

    // The client's root halves over the human client's facade, on A's
    // sockets, as the desktop window composes them.
    let admin = Tap::install(&world.a.admin_socket());
    let store_dir = tempfile::tempdir().expect("a store directory");
    let store = HumanStore::open(
        &store_dir.path().join("state/human.sqlite"),
        StoreOptions::default(),
    )
    .expect("a store");
    let client = TransportClient::new(
        world.a.binding(),
        world.a.binding(),
        store,
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![human()],
            channels: Vec::new(),
            max_payload_bytes: MAX_PAYLOAD_BYTES,
        },
        Box::new(wall_ms),
        0,
    )
    .expect("an empty store's pending rows");
    let mut facade = FacadeSide::new(client, wall_ms);
    let person = Scripted::default();
    let mut model = ModelSide::new(person.clone(), Nothing);
    model.apply(Update::Listed(facade.listing().expect("a listing")));
    let mut commands = Vec::new();
    for _ in 0..4 {
        pump(&mut model, &mut facade, &mut commands).await;
    }

    // The person opens the add page and scans the hostile code.
    person.0.borrow_mut().extend([
        vec![ViewEvent::Peers(PeersInput::OpenAdd)],
        vec![ViewEvent::Peers(PeersInput::ScanPressed(
            ScanControl::Camera,
        ))],
        vec![ViewEvent::Peers(PeersInput::Scanned(lines.join("\n")))],
    ]);
    let deadline = Instant::now() + PATIENCE;
    let after = loop {
        pump(&mut model, &mut facade, &mut commands).await;
        let now = addresses(&world.a).await;
        if now.len() > before.len() {
            break now;
        }
        assert!(
            Instant::now() < deadline,
            "the scanned code was never added: {now:?}; commands {commands:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    let mut want = before.clone();
    want.extend(lines.iter().map(|line| BootstrapAddress {
        address: line.clone(),
        source: AddressSource::Added,
        dialled: true,
    }));
    assert_eq!(after, want, "its addresses and nothing else");
    assert_eq!(
        allowed(&world.a).await,
        trusted,
        "the trust set unchanged: {hostile} not added to it"
    );
    assert!(
        commands.contains(&Command::AddAddresses(lines.clone())),
        "the control: the scan was one add: {commands:?}"
    );
    assert!(
        !commands.iter().any(|c| matches!(c, Command::SetTrust(_))),
        "no trust change commanded: {commands:?}"
    );
    let methods = asked(&admin.from(i32::try_from(std::process::id()).expect("a pid"))).methods;
    assert!(
        methods.contains("admin.peers.add_addresses"),
        "the control: the tap saw the add: {methods:?}"
    );
    assert!(
        !methods.contains("admin.trust.set"),
        "no trust change asked: {methods:?}"
    );
}

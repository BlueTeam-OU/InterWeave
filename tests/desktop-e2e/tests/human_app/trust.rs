// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `human-client-ui.md` §13's trust-mutation bullet in the shipped client
//! (plan §18, B9): the trust settings show the exact target `PeerId`, the
//! change is made only on its confirmation, and it reaches the daemon
//! over the admin socket alone -- never the data socket, whatever the
//! client asks there.
//!
//! The change is the first identity's (ADR-0067 decision 6): its own set,
//! seeded from what the device trusted, loses B in both directions, while
//! the operator's list, the trust overlay and `config.yaml` are as they
//! were. And it outlives the daemon (ADR-0028 A 2026-11-22): after a
//! restart the identity's set still lacks B, and the client shows no
//! trusted peer. No row says where it came from (identities §16 Q5).
//!
//! What this does not prove: an allow typed into the field (its view and
//! root tests are `ui-slint`'s and `app-core`'s), or an added peer's row
//! across a restart (the daemon's own tests carry the overlay's both
//! halves).

use interweave_human_ui_model::{UiText, fill, placeholder_en};
use interweave_local_client_api::{
    AdminBinding as _, AdminCapability, AdminPort as _, EndpointSetKind, TrustDirection,
};

use crate::a11y::{Bus, Element};
use crate::common::human;
use crate::daemon_link::{Tap, asked};
use crate::display;
use crate::harness::{self as app, identity_set, until_lease};
use crate::ring;
use crate::world::two_daemons;

fn text(key: UiText) -> &'static str {
    placeholder_en::text(key)
}

/// The operator's list as the daemon holds it, sorted: what an identity's
/// trust act must leave as it is.
async fn operators(home: &crate::common::Home) -> Vec<String> {
    let port = home
        .binding()
        .admin([AdminCapability::Trust].into())
        .await
        .expect("an admin port");
    let mut peers: Vec<String> = port
        .trust()
        .await
        .expect("the operator's list")
        .peers()
        .map(|p| p.as_str().to_owned())
        .collect();
    peers.sort();
    peers
}

/// The trust overlay's bytes, if there is one, and `config.yaml`'s.
fn files(home: &crate::common::Home) -> (Option<Vec<u8>>, Vec<u8>) {
    (
        std::fs::read(
            home.paths
                .state_dir()
                .join(interweave_profile_config::trust_overlay::TRUST_OVERLAY_FILE),
        )
        .ok(),
        std::fs::read(home.paths.config_file()).expect("config.yaml"),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_trust_removal_shows_the_exact_peer_id_and_reaches_the_daemon_only_over_admin() {
    let _focus = display::exclusive().await;
    let mut world = two_daemons().await;
    let data = Tap::install(&world.a.data_socket());
    let admin = Tap::install(&world.a.admin_socket());
    let bus = Bus::connect().await;
    // What the revoke must leave as it is: the operator's list, the trust
    // overlay, the profile.
    let operators_before = operators(&world.a).await;
    assert!(
        operators_before.contains(&world.b_peer.as_str().to_owned()),
        "the control: the operator's list holds B"
    );
    let files_before = files(&world.a);
    let mut client = app::start(&world.a);
    until_lease(&world.a.binding(), true, &client).await;
    let pid = i32::try_from(client.child.id()).expect("a pid");
    let window = bus.window_of(client.child.id(), || client.log()).await;
    display::focus(client.child.id(), || client.log()).await;
    let log = || client.log();
    let button = |name: &'static str| {
        move |e: &Element| e.role == "button" && e.name == name && e.actions == ["click"]
    };

    // Trust, by the app's options (#304 F12).
    let options = window
        .until("the app's options", button(text(UiText::AppOptions)), log)
        .await;
    window.activate(&options, "click").await;
    let open = window
        .until(
            "the app's Trust entry",
            button(text(UiText::TrustHeading)),
            log,
        )
        .await;
    window.activate(&open, "click").await;

    // B's PeerId whole, on its row and on its removal's description; this
    // profile's own labelled beside them.
    let b = world.b_peer.as_str();
    window
        .until(
            "B's PeerId, whole, on its row",
            |e: &Element| e.name == b,
            log,
        )
        .await;
    let remove = window
        .until(
            "B's removal, naming B",
            |e: &Element| {
                e.role == "button" && e.name == text(UiText::RemoveTrust) && e.description == b
            },
            log,
        )
        .await;
    // Section 13's accessibility bullet, its trust leg: every trust control
    // is labelled for what it does, and each button can be pressed.
    let tree = window.read().await;
    let has = |role: &str, name: &str, actions: &[&str]| {
        tree.iter()
            .any(|e| e.role == role && e.name == name && e.actions == actions)
    };
    for (role, name, actions) in [
        ("entry", text(UiText::OwnPeerId), &[][..]),
        ("entry", text(UiText::PeerIdToTrust), &[][..]),
        ("button", text(UiText::TrustPeer), &["click"][..]),
        ("button", text(UiText::BackToConversations), &["click"][..]),
    ] {
        assert!(
            has(role, name, actions),
            "{role} {name:?} {actions:?} in the tree:\n{}",
            crate::a11y::describe(&tree)
        );
    }
    window.activate(&remove, "click").await;

    // The confirmation names the exact PeerId and what follows; nothing
    // has reached the daemon yet.
    let question = fill(text(UiText::ConfirmRevoke), &[("peer", b)]);
    window
        .until(
            "the confirmation, naming B whole",
            |e: &Element| e.name == question,
            log,
        )
        .await;
    assert!(
        !asked(&admin.from(pid)).methods.contains("admin.trust.set"),
        "a proposal asks the daemon nothing"
    );
    let confirm = window
        .until(
            // The dialog's act names it, Remove trust (#304 F4), and is
            // described by the question; the row's own Remove trust is
            // described by its PeerId alone.
            "the confirmation's Remove trust, describing it",
            |e: &Element| {
                e.role == "button"
                    && e.name == text(UiText::RemoveTrust)
                    && e.description == question
            },
            log,
        )
        .await;
    window.activate(&confirm, "click").await;
    let done = fill(text(UiText::PeerUntrusted), &[("peer", b)]);
    window
        .until("the outcome, naming B", |e: &Element| e.name == done, log)
        .await;

    // The daemon holds it: the first identity's own set no longer allows
    // B, either way, read by this test on its own admin connection; the
    // operator's list, the trust overlay and the profile are unchanged.
    let identity_lacks_b = |when: &'static str| {
        let binding = world.a.binding();
        let b = world.b_peer.clone();
        async move {
            for direction in TrustDirection::ALL {
                let view = identity_set(&binding, &human(), direction).await;
                assert_eq!(view.set, EndpointSetKind::Own, "{when}: {view:?}");
                assert!(
                    !view.allowed.iter().any(|row| row.peer == b),
                    "{when}: the identity no longer allows B, {direction:?}: {view:?}"
                );
            }
        }
    };
    identity_lacks_b("after the revoke").await;
    assert_eq!(
        operators(&world.a).await,
        operators_before,
        "the operator's list is not the identity's"
    );
    assert!(
        files(&world.a) == files_before,
        "the trust overlay and config.yaml are as they were"
    );

    // Over the admin socket and nowhere else: the trust capability and the
    // set on the client's admin connections, nothing administrative asked
    // or sent on its data connection. Control: the data connection was
    // seen leasing its endpoint.
    assert!(client.terminate().success(), "{}", client.log());
    let on_admin = asked(&admin.from(pid));
    let on_data = asked(&data.from(pid));
    assert!(
        on_admin.capabilities.contains("admin.trust")
            && on_admin.methods.contains("admin.trust.set"),
        "the trust set went over the admin socket: {on_admin:?}"
    );
    assert_eq!(
        on_data.endpoint.as_deref(),
        Some(human().as_str()),
        "control: the data tap read the client's lease: {on_data:?}"
    );
    let administrative =
        |names: &std::collections::BTreeSet<String>| names.iter().any(|n| n.starts_with("admin."));
    assert!(
        !administrative(&on_data.capabilities) && !administrative(&on_data.methods),
        "no administrative authority asked for or used on the data socket: {on_data:?}"
    );

    // THE CHANGE OUTLIVES THE DAEMON: restarted, the identity's set still
    // does not allow B.
    assert!(
        world.a_daemon.terminate().await.success(),
        "{}",
        world.a_daemon.log()
    );
    world.a_daemon = world.a.start(&[]);
    world.a_daemon.serving(&world.a).await;
    identity_lacks_b("after the daemon's restart").await;

    // And the client shows it: no trusted peer, after its own restart.
    let mut client = app::start(&world.a);
    until_lease(&world.a.binding(), true, &client).await;
    let window = bus.window_of(client.child.id(), || client.log()).await;
    display::focus(client.child.id(), || client.log()).await;
    let log = || client.log();
    // Trust, by the app's options (#304 F12).
    let options = window
        .until(
            "the app's options, after the restart",
            button(text(UiText::AppOptions)),
            log,
        )
        .await;
    window.activate(&options, "click").await;
    let open = window
        .until(
            "the app's Trust entry, after the restart",
            button(text(UiText::TrustHeading)),
            log,
        )
        .await;
    window.activate(&open, "click").await;
    window
        .until(
            "no trusted peer, after the restart",
            |e: &Element| e.name == text(UiText::NoTrustedPeer),
            log,
        )
        .await;
    assert!(client.terminate().success(), "{}", client.log());
}

/// #549 in the main window: this profile's own `PeerId` on Trust, a text
/// field, loses the ring Tab drew when a person clicks it (`ring`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_click_on_the_own_peer_id_after_tab_takes_its_ring_away() {
    let _focus = display::exclusive().await;
    let world = two_daemons().await;
    let bus = Bus::connect().await;
    let mut client = app::start(&world.a);
    until_lease(&world.a.binding(), true, &client).await;
    let pid = client.child.id();
    let window = bus.window_of(pid, || client.log()).await;
    display::focus(pid, || client.log()).await;
    let log = || client.log();
    let button = |name: &'static str| {
        move |e: &Element| e.role == "button" && e.name == name && e.actions == ["click"]
    };
    let options = window
        .until("the app's options", button(text(UiText::AppOptions)), log)
        .await;
    window.activate(&options, "click").await;
    let open = window
        .until(
            "the app's Trust entry",
            button(text(UiText::TrustHeading)),
            log,
        )
        .await;
    window.activate(&open, "click").await;
    let own = window
        .until(
            "this profile's PeerId",
            |e: &Element| e.role == "entry" && e.name == text(UiText::OwnPeerId),
            log,
        )
        .await;
    ring::tab_then_click(&window, &own, pid, log).await;
    assert!(client.terminate().success(), "{}", client.log());
}

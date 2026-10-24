// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The phone and a desktop daemon over a radio network: no `adb reverse`
//! anywhere, the phone listening on its Wi-Fi interface and the desktop
//! dialling the phone's LAN address after the person's add -- the
//! `admin.peers.add_address` method `transportctl peers add` calls. A
//! direct message crosses the one connection each way, and each side is
//! owed the route-begin notice reading `direct`. `adb` carries the
//! instrumentation and the status read only.
//!
//! Why the desktop dials: the host this suite was written on is a VM
//! behind NAT, which the phone cannot dial into, while the VM reaches
//! the LAN; one established connection carries both directions, so
//! which side dialled changes nothing a message crosses.
//!
//! The control is in the same run: before the add, D holds no address
//! for the phone and the phone none for D, and D's row for the phone
//! reads not connected while the phone is up and listening -- so the
//! connection that follows is D's dial of the address the add gave it.
//! And `adb reverse --list` is read empty before and after, so no port
//! of this host is reachable from the phone through adb.
//!
//! What this does NOT prove, by name: the phone dialling the desktop
//! (the NAT above); a relayed path, DCUtR or AutoNAT over the radio (the
//! profile's infrastructure is the shipped example's unreachable
//! placeholders); the phone's network callbacks on a change of network;
//! more than one LAN, or a LAN that isolates its clients; the phone in
//! the background or dozing; a scanned code (the add is the method
//! itself, not the client's screen). On the host, without a device, it
//! is ignored like every device case: a stand-in shares this host's
//! interfaces, so it could not show a radio either.
//!
//! Run it with the device's environment (`src/adb.rs`) and
//! `INTERWEAVE_ANDROID_WIFI_ADDRESS` set to the phone's Wi-Fi IPv4
//! address, under the device's lease.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddrV4};

use interweave_android_e2e_cases::{PathsArgs, Told, cases, close_exchange, keys};
use interweave_android_e2e_tests::adb::AdbDevice;
use interweave_android_e2e_tests::wifi::{
    established_to, listening_at, phone_address, reversed, until_listening,
};
use interweave_android_e2e_tests::{
    Desktop, Device, PATIENCE, desktop_answers, free_port, lease_request,
};
use interweave_local_client_api::{
    AddressChange, AdminBinding as _, AdminCapability, AdminPort as _, DataSessionBinding as _,
    ROUTE_ESTABLISHED,
};
use interweave_test_support::e2e::example;
use interweave_transport_api::PeerPath;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device on this host's LAN, adb, and the app's androidTest build"]
async fn over_wifi_the_desktop_dials_the_phone_and_a_message_crosses_each_way() {
    let phone_ip = phone_address();
    assert_eq!(reversed(), "", "a port reversed before the run");

    let mut phone = AdbDevice::connect();
    let mut d = Desktop::new();
    let (d_peer, phone_peer) = (d.peer.clone(), phone.peer());
    // Any port: the phone's, which nothing on this host holds; this host's
    // free-port answer only makes two runs unlikely to share one.
    let phone_port = free_port(Ipv4Addr::LOCALHOST);
    let listener = SocketAddrV4::new(phone_ip, phone_port);
    let to_phone = format!(
        "/ip4/{phone_ip}/tcp/{phone_port}/p2p/{}",
        phone_peer.as_str()
    );

    // D listens on the wildcard: a dial to the LAN leaves from this
    // host's own interface, never a loopback listener's port.
    let d_port = free_port(Ipv4Addr::UNSPECIFIED);
    d.start(&example(
        "human-desktop.yaml",
        &phone_peer,
        &format!("/ip4/0.0.0.0/tcp/{d_port}"),
        None,
    ))
    .await;
    phone.start(&listening_at(
        &example("human-android.yaml", &d_peer, "", None),
        phone_port,
    ));

    let d_session = d.binding().open(lease_request()).await.expect("D leases");
    let admin = d
        .binding()
        .admin(BTreeSet::from([
            AdminCapability::Peers,
            AdminCapability::Status,
        ]))
        .await
        .expect("D's admin port");
    let mut d_told = Told::default();
    let log = |side: &dyn Device| format!("{}\n--- the phone ---\n{}", d.log(), side.log());

    let run = phone.run_case(
        cases::PATHS,
        &PathsArgs {
            desktop: d_peer.clone(),
            path: PeerPath::Direct,
            serial: 1,
            deadline: PATIENCE * 2,
        }
        .to_json(),
    );
    until_listening(listener, &run, || log(&phone)).await;
    // The control: the phone up and listening, and nobody connected,
    // since neither side holds an address for the other.
    let connected = || async {
        admin
            .peers()
            .await
            .expect("D's peer rows")
            .iter()
            .any(|row| row.peer == phone_peer && row.connected)
    };
    assert!(
        !connected().await,
        "connected before the add:\n{}",
        log(&phone)
    );

    let added = admin.add_bootstrap_address(to_phone.clone()).await;
    assert!(
        matches!(added, Ok(AddressChange::Added)),
        "D's add of {to_phone}: {added:?}"
    );
    desktop_answers(&d_session, &mut d_told, &phone_peer, 1, &run, || {
        log(&phone)
    })
    .await;
    // The connection that carried both is this host's, to the phone's
    // LAN address.
    let sockets = established_to(listener);
    assert!(
        !sockets.is_empty(),
        "no TCP connection from this host to {listener} while the exchange held"
    );
    let closed = close_exchange(&d_session, &phone_peer, 1).await;
    let out = run.passed(|| format!("{}\n--- D's close ---\n{closed:?}", log(&phone)));
    assert_eq!(
        out[keys::PATH],
        PeerPath::Direct.label(),
        "the phone of D: direct"
    );
    let (_, notice) = d_told
        .path_notice(&d_session, &phone_peer, ROUTE_ESTABLISHED, 0, PATIENCE)
        .await
        .unwrap_or_else(|e| panic!("D of the phone: {e}"));
    assert_eq!(notice, (None, PeerPath::Direct), "D of the phone: direct");
    assert_eq!(reversed(), "", "a port reversed during the run");
    println!(
        "over Wi-Fi: D {} -> phone {to_phone}; sockets: {sockets:?}",
        d_peer.as_str()
    );

    drop(d_session);
    phone.stop();
    d.stop().await;
}

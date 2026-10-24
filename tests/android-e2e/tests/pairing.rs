// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Plan §29 (e)'s two rows only a phone shows, between two Android
//! sides on one LAN, each side's half a case of
//! `interweave-android-e2e-cases` run in its own process:
//!
//! - **An added address survives a restart.** A trusts B and B trusts A,
//!   neither authors a static-bootstrap entry and neither enables the
//!   peer cache, so the bootstrap-peers overlay is the only producer of
//!   an address. A's `added_address` case reads A unconnected to B for a
//!   window (the control: nothing else dials B), adds B's LAN address
//!   through the binding, and waits for the connection; B's `await_peer`
//!   holds no address for A, so the connection it sees is A's dial, and
//!   it ends only once A has left, which the host makes happen after A's
//!   case passes. A is killed and started again, and A's case finds the address still
//!   listed as added and connects again, with no add; B sees it again.
//! - **Two fresh profiles connect after both trust, and not before.**
//!   Each trusts nobody. Step 1: each adds the other's address, in place
//!   of the scan, and for a window no message either sends is accepted
//!   and none arrives. Step 2: A trusts B and B does not, and still
//!   nothing crosses either way. A dials and its connection comes up, and
//!   B closes it once established, so A's row may end reading
//!   `connected` as its last outcome (the stand-ins, 2026-10-24): what
//!   the step asserts is that no message is accepted, never that no
//!   connection was made. Step 3: B trusts A,
//!   and a message crosses each way: the positive control, over the
//!   addresses step 1 added and nothing else.
//!
//! Two runs of each: on two host stand-ins over this host's private
//! address (ADR-0052 refuses a loopback add), and on two devices
//! (`#[ignore]`d), run with `--ignored`, the instrumentation named as in
//! `src/adb.rs`, and each phone's `adb -s` serial and Wi-Fi IPv4
//! address in [`SERIAL_A`], [`ADDRESS_A`], [`SERIAL_B`] and
//! [`ADDRESS_B`]. On the devices the second row runs FIRST, on profiles
//! whose app data was cleared -- `pair` fails a side that trusts the
//! other before its trust step -- and the first after it on the pair it
//! made: each side's case removes an address an earlier run added, and
//! says so. No port is reversed on either phone, before or after.
//!
//! What this does NOT prove, by name: the Scan screen (the add is the
//! binding's function in its place); the one-sided step in the other
//! direction (B trusting first); a LAN that isolates its clients; and,
//! on the stand-ins, a radio, a process death (a dropped host runs its
//! destructors) or two machines (one host's private address both ways).

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use interweave_android_e2e_cases::{
    AddedAddressArgs, AwaitPeerArgs, PairArgs, PairExpect, cases, keys,
};
use interweave_android_e2e_tests::adb::AdbDevice;
use interweave_android_e2e_tests::wifi::{listening_at, reversed_on, until_listening};
use interweave_android_e2e_tests::{Device, HostStandIn, PATIENCE, free_port, log_capture};
use interweave_test_support::e2e::example;
use interweave_test_support::net::require_private_interface_v4;
use interweave_transport_api::TransportIdentity;

/// One Android side: the device, and where it listens on the LAN.
struct Side<D> {
    device: D,
    at: SocketAddrV4,
}

impl<D: Device> Side<D> {
    fn new(device: D, ip: Ipv4Addr) -> Self {
        // Any port: the side's, which nothing on this host holds, on a
        // device; this host's free-port answer on a stand-in.
        Self {
            device,
            at: SocketAddrV4::new(ip, free_port(Ipv4Addr::UNSPECIFIED)),
        }
    }

    fn address(&self) -> String {
        format!(
            "/ip4/{}/tcp/{}/p2p/{}",
            self.at.ip(),
            self.at.port(),
            self.device.peer().as_str()
        )
    }
}

/// The shipped Android example for a side listening at `port`, trusting
/// `other`, or nobody when `trusting` is false.
fn profile(other: &TransportIdentity, port: u16, trusting: bool) -> String {
    let config = listening_at(&example("human-android.yaml", other, "", None), port);
    if trusting {
        return config;
    }
    let trusted = format!("allowed_peers: [\"{}\"]", other.as_str());
    assert_eq!(
        config.matches(&trusted).count(),
        1,
        "the example trusts once"
    );
    config.replace(&trusted, "allowed_peers: []")
}

/// `config` with the peer cache off, and Kademlia, which it seeds, so the
/// overlay is the only producer of an address after a restart: the cache
/// keeps a reached peer across one.
fn overlay_only(config: &str) -> String {
    let mut out = config.to_owned();
    for provider in ["peer-cache", "kademlia"] {
        let on = format!("{{ type: {provider}, enabled: true,");
        assert_eq!(out.matches(&on).count(), 1, "{provider} is on once");
        out = out.replace(&on, &format!("{{ type: {provider}, enabled: false,"));
    }
    out
}

fn passed(out: &serde_json::Map<String, serde_json::Value>, what: &str, log: &str) {
    assert_eq!(
        out.get(keys::RESULT).and_then(serde_json::Value::as_str),
        Some(keys::PASS),
        "{what}: {out:?}\n{log}"
    );
}

/// The first row: A adds B's address, connects, restarts, and connects
/// again from the overlay alone.
async fn added_address_survives_a_restart<D: Device>(a: &mut Side<D>, b: &mut Side<D>) {
    let (a_peer, b_peer) = (a.device.peer(), b.device.peer());
    b.device
        .start(&overlay_only(&profile(&a_peer, b.at.port(), true)));
    a.device
        .start(&overlay_only(&profile(&b_peer, a.at.port(), true)));
    let log = |a: &Side<D>, b: &Side<D>| {
        format!(
            "--- A ---\n{}\n--- B ---\n{}",
            a.device.log(),
            b.device.log()
        )
    };

    for (step, add) in [("before the restart", true), ("after the restart", false)] {
        let far = b.device.run_case(
            cases::AWAIT_PEER,
            &AwaitPeerArgs {
                peer: a_peer.clone(),
                deadline: PATIENCE * 2,
            }
            .to_json(),
        );
        until_listening(b.at, &far, || log(a, b)).await;
        let near = a
            .device
            .run_case(
                cases::ADDED_ADDRESS,
                &AddedAddressArgs {
                    peer: b_peer.clone(),
                    address: b.address(),
                    add,
                    before: Duration::from_secs(3),
                    deadline: PATIENCE,
                }
                .to_json(),
            )
            .result();
        passed(&near, &format!("A {step}"), &log(a, b));
        // A has read its row; ending it is what ends B's case, which
        // waits for A to leave so its runtime outlives that read.
        a.device.kill();
        let far = far.result();
        passed(&far, &format!("B {step}"), &log(a, b));
        println!(
            "{step}: A connected {} ms after holding B's address (removed first: {}); \
             B saw it after {} ms",
            near[keys::CONNECTED_AFTER],
            near.get(keys::REMOVED_FIRST)
                .map_or("-", |v| v.as_str().unwrap_or("?")),
            far[keys::CONNECTED_AFTER]
        );
        if add {
            a.device.restart();
        }
    }
    a.device.stop();
    b.device.stop();
}

/// One step of the second row, both sides at once.
fn pair_step<D: Device>(
    a: &Side<D>,
    b: &Side<D>,
    a_args: &PairArgs,
    b_args: &PairArgs,
) -> [serde_json::Map<String, serde_json::Value>; 2] {
    let on_a = a.device.run_case(cases::PAIR, &a_args.to_json());
    let on_b = b.device.run_case(cases::PAIR, &b_args.to_json());
    [on_a.result(), on_b.result()]
}

/// The second row: two fresh sides connect after both trust, and not
/// before; each apart step lasts `window` from each side's start.
fn fresh_pair_connects_after_trust<D: Device>(a: &mut Side<D>, b: &mut Side<D>, window: Duration) {
    let (a_peer, b_peer) = (a.device.peer(), b.device.peer());
    a.device.start(&profile(&b_peer, a.at.port(), false));
    b.device.start(&profile(&a_peer, b.at.port(), false));
    let log = |a: &Side<D>, b: &Side<D>| {
        format!(
            "--- A ---\n{}\n--- B ---\n{}",
            a.device.log(),
            b.device.log()
        )
    };
    let step = |peer: &TransportIdentity, address: Option<String>, trust, expect| PairArgs {
        peer: peer.clone(),
        address,
        trust,
        expect,
        serial: 1,
    };

    let [on_a, on_b] = pair_step(
        a,
        b,
        &step(&b_peer, Some(b.address()), false, PairExpect::Apart(window)),
        &step(&a_peer, Some(a.address()), false, PairExpect::Apart(window)),
    );
    passed(&on_a, "A, neither trusting", &log(a, b));
    passed(&on_b, "B, neither trusting", &log(a, b));
    println!(
        "neither trusting: A tried {} sends, B {}; A's row {}, B's {}",
        on_a[keys::TRIES],
        on_b[keys::TRIES],
        on_a[keys::LAST_OUTCOME],
        on_b[keys::LAST_OUTCOME]
    );

    let [on_a, on_b] = pair_step(
        a,
        b,
        &step(&b_peer, None, true, PairExpect::Apart(window)),
        &step(&a_peer, None, false, PairExpect::Apart(window)),
    );
    passed(&on_a, "A, trusting alone", &log(a, b));
    passed(&on_b, "B, not trusting", &log(a, b));
    assert_eq!(on_a[keys::TRUSTED], "true", "A trusts B");
    assert_eq!(on_b[keys::TRUSTED], "false", "B does not trust A");
    println!(
        "A trusting alone: A tried {} sends, B {}; A's row {}",
        on_a[keys::TRIES],
        on_b[keys::TRIES],
        on_a[keys::LAST_OUTCOME]
    );

    // Each side a fresh process, as on a device, where every case is one.
    a.device.kill();
    b.device.kill();
    a.device.restart();
    b.device.restart();
    let crosses = |first| PairExpect::Crosses {
        deadline: PATIENCE,
        first,
    };
    let [on_a, on_b] = pair_step(
        a,
        b,
        &step(&b_peer, None, false, crosses(true)),
        &step(&a_peer, None, true, crosses(false)),
    );
    passed(&on_a, "A, both trusting", &log(a, b));
    passed(&on_b, "B, both trusting", &log(a, b));
    assert_eq!(
        (&on_a[keys::TRUSTED], &on_b[keys::TRUSTED]),
        (&"true".into(), &"true".into()),
        "both trust"
    );
    a.device.stop();
    b.device.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn on_stand_ins_an_added_address_is_dialled_and_survives_a_restart() {
    log_capture::install();
    let ip = require_private_interface_v4();
    let mut a = Side::new(HostStandIn::new(), ip);
    let mut b = Side::new(HostStandIn::new(), ip);
    added_address_survives_a_restart(&mut a, &mut b).await;
}

#[test]
fn on_stand_ins_two_fresh_profiles_connect_after_both_trust_and_not_before() {
    log_capture::install();
    let ip = require_private_interface_v4();
    let mut a = Side::new(HostStandIn::new(), ip);
    let mut b = Side::new(HostStandIn::new(), ip);
    fresh_pair_connects_after_trust(&mut a, &mut b, Duration::from_secs(5));
}

/// Phone A's `adb -s` serial.
const SERIAL_A: &str = "INTERWEAVE_ANDROID_SERIAL_A";
/// Phone A's Wi-Fi IPv4 address.
const ADDRESS_A: &str = "INTERWEAVE_ANDROID_WIFI_ADDRESS_A";
/// Phone B's `adb -s` serial.
const SERIAL_B: &str = "INTERWEAVE_ANDROID_SERIAL_B";
/// Phone B's Wi-Fi IPv4 address.
const ADDRESS_B: &str = "INTERWEAVE_ANDROID_WIFI_ADDRESS_B";

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"))
}

/// The two phones, each checked to reverse no port.
fn phones() -> (Side<AdbDevice>, Side<AdbDevice>) {
    let side = |serial: &str, address: &str| {
        let serial = env(serial);
        assert_eq!(reversed_on(&serial), "", "a port reversed on {serial}");
        let ip = env(address)
            .parse()
            .unwrap_or_else(|e| panic!("{address} is not an IPv4 address: {e}"));
        Side::new(AdbDevice::connect_to(&serial), ip)
    };
    (side(SERIAL_A, ADDRESS_A), side(SERIAL_B, ADDRESS_B))
}

fn none_reversed() {
    for serial in [env(SERIAL_A), env(SERIAL_B)] {
        assert_eq!(reversed_on(&serial), "", "a port reversed on {serial}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs two devices on one LAN, adb, and the app's androidTest build on both"]
async fn on_two_phones_an_added_address_is_dialled_and_survives_a_restart() {
    let (mut a, mut b) = phones();
    added_address_survives_a_restart(&mut a, &mut b).await;
    none_reversed();
}

#[test]
#[ignore = "needs two devices on one LAN with cleared app data, adb, and the app's androidTest build on both"]
fn on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before() {
    let (mut a, mut b) = phones();
    // Each case starts its own process and runtime, a few seconds apart
    // on the two phones: the window outlasts both starts.
    fresh_pair_connects_after_trust(&mut a, &mut b, Duration::from_secs(15));
    none_reversed();
}

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
//! - **The same with the peer cache on**, and Kademlia still off, which
//!   the cache seeds: four variants that tell the two sources apart by
//!   taking each away. Both kept; the overlay's entry removed after the
//!   first connection (the cache alone); the cache's file removed between
//!   the starts (the overlay alone); and both taken away, under which A
//!   stays unconnected for a window -- the control that nothing else
//!   dials B. Between the starts A's cache file is read, with no runtime
//!   up, and the run says whether it named B.
//! - **The same with A rebooted** in place of the force-stop (phones
//!   only): A's process dies with the device, the platform starts none of
//!   the app's processes after the boot (the app declares no boot
//!   receiver), and the dial recurs when the next case starts the app.
//! - **Two fresh profiles exchange messages after both trust, and not
//!   before** (the gate's "connect", read as a connection that carries a
//!   message).
//!   Each trusts nobody. Step 1: each adds the other's address, in place
//!   of the scan, and for a window no message either sends is accepted
//!   and none arrives. Step 2: A trusts B and B does not, and still
//!   nothing crosses either way. A's row may end reading `connected`
//!   as its last outcome (the stand-ins and one phone run, 2026-10-24),
//!   since B classes A after Noise authenticates it (`SECURITY.md`,
//!   Connectivity infrastructure admission): the step asserts that no
//!   message is accepted, not that no connection was made. Step 3: B trusts A,
//!   and a message crosses each way: the positive control. The step had
//!   the addresses step 1 added. This row's profile keeps the shipped
//!   peer cache and Kademlia, so another source is not excluded.
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
//! binding's function in its place); a dial at boot, which the product
//! does not make; the cache rows with Kademlia on, the shipped set; the one-sided step in the other
//! direction (B trusting first); a LAN that isolates its clients; and,
//! on the stand-ins, a radio, a process death (a dropped host runs its
//! destructors) or two machines (one host's private address both ways).

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use interweave_android_e2e_cases::{
    AddedAddressArgs, AwaitPeerArgs, PairArgs, PairExpect, PeerCacheArgs, cases, keys,
};
use interweave_android_e2e_tests::adb::{AdbDevice, reboot_and_await};
use interweave_android_e2e_tests::wifi::{listening_at, reversed_on, until_listening};
use interweave_android_e2e_tests::{
    Device, HostStandIn, PATIENCE, PROFILE, free_port, log_capture,
};
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

/// `config` with each of `providers` switched off.
fn without(config: &str, providers: &[&str]) -> String {
    let mut out = config.to_owned();
    for provider in providers {
        let on = format!("{{ type: {provider}, enabled: true,");
        assert_eq!(out.matches(&on).count(), 1, "{provider} is on once");
        out = out.replace(&on, &format!("{{ type: {provider}, enabled: false,"));
    }
    out
}

/// `config` with the peer cache off, and Kademlia, which it seeds, so the
/// overlay is the only producer of an address after a restart: the cache
/// keeps a reached peer across one.
fn overlay_only(config: &str) -> String {
    without(config, &["peer-cache", "kademlia"])
}

/// Which sources still hold B's address when A starts again. With the
/// cache on, Kademlia is off in every variant: it is seeded from the
/// cache and would be a third producer the row cannot tell apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sources {
    /// The cache off: the overlay alone (row 1).
    OverlayOnly,
    /// The cache on, and both kept.
    Both,
    /// The cache on, and the overlay's entry removed after the first hold.
    CacheAlone,
    /// The cache on, and its file removed between the two starts.
    OverlayAlone,
    /// The cache on, and both taken away: the control, under which A
    /// stays unconnected.
    Neither,
}

impl Sources {
    fn cache_on(self) -> bool {
        self != Self::OverlayOnly
    }

    fn overlay_kept(self) -> bool {
        !matches!(self, Self::CacheAlone | Self::Neither)
    }

    fn cache_kept(self) -> bool {
        !matches!(self, Self::OverlayAlone | Self::Neither)
    }

    fn profile(self, config: &str) -> String {
        if self.cache_on() {
            without(config, &["kademlia"])
        } else {
            overlay_only(config)
        }
    }
}

fn passed(out: &serde_json::Map<String, serde_json::Value>, what: &str, log: &str) {
    assert_eq!(
        out.get(keys::RESULT).and_then(serde_json::Value::as_str),
        Some(keys::PASS),
        "{what}: {out:?}\n{log}"
    );
}

/// The first row and the peer cache's: A adds B's address, connects, is
/// ended by `between` (which returns once A can run a case again), starts
/// again, and connects
/// again from what `sources` leaves -- or, with nothing left, stays
/// unconnected. On a cache row the cache's file is read between the two
/// starts, and removed when `sources` takes the cache away.
async fn added_address_across<D: Device>(
    a: &mut Side<D>,
    b: &mut Side<D>,
    sources: Sources,
    between: impl FnOnce(&mut Side<D>),
) {
    let mut between = Some(between);
    let (a_peer, b_peer) = (a.device.peer(), b.device.peer());
    let log = |a: &Side<D>, b: &Side<D>| {
        format!(
            "--- A ---\n{}\n--- B ---\n{}",
            a.device.log(),
            b.device.log()
        )
    };
    let cache = |a: &Side<D>, forget| {
        let out = a
            .device
            .run_case(
                cases::PEER_CACHE,
                &PeerCacheArgs {
                    profile: PROFILE.to_owned(),
                    peer: b_peer.clone(),
                    forget,
                }
                .to_json(),
            )
            .result();
        passed(&out, "A's peer cache", "");
        out
    };
    b.device
        .start(&overlay_only(&profile(&a_peer, b.at.port(), true)));
    if sources.cache_on() {
        // An earlier run's cache would hold B at another port: the
        // window before the add is then this run's alone.
        let out = cache(a, true);
        println!(
            "{sources:?}: A's cache before the first start: file {}, forgotten {}",
            out[keys::CACHE_FILE],
            out[keys::FORGOTTEN]
        );
    }
    a.device
        .start(&sources.profile(&profile(&b_peer, a.at.port(), true)));

    for (step, add) in [("before the restart", true), ("after the restart", false)] {
        let apart = (!add && sources == Sources::Neither).then_some(Duration::from_secs(15));
        // The far end is up and listening in every step, the control's
        // included: on a device B's runtime lives only while a B case
        // runs, and a control measured against nothing listening would
        // pass whatever A still held. B's window is meant to outlast A's
        // (A's starts only once B listens); the assertion below fails the
        // control if it did not.
        let far = b.device.run_case(
            cases::AWAIT_PEER,
            &AwaitPeerArgs {
                peer: a_peer.clone(),
                deadline: PATIENCE * 2,
                apart: apart.map(|apart| apart + Duration::from_secs(15)),
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
                    hold: Duration::from_secs(5),
                    remove_after: add && !sources.overlay_kept(),
                    listed: add || sources.overlay_kept(),
                    apart,
                }
                .to_json(),
            )
            .result();
        passed(&near, &format!("A {step}"), &log(a, b));
        if apart.is_some() {
            assert!(
                !far.is_finished(),
                "B's case ended before A's window did: part of the control ran \
                 with nothing listening\n{}",
                log(a, b)
            );
            let far = far.result();
            passed(&far, &format!("B {step}"), &log(a, b));
            println!(
                "{step}: A {} stayed unconnected to {} for {apart:?} with neither source left, \
                 B up and listening throughout",
                a_peer.as_str(),
                b.address()
            );
            break;
        }
        // A has read its row; ending it is what ends B's case, which
        // waits for A to leave so its runtime outlives that read.
        match between.take() {
            Some(between) if add => between(a),
            _ => a.device.kill(),
        }
        let far = far.result();
        passed(&far, &format!("B {step}"), &log(a, b));
        println!(
            "{step}: A {} connected to {} {} ms after holding it (removed first: {}, \
             removed after: {}); B saw it after {} ms",
            a_peer.as_str(),
            b.address(),
            near[keys::CONNECTED_AFTER],
            near.get(keys::REMOVED_FIRST)
                .map_or("-", |v| v.as_str().unwrap_or("?")),
            near.get(keys::REMOVED_AFTER)
                .map_or("-", |v| v.as_str().unwrap_or("?")),
            far[keys::CONNECTED_AFTER]
        );
        if add {
            if sources.cache_on() {
                let out = cache(a, !sources.cache_kept());
                println!(
                    "{sources:?}: A's cache between the starts: file {}, names B {}, forgotten {}",
                    out[keys::CACHE_FILE],
                    out[keys::CACHED],
                    out[keys::FORGOTTEN]
                );
                if sources == Sources::CacheAlone {
                    assert_eq!(
                        out[keys::CACHED],
                        "true",
                        "the cache's file does not name B after A ended: the cache alone \
                         cannot dial it ({out:?})"
                    );
                }
            }
            a.device.restart();
        }
    }
    a.device.stop();
    b.device.stop();
}

/// Row 1's ending and return: the platform's force-stop, then the next
/// case's process.
async fn added_address_survives_a_restart<D: Device>(
    a: &mut Side<D>,
    b: &mut Side<D>,
    sources: Sources,
) {
    added_address_across(a, b, sources, |a| a.device.kill()).await;
}

/// One step of the second row, both sides at once.
fn pair_step<D: Device>(
    a: &mut Side<D>,
    b: &mut Side<D>,
    a_args: &PairArgs,
    b_args: &PairArgs,
) -> [serde_json::Map<String, serde_json::Value>; 2] {
    let on_a = a.device.run_case(cases::PAIR, &a_args.to_json());
    let on_b = b.device.run_case(cases::PAIR, &b_args.to_json());
    // In a crossing, A sends the close and ends once B has left, and B
    // ends on reading it: so B's result first, then B ended -- which a
    // device's process does with its case, and a stand-in's runtime does
    // not -- then A's.
    if matches!(a_args.expect, PairExpect::Crosses { first: true, .. }) {
        let on_b = on_b.result();
        b.device.kill();
        return [on_a.result(), on_b];
    }
    [on_a.result(), on_b.result()]
}

/// The second row: two fresh sides exchange messages after both trust,
/// and not before; each apart step lasts `window` from each side's start.
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

    println!(
        "fresh pair: A {} at {}, B {} at {}",
        a_peer.as_str(),
        a.address(),
        b_peer.as_str(),
        b.address()
    );
    let (a_at, b_at) = (a.address(), b.address());
    let [on_a, on_b] = pair_step(
        &mut *a,
        &mut *b,
        &step(&b_peer, Some(b_at), false, PairExpect::Apart(window)),
        &step(&a_peer, Some(a_at), false, PairExpect::Apart(window)),
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
        &mut *a,
        &mut *b,
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
        &mut *a,
        &mut *b,
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
    println!(
        "both trusting: a message crossed each way; A's close {}; A's row {}, B's {}",
        on_a[keys::CLOSE],
        on_a[keys::LAST_OUTCOME],
        on_b[keys::LAST_OUTCOME]
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
    added_address_survives_a_restart(&mut a, &mut b, Sources::OverlayOnly).await;
}

async fn on_stand_ins(sources: Sources) {
    log_capture::install();
    let ip = require_private_interface_v4();
    let mut a = Side::new(HostStandIn::new(), ip);
    let mut b = Side::new(HostStandIn::new(), ip);
    added_address_survives_a_restart(&mut a, &mut b, sources).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn on_stand_ins_with_the_cache_on_both_sources_kept_reconnect() {
    on_stand_ins(Sources::Both).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn on_stand_ins_with_the_cache_on_the_cache_alone_reconnects() {
    on_stand_ins(Sources::CacheAlone).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn on_stand_ins_with_the_cache_on_the_overlay_alone_reconnects() {
    on_stand_ins(Sources::OverlayAlone).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn on_stand_ins_with_the_cache_on_and_neither_source_kept_nothing_dials() {
    on_stand_ins(Sources::Neither).await;
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
    added_address_survives_a_restart(&mut a, &mut b, Sources::OverlayOnly).await;
    none_reversed();
}

async fn on_two_phones(sources: Sources) {
    let (mut a, mut b) = phones();
    added_address_survives_a_restart(&mut a, &mut b, sources).await;
    none_reversed();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs two devices on one LAN, adb, and the app's androidTest build on both"]
async fn on_two_phones_with_the_cache_on_both_sources_kept_reconnect() {
    on_two_phones(Sources::Both).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs two devices on one LAN, adb, and the app's androidTest build on both"]
async fn on_two_phones_with_the_cache_on_the_cache_alone_reconnects() {
    on_two_phones(Sources::CacheAlone).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs two devices on one LAN, adb, and the app's androidTest build on both"]
async fn on_two_phones_with_the_cache_on_the_overlay_alone_reconnects() {
    on_two_phones(Sources::OverlayAlone).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs two devices on one LAN, adb, and the app's androidTest build on both"]
async fn on_two_phones_with_the_cache_on_and_neither_source_kept_nothing_dials() {
    on_two_phones(Sources::Neither).await;
}

/// How long the reboot row waits for A to come back, in seconds: someone
/// may have to `adb connect` its new wireless-debugging port.
const REBOOT_PATIENCE: &str = "INTERWEAVE_ANDROID_REBOOT_PATIENCE_S";

/// Row 1 with A rebooted in place of the force-stop: the address added
/// before the reboot is still listed and dialled when the app next runs.
/// The app declares no boot receiver, so nothing of it runs until a case
/// starts it, and the row asserts that the platform started none of its
/// processes; what it shows is the dial at the app's next start, not at
/// boot.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs two devices on one LAN, adb, the app's androidTest build on both, and A rebooted"]
async fn on_two_phones_an_added_address_survives_a_reboot() {
    let (mut a, mut b) = phones();
    let patience = Duration::from_secs(
        std::env::var(REBOOT_PATIENCE)
            .map_or(Ok(1200), |s| s.parse())
            .unwrap_or_else(|e| panic!("{REBOOT_PATIENCE}: {e}")),
    );
    let serial_b = env(SERIAL_B);
    added_address_across(&mut a, &mut b, Sources::OverlayOnly, |a| {
        let peer = a.device.peer();
        let rebooted = reboot_and_await(&env(SERIAL_A), patience);
        println!("A rebooted: {rebooted:?}");
        assert_ne!(rebooted.boot_id.0, rebooted.boot_id.1, "a new boot");
        assert_eq!(
            rebooted.app_pids, "",
            "the platform started the app after the boot, which declares no boot receiver"
        );
        assert_eq!(reversed_on(&rebooted.serial), "", "a port reversed on A");
        a.device = AdbDevice::connect_to(&rebooted.serial);
        assert_eq!(a.device.peer(), peer, "the same profile after the reboot");
    })
    .await;
    assert_eq!(reversed_on(&serial_b), "", "a port reversed on B");
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

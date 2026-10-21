// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android side's circuit route through a relay that is down is
//! HELD (ADR-0011 A 2026-10-21): the hop that failed is not tried again
//! until the hold ends, whoever asks -- above all the composition's
//! reconnect round, which ticks every second and, before the hold, was
//! admitted straight back into the same failed hop (rust-ui-dev-01's
//! Pixel measurement of 2026-10-21, one `relay_circuit` dial a second).
//!
//! R, the Android side, has one route to D, a desktop daemon: the circuit
//! through the test relay, as in `paths.rs`. The control first: with the
//! relay up, R's route comes up and nothing is held -- so the held run
//! below is the relay's being down and not a profile on which every dial
//! fails. Then the relay is dropped, R starts again, and R's own
//! `route_hold` case (`interweave-android-e2e-cases`) waits for the hold
//! on the admin port's peer row and reads the row across six reconnect
//! ticks, failing if the hold moves or D connects. Last, R's log: the
//! reconnect round's refusal names the class and the same end, the hop
//! that failed was logged before it -- the control that the log was read
//! at all -- and no dial to D is logged after it.
//!
//! Two runs: R on the host stand-in, and R on a device (`#[ignore]`d, run
//! with `--ignored` and the device's environment, `src/adb.rs`).
//!
//! What this does NOT prove, by name: a hold's end -- the first lasts the
//! retry base, 30 s, and the watch stays inside it; the hold lifted by
//! the relay coming back (`relay_reached`), which
//! `tests/connectivity/tests/relayed_paths.rs` holds on the host; a
//! relay down from the radio network rather than a refused loopback port
//! behind adb's reversed tunnel; and every limit `paths.rs` names.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_android_e2e_cases::{RouteHoldArgs, cases, keys};
use interweave_android_e2e_tests::adb::{AdbDevice, CUT_LINES};
use interweave_android_e2e_tests::{
    Desktop, Device, HostStandIn, PATIENCE, Relay, free_port, log_capture, relayed_example_of,
};
use interweave_transport_api::TransportIdentity;

/// Where the Android side listens, as in `paths.rs`.
const WILDCARD: &str = "/ip4/0.0.0.0/tcp/0";

/// How long the hold must stand as first read: six of the reconnect
/// round's one-second ticks, well inside the 30 s first hold.
const WATCH: Duration = Duration::from_secs(6);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_android_side_holds_a_circuit_route_through_a_relay_that_is_down() {
    log_capture::install();
    held_route(HostStandIn::new()).await;
}

/// [`Device::fresh_log`] on the stand-in: what `log` returns after it is
/// only what was logged after it. The control: the earlier line is there
/// before the call.
#[test]
fn the_stand_ins_fresh_log_begins_after_its_call() {
    log_capture::install();
    let mut r = HostStandIn::new();
    tracing::warn!(target: "interweave::connectivity", "fresh log: before");
    assert!(r.log().contains("fresh log: before"), "{}", r.log());
    r.fresh_log();
    tracing::warn!(target: "interweave::connectivity", "fresh log: after");
    let log = r.log();
    assert!(log.contains("fresh log: after"), "{log}");
    assert!(!log.contains("fresh log: before"), "{log}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
async fn on_a_device_the_phone_holds_a_circuit_route_through_a_relay_that_is_down() {
    held_route(AdbDevice::connect()).await;
}

fn args(d_peer: &TransportIdentity) -> serde_json::Value {
    RouteHoldArgs {
        desktop: d_peer.clone(),
        watch: WATCH,
        deadline: PATIENCE,
    }
    .to_json()
}

/// The TCP port of `address` (`/ip4/127.0.0.1/tcp/<port>/...`).
fn tcp_port(address: &str) -> u16 {
    address
        .split('/')
        .skip_while(|p| *p != "tcp")
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| panic!("no TCP port in {address}"))
}

async fn held_route<R: Device>(mut r: R) {
    let mut d = Desktop::new();
    let (d_peer, r_peer) = (d.peer.clone(), r.peer());
    let relay = Relay::start(&[&d_peer, &r_peer]).await;
    let d_listen = format!(
        "/ip4/127.0.0.1/tcp/{}",
        free_port(std::net::Ipv4Addr::LOCALHOST)
    );
    d.start(&relayed_example_of(
        "human-desktop.yaml",
        &[&r_peer],
        &d_listen,
        &relay,
        None,
    ))
    .await;
    r.reach(tcp_port(&relay.address));
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !relay
        .seen()
        .await
        .reservations
        .contains(&d_peer.as_str().parse().expect("a libp2p identity"))
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "D never reserved on the relay:\n{}",
            d.log()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    r.start(&relayed_example_of(
        "human-android.yaml",
        &[&d_peer],
        WILDCARD,
        &relay,
        Some(&relay.circuit_to(&d_peer)),
    ));

    // The control: the relay up, R's route to D comes up and the case
    // finds it connected, never held.
    let out = r.run_case(cases::ROUTE_HOLD, &args(&d_peer)).result();
    let detail = out.get(keys::DETAIL).and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        out.get(keys::RESULT).and_then(|v| v.as_str()) == Some(keys::FAIL)
            && detail.contains("connected to the desktop"),
        "the control, the relay up: {out:?}\n{}",
        r.log()
    );

    // The relay goes, and R starts again with nothing but the circuit
    // through it: the first dial's hop fails, and the route is held.
    drop(relay);
    r.kill();
    // The held run's log alone, begun while no runtime is up: the
    // control's lines must not stand in for it (a first dial can race the
    // relay's connection there too), and the held run's first dial fails
    // as soon as its runtime starts.
    r.fresh_log();
    r.restart();
    let out = r
        .run_case(cases::ROUTE_HOLD, &args(&d_peer))
        .passed(|| r.log());
    let held: u64 = out[keys::HELD_UNTIL]
        .as_str()
        .and_then(|n| n.parse().ok())
        .expect("a time");
    let reads: u32 = out[keys::READS]
        .as_str()
        .and_then(|n| n.parse().ok())
        .expect("a count");
    // At least one read per reconnect tick across the watch: what the
    // hold must stand through. Not a throughput bound -- a read's cost is
    // the platform's.
    let ticks = u32::try_from(WATCH.as_secs()).expect("a short watch");
    assert!(reads >= ticks, "the watch read the row {reads} times");

    // R's log says the same: the refusal with the class and the end, the
    // failed hop before it, and no dial to D after it. The end is the
    // case's within a second: the holds one failed dial sets on each
    // circuit address end milliseconds apart, and the line and the case
    // read the latest at different moments (the case's SAME_FAILURE_MS).
    let log = r.log();
    let lines: Vec<&str> = log.lines().collect();
    let to_d = format!("peer={}", d_peer.as_str());
    assert!(
        !log.contains(CUT_LINES),
        "the held run's log was cut at its front, so its first lines cannot be read:\n{log}"
    );
    let held_run = &lines[..];
    let refusal = held_run
        .iter()
        .position(|l| {
            l.contains("reconnect refused")
                && l.contains(&to_d)
                && l.contains("class=RouteHeld")
                && l.split_once("held_until_ms=")
                    .and_then(|(_, rest)| rest.split_whitespace().next())
                    .and_then(|n| n.parse::<u64>().ok())
                    .is_some_and(|logged| logged.abs_diff(held) <= 1_000)
        })
        .unwrap_or_else(|| panic!("no held refusal ending near {held} for D:\n{log}"));
    let dial_to_d = |l: &str| l.contains("dial failed") && l.contains(&to_d);
    assert!(
        held_run[..refusal].iter().any(|l| dial_to_d(l)),
        "the held run's failed hop to D is logged before the hold:\n{log}"
    );
    let after: Vec<&str> = held_run[refusal..]
        .iter()
        .copied()
        .filter(|l| dial_to_d(l))
        .collect();
    assert!(
        after.is_empty(),
        "a dial to D logged inside the hold: {after:?}\n{log}"
    );

    r.stop();
    d.stop().await;
}

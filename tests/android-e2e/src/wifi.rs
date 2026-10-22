// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A device run over the phone's Wi-Fi rather than adb's tunnel: the
//! phone listens on its wildcard at a port this host fixes, and a desktop
//! daemon dials the phone's LAN address, given to it by the person's add
//! (`admin.peers.add_address`, what `transportctl peers add` calls). adb
//! still carries the instrumentation and the status read.
//!
//! The desktop dials because the host this was written on is a VM behind
//! NAT: the phone cannot dial into it, and the VM reaches the LAN. One
//! established connection carries both directions.
//!
//! What a run here shows is checked, not assumed: [`reversed`] reads
//! `adb reverse --list`, which must be empty, and [`established_to`]
//! reads this host's own TCP table for a connection to the phone's
//! address.

use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::process::Command;
use std::time::Duration;

use crate::{CaseRun, PATIENCE};

/// The environment variable naming the phone's Wi-Fi IPv4 address.
pub const ADDRESS_ENV: &str = "INTERWEAVE_ANDROID_WIFI_ADDRESS";

/// The Android example's one listener, which a Wi-Fi run fixes to a port
/// the desktop can be given.
const EPHEMERAL: &str = "\"/ip4/0.0.0.0/tcp/0\"";

/// The phone's Wi-Fi address, from [`ADDRESS_ENV`].
///
/// # Panics
/// If it is unset or not an IPv4 address.
#[must_use]
pub fn phone_address() -> Ipv4Addr {
    std::env::var(ADDRESS_ENV)
        .unwrap_or_else(|_| panic!("{ADDRESS_ENV} names no address for the phone"))
        .parse()
        .unwrap_or_else(|e| panic!("{ADDRESS_ENV} is not an IPv4 address: {e}"))
}

/// `config`, an Android profile, listening on the wildcard at `port`
/// instead of an ephemeral one.
///
/// # Panics
/// If the profile does not listen on exactly the one ephemeral wildcard.
#[must_use]
pub fn listening_at(config: &str, port: u16) -> String {
    assert_eq!(
        config.matches(EPHEMERAL).count(),
        1,
        "the profile listens on {EPHEMERAL} once"
    );
    config.replace(EPHEMERAL, &format!("\"/ip4/0.0.0.0/tcp/{port}\""))
}

/// Wait until the phone's listener at `at` answers over the LAN -- its
/// runtime is up -- while `run`, the case that started it, has not ended.
///
/// # Panics
/// If it does not answer within [`PATIENCE`], or the case ends first,
/// with `log`.
pub async fn until_listening(at: SocketAddrV4, run: &CaseRun, log: impl Fn() -> String) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while TcpStream::connect_timeout(&at.into(), Duration::from_secs(1)).is_err() {
        assert!(
            tokio::time::Instant::now() < deadline && !run.is_finished(),
            "the phone never listened at {at}: {}\n{}",
            run.so_far(),
            log()
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// `adb reverse --list`, trimmed: empty when no port is reversed.
///
/// # Panics
/// If adb fails.
#[must_use]
pub fn reversed() -> String {
    let out = Command::new("adb")
        .args(["reverse", "--list"])
        .output()
        .expect("adb");
    assert!(
        out.status.success(),
        "adb reverse --list: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// This host's established TCP connections to `peer`, as `ss` lists them.
///
/// # Panics
/// If `ss` cannot run or fails.
#[must_use]
pub fn established_to(peer: SocketAddrV4) -> Vec<String> {
    let out = Command::new("ss")
        .args(["-Htn", "state", "established", "dst", &peer.to_string()])
        .output()
        .expect("ss");
    assert!(
        out.status.success(),
        "ss: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The listener is fixed in place, and a profile that does not listen
    /// on the one ephemeral wildcard is refused rather than left as it is.
    #[test]
    fn the_android_listener_is_fixed_to_the_given_port() {
        let config = "transport:\n  listen: { addresses: [\"/ip4/0.0.0.0/tcp/0\"] }\n";
        assert_eq!(
            listening_at(config, 41_000),
            "transport:\n  listen: { addresses: [\"/ip4/0.0.0.0/tcp/41000\"] }\n"
        );
        let fixed = std::panic::catch_unwind(|| listening_at("listen: []", 41_000));
        assert!(fixed.is_err(), "a profile with no ephemeral listener");
    }

    /// `ss` reads this host's own table: nothing before the connect (the
    /// control), and the one connection this test holds after it.
    #[test]
    fn established_to_lists_a_connection_this_host_holds() {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
        let std::net::SocketAddr::V4(at) = listener.local_addr().expect("addr") else {
            panic!("an IPv4 listener");
        };
        assert!(established_to(at).is_empty(), "nothing connected yet");
        let client = TcpStream::connect(at).expect("connect");
        let (server, _) = listener.accept().expect("accept");
        assert_eq!(established_to(at).len(), 1, "the one connection");
        drop((client, server));
    }
}

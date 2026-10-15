// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton

//! The host's private-range address, for tests that need two runtimes
//! to learn each other's advertised addresses, and the ports their
//! profiles are written to listen on.
//!
//! # Why not loopback
//!
//! ADR-0052's floor refuses a loopback address whoever supplies it, and
//! since A 2026-09-20 that covers what a peer advertises through
//! Identify, what Kademlia routes and what the relay and AutoNAT drivers
//! learn. Two runtimes on `127.0.0.1` therefore never learn or route
//! each other. Rule 3 admits a private (RFC 1918) address beside a
//! private listener of the same family, which is exactly what two
//! runtimes on this host's own private address are. There is no
//! test-only knob to admit loopback, by design.
//!
//! # Why a host without one FAILS the test
//!
//! An earlier shape returned early with an `eprintln!`, and described
//! that as standing the test down "loudly". It was neither: an early
//! `return` from a `#[test]` is a PASS, and libtest captures a passing
//! test's stderr, so the message was not even shown. On a host whose
//! route to 10/8 is not via an RFC 1918 source address, the evidence
//! these tests are cited for would have disappeared green (#111
//! re-review P2-3). A test that cannot ask its question has not
//! answered it, so this panics and names the reason -- the policy this
//! crate's root states for every harness. The hosted CI runners have a
//! private address.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, TcpListener, UdpSocket};
use std::sync::{Mutex, PoisonError};

/// This host's private-range (RFC 1918) IPv4 address, or a panic that
/// says why the test cannot run here.
///
/// Read off an unconnected UDP socket "connected" to a private
/// destination, which asks the kernel which source address it would
/// use: no packet is sent.
///
/// # Panics
/// When the host has no non-loopback RFC 1918 IPv4 address -- the test
/// that asked cannot produce its evidence on this host, and must not
/// pass as though it had.
#[must_use]
pub fn require_private_interface_v4() -> Ipv4Addr {
    private_interface_v4().unwrap_or_else(|| {
        panic!(
            "this host has no private-range (RFC 1918) IPv4 interface. The test needs one: \
             ADR-0052 refuses a loopback address a peer supplies, so two runtimes on \
             127.0.0.1 never learn or route each other, and there is no knob to admit it. \
             Run it on a host with a private address (the hosted CI runners have one)."
        )
    })
}

fn private_interface_v4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("10.255.255.255:9").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if ip.is_private() && !ip.is_loopback() => Some(ip),
        _ => None,
    }
}

/// Every port [`free_port`] has answered in this process.
static HANDED_OUT: Mutex<BTreeSet<u16>> = Mutex::new(BTreeSet::new());

/// A port nothing listens on at the moment of asking, and never one this
/// process was answered before
/// (`free_port_never_answers_the_same_port_twice`).
///
/// NEVER TWICE, because two asked in a row are two profiles' listen
/// addresses, written before either daemon starts. The kernel may hand a
/// port just released straight back; libp2p-tcp listens with
/// `SO_REUSEPORT`, so both daemons then bind it, and each one's route to
/// the other is its own listen address, which libp2p-swarm strips from a
/// dial: `NoAddresses`, the route dropped from the book, and no
/// connection either way (desktop-e2e's `human_app`, PR run 38523588747
/// of #276; reproduced by giving both daemons one port). A repeat is held
/// bound while the next port is asked, so the kernel cannot answer it
/// again.
///
/// What this does not close: another process may bind the port between
/// this answer and the daemon's bind, which fails that daemon's start.
///
/// # Panics
/// When the host will not bind `ip`.
#[must_use]
pub fn free_port(ip: Ipv4Addr) -> u16 {
    let mut handed_out = HANDED_OUT.lock().unwrap_or_else(PoisonError::into_inner);
    let mut repeats = Vec::new();
    loop {
        let listener = TcpListener::bind((ip, 0)).expect("a port");
        let port = listener.local_addr().expect("an address").port();
        if handed_out.insert(port) {
            return port;
        }
        repeats.push(listener);
    }
}

#[cfg(test)]
mod tests {
    use super::free_port;
    use std::collections::BTreeSet;
    use std::net::Ipv4Addr;

    /// Far more asks than a test process makes, so the kernel's own
    /// answers repeat many times over (how many depends on the host's
    /// ephemeral range and its choice of port): each repeat must be
    /// refused, not returned.
    #[test]
    fn free_port_never_answers_the_same_port_twice() {
        let mut seen = BTreeSet::new();
        for _ in 0..2_000 {
            let port = free_port(Ipv4Addr::LOCALHOST);
            assert!(seen.insert(port), "port {port} answered twice");
        }
    }
}

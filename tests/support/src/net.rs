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

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::net::{IpAddr, Ipv4Addr, TcpListener, UdpSocket};
use std::path::PathBuf;
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

/// Every port [`free_port`] has answered in this process, each with the
/// claim that keeps another process from being answered it.
static HANDED_OUT: Mutex<BTreeMap<u16, File>> = Mutex::new(BTreeMap::new());

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
/// NOR TWICE ACROSS PROCESSES on this host's temporary directory: a test
/// runner that gives every test a process of its own (nextest) runs
/// tests of different binaries at once, and a set kept in one process
/// cannot see another's. Each answer holds an advisory lock on a file
/// named by the port ([`claim`]) for the rest of the process, so another
/// process skips a port held there, and the kernel releases the lock
/// when the process ends however it ends: nothing is left to go stale
/// (`a_port_another_process_claims_is_refused_until_it_ends`,
/// `free_port_holds_the_claim_on_what_it_answers`).
///
/// What this does not close: a process that is not a `free_port` caller
/// -- or one under another temporary directory, such as another account
/// on a shared host -- may bind the port between this answer and the
/// daemon's bind, which fails that daemon's start.
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
        if !handed_out.contains_key(&port)
            && let Some(held) = claim(port)
        {
            handed_out.insert(port, held);
            return port;
        }
        repeats.push(listener);
    }
}

/// Where the claims are: the temporary directory, which every process of
/// one test run shares.
fn claims_dir() -> PathBuf {
    std::env::temp_dir().join("interweave-free-port")
}

/// `port`'s claim, if no other holder has it: an exclusive advisory lock
/// on its file, held while the returned file is open. The lock belongs
/// to the open file, not the process, so a second open in this process
/// is refused as another process's would be.
///
/// # Panics
/// When the claims directory or a claim file cannot be made or locked.
fn claim(port: u16) -> Option<File> {
    let dir = claims_dir();
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("the port claims at {}: {e}", dir.display()));
    let path = dir.join(port.to_string());
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("the port claim {}: {e}", path.display()));
    match file.try_lock() {
        Ok(()) => Some(file),
        Err(TryLockError::WouldBlock) => None,
        Err(TryLockError::Error(e)) => panic!("locking {}: {e}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::{claim, free_port};
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

    /// A port held by ANOTHER PROCESS is refused, and claimable once that
    /// process ends: `flock(1)` holds the claim file as a second test
    /// process would, by the same `flock(2)` lock. The control is the
    /// claim after the holder is gone, which succeeds.
    #[test]
    fn a_port_another_process_claims_is_refused_until_it_ends() {
        // A port no claim of this process's covers: free_port's answers
        // all are, and a fresh one from the kernel need not be.
        let port = loop {
            let port = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
                .and_then(|l| l.local_addr())
                .expect("a port")
                .port();
            if let Some(probe) = claim(port) {
                drop(probe);
                break port;
            }
        };
        let file = super::claims_dir().join(port.to_string());
        let mut holder = std::process::Command::new("flock")
            .arg("--no-fork")
            .arg(&file)
            .args(["sleep", "30"])
            .spawn()
            .expect("flock(1): util-linux");
        // The holder has the lock once our claim is refused; bounded, so
        // a holder that never locks fails here rather than hanging.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while let Some(probe) = claim(port) {
            drop(probe);
            assert!(
                std::time::Instant::now() < deadline,
                "flock never held {}",
                file.display()
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        holder.kill().expect("the holder ends");
        holder.wait().expect("reaped");
        assert!(
            claim(port).is_some(),
            "port {port} still held after its holder ended"
        );
    }

    /// What `free_port` answers is held: a fresh claim on it is refused.
    #[test]
    fn free_port_holds_the_claim_on_what_it_answers() {
        let port = free_port(Ipv4Addr::LOCALHOST);
        assert!(claim(port).is_none(), "port {port} answered unclaimed");
    }
}

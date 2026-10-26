// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! ADR-0052 rule 10 (A 2026-10-26) at Identify: a listener bound on a
//! private-range address the platform classifies as on no local network
//! -- a phone's carrier-assigned cellular address -- is not advertised
//! as a listen address. The runtime decides what Identify advertises, as
//! it already decides what the crate learns.
//!
//! libp2p-identify 0.48.0 advertises its `listen_addresses` beside the
//! confirmed external ones (`all_addresses`), and learns the former from
//! nothing but `FromSwarm::NewListenAddr` and `FromSwarm::ExpiredListenAddr`
//! (the Swarm reports a closed listener's addresses as expired first).
//! So the one door is those two events: [`ListenScope`] withholds a
//! `NewListenAddr` on a withheld IP, and when the classification moves
//! ([`ListenScope::withhold`]) tells the crate an `ExpiredListenAddr`
//! for each bound address now withheld and a `NewListenAddr` for each
//! released -- the crate then updates every connected handler's set
//! itself. Everything else passes through untouched.
//!
//! What it does not reach: a peer that ran Identify with this node
//! before the classification arrived has the address already, and what
//! it learned is its own; the next exchange omits it. The address is
//! still bound, so a connection to or from it is unchanged -- only what
//! this node says of itself is.

use std::collections::{BTreeSet, HashMap};
use std::net::IpAddr;
use std::task::{Context, Poll};

use libp2p::core::Endpoint;
use libp2p::core::transport::{ListenerId, PortUse};
use libp2p::multiaddr::Protocol;
use libp2p::swarm::{
    ConnectionDenied, ConnectionId, ExpiredListenAddr, FromSwarm, NetworkBehaviour, NewListenAddr,
    THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
};
use libp2p::{Multiaddr, PeerId};

/// A behaviour told of this node's listen addresses less those on a
/// withheld IP.
pub struct ListenScope<B> {
    inner: B,
    /// Every address the Swarm reported bound and not yet expired, with
    /// its listener: what a classification move is replayed from.
    /// Bounded by the listeners this PROCESS binds.
    bound: HashMap<Multiaddr, ListenerId>,
    /// The IPs withheld, canonical (an IPv4-mapped form is its IPv4).
    withheld: BTreeSet<IpAddr>,
}

impl<B> ListenScope<B> {
    /// Wrap `inner`, withholding nothing.
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            bound: HashMap::new(),
            withheld: BTreeSet::new(),
        }
    }

    /// The wrapped behaviour.
    pub const fn inner(&self) -> &B {
        &self.inner
    }

    fn is_withheld(&self, address: &Multiaddr) -> bool {
        host_ip(address).is_some_and(|ip| self.withheld.contains(&ip))
    }
}

impl<B: NetworkBehaviour> ListenScope<B> {
    /// Withhold the listen addresses on `ips` from now on, releasing any
    /// no longer named: the crate is told each bound address that moved,
    /// as an expiry or a new listen address. Returns how many moved.
    /// Pinned by `a_withheld_listen_address_is_never_advertised`.
    pub fn withhold(&mut self, ips: &BTreeSet<IpAddr>) -> usize {
        let before: Vec<(Multiaddr, ListenerId, bool)> = self
            .bound
            .iter()
            .map(|(addr, id)| (addr.clone(), *id, self.is_withheld(addr)))
            .collect();
        self.withheld = ips.iter().map(IpAddr::to_canonical).collect();
        let mut moved = 0;
        for (addr, listener_id, was) in before {
            match (was, self.is_withheld(&addr)) {
                (false, true) => {
                    self.inner
                        .on_swarm_event(FromSwarm::ExpiredListenAddr(ExpiredListenAddr {
                            listener_id,
                            addr: &addr,
                        }));
                }
                (true, false) => {
                    self.inner
                        .on_swarm_event(FromSwarm::NewListenAddr(NewListenAddr {
                            listener_id,
                            addr: &addr,
                        }));
                }
                _ => continue,
            }
            moved += 1;
        }
        moved
    }
}

/// The host IP `address` starts with, an IPv4-mapped IPv6 read as its
/// IPv4.
fn host_ip(address: &Multiaddr) -> Option<IpAddr> {
    match address.iter().next()? {
        Protocol::Ip4(ip) => Some(IpAddr::V4(ip)),
        Protocol::Ip6(ip) => Some(IpAddr::V6(ip).to_canonical()),
        _ => None,
    }
}

impl<B: NetworkBehaviour> NetworkBehaviour for ListenScope<B> {
    type ConnectionHandler = B::ConnectionHandler;
    type ToSwarm = B::ToSwarm;

    fn handle_pending_inbound_connection(
        &mut self,
        id: ConnectionId,
        local: &Multiaddr,
        remote: &Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        self.inner
            .handle_pending_inbound_connection(id, local, remote)
    }

    fn handle_established_inbound_connection(
        &mut self,
        id: ConnectionId,
        peer: PeerId,
        local: &Multiaddr,
        remote: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.inner
            .handle_established_inbound_connection(id, peer, local, remote)
    }

    fn handle_pending_outbound_connection(
        &mut self,
        id: ConnectionId,
        peer: Option<PeerId>,
        addresses: &[Multiaddr],
        role: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        self.inner
            .handle_pending_outbound_connection(id, peer, addresses, role)
    }

    fn handle_established_outbound_connection(
        &mut self,
        id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role: Endpoint,
        port: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.inner
            .handle_established_outbound_connection(id, peer, addr, role, port)
    }

    fn on_swarm_event(&mut self, event: FromSwarm<'_>) {
        match &event {
            FromSwarm::NewListenAddr(new) => {
                self.bound.insert(new.addr.clone(), new.listener_id);
                if self.is_withheld(new.addr) {
                    return;
                }
            }
            FromSwarm::ExpiredListenAddr(expired) => {
                self.bound.remove(expired.addr);
                // The crate never had it, so it has nothing to expire.
                if self.is_withheld(expired.addr) {
                    return;
                }
            }
            _ => {}
        }
        self.inner.on_swarm_event(event);
    }

    fn on_connection_handler_event(
        &mut self,
        peer: PeerId,
        id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        self.inner.on_connection_handler_event(peer, id, event);
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        self.inner.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use libp2p::swarm::ListenAddresses;

    use super::*;

    /// A stand-in that keeps what it is told the way libp2p-identify
    /// does -- through `ListenAddresses`, the crate's own type.
    #[derive(Default)]
    struct Told(ListenAddresses);

    impl NetworkBehaviour for Told {
        type ConnectionHandler = libp2p::swarm::dummy::ConnectionHandler;
        type ToSwarm = std::convert::Infallible;

        fn handle_established_inbound_connection(
            &mut self,
            _: ConnectionId,
            _: PeerId,
            _: &Multiaddr,
            _: &Multiaddr,
        ) -> Result<THandler<Self>, ConnectionDenied> {
            Ok(libp2p::swarm::dummy::ConnectionHandler)
        }

        fn handle_established_outbound_connection(
            &mut self,
            _: ConnectionId,
            _: PeerId,
            _: &Multiaddr,
            _: Endpoint,
            _: PortUse,
        ) -> Result<THandler<Self>, ConnectionDenied> {
            Ok(libp2p::swarm::dummy::ConnectionHandler)
        }

        fn on_swarm_event(&mut self, event: FromSwarm<'_>) {
            self.0.on_swarm_event(&event);
        }

        fn on_connection_handler_event(
            &mut self,
            _: PeerId,
            _: ConnectionId,
            event: THandlerOutEvent<Self>,
        ) {
            match event {}
        }

        fn poll(
            &mut self,
            _: &mut Context<'_>,
        ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
            Poll::Pending
        }
    }

    fn told(scope: &ListenScope<Told>) -> BTreeSet<String> {
        scope.inner().0.iter().map(ToString::to_string).collect()
    }

    fn set(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|a| (*a).to_owned()).collect()
    }

    /// Bound before the classification (Android's start), withheld once
    /// it arrives, released when it is withdrawn; bound while withheld,
    /// never told; expired while withheld, nothing to expire. The Wi-Fi
    /// address is the control throughout.
    #[test]
    fn a_withheld_listen_address_is_never_advertised() {
        let mut scope = ListenScope::new(Told::default());
        let listener = ListenerId::next();
        let bind = |scope: &mut ListenScope<Told>, addr: &str| {
            let addr: Multiaddr = addr.parse().expect("an address");
            scope.on_swarm_event(FromSwarm::NewListenAddr(NewListenAddr {
                listener_id: listener,
                addr: &addr,
            }));
        };
        let expire = |scope: &mut ListenScope<Told>, addr: &str| {
            let addr: Multiaddr = addr.parse().expect("an address");
            scope.on_swarm_event(FromSwarm::ExpiredListenAddr(ExpiredListenAddr {
                listener_id: listener,
                addr: &addr,
            }));
        };
        let (wifi, cellular) = ("/ip4/192.168.1.5/tcp/4001", "/ip4/10.20.30.40/tcp/4001");
        bind(&mut scope, wifi);
        bind(&mut scope, cellular);
        assert_eq!(told(&scope), set(&[wifi, cellular]), "nothing classified");

        let carrier = BTreeSet::from(["10.20.30.40".parse().expect("an IP")]);
        assert_eq!(scope.withhold(&carrier), 1);
        assert_eq!(told(&scope), set(&[wifi]), "withheld once classified");
        assert_eq!(
            scope.withhold(&carrier),
            0,
            "the same classification moves nothing"
        );

        // A second listener on the carrier's IP, bound while withheld.
        let quic = "/ip4/10.20.30.40/udp/4001/quic-v1";
        bind(&mut scope, quic);
        assert_eq!(told(&scope), set(&[wifi]));
        // An IPv4-mapped form is the same IP.
        let mapped = "/ip6/::ffff:10.20.30.40/tcp/4002";
        bind(&mut scope, mapped);
        assert_eq!(told(&scope), set(&[wifi]));
        expire(&mut scope, mapped);

        assert_eq!(scope.withhold(&BTreeSet::new()), 2, "released");
        assert_eq!(told(&scope), set(&[wifi, cellular, quic]));

        assert_eq!(scope.withhold(&carrier), 2);
        expire(&mut scope, cellular);
        assert_eq!(
            scope.withhold(&BTreeSet::new()),
            1,
            "only what is still bound"
        );
        assert_eq!(told(&scope), set(&[wifi, quic]));
        expire(&mut scope, wifi);
        assert_eq!(told(&scope), set(&[quic]));
    }
}

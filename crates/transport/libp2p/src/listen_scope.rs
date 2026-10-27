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
//! The second case (A 2026-10-27): while a platform view is present, a
//! listener bound on a private-range address the view does not hold is
//! on no local network too ([`is_outside_the_view`]); and so, with or
//! without a view, is one in the shared address space, `100.64.0.0/10`
//! (A 2026-10-27 (iii), [`lies_on_no_local_network`]). The scope holds
//! the view itself and judges each `NewListenAddr` against it as the
//! Swarm reports it, so a listener the wildcard reports after the view
//! never reaches the crate -- deciding it later, from the runtime's next
//! [`ListenScope::withhold`], would let the crate tell its connections'
//! handlers the address in between.
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

use interweave_transport_runtime::reachability::{
    is_private_range, is_shared_address_space, literal_host,
};

/// ADR-0052 rule 10's second case (A 2026-10-27): whether a private-range
/// IP this host binds lies on no local network because the platform's
/// `view` does not hold it -- a network the platform withholds from the
/// app is one no peer reaches it on. Read for a DIRECT listener only,
/// never a circuit one, whose leading IP is the relay's. The one
/// predicate, applied by the runtime's detector
/// (`NetworkSet::derive_off_local`) and by this door.
pub(crate) fn is_outside_the_view(ip: IpAddr, view: &BTreeSet<IpAddr>) -> bool {
    let ip = ip.to_canonical();
    is_private_range(ip) && !view.iter().any(|held| held.to_canonical() == ip)
}

/// ADR-0052 rule 10 for a DIRECT listener's IP, apart from the view's
/// own classification: in the shared address space (RFC 6598) whatever
/// the platform says (A 2026-10-27 (iii)), since rule 2's floor refuses
/// that range in every address a peer supplies: no peer learns such an
/// address of this node through the peer door, one that reaches it does
/// so from an entry its own operator wrote (rule 9), which needs no
/// advertisement, so offering it serves nobody, whoever numbered it;
/// else, while `view` is present, a private IP
/// outside it ([`is_outside_the_view`]). The one predicate the runtime's
/// detector (`NetworkSet::derive_off_local`) and this door apply.
pub(crate) fn lies_on_no_local_network(ip: IpAddr, view: Option<&BTreeSet<IpAddr>>) -> bool {
    is_shared_address_space(ip) || view.is_some_and(|view| is_outside_the_view(ip, view))
}

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
    /// The platform's last view, `None` before the first: a direct
    /// listen address on a private IP outside it is withheld too.
    /// Bounded by the platform's own view.
    view: Option<BTreeSet<IpAddr>>,
}

impl<B> ListenScope<B> {
    /// Wrap `inner`, withholding nothing.
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            bound: HashMap::new(),
            withheld: BTreeSet::new(),
            view: None,
        }
    }

    /// The wrapped behaviour.
    pub const fn inner(&self) -> &B {
        &self.inner
    }

    fn is_withheld(&self, address: &Multiaddr) -> bool {
        host_ip(address).is_some_and(|ip| self.withheld.contains(&ip))
            || literal_host(&address.to_string())
                .is_some_and(|ip| lies_on_no_local_network(ip, self.view.as_ref()))
    }
}

impl<B: NetworkBehaviour> ListenScope<B> {
    /// Withhold the listen addresses on `ips` from now on, and every
    /// direct one [`lies_on_no_local_network`] names under `view` -- in
    /// the shared address space whatever `view` is, and while `view` is
    /// `Some` on a private IP it does not hold -- releasing any no longer
    /// withheld: the crate is told each bound address that moved, as an
    /// expiry or a new listen address. Returns how many moved. Pinned by
    /// `a_withheld_listen_address_is_never_advertised`,
    /// `a_private_listen_address_outside_the_view_is_never_advertised`
    /// and `a_shared_address_space_listen_address_is_never_advertised`.
    pub fn withhold(&mut self, ips: &BTreeSet<IpAddr>, view: Option<&BTreeSet<IpAddr>>) -> usize {
        let before: Vec<(Multiaddr, ListenerId, bool)> = self
            .bound
            .iter()
            .map(|(addr, id)| (addr.clone(), *id, self.is_withheld(addr)))
            .collect();
        self.withheld = ips.iter().map(IpAddr::to_canonical).collect();
        self.view = view.map(|view| view.iter().map(IpAddr::to_canonical).collect());
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
        assert_eq!(scope.withhold(&carrier, None), 1);
        assert_eq!(told(&scope), set(&[wifi]), "withheld once classified");
        assert_eq!(
            scope.withhold(&carrier, None),
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

        assert_eq!(scope.withhold(&BTreeSet::new(), None), 2, "released");
        assert_eq!(told(&scope), set(&[wifi, cellular, quic]));

        assert_eq!(scope.withhold(&carrier, None), 2);
        expire(&mut scope, cellular);
        assert_eq!(
            scope.withhold(&BTreeSet::new(), None),
            1,
            "only what is still bound"
        );
        assert_eq!(told(&scope), set(&[wifi, quic]));
        expire(&mut scope, wifi);
        assert_eq!(told(&scope), set(&[quic]));
    }

    /// The shared address space (A 2026-10-27 (iii)): a direct listen address in
    /// `100.64.0.0/10` is withheld at the door with no view and with a
    /// view naming it, an IPv4-mapped form included; a private one with
    /// no view is told -- THE CONTROL -- and so is a circuit through a
    /// relay at a shared IP.
    #[test]
    fn a_shared_address_space_listen_address_is_never_advertised() {
        let mut scope = ListenScope::new(Told::default());
        let listener = ListenerId::next();
        let bind = |scope: &mut ListenScope<Told>, addr: &str| {
            let addr: Multiaddr = addr.parse().expect("an address");
            scope.on_swarm_event(FromSwarm::NewListenAddr(NewListenAddr {
                listener_id: listener,
                addr: &addr,
            }));
        };
        let (lan, shared, mapped) = (
            "/ip4/192.168.1.5/tcp/4001",
            "/ip4/100.64.3.9/tcp/4001",
            "/ip6/::ffff:100.64.3.9/tcp/4002",
        );
        let circuit = "/ip4/100.100.1.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/p2p-circuit";
        bind(&mut scope, lan);
        bind(&mut scope, shared);
        bind(&mut scope, mapped);
        bind(&mut scope, circuit);
        assert_eq!(told(&scope), set(&[lan, circuit]), "no view");
        let view: BTreeSet<IpAddr> = ["192.168.1.5", "100.64.3.9"]
            .iter()
            .map(|ip| ip.parse().expect("an IP"))
            .collect();
        assert_eq!(
            scope.withhold(&BTreeSet::new(), Some(&view)),
            0,
            "a view moves nothing"
        );
        assert_eq!(told(&scope), set(&[lan, circuit]));
    }

    /// The second case (A 2026-10-27): with no view a private listener
    /// the platform never named is told -- THE CONTROL, the desktop --
    /// and once a view is present one outside it is withheld whether it
    /// was bound before the view or after, while a circuit listener
    /// through a private relay and a global address outside the view are
    /// told; a view naming the IP releases it, an IPv4-mapped form
    /// included. The Wi-Fi address is the control throughout.
    #[test]
    fn a_private_listen_address_outside_the_view_is_never_advertised() {
        let mut scope = ListenScope::new(Told::default());
        let listener = ListenerId::next();
        let bind = |scope: &mut ListenScope<Told>, addr: &str| {
            let addr: Multiaddr = addr.parse().expect("an address");
            scope.on_swarm_event(FromSwarm::NewListenAddr(NewListenAddr {
                listener_id: listener,
                addr: &addr,
            }));
        };
        let ip = |s: &str| -> IpAddr { s.parse().expect("an IP") };
        let (wifi, background, ims) = (
            "/ip4/192.168.1.5/tcp/4001",
            "/ip4/10.20.30.40/tcp/4001",
            "/ip4/10.99.0.7/tcp/4001",
        );
        let circuit = "/ip4/10.1.1.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/p2p-circuit";
        let global = "/ip4/203.0.113.9/tcp/4001";
        bind(&mut scope, wifi);
        bind(&mut scope, background);
        assert_eq!(
            told(&scope),
            set(&[wifi, background]),
            "the control: no view, nothing outside it"
        );

        let view = BTreeSet::from([ip("192.168.1.5")]);
        assert_eq!(scope.withhold(&BTreeSet::new(), Some(&view)), 1);
        assert_eq!(told(&scope), set(&[wifi]), "bound before the view");
        bind(&mut scope, ims);
        bind(&mut scope, circuit);
        bind(&mut scope, global);
        assert_eq!(
            told(&scope),
            set(&[wifi, circuit, global]),
            "bound after the view: withheld at the door; a relay's IP and a global one are not judged"
        );

        let mapped = "/ip6/::ffff:10.20.30.40/tcp/4002";
        bind(&mut scope, mapped);
        let named = BTreeSet::from([ip("192.168.1.5"), ip("::ffff:10.20.30.40")]);
        assert_eq!(scope.withhold(&BTreeSet::new(), Some(&named)), 2);
        assert_eq!(
            told(&scope),
            set(&[wifi, background, circuit, global, mapped]),
            "a view naming the IP releases it, either form"
        );
        assert_eq!(scope.withhold(&BTreeSet::new(), None), 1, "no view again");
        assert_eq!(
            told(&scope),
            set(&[wifi, background, ims, circuit, global, mapped])
        );
    }
}

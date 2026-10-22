// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! ADR-0062 decision 4's show side: the one place that decides which of
//! this node's addresses a person is shown, and in which order
//! (`admin.peers.own_addresses`, LOCAL-IPC.md).
//!
//! The Swarm task hands over what it holds -- the AutoNAT-verified
//! addresses in confirmation order, the active reservations' circuit
//! addresses, the listeners in bind order -- and [`select`] does the
//! rest, so no caller (the daemon's admin socket, the embedded host)
//! can apply a floor of its own or forget one. The client filters
//! nothing: a row it receives is one an add would take.
//!
//! Relay-only is ADR-0062 decision 4's definition: no externally
//! confirmed address exists at the moment of the read. It is decided on
//! what AutoNAT confirmed, before the floor, so a confirmed address the
//! floor would omit still means the node is not relay-only.

use interweave_local_client_api::{MAX_OWN_ADDRESSES, OwnAddress, OwnAddressKind, OwnAddresses};
use interweave_profile_config::bootstrap_overlay::entry_grammar;
use interweave_transport_api::TransportIdentity;
use interweave_transport_runtime::reachability::special_use_host;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};

use crate::served_addresses::is_relayed;

/// This node's own addresses, selected and floored.
///
/// `confirmed` are the AutoNAT-verified addresses in the order they were
/// confirmed, `circuits` the active reservations' addresses, `listen`
/// the bound listener addresses in bind order, each as the runtime holds
/// it. Each row ends in `/p2p/<peer>` (appended when absent; an address
/// naming another peer is not this node's and is omitted), meets the
/// add's own rule and ADR-0052 rule 2's floor, and appears once, under
/// the first kind that names it.
pub(super) fn select<'a>(
    peer: PeerId,
    identity: TransportIdentity,
    confirmed: impl IntoIterator<Item = &'a str>,
    circuits: impl IntoIterator<Item = &'a str>,
    listen: impl IntoIterator<Item = &'a Multiaddr>,
) -> OwnAddresses {
    let admitted = |address: &Multiaddr, relayed: bool| -> Option<String> {
        if is_relayed(address) != relayed {
            return None;
        }
        let address = with_peer(address, peer)?.to_string();
        (entry_grammar(&address).is_ok() && special_use_host(&address).is_none()).then_some(address)
    };
    let parsed = |a: &str| a.parse::<Multiaddr>().ok();

    let mut rows: Vec<OwnAddress> = Vec::new();
    let push = |address: String, kind: OwnAddressKind, rows: &mut Vec<OwnAddress>| {
        if !rows.iter().any(|r| r.address == address) {
            rows.push(OwnAddress { address, kind });
        }
    };
    // Relay-only is read off the list as AutoNAT handed it over, every
    // entry counted: an entry the rows below omit -- unparsable, relayed,
    // floored -- is still a confirmation, so it never turns a confirmed
    // node relay-only.
    let confirmed: Vec<&str> = confirmed.into_iter().collect();
    let relay_only = confirmed.is_empty();
    for address in confirmed.iter().copied().filter_map(parsed) {
        if let Some(a) = admitted(&address, false) {
            push(a, OwnAddressKind::ConfirmedExternal, &mut rows);
        }
    }
    if relay_only {
        for address in circuits.into_iter().filter_map(parsed) {
            if let Some(a) = admitted(&address, true) {
                push(a, OwnAddressKind::RelayCircuit, &mut rows);
            }
        }
    }
    for address in listen {
        if let Some(a) = admitted(address, false) {
            push(a, OwnAddressKind::Listen, &mut rows);
        }
    }
    let truncated = rows.len() > MAX_OWN_ADDRESSES;
    rows.truncate(MAX_OWN_ADDRESSES);
    OwnAddresses {
        peer: identity,
        addresses: rows,
        truncated,
    }
}

/// `address` ending in `/p2p/<peer>`: as it is when it already does,
/// with it appended when it ends in no peer, `None` when it ends in
/// another's.
fn with_peer(address: &Multiaddr, peer: PeerId) -> Option<Multiaddr> {
    match address.iter().last() {
        Some(Protocol::P2p(last)) if last == peer => Some(address.clone()),
        Some(Protocol::P2p(_)) => None,
        _ => Some(address.clone().with(Protocol::P2p(peer))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn me() -> (PeerId, TransportIdentity) {
        let peer = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let identity = TransportIdentity::parse(peer.to_base58()).expect("a canonical identity");
        (peer, identity)
    }

    fn ma(s: &str) -> Multiaddr {
        s.parse().expect("a multiaddr")
    }

    fn rows(own: &OwnAddresses) -> Vec<(String, OwnAddressKind)> {
        own.addresses
            .iter()
            .map(|r| (r.address.clone(), r.kind))
            .collect()
    }

    /// Decision 4's order: confirmed first in confirmation order, the
    /// circuit route left out beside one, listeners last in bind order,
    /// each with this node's `PeerId`.
    #[test]
    fn the_list_is_confirmed_then_listen_and_no_circuit_beside_a_confirmed_one() {
        let (peer, identity) = me();
        let relay = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let circuit = format!("/ip4/93.184.216.34/tcp/4001/p2p/{relay}/p2p-circuit");
        let own = select(
            peer,
            identity.clone(),
            ["/ip4/84.17.40.7/tcp/4001", "/ip4/84.17.40.3/tcp/4001"],
            [circuit.as_str()],
            &[
                ma("/ip4/192.168.1.20/tcp/4001"),
                ma("/ip4/10.0.0.5/tcp/4001"),
            ],
        );
        assert_eq!(own.peer, identity);
        assert_eq!(
            rows(&own),
            [
                (
                    format!("/ip4/84.17.40.7/tcp/4001/p2p/{peer}"),
                    OwnAddressKind::ConfirmedExternal
                ),
                (
                    format!("/ip4/84.17.40.3/tcp/4001/p2p/{peer}"),
                    OwnAddressKind::ConfirmedExternal
                ),
                (
                    format!("/ip4/192.168.1.20/tcp/4001/p2p/{peer}"),
                    OwnAddressKind::Listen
                ),
                (
                    format!("/ip4/10.0.0.5/tcp/4001/p2p/{peer}"),
                    OwnAddressKind::Listen
                ),
            ]
        );
        assert!(!own.truncated);
    }

    /// Relay-only: with nothing confirmed the circuit route comes before
    /// the listeners. The control: a confirmed entry, even one no row
    /// shows -- floored, relayed or unparsable -- means the node is not
    /// relay-only, so no circuit row.
    #[test]
    fn a_relay_only_node_shows_its_circuit_route_before_its_listeners() {
        let (peer, identity) = me();
        let relay = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let circuit = format!("/ip4/93.184.216.34/tcp/4001/p2p/{relay}/p2p-circuit");
        let listen = [ma("/ip4/192.168.1.20/tcp/4001")];
        for omitted in [
            "/ip4/127.0.0.1/tcp/4001",
            circuit.as_str(),
            "not a multiaddr",
        ] {
            let confirmed_omitted = select(
                peer,
                identity.clone(),
                [omitted],
                [circuit.as_str()],
                &listen,
            );
            assert_eq!(
                rows(&confirmed_omitted),
                [(
                    format!("/ip4/192.168.1.20/tcp/4001/p2p/{peer}"),
                    OwnAddressKind::Listen
                )],
                "{omitted} confirmed, though not shown: not relay-only"
            );
        }
        let own = select(peer, identity, [], [circuit.as_str()], &listen);
        assert_eq!(
            rows(&own),
            [
                (
                    format!("{circuit}/p2p/{peer}"),
                    OwnAddressKind::RelayCircuit
                ),
                (
                    format!("/ip4/192.168.1.20/tcp/4001/p2p/{peer}"),
                    OwnAddressKind::Listen
                ),
            ]
        );
    }

    /// The floor is the add's: every special-use class is omitted, a
    /// private range kept beside them as the control; a listener's
    /// circuit address is not a listen row and a direct address is not
    /// a circuit row.
    #[test]
    fn every_row_is_in_the_classes_the_add_admits() {
        let (peer, identity) = me();
        let relay = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let loopback_relay = format!("/ip4/127.0.0.1/tcp/4001/p2p/{relay}/p2p-circuit");
        let listen = [
            ma("/ip4/127.0.0.1/tcp/4001"),
            ma("/ip4/169.254.10.1/tcp/4001"),
            ma("/ip4/0.0.0.0/tcp/4001"),
            ma("/ip6/::1/tcp/4001"),
            ma("/ip6/fe80::1/tcp/4001"),
            ma(&format!(
                "/ip4/93.184.216.34/tcp/4001/p2p/{relay}/p2p-circuit"
            )),
            ma("/ip4/172.16.0.2/tcp/4001"),
        ];
        let own = select(
            peer,
            identity,
            [],
            [loopback_relay.as_str(), "/ip4/84.17.40.7/tcp/4001"],
            &listen,
        );
        assert_eq!(
            rows(&own),
            [(
                format!("/ip4/172.16.0.2/tcp/4001/p2p/{peer}"),
                OwnAddressKind::Listen
            )]
        );
        for row in &own.addresses {
            assert!(entry_grammar(&row.address).is_ok());
        }
    }

    /// An address already ending in this node's `PeerId` is kept as it is
    /// and one ending in another peer's is not this node's; an address in
    /// two sources appears once, under the first.
    #[test]
    fn the_peer_suffix_is_this_nodes_and_an_address_appears_once() {
        let (peer, identity) = me();
        let other = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let own = select(
            peer,
            identity,
            [format!("/ip4/84.17.40.7/tcp/4001/p2p/{peer}").as_str()],
            [],
            &[
                ma("/ip4/84.17.40.7/tcp/4001"),
                ma(&format!("/ip4/192.168.1.20/tcp/4001/p2p/{other}")),
            ],
        );
        assert_eq!(
            rows(&own),
            [(
                format!("/ip4/84.17.40.7/tcp/4001/p2p/{peer}"),
                OwnAddressKind::ConfirmedExternal
            )]
        );
    }

    /// The bound: the 65th row and on are dropped and said to be; exactly
    /// 64 is not truncated.
    #[test]
    fn the_list_is_bounded_and_says_when_it_dropped_rows() {
        let (peer, identity) = me();
        let listen = |n: u16| -> Vec<Multiaddr> {
            (0..n)
                .map(|i| ma(&format!("/ip4/192.168.1.20/tcp/{}", 4000 + i)))
                .collect()
        };
        let full = select(peer, identity.clone(), [], [], &listen(64));
        assert_eq!(full.addresses.len(), MAX_OWN_ADDRESSES);
        assert!(!full.truncated);
        let over = select(peer, identity, [], [], &listen(70));
        assert_eq!(over.addresses.len(), MAX_OWN_ADDRESSES);
        assert!(over.truncated);
        assert_eq!(over.addresses[63], full.addresses[63]);
    }
}

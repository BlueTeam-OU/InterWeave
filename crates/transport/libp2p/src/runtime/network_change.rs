// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `transport/libp2p/CONNECTIVITY.md` §14: what a network change IS to
//! this runtime (step 10; the platform's view since §20 step 5).
//!
//! A network change is a move in the set of IP addresses this host is
//! known to hold. Two sources report that set, and both feed this ONE
//! detector (architect-cto's ruling of 2026-10-09, relay seq 33736):
//!
//! - the LISTENERS: the addresses they have bound. A wildcard listener
//!   reports each interface's address as it comes and goes, so a Wi-Fi
//!   hand-over, a VPN coming up or a hotspot going away arrive as the
//!   bound set changing; a `Listen` or `StopListening` command that adds
//!   or removes an IP is the same change made by hand, which is how the
//!   wire tests raise one on a single host. On Android `if-watch`
//!   polls `getifaddrs` every 10 s (its netlink backend is Linux-only),
//!   and whether that answers an app at all is a device question;
//! - the PLATFORM's view (`NetworkView`), a snapshot of every usable
//!   address the OS reports, handed in by the host's own network
//!   monitor. Until the first view arrives the platform's view is
//!   unknown, and the listeners are the only source.
//!
//! Compared as IPs, not as listener addresses: two listeners on one IP
//! (two ports, TCP and QUIC) losing one leave the host on the same
//! network, and a view names no ports. Each source's observation is
//! compared with THAT SOURCE's previous one, and the difference is
//! applied to the one known set; a change is reported only when the
//! known set moves. So the platform's view reports a hand-over at once
//! and the listener poll that sees the same move 10 s later finds
//! nothing left to report -- §14's handling runs once. Where the two
//! sources disagree for good (an address one reports and the other
//! never does), neither keeps re-reporting it, since each only applies
//! its own differences. Where the VIEW lags the listeners across two
//! moves -- an address gone and back on the listeners before the view
//! saw it go -- the late view reports a move that already reversed and
//! the next view reverses it again: a bounded spurious pair, never a
//! stuck set. The listeners lagging the view cannot do that, since the
//! view protects what it names (below).
//! A Wi-Fi reconnect that returns the same addresses (SPIKE-008 L7, a
//! new network id on the same Wi-Fi) is NO change here: the host passes
//! a view only when its addresses differ, and the connections that
//! survived prove themselves by their own keepalives.
//!
//! What a set says nothing about is left out of the comparison:
//! loopback, unspecified and link-local addresses are bound to an
//! interface, not to a network, and one coming or going is not a move
//! (PR #89 round 5 found the opposite mistake: comparing through the
//! PROBE candidate rule removed every private listener too, so a NAT'd
//! profile could not see a move between LANs at all).
//!
//! Detected ONCE, here, and told to every subsystem that holds
//! network-dependent state -- the AutoNAT client's evidence, the DCUtR
//! wrapper's attempts and cooldowns, the dial backoff -- rather than by
//! each of them: the AutoNAT adapter used to compare the set itself, so
//! with the client off a change was seen by nothing. The first time the
//! set fills is not a change -- nothing is reported or invalidated --
//! but it runs an addition's lift ([`NetworkSet::take_filled`]): a
//! runtime started offline dialled and failed before it, and coming
//! online is exactly the move the lift is for (ADR-0011 A 2026-10-09).
//! Every later difference is a change, including the set emptying and
//! filling again, since the interface that returns is not known to be
//! the one that left -- and only a removal INVALIDATES what was known
//! (`NetworkChange::invalidates`): an addition is reported, offered,
//! and makes the peers held off dialable once.
//!
//! THE VIEW, ONCE PRESENT, IS AUTHORITATIVE for the addresses it names
//! (the same ruling): the listeners add to the known set and never
//! remove an address the view still holds -- a listener closed while
//! the platform still reports its address leaves the set intact, and
//! the view's own later departure of it is the removal. Without that,
//! a one-sided drop took the address out of the set while the platform
//! held it, and its real departure later was no removal at all, so
//! connections made from it in between survived it.
//!
//! Pinned by `a_network_change_is_a_move_of_the_known_ip_set_after_it_first_fills`,
//! `the_two_sources_report_one_move_once` and
//! `interface_scoped_addresses_and_only_those_are_left_out_of_the_comparison`;
//! on the wire by `tests/connectivity/tests/network_change.rs` and
//! `dcutr.rs`'s `a_network_change_lifts_the_cooldown_and_keeps_the_reservation`.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

use libp2p::Multiaddr;
use libp2p::core::ConnectedPoint;
use libp2p::multiaddr::Protocol;

use interweave_transport_runtime::reachability::{is_private_range, literal_host};

use super::messages::PeerPath;

/// The IP set this host is known to hold, and what each source last
/// said of it -- without the interface-scoped addresses.
#[derive(Debug, Default)]
pub(super) struct NetworkSet {
    /// The one set a change is a move of.
    known: BTreeSet<IpAddr>,
    /// The IPs the listeners had bound at their last observation.
    listeners: BTreeSet<IpAddr>,
    /// The platform's last view; `None` until the first, when the
    /// platform's view is unknown rather than empty.
    platform: Option<BTreeSet<IpAddr>>,
    /// The private-range IPs of the platform's last view that it
    /// classifies as lying on NO LOCAL NETWORK (ADR-0052 rule 10, A
    /// 2026-10-26): a phone's carrier-assigned cellular address.
    /// Bounded by the view's addresses.
    classified: BTreeSet<IpAddr>,
    /// The IPs on NO LOCAL NETWORK (ADR-0052 rule 10): those the view
    /// classifies, and -- while a view is present (A 2026-10-27) -- every
    /// private-range IP a direct listener binds that the view does not
    /// hold, since a network the platform withholds from the app is one
    /// no peer reaches it on (a cellular network kept in the background,
    /// a carrier-restricted one). Known and bound like any other -- a
    /// dial, a reservation and the carrier NAT's public address run over
    /// it -- but never this node's address toward anyone
    /// ([`Self::offers`]). Derived from the view and the listeners after
    /// every observation of either ([`Self::derive_off_local`]), so
    /// neither order of the two decides it. Empty with no view, so the
    /// desktop classifies nothing; bounded by the view's addresses and
    /// the listeners this process binds.
    off_local: BTreeSet<IpAddr>,
    /// Whether the known set has ever held an address: the first time
    /// it fills is not a change.
    filled_once: bool,
    /// The first fill happened and its lift has not been taken.
    filled_untaken: bool,
    /// Where what this node does not offer ([`Self::not_offered`]) is
    /// published, for rule 3 at the root funnel ([`crate::held_listeners`]).
    published: crate::held_listeners::HeldListeners,
    /// The IPs of the bound addresses rule 3 reads as an own listener:
    /// those `literal_host` takes, as `is_punchable_address` reads an
    /// own listener -- not a circuit listener, whose leading IP is the
    /// relay's report and which rule 3 never counts, though `listeners`
    /// holds its first IP for the known set
    /// (`every_way_rule_3s_boundary_widens_is_counted_and_nothing_else`,
    /// its circuit step).
    direct_listeners: BTreeSet<IpAddr>,
    /// The private-range IPs of `direct_listeners` this node offered at
    /// the last observation: rule 3's own-listener boundary, kept to
    /// count each widening of it ([`Self::publish`]).
    offered_private: BTreeSet<IpAddr>,
}

/// What changed: the IPs that left the known set and those that joined
/// it, each sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NetworkChange {
    /// Known at the last move and not now: the IPs that DEPARTED this
    /// host.
    pub removed: Vec<IpAddr>,
    /// Known now and not at the last move.
    pub added: Vec<IpAddr>,
}

impl NetworkChange {
    /// Whether what this profile knew about its reachability is stale:
    /// only when an address LEFT the set. §14 item 1 invalidates the
    /// evidence "for removed direct addresses"; an addition alone -- an
    /// interface coming up, a VPN, a wildcard listener reporting one
    /// more of a multi-homed host's addresses at startup -- says
    /// nothing against the addresses still held, so it is reported and
    /// its address becomes a candidate on the next tick, and no
    /// evidence is forgotten and no attempt given up for it. Pinned by
    /// `a_network_change_is_a_move_of_the_known_ip_set_after_it_first_fills`
    /// and, on the wire, by `dcutr.rs`'s network-change test (an added
    /// address leaves the cooldown standing; a removed one lifts it).
    #[must_use]
    pub(super) fn invalidates(&self) -> bool {
        !self.removed.is_empty()
    }

    /// Whether an address JOINED: the move after which a peer held off
    /// by failures on the old network is worth one dial at once
    /// (`ConnectionManager::network_added`).
    #[must_use]
    pub(super) fn adds(&self) -> bool {
        !self.added.is_empty()
    }
}

impl NetworkSet {
    /// Observe the listeners' bound set: `Some` when it moved the known
    /// set after that set first filled.
    pub(super) fn observe_listeners<'a>(
        &mut self,
        bound: impl Iterator<Item = &'a Multiaddr>,
    ) -> Option<NetworkChange> {
        let bound: Vec<&Multiaddr> = bound.collect();
        self.direct_listeners = bound
            .iter()
            .filter_map(|a| literal_host(&a.to_string()))
            .collect();
        self.derive_off_local();
        let now: BTreeSet<IpAddr> = bound
            .into_iter()
            .filter(|a| !is_interface_scoped(a))
            .filter_map(first_ip)
            .collect();
        let before = std::mem::replace(&mut self.listeners, now.clone());
        let held_by_view = self.platform.clone().unwrap_or_default();
        self.apply(&before, &now, &held_by_view)
    }

    /// Observe the platform's view: `Some` when it moved the known set
    /// after that set first filled. The first view is compared with
    /// nothing, so it can add what the listeners have not bound and
    /// remove nothing: before it the platform said nothing, which is
    /// not the same as saying an address was gone.
    ///
    /// `off_local_network` is the view's classification
    /// (`NetworkView::off_local_network`): its private-range members
    /// that are among the view's addresses join [`Self::off_local`],
    /// replacing the last view's, beside the bound private IPs the view
    /// does not hold. It moves nothing in the known set, so a
    /// classification alone is no change.
    pub(super) fn observe_view(
        &mut self,
        addresses: &[IpAddr],
        off_local_network: &[IpAddr],
    ) -> Option<NetworkChange> {
        let now: BTreeSet<IpAddr> = addresses
            .iter()
            .copied()
            .filter(|ip| !is_interface_scoped_ip(*ip))
            .collect();
        self.classified = off_local_network
            .iter()
            .copied()
            .filter(|ip| now.contains(ip) && is_private_range(*ip))
            .collect();
        let before = self.platform.replace(now.clone()).unwrap_or_default();
        self.derive_off_local();
        self.apply(&before, &now, &BTreeSet::new())
    }

    /// The private-range IPs on no local network ([`Self::offers`]
    /// withholds them): what the view classifies, and the bound ones it
    /// does not hold.
    pub(super) const fn off_local(&self) -> &BTreeSet<IpAddr> {
        &self.off_local
    }

    /// The platform's last view, `None` before the first: what Identify's
    /// door judges a listen address against as the Swarm reports it
    /// (`ListenScope::withhold`), before this set has observed it.
    pub(super) const fn view(&self) -> Option<&BTreeSet<IpAddr>> {
        self.platform.as_ref()
    }

    /// [`Self::off_local`] from what the two sources last said: the
    /// view's classification, and each direct listener's IP outside the
    /// view while there is one (`listen_scope::is_outside_the_view`, the
    /// predicate Identify's door applies too). A circuit listener is not
    /// among them: its leading IP is the relay's, which no view of this
    /// host's holds. Pinned by
    /// `a_bound_private_ip_no_view_holds_is_on_no_local_network_in_either_order`.
    fn derive_off_local(&mut self) {
        let outside: Vec<IpAddr> = self.platform.as_ref().map_or_else(Vec::new, |view| {
            self.direct_listeners
                .iter()
                .copied()
                .filter(|ip| crate::listen_scope::is_outside_the_view(*ip, view))
                .collect()
        });
        self.off_local = self.classified.iter().copied().chain(outside).collect();
    }

    /// The IPs that are not this node's address toward anyone, bound or
    /// not: those on no local network ([`Self::off_local`]), and the
    /// bound ones the host no longer holds -- [`Self::offers`]'s two
    /// refusals, for the readers that cannot ask this set (the root
    /// funnel's rule 3, Identify's listen set). Each half is there for
    /// one order of the view and the listener poll: a classified IP
    /// bound only after the view stays withheld by the first, and a
    /// classified IP the view drops while still bound by the second.
    /// Pinned by `a_departed_off_local_ip_stays_withheld_while_bound` and
    /// `a_classified_ip_is_withheld_before_any_listener_binds_it`.
    pub(super) fn not_offered(&self) -> BTreeSet<IpAddr> {
        self.listeners
            .iter()
            .filter(|ip| !self.known.contains(*ip))
            .chain(&self.off_local)
            .copied()
            .collect()
    }

    /// A detector that publishes what this node does not offer
    /// ([`Self::not_offered`]) to `held` after every observation.
    pub(super) fn publishing_to(held: crate::held_listeners::HeldListeners) -> Self {
        Self {
            published: held,
            ..Self::default()
        }
    }

    /// The own listeners rule 3 counts (ADR-0052 A 2026-10-09, rule 10 A
    /// 2026-10-26): those of `bound` this host [`offers`](Self::offers),
    /// as strings.
    pub(super) fn own_listeners<'a>(
        &self,
        bound: impl Iterator<Item = &'a Multiaddr>,
    ) -> Vec<String> {
        bound
            .filter(|a| self.offers(a))
            .map(ToString::to_string)
            .collect()
    }

    /// The count of rule 3's boundary widening (`Self::publish`), for
    /// the task's own learn sites (`dialing::HeldBack`).
    pub(super) fn widenings(&self) -> u64 {
        self.published.widenings().count()
    }

    /// Whether the first fill of the known set has happened since this
    /// was last asked: the runtime runs an addition's lift for it,
    /// though it reported no change. `true` once.
    pub(super) fn take_filled(&mut self) -> bool {
        std::mem::take(&mut self.filled_untaken)
    }

    /// Whether `address` is on an IP this host is known to hold, or is
    /// interface-scoped or names no IP: what may still be offered as a
    /// candidate. A listener on an IP the view has said departed is
    /// bound until the listener poll catches up, and must not be offered
    /// meanwhile.
    pub(super) fn holds(&self, address: &Multiaddr) -> bool {
        first_ip(address).is_none_or(|ip| self.holds_ip(ip))
    }

    fn holds_ip(&self, ip: IpAddr) -> bool {
        is_interface_scoped_ip(ip) || self.known.contains(&ip)
    }

    /// Whether `address` is THIS NODE'S ADDRESS TOWARD ANYONE: one it
    /// [`holds`](Self::holds), and not on an IP the platform classifies
    /// as a private-range address on no local network (ADR-0052 rule
    /// 10). What is shown (`own_addresses::select`), offered to DCUtR,
    /// and counted as an own private listener for rule 3 -- the three
    /// sites the rule names beside Identify, which is told
    /// [`Self::not_offered`] through its own wrapper (`listen_scope`).
    /// AutoNAT's candidates stay on `holds`: a private address is never
    /// one, and a global cellular one is AutoNAT's to decide. Pinned by
    /// `a_private_address_on_no_local_network_is_held_and_not_offered`.
    pub(super) fn offers(&self, address: &Multiaddr) -> bool {
        first_ip(address).is_none_or(|ip| self.offers_ip(ip))
    }

    /// [`Self::offers`] for an address on `ip`: the one predicate, read
    /// by the widening count as by every site that asks of an address.
    fn offers_ip(&self, ip: IpAddr) -> bool {
        self.holds_ip(ip) && !self.off_local.contains(&ip)
    }

    /// Apply one source's difference to the known set; an IP in
    /// `protected` is never removed.
    fn apply(
        &mut self,
        before: &BTreeSet<IpAddr>,
        now: &BTreeSet<IpAddr>,
        protected: &BTreeSet<IpAddr>,
    ) -> Option<NetworkChange> {
        let removed: Vec<IpAddr> = before
            .difference(now)
            .filter(|ip| !protected.contains(*ip))
            .filter(|ip| self.known.remove(*ip))
            .copied()
            .collect();
        let added: Vec<IpAddr> = now
            .difference(before)
            .filter(|ip| self.known.insert(**ip))
            .copied()
            .collect();
        let moved = !removed.is_empty() || !added.is_empty();
        let change = (moved && self.filled_once).then_some(NetworkChange { removed, added });
        if !self.filled_once && !self.known.is_empty() {
            self.filled_once = true;
            self.filled_untaken = true;
        }
        self.publish();
        change
    }

    /// Publish what rule 3 at the root funnel must not count:
    /// [`Self::not_offered`]; and count a WIDENING of rule 3's boundary
    /// when this node now offers a private-range listener IP it did not
    /// at the last observation -- a listener reporting its LAN address
    /// after its loopback one, an IP joining the known set, the view
    /// lifting a rule 10 classification. Read from `direct_listeners`,
    /// the addresses rule 3 itself counts, so a circuit listener on a
    /// relay-reported IP neither widens nor masks a later direct
    /// listener on that IP (review F1 of #468, second engine). Every
    /// input `offers` reads reaches the known set or `off_local` through
    /// `apply`, which ends here
    /// (`every_way_rule_3s_boundary_widens_is_counted_and_nothing_else`).
    /// Counted per IP, a superset of rule 3's per-family answer turning
    /// yes: a second LAN of a family already held counts too, and costs
    /// the reader one offer more.
    fn publish(&mut self) {
        self.published.set_unheld(self.not_offered());
        let offered: BTreeSet<IpAddr> = self
            .direct_listeners
            .iter()
            .copied()
            .filter(|ip| is_private_range(*ip) && self.offers_ip(*ip))
            .collect();
        if !offered.is_subset(&self.offered_private) {
            self.published.widen();
        }
        self.offered_private = offered;
    }
}

/// Whether a removal whose departed IPs are `departed` -- the change's
/// `removed`, which the known set no longer holds, so two listeners on
/// one IP losing one depart nothing -- closes a connection running from
/// `local_ip` over `path` (`transport/libp2p/CONNECTIVITY.md` §14 item
/// 5).
///
/// A known local IP closes when it departed. An UNKNOWN one on a direct
/// connection -- an outbound dial by name, whose resolved address libp2p
/// does not report ([`local_ip_of`]) -- closes on ANY departure: a relay
/// control connection configured by name is exactly what the rule is
/// for, and leaving it to the keepalive left it standing some 45 s
/// (#129 review F3); closing it when it may have survived costs one
/// reconnect. A relayed connection is never closed here: it runs over
/// its relay's connection, which is judged by its own IP and takes the
/// circuit with it. Pinned by
/// `a_removal_closes_a_departed_ip_and_an_unknown_direct_one_only`.
pub(super) fn closes(
    local_ip: Option<IpAddr>,
    path: PeerPath,
    departed: &BTreeSet<IpAddr>,
) -> bool {
    match local_ip {
        Some(ip) => departed.contains(&ip),
        None => path == PeerPath::Direct && !departed.is_empty(),
    }
}

/// The IP a connection runs from on this host, or `None` where it
/// cannot be known.
///
/// An INBOUND connection's is its endpoint's local address. An
/// OUTBOUND one's is not in anything libp2p reports -- its endpoint
/// holds the remote only -- so it is read from the kernel: the source
/// IP the routing table picks for that remote, found by `connect` on a
/// UDP socket, which sends nothing. That is the IP the TCP dial itself
/// ran from because `libp2p-tcp` 0.45.0 binds every dial to the
/// UNSPECIFIED address (only the port is reused, `lib.rs:111-128`,
/// `:379-403`), leaving the source to the same routing decision, taken
/// milliseconds earlier. A relayed connection is `None`: it runs over
/// the relay's connection, which is closed by its own local IP and
/// takes the circuit with it; so is a remote given by name (`/dns4`),
/// whose resolved IP libp2p does not report -- [`closes`] says what a
/// removal does with each. Pinned on the wire by
/// `tests/connectivity/tests/network_change.rs`.
pub(super) fn local_ip_of(endpoint: &ConnectedPoint) -> Option<IpAddr> {
    if endpoint.is_relayed() {
        return None;
    }
    match endpoint {
        ConnectedPoint::Listener { local_addr, .. } => first_ip(local_addr),
        ConnectedPoint::Dialer { address, .. } => first_ip(address).and_then(route_source),
    }
}

/// The source IP this host would use toward `remote` now.
fn route_source(remote: IpAddr) -> Option<IpAddr> {
    let any: SocketAddr = match remote {
        IpAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
        IpAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
    };
    let socket = UdpSocket::bind(any).ok()?;
    // The port is arbitrary: a UDP `connect` only fixes the peer.
    socket.connect((remote, 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified()).then_some(ip)
}

fn first_ip(address: &Multiaddr) -> Option<IpAddr> {
    match address.iter().next()? {
        Protocol::Ip4(ip) => Some(IpAddr::V4(ip)),
        Protocol::Ip6(ip) => Some(IpAddr::V6(ip)),
        _ => None,
    }
}

/// Whether `address` is bound to an interface rather than a network:
/// loopback, unspecified, or link-local. Such a listener coming or
/// going says nothing about where this profile is, so it is left out
/// of the network-change comparison; everything else, private and
/// CGNAT ranges included, is in. Pinned by
/// `interface_scoped_addresses_and_only_those_are_left_out_of_the_comparison`.
pub(super) fn is_interface_scoped(address: &Multiaddr) -> bool {
    first_ip(address).is_some_and(is_interface_scoped_ip)
}

/// [`is_interface_scoped`] for an IP, as the platform's view names one.
fn is_interface_scoped_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_unspecified() || ip.is_link_local(),
        IpAddr::V6(ip) => {
            ip.is_loopback() || ip.is_unspecified() || (ip.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn addrs(list: &[&str]) -> Vec<Multiaddr> {
        list.iter().map(|a| a.parse().expect("a literal")).collect()
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().expect("an ip")
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "compared with what `observe_*` returns"
    )]
    fn change(removed: &[&str], added: &[&str]) -> Option<NetworkChange> {
        Some(NetworkChange {
            removed: removed.iter().map(|a| ip(a)).collect(),
            added: added.iter().map(|a| ip(a)).collect(),
        })
    }

    /// ADR-0052 rule 10 (A 2026-10-26) on the set: a private-range
    /// address the view classifies as on no local network stays held --
    /// known, so its departure is still a change -- and is offered
    /// nowhere: not as an own listener for rule 3, not to the root
    /// funnel's copy. A global cellular address and an entry the view
    /// does not hold are outside the rule; a classification alone moves
    /// nothing.
    #[test]
    fn a_private_address_on_no_local_network_is_held_and_not_offered() {
        use interweave_transport_runtime::reachability::{CandidateRefusal, is_punchable_address};
        let held = crate::held_listeners::HeldListeners::new();
        let mut set = NetworkSet::publishing_to(held.clone());
        let wifi = "/ip4/192.168.1.5/tcp/4001";
        let cellular = "/ip4/10.20.30.40/tcp/4001";
        let global_cellular = "/ip6/2a00:1450::7/tcp/4001";
        let bound = addrs(&[wifi, cellular, global_cellular]);
        assert_eq!(set.observe_listeners(bound.iter()), None, "the first fill");
        let addresses = [ip("192.168.1.5"), ip("10.20.30.40"), ip("2a00:1450::7")];
        let classified = [ip("10.20.30.40"), ip("2a00:1450::7"), ip("172.16.9.9")];
        assert_eq!(
            set.observe_view(&addresses, &classified),
            None,
            "the view names what the listeners bound, a classification moves nothing"
        );
        assert_eq!(set.off_local(), &BTreeSet::from([ip("10.20.30.40")]));
        let [wifi_a, cellular_a, global_a] = [&bound[0], &bound[1], &bound[2]];
        assert!(set.holds(cellular_a), "held: it is the host's");
        assert!(
            !set.offers(cellular_a),
            "and no address of this node toward anyone"
        );
        assert!(set.offers(wifi_a), "the control: a local network's");
        assert!(
            set.offers(global_a),
            "a global cellular address is AutoNAT's"
        );
        assert!(
            !held.holds(cellular_a),
            "the root funnel's rule 3 does not count it"
        );
        assert!(held.holds(wifi_a));
        let candidate = "/ip4/10.1.2.3/tcp/4001";
        let own = set.own_listeners(bound.iter());
        assert_eq!(own, vec![wifi.to_owned(), global_cellular.to_owned()]);
        assert_eq!(
            is_punchable_address(candidate, own.iter().map(String::as_str)),
            Ok(()),
            "beside the Wi-Fi listener"
        );

        // Wi-Fi goes: the cellular address alone admits no private
        // candidate, though it is a bound private listener.
        assert_eq!(
            set.observe_view(&addresses[1..], &classified),
            change(&["192.168.1.5"], &[])
        );
        let own = set.own_listeners(bound.iter());
        assert_eq!(
            is_punchable_address(candidate, own.iter().map(String::as_str)),
            Err(CandidateRefusal::PrivateWithoutPrivateListener)
        );
        // The control: unclassified, the same listener counts, as before
        // the rule and as on the desktop.
        assert_eq!(set.observe_view(&addresses[1..], &[]), None);
        let own = set.own_listeners(bound.iter());
        assert_eq!(
            is_punchable_address(candidate, own.iter().map(String::as_str)),
            Ok(())
        );
        assert!(held.holds(cellular_a));
        // And its departure is a change like any other.
        assert_eq!(
            set.observe_view(&addresses[2..], &classified),
            change(&["10.20.30.40"], &[])
        );
    }

    /// Rule 3's boundary WIDENS when this node comes to offer a
    /// private-range listener IP it did not, and the record counts every
    /// such move and no other (`HeldListeners::widenings`). Each step
    /// reads the discovery verdict on a private candidate through
    /// `own_listeners`, as the `Learn` command does: a verdict that
    /// turns from refused to admitted is a counted move, every time.
    /// The first step is a wildcard listener's loopback report arriving
    /// before its LAN one -- a cache's private route offered then was
    /// refused, and is admitted after the second (j133).
    #[test]
    fn every_way_rule_3s_boundary_widens_is_counted_and_nothing_else() {
        use interweave_transport_runtime::reachability::is_discovered_address;
        let held = crate::held_listeners::HeldListeners::new();
        let widenings = held.widenings();
        let mut set = NetworkSet::publishing_to(held);
        let candidate = "/ip4/10.137.0.9/tcp/4001";
        let lo = "/ip4/127.0.0.1/tcp/4001";
        let lan = "/ip4/192.168.1.5/tcp/4001";
        let global = "/ip6/2a00:1450::7/tcp/4001";
        let second_lan = "/ip4/10.0.0.9/tcp/4001";
        let circuit = "/ip4/192.168.1.5/tcp/4002/p2p/12D3KooWCLxLXFHqvfsHVLDcNsSpZBQq1M1KMRgQRLLLnHTv7oQD/p2p-circuit";
        let mut bound: Vec<Multiaddr> = Vec::new();
        let mut was_admitted = false;
        let mut last = 0;
        let mut step = |set: &mut NetworkSet, bound: &[Multiaddr], what: &str, moved: bool| {
            let own = set.own_listeners(bound.iter());
            let admitted = is_discovered_address(candidate, own.iter().map(String::as_str)).is_ok();
            let count = widenings.count();
            assert_eq!(count != last, moved, "{what}: count {last} -> {count}");
            if admitted && !was_admitted {
                assert!(
                    moved,
                    "{what}: admitted now, refused before, and not counted"
                );
            }
            was_admitted = admitted;
            last = count;
            admitted
        };

        bound.extend(addrs(&[lo]));
        let _ = set.observe_listeners(bound.iter());
        assert!(!step(&mut set, &bound, "loopback alone", false), "refused");
        // A circuit listener whose relay reported this host's LAN IP:
        // rule 3 counts no circuit as an own listener, so neither does
        // the count, and the direct listener on that IP still widens.
        bound.extend(addrs(&[circuit]));
        let _ = set.observe_listeners(bound.iter());
        assert!(
            !step(&mut set, &bound, "a circuit listener on the LAN IP", false),
            "refused"
        );
        bound.extend(addrs(&[lan]));
        let _ = set.observe_listeners(bound.iter());
        assert!(
            step(&mut set, &bound, "the LAN address after it", true),
            "admitted"
        );
        let _ = set.observe_listeners(bound.iter());
        step(&mut set, &bound, "the same listeners again", false);
        bound.extend(addrs(&[global]));
        let _ = set.observe_listeners(bound.iter());
        step(&mut set, &bound, "a global listener", false);

        let view = [ip("192.168.1.5"), ip("2a00:1450::7")];
        let _ = set.observe_view(&view, &[ip("192.168.1.5")]);
        assert!(
            !step(
                &mut set,
                &bound,
                "the view classifies the LAN off-local",
                false
            ),
            "a narrowing is refused and not counted"
        );
        let _ = set.observe_view(&view, &[]);
        assert!(step(&mut set, &bound, "the classification lifted", true));

        bound.retain(|a| a.to_string() != lan);
        let _ = set.observe_listeners(bound.iter());
        assert!(
            !step(
                &mut set,
                &bound,
                "the LAN listener closed, the view still holding its IP",
                false
            ),
            "rule 3 counts a bound listener only"
        );
        bound.extend(addrs(&[lan]));
        let _ = set.observe_listeners(bound.iter());
        assert!(step(&mut set, &bound, "the LAN listener bound again", true));
        // A private listener on an IP the view does not hold is on no
        // local network (rule 10, A 2026-10-27): no widening until a view
        // names it, and the view naming it is one.
        bound.extend(addrs(&[second_lan]));
        let _ = set.observe_listeners(bound.iter());
        step(
            &mut set,
            &bound,
            "a private listener outside the view",
            false,
        );
        let _ = set.observe_view(&[view[0], view[1], ip("10.0.0.9")], &[]);
        step(
            &mut set,
            &bound,
            "a second LAN of the family named by the view, the per-IP superset",
            true,
        );
    }

    /// ADR-0052 rule 10's second case (A 2026-10-27): while a view is
    /// present, a private-range IP a direct listener binds that the view
    /// does not hold is on no local network -- held, in the known set,
    /// and offered nowhere -- whether the view or the bind came first;
    /// a view that names it lifts it, and the listener letting it go
    /// drops it. THE CONTROL is the same listener with no view (the
    /// desktop), offered; beside it a global address outside the view,
    /// AutoNAT's to decide, and a circuit listener through a private
    /// relay, whose IP is the relay's, are offered throughout.
    #[test]
    fn a_bound_private_ip_no_view_holds_is_on_no_local_network_in_either_order() {
        let wifi = "/ip4/192.168.1.5/tcp/4001";
        let unviewed = "/ip4/10.20.30.40/tcp/4001";
        let global = "/ip6/2a00:1450::7/tcp/4001";
        let circuit = "/ip4/10.1.1.1/tcp/4001/p2p/12D3KooWCLxLXFHqvfsHVLDcNsSpZBQq1M1KMRgQRLLLnHTv7oQD/p2p-circuit";
        let carrier = ip("10.20.30.40");
        let bound = addrs(&[wifi, unviewed, global, circuit]);
        let offered = |set: &NetworkSet| -> Vec<String> { set.own_listeners(bound.iter()) };
        let everything: Vec<String> = bound.iter().map(ToString::to_string).collect();
        let but_unviewed: Vec<String> = [wifi, global, circuit].map(str::to_owned).to_vec();

        // THE CONTROL: no view, nothing is outside it.
        let held = crate::held_listeners::HeldListeners::new();
        let mut desktop = NetworkSet::publishing_to(held.clone());
        let _ = desktop.observe_listeners(bound.iter());
        assert!(desktop.off_local().is_empty(), "no view, nothing off-local");
        assert_eq!(offered(&desktop), everything, "the control: offered");
        assert!(held.holds(&bound[1]), "and counted by rule 3");

        // The bind first, then a view that never names it.
        let held = crate::held_listeners::HeldListeners::new();
        let mut set = NetworkSet::publishing_to(held.clone());
        let _ = set.observe_listeners(bound.iter());
        let _ = set.observe_view(&[ip("192.168.1.5"), ip("2a00:1450::7")], &[]);
        assert_eq!(
            set.off_local(),
            &BTreeSet::from([carrier]),
            "bind, then view"
        );
        assert!(set.holds(&bound[1]), "held: still the host's and bound");
        assert_eq!(offered(&set), but_unviewed);
        assert_eq!(set.not_offered(), BTreeSet::from([carrier]), "Identify's");
        assert!(!held.holds(&bound[1]), "no own listener for rule 3");
        assert!(held.holds(&bound[0]), "the Wi-Fi control is");

        // A view naming it lifts it; one dropping it again withholds it.
        let _ = set.observe_view(&[ip("192.168.1.5"), ip("2a00:1450::7"), carrier], &[]);
        assert!(set.off_local().is_empty(), "named by the view: lifted");
        assert_eq!(offered(&set), everything);
        assert!(held.holds(&bound[1]));
        let _ = set.observe_view(&[ip("192.168.1.5"), ip("2a00:1450::7")], &[]);
        assert_eq!(set.off_local(), &BTreeSet::from([carrier]));
        // The listener letting it go drops it.
        let _ = set.observe_listeners(bound.iter().filter(|a| **a != bound[1]));
        assert!(set.off_local().is_empty(), "unbound: dropped");
        assert!(set.not_offered().is_empty());

        // The view first, then the bind: the same answer.
        let held = crate::held_listeners::HeldListeners::new();
        let mut set = NetworkSet::publishing_to(held.clone());
        let _ = set.observe_view(&[ip("192.168.1.5"), ip("2a00:1450::7")], &[]);
        assert!(set.off_local().is_empty(), "nothing bound yet");
        let _ = set.observe_listeners(bound.iter());
        assert_eq!(
            set.off_local(),
            &BTreeSet::from([carrier]),
            "view, then bind"
        );
        assert_eq!(offered(&set), but_unviewed);
        assert!(!held.holds(&bound[1]));
    }

    /// A classified IP no listener binds yet is withheld all the same,
    /// so a listener the poll reports after the view is never told to
    /// Identify; the Wi-Fi listener is the control.
    #[test]
    fn a_classified_ip_is_withheld_before_any_listener_binds_it() {
        let mut set = NetworkSet::default();
        let carrier = ip("10.20.30.40");
        let _ = set.observe_listeners(addrs(&["/ip4/192.168.1.5/tcp/4001"]).iter());
        let _ = set.observe_view(&[ip("192.168.1.5"), carrier], &[carrier]);
        assert_eq!(
            set.not_offered(),
            BTreeSet::from([carrier]),
            "unbound, withheld"
        );
        let _ = set.observe_listeners(
            addrs(&["/ip4/192.168.1.5/tcp/4001", "/ip4/10.20.30.40/tcp/4001"]).iter(),
        );
        assert_eq!(set.not_offered(), BTreeSet::from([carrier]), "bound, still");
    }

    /// A classified IP the view drops while its listener is still bound
    /// (the listener poll lags the view) stays not offered until the
    /// listener lets it go -- what Identify is told -- and a later view
    /// naming it unclassified releases it. The Wi-Fi listener is the
    /// control throughout.
    #[test]
    fn a_departed_off_local_ip_stays_withheld_while_bound() {
        let mut set = NetworkSet::default();
        let (wifi, cellular) = ("/ip4/192.168.1.5/tcp/4001", "/ip4/10.20.30.40/tcp/4001");
        let carrier = ip("10.20.30.40");
        let _ = set.observe_listeners(addrs(&[wifi, cellular]).iter());
        let _ = set.observe_view(&[ip("192.168.1.5"), carrier], &[carrier]);
        assert_eq!(set.not_offered(), BTreeSet::from([carrier]), "classified");

        let _ = set.observe_view(&[ip("192.168.1.5")], &[]);
        assert_eq!(
            set.off_local(),
            &BTreeSet::from([carrier]),
            "unclassified now, and bound outside the view (A 2026-10-27)"
        );
        assert_eq!(
            set.not_offered(),
            BTreeSet::from([carrier]),
            "departed and still bound: still not offered"
        );
        let _ = set.observe_listeners(addrs(&[wifi]).iter());
        assert!(set.not_offered().is_empty(), "the listener let it go");

        // Bound again and named unclassified: offered, as on the desktop.
        let _ = set.observe_listeners(addrs(&[wifi, cellular]).iter());
        let _ = set.observe_view(&[ip("192.168.1.5"), carrier], &[]);
        assert!(
            set.not_offered().is_empty(),
            "released by an unclassified view"
        );
    }

    #[test]
    fn a_network_change_is_a_move_of_the_known_ip_set_after_it_first_fills() {
        let mut set = NetworkSet::default();
        // Nothing bound yet, then the first bind: not a change.
        assert_eq!(set.observe_listeners(addrs(&[]).iter()), None);
        let lan_a = "/ip4/192.168.1.5/tcp/4001";
        assert_eq!(
            set.observe_listeners(addrs(&[lan_a]).iter()),
            None,
            "the first bind"
        );
        assert_eq!(
            set.observe_listeners(addrs(&[lan_a]).iter()),
            None,
            "unchanged"
        );
        // A loopback listener joins: outside the comparison, no change.
        assert_eq!(
            set.observe_listeners(addrs(&[lan_a, "/ip4/127.0.0.1/tcp/4001"]).iter()),
            None
        );
        // A second PORT on the same IP comes and goes: the host is on the
        // same network, so neither is a change.
        let lan_a_quic = "/ip4/192.168.1.5/udp/4001/quic-v1";
        assert_eq!(
            set.observe_listeners(addrs(&[lan_a, lan_a_quic]).iter()),
            None,
            "a second listener on a known IP"
        );
        assert_eq!(
            set.observe_listeners(addrs(&[lan_a_quic]).iter()),
            None,
            "one of two listeners on one IP going"
        );
        // A second IP joins: a change, reported, and it invalidates
        // nothing -- what was known about 192.168.1.5 stands.
        let vpn = "/ip4/10.8.0.2/tcp/4001";
        let joined = set.observe_listeners(addrs(&[lan_a, vpn]).iter());
        assert_eq!(joined, change(&[], &["10.8.0.2"]));
        let joined = joined.expect("a change");
        assert!(!joined.invalidates(), "an addition alone is not a move");
        assert!(joined.adds());
        let left = set
            .observe_listeners(addrs(&[lan_a]).iter())
            .expect("a change");
        assert!(left.invalidates() && !left.adds());
        // Another LAN: the move, and it invalidates.
        let lan_b = "/ip4/10.0.0.7/tcp/4001";
        assert_eq!(
            set.observe_listeners(addrs(&["/ip4/127.0.0.1/tcp/4001", lan_b]).iter()),
            change(&["192.168.1.5"], &["10.0.0.7"])
        );
        // The set empties -- the interface went away -- and fills again:
        // both are changes, since what returns is not known to be what
        // left.
        assert_eq!(
            set.observe_listeners(addrs(&[]).iter()),
            change(&["10.0.0.7"], &[])
        );
        assert_eq!(
            set.observe_listeners(addrs(&[lan_b]).iter()),
            change(&[], &["10.0.0.7"])
        );
        // Duplicates -- two listeners on one address -- are one.
        assert_eq!(set.observe_listeners(addrs(&[lan_b, lan_b]).iter()), None);
    }

    #[test]
    fn the_two_sources_report_one_move_once() {
        let mut set = NetworkSet::default();
        let wifi = "/ip4/192.168.1.5/tcp/0";
        assert_eq!(
            set.observe_listeners(addrs(&[wifi]).iter()),
            None,
            "the first bind"
        );
        // The first view agrees: nothing moved. Its interface-scoped
        // addresses are left out as the listeners' are.
        assert_eq!(
            set.observe_view(&[ip("192.168.1.5"), ip("127.0.0.1"), ip("fe80::1")], &[]),
            None
        );
        // THE HAND-OVER, seen by the view first: reported at once ...
        assert_eq!(
            set.observe_view(&[ip("100.64.3.9")], &[]),
            change(&["192.168.1.5"], &["100.64.3.9"])
        );
        // ... and the listener poll that sees the same move later finds
        // nothing left to report.
        assert_eq!(
            set.observe_listeners(addrs(&["/ip4/100.64.3.9/tcp/0"]).iter()),
            None,
            "the late poll"
        );
        // The same view twice is no change.
        assert_eq!(
            set.observe_view(&[ip("100.64.3.9")], &[]),
            None,
            "idempotent"
        );
        // OFFLINE: an empty view removes everything it named.
        assert_eq!(set.observe_view(&[], &[]), change(&["100.64.3.9"], &[]));
        assert_eq!(
            set.observe_listeners(addrs(&[]).iter()),
            None,
            "the late poll"
        );
        // Back, seen by the listeners first this time.
        assert_eq!(
            set.observe_listeners(addrs(&[wifi]).iter()),
            change(&[], &["192.168.1.5"])
        );
        assert_eq!(
            set.observe_view(&[ip("192.168.1.5")], &[]),
            None,
            "the late view"
        );
        // A lasting disagreement -- an address only the view names --
        // is reported once and not again.
        assert_eq!(
            set.observe_view(&[ip("192.168.1.5"), ip("10.8.0.2")], &[]),
            change(&[], &["10.8.0.2"])
        );
        assert_eq!(set.observe_listeners(addrs(&[wifi]).iter()), None);
        assert_eq!(set.observe_listeners(addrs(&[wifi]).iter()), None);
    }

    #[test]
    fn a_first_view_before_any_bind_fills_the_set_and_is_not_a_change() {
        let mut set = NetworkSet::default();
        assert_eq!(
            set.observe_view(&[ip("192.168.1.5")], &[]),
            None,
            "the first fill"
        );
        assert_eq!(
            set.observe_listeners(addrs(&["/ip4/192.168.1.5/tcp/0"]).iter()),
            None,
            "the listener binds what the view already said"
        );
        assert_eq!(
            set.observe_view(&[], &[]),
            change(&["192.168.1.5"], &[]),
            "and the set it filled moves"
        );
    }

    #[test]
    fn the_first_fill_is_no_change_and_its_lift_is_taken_once() {
        let mut set = NetworkSet::default();
        // Offline at start: loopback only, and an empty first view.
        assert_eq!(
            set.observe_listeners(addrs(&["/ip4/127.0.0.1/tcp/0"]).iter()),
            None
        );
        assert_eq!(set.observe_view(&[], &[]), None);
        assert!(!set.take_filled(), "nothing has filled");
        // Online: the first fill, reported as no change ...
        assert_eq!(set.observe_view(&[ip("192.168.1.5")], &[]), None);
        // ... whose lift is taken once.
        assert!(set.take_filled());
        assert!(!set.take_filled(), "once");
        // A later addition is a change, and no fill.
        assert_eq!(
            set.observe_view(&[ip("192.168.1.5"), ip("10.8.0.2")], &[]),
            change(&[], &["10.8.0.2"])
        );
        assert!(!set.take_filled());
        // Emptied and refilled: a change each way, never a first fill.
        let _ = set.observe_view(&[], &[]);
        assert!(set.observe_view(&[ip("192.168.1.5")], &[]).is_some());
        assert!(!set.take_filled());
    }

    #[test]
    fn the_view_once_present_holds_what_it_names_against_the_listeners() {
        let mut set = NetworkSet::default();
        let x = "/ip4/192.168.1.5/tcp/0";
        let _ = set.observe_listeners(addrs(&[x]).iter());
        assert_eq!(set.observe_view(&[ip("192.168.1.5")], &[]), None);
        // The listener drops X while the view still holds it: no
        // removal, and X is still held.
        assert_eq!(set.observe_listeners(addrs(&[]).iter()), None);
        assert!(set.holds(&x.parse().expect("valid")));
        // The view then drops X: that is the removal.
        assert_eq!(set.observe_view(&[], &[]), change(&["192.168.1.5"], &[]));
        // The control: an address the view never named is removed by
        // the listeners as ever.
        let y = "/ip4/10.0.0.7/tcp/0";
        assert_eq!(
            set.observe_listeners(addrs(&[y]).iter()),
            change(&[], &["10.0.0.7"])
        );
        assert_eq!(
            set.observe_listeners(addrs(&[]).iter()),
            change(&["10.0.0.7"], &[])
        );
    }

    #[test]
    fn a_listener_is_held_while_its_ip_is_known() {
        let mut set = NetworkSet::default();
        let x: Multiaddr = "/ip4/192.168.1.5/tcp/4001".parse().expect("valid");
        let _ = set.observe_listeners(std::iter::once(&x));
        assert!(set.holds(&x));
        let _ = set.observe_view(&[ip("192.168.1.5")], &[]);
        let _ = set.observe_view(&[], &[]);
        assert!(!set.holds(&x), "the view said it departed");
        for always in [
            "/ip4/127.0.0.1/tcp/1",
            "/ip6/fe80::1/tcp/1",
            "/dns4/example.invalid/tcp/1",
        ] {
            assert!(set.holds(&always.parse().expect("valid")), "{always}");
        }
    }

    #[test]
    fn the_detector_publishes_the_bound_ips_it_no_longer_holds() {
        let held = crate::held_listeners::HeldListeners::new();
        let mut set = NetworkSet::publishing_to(held.clone());
        let x: Multiaddr = "/ip4/192.168.1.5/tcp/4001".parse().expect("valid");
        let loopback: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().expect("valid");
        let bound = [x.clone(), loopback.clone()];
        let _ = set.observe_listeners(bound.iter());
        let _ = set.observe_view(&[ip("192.168.1.5")], &[]);
        assert!(held.holds(&x), "the control: held");
        assert_eq!(
            set.own_listeners(bound.iter()),
            vec![x.to_string(), loopback.to_string()]
        );
        // The view says X departed; its listener is still bound.
        let _ = set.observe_view(&[], &[]);
        assert!(!held.holds(&x), "published as no longer held");
        assert_eq!(
            set.own_listeners(bound.iter()),
            vec![loopback.to_string()],
            "and not this node's for rule 3"
        );
        // The poll catches up: X is no longer bound, nothing is unheld.
        let _ = set.observe_listeners(std::iter::once(&loopback));
        assert!(held.holds(&x), "unheld means bound and not held");
    }

    /// The module note's "bounded spurious pair, never a stuck set": the
    /// VIEW lagging the listeners across two moves -- X gone and back on
    /// the listeners before the view saw it go -- reports a move that
    /// already reversed, and the next view reverses it again; X ends held.
    #[test]
    fn a_view_lagging_two_moves_makes_one_spurious_pair_and_ends_held() {
        let mut set = NetworkSet::default();
        let x: Multiaddr = "/ip4/192.168.1.5/tcp/4001".parse().expect("valid");
        let _ = set.observe_listeners(std::iter::once(&x));
        let _ = set.observe_view(&[ip("192.168.1.5")], &[]);
        // The listeners lose X and regain it: the view holds X, so no
        // change either way.
        assert_eq!(set.observe_listeners(std::iter::empty()), None);
        assert_eq!(set.observe_listeners(std::iter::once(&x)), None);
        // The late view drops X: the spurious removal ...
        assert_eq!(set.observe_view(&[], &[]), change(&["192.168.1.5"], &[]));
        // ... and the next view names it again: the spurious addition.
        assert_eq!(
            set.observe_view(&[ip("192.168.1.5")], &[]),
            change(&[], &["192.168.1.5"])
        );
        assert!(set.holds(&x), "never a stuck set");
        assert_eq!(
            set.observe_view(&[ip("192.168.1.5")], &[]),
            None,
            "and quiet"
        );
    }

    #[test]
    fn a_first_view_removes_nothing_it_never_named() {
        let mut set = NetworkSet::default();
        let _ = set.observe_listeners(addrs(&["/ip4/192.168.1.5/tcp/0"]).iter());
        // The platform's first word omits what the listeners bound: it
        // adds what it names, and says nothing of the rest -- before it,
        // the platform's view was unknown, not empty.
        assert_eq!(
            set.observe_view(&[ip("10.8.0.2")], &[]),
            change(&[], &["10.8.0.2"])
        );
        // Its second word is compared with its first: dropping what it
        // named is a removal, and what it never named still stands.
        assert_eq!(set.observe_view(&[], &[]), change(&["10.8.0.2"], &[]));
    }

    #[test]
    fn a_removal_closes_a_departed_ip_and_an_unknown_direct_one_only() {
        let lan: IpAddr = "192.168.1.5".parse().expect("ip");
        let other: IpAddr = "10.0.0.7".parse().expect("ip");
        let departed = BTreeSet::from([lan]);
        let none = BTreeSet::new();
        assert!(closes(Some(lan), PeerPath::Direct, &departed));
        assert!(
            !closes(Some(other), PeerPath::Direct, &departed),
            "still bound"
        );
        assert!(
            closes(None, PeerPath::Direct, &departed),
            "a named dial, on any departure"
        );
        assert!(
            !closes(None, PeerPath::Direct, &none),
            "an addition departs nothing"
        );
        assert!(
            !closes(None, PeerPath::Relayed, &departed),
            "a circuit follows its relay"
        );
    }

    #[test]
    fn a_connections_local_ip_is_its_listener_address_or_the_route_toward_its_remote() {
        let inbound = ConnectedPoint::Listener {
            local_addr: "/ip4/192.168.1.5/tcp/4001".parse().expect("valid"),
            send_back_addr: "/ip4/192.168.1.9/tcp/5555".parse().expect("valid"),
        };
        assert_eq!(
            local_ip_of(&inbound),
            Some("192.168.1.5".parse().expect("ip"))
        );
        // Toward loopback the kernel's route runs from loopback.
        let outbound = ConnectedPoint::Dialer {
            address: "/ip4/127.0.0.1/tcp/4001".parse().expect("valid"),
            role_override: libp2p::core::Endpoint::Dialer,
            port_use: libp2p::core::transport::PortUse::Reuse,
        };
        assert_eq!(
            local_ip_of(&outbound),
            Some("127.0.0.1".parse().expect("ip"))
        );
        // AND TOWARD A REMOTE ELSEWHERE, where the two differ: what is
        // recorded is this host's source, never the remote's own IP --
        // `None` where no route exists, an address of this host where one
        // does (#129 review F4: toward loopback the two coincide and a
        // mutation reading the remote passed).
        let remote: IpAddr = "192.0.2.1".parse().expect("ip");
        let elsewhere = ConnectedPoint::Dialer {
            address: "/ip4/192.0.2.1/tcp/4001".parse().expect("valid"),
            role_override: libp2p::core::Endpoint::Dialer,
            port_use: libp2p::core::transport::PortUse::Reuse,
        };
        assert_ne!(local_ip_of(&elsewhere), Some(remote));
        // A circuit and a name are not known.
        let circuit = ConnectedPoint::Dialer {
            address: "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/p2p-circuit"
                .parse()
                .expect("valid"),
            role_override: libp2p::core::Endpoint::Dialer,
            port_use: libp2p::core::transport::PortUse::Reuse,
        };
        assert_eq!(local_ip_of(&circuit), None);
        let named = ConnectedPoint::Dialer {
            address: "/dns4/example.invalid/tcp/4001".parse().expect("valid"),
            role_override: libp2p::core::Endpoint::Dialer,
            port_use: libp2p::core::transport::PortUse::Reuse,
        };
        assert_eq!(local_ip_of(&named), None);
    }

    #[test]
    fn interface_scoped_addresses_and_only_those_are_left_out_of_the_comparison() {
        for scoped in [
            "/ip4/127.0.0.1/tcp/1",
            "/ip4/0.0.0.0/tcp/1",
            "/ip4/169.254.1.1/tcp/1",
            "/ip6/::1/tcp/1",
            "/ip6/::/tcp/1",
            "/ip6/fe80::1/tcp/1",
        ] {
            assert!(
                is_interface_scoped(&scoped.parse().expect("a literal")),
                "{scoped}"
            );
        }
        for counted in [
            "/ip4/192.168.1.5/tcp/1",
            "/ip4/10.0.0.7/tcp/1",
            "/ip4/100.64.0.1/tcp/1",
            "/ip4/8.8.8.8/tcp/1",
            "/ip6/fd12::1/tcp/1",
            "/ip6/2001:4860:4860::8888/tcp/1",
            "/dns4/example.invalid/tcp/1",
        ] {
            assert!(
                !is_interface_scoped(&counted.parse().expect("a literal")),
                "{counted}"
            );
        }
    }
}

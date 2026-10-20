// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The platform's networks, as the runtime is told them (plan section 20
//! step 5; human-client-android.md, "Android network lifecycle").
//!
//! The Service's `ConnectivityManager` callbacks report one network at a
//! time -- its addresses and DNS facts as its `LinkProperties` change,
//! then its loss -- and which network is the default, and the runtime
//! takes one thing ([`NetworkView`]): a SNAPSHOT of every address of every
//! usable network, empty when there is none, with the ACTIVE (default)
//! network's resolvers and its strict-mode Private DNS host
//! (`CONNECTIVITY.md` section 14, A 2026-10-14). The snapshot is made here
//! rather than in Kotlin so it is tested on the host. A network's handle
//! is the bridge's key and stops here: the view carries local facts alone
//! (ADR-0001's layering).
//!
//! The runtime is told only when the snapshot -- addresses or DNS facts --
//! differs from the last one it was told, and a runtime that starts is told the current one at once:
//! the callback reported every network when it was registered, before
//! that runtime existed, and does not report them again. The tests below:
//! `a_report_that_leaves_the_addresses_as_they_were_tells_the_runtime_nothing`,
//! `the_dns_facts_are_the_default_networks_and_unknown_without_one` and
//! `a_runtime_that_starts_hears_the_networks_reported_before_it_existed`.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::Arc;

use interweave_transport_embedded::{EmbeddedHost, NetworkView};

/// What takes a view: the embedded host, or a test's recorder.
pub trait NetworkSink {
    /// Take the platform's current snapshot. Must not wait.
    fn network_changed(&self, view: NetworkView);
}

impl NetworkSink for EmbeddedHost {
    fn network_changed(&self, view: NetworkView) {
        Self::network_changed(self, view);
    }
}

impl<T: NetworkSink> NetworkSink for Arc<T> {
    fn network_changed(&self, view: NetworkView) {
        T::network_changed(self, view);
    }
}

/// One network as the callback reports it, the whole of it each time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Link {
    /// Every address it has.
    pub addresses: Vec<IpAddr>,
    /// Its resolvers, in the platform's order (`getDnsServers`).
    pub dns_servers: Vec<IpAddr>,
    /// Its Private DNS host name when Private DNS is in strict mode on it,
    /// else `None`.
    pub private_dns: Option<String>,
}

/// Every network the callback reports usable, by its handle, which of
/// them is the default, and the snapshot they make.
#[derive(Debug, Default)]
pub struct Links {
    /// `None` while no callback is registered: the platform's view is
    /// unknown, which is not the same as offline.
    links: Option<BTreeMap<u64, Link>>,
    /// The default network's handle, as the default-network callback
    /// last named it.
    default: Option<u64>,
}

impl Links {
    /// The snapshot: every address of every network, sorted, a duplicate
    /// once, and the default network's DNS facts. Its resolvers are
    /// UNOBSERVED (`None`, the runtime keeps the resolver it has) while no
    /// default is named, or the default has not reported a link with
    /// addresses -- not yet, or lost; a default with addresses gives its
    /// list, and an empty one is UNAVAILABLE, which the runtime acts on
    /// (`CONNECTIVITY.md` section 14, A 2026-10-20). `None` while no
    /// callback is registered.
    #[must_use]
    pub fn view(&self) -> Option<NetworkView> {
        let links = self.links.as_ref()?;
        let mut addresses: Vec<IpAddr> = links
            .values()
            .flat_map(|link| link.addresses.iter().copied())
            .collect();
        addresses.sort_unstable();
        addresses.dedup();
        let active = self.default.and_then(|network| links.get(&network));
        let view = NetworkView::new(addresses)
            .with_private_dns(active.and_then(|l| l.private_dns.clone()));
        Some(match active.filter(|l| !l.addresses.is_empty()) {
            Some(observed) => view.with_dns_servers(observed.dns_servers.clone()),
            None => view,
        })
    }

    /// The callback was registered: no network is known yet, which is
    /// offline until it reports one.
    fn watch(&mut self) {
        self.links = Some(BTreeMap::new());
        self.default = None;
    }

    /// The callback was unregistered: the view is unknown again.
    fn unwatch(&mut self) {
        self.links = None;
        self.default = None;
    }

    /// `network` is now `link`, the whole of it. Ignored while no
    /// callback is registered: a report racing the unregistration says
    /// nothing about a later registration.
    fn link(&mut self, network: u64, link: Link) {
        if let Some(links) = self.links.as_mut() {
            links.insert(network, link);
        }
    }

    /// The default network is now `network`, or there is none.
    fn default_is(&mut self, network: Option<u64>) {
        if self.links.is_some() {
            self.default = network;
        }
    }

    /// `network` is gone.
    fn lost(&mut self, network: u64) {
        if let Some(links) = self.links.as_mut() {
            links.remove(&network);
        }
    }
}

/// A network's addresses as the bridge passes them across JNI: each one
/// its length (4 or 16) then its bytes in network order, as Java's
/// `InetAddress.getAddress` gives them -- no scope, no text to parse. A
/// length that is neither, or bytes cut short, ends the reading there:
/// what came before it stands, nothing after it is guessed at.
#[must_use]
pub fn addresses_from_wire(mut wire: &[u8]) -> Vec<IpAddr> {
    let mut addresses = Vec::new();
    while let Some((&len, rest)) = wire.split_first() {
        let address = match (len, rest) {
            (4, [a, b, c, d, ..]) => IpAddr::from([*a, *b, *c, *d]),
            (16, rest) if rest.len() >= 16 => {
                let mut octets = [0_u8; 16];
                octets.copy_from_slice(&rest[..16]);
                IpAddr::from(octets)
            }
            _ => break,
        };
        addresses.push(address);
        wire = &rest[usize::from(len)..];
    }
    addresses
}

/// The callback's reports and the runtime that hears them, under one lock
/// of their own: never the start's, which a start holds for seconds while
/// the callback must not wait (`tests/service.rs`:
/// `a_network_report_never_waits_on_a_start`).
pub struct NetworkSlot<S> {
    links: Links,
    /// What the attached runtime was last told.
    told: Option<NetworkView>,
    sink: Option<S>,
}

impl<S> Default for NetworkSlot<S> {
    fn default() -> Self {
        Self {
            links: Links::default(),
            told: None,
            sink: None,
        }
    }
}

/// What the runtime was told, by count: a log of it names no address,
/// resolver or host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Told {
    /// How many addresses.
    pub addresses: usize,
    /// How many resolvers: `None` when they are unobserved, and
    /// `Some(0)` when the default network names none, which is unavailable.
    pub dns_servers: Option<usize>,
    /// Whether a strict-mode Private DNS host was named.
    pub strict_private_dns: bool,
}

impl Told {
    fn of(view: &NetworkView) -> Self {
        Self {
            addresses: view.addresses.len(),
            dns_servers: view.dns_servers.as_ref().map(Vec::len),
            strict_private_dns: view.private_dns.is_some(),
        }
    }
}

/// One report from the platform's callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkReport {
    /// The callback was registered.
    Watching,
    /// A network, the whole of it.
    Link {
        /// The platform's handle for the network.
        network: u64,
        /// What it has now.
        link: Link,
    },
    /// Which network is the default, if any.
    Default {
        /// The platform's handle for it; `None` when there is none.
        network: Option<u64>,
    },
    /// A network is gone.
    Lost {
        /// The platform's handle for the network.
        network: u64,
    },
    /// The callback was unregistered.
    Unwatched,
}

impl<S: NetworkSink> NetworkSlot<S> {
    /// Take one report, and tell the attached runtime when the snapshot
    /// moved: what it was told, by count, `None` when it was told nothing.
    pub fn report(&mut self, report: NetworkReport) -> Option<Told> {
        match report {
            NetworkReport::Watching => self.links.watch(),
            NetworkReport::Link { network, link } => self.links.link(network, link),
            NetworkReport::Default { network } => self.links.default_is(network),
            NetworkReport::Lost { network } => self.links.lost(network),
            NetworkReport::Unwatched => self.links.unwatch(),
        }
        self.tell()
    }

    /// A runtime started: it hears the current snapshot now, if one is
    /// known, and every change after.
    pub fn attach(&mut self, sink: S) {
        self.sink = Some(sink);
        self.told = None;
        let _ = self.tell();
    }

    /// What the attached runtime was last told, by count; `None` when it
    /// was told nothing yet, or none is attached.
    #[must_use]
    pub fn told(&self) -> Option<Told> {
        self.told.as_ref().map(Told::of)
    }

    /// The runtime stops: it hears nothing more, and the slot lets go of
    /// it so its stop is not held up by this hold.
    pub fn detach(&mut self) -> Option<S> {
        self.told = None;
        self.sink.take()
    }

    /// The snapshot, to the attached runtime, unless it was told it last.
    /// An unknown view is never sent: an unregistered callback is not
    /// offline, and the runtime keeps the last view it had.
    fn tell(&mut self) -> Option<Told> {
        let (Some(sink), Some(view)) = (self.sink.as_ref(), self.links.view()) else {
            return None;
        };
        if self.told.as_ref() == Some(&view) {
            return None;
        }
        let told = Told::of(&view);
        sink.network_changed(view.clone());
        self.told = Some(view);
        Some(told)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Default)]
    struct Heard(Rc<RefCell<Vec<NetworkView>>>);

    impl NetworkSink for Heard {
        fn network_changed(&self, view: NetworkView) {
            self.0.borrow_mut().push(view);
        }
    }

    impl Heard {
        fn take(&self) -> Vec<Vec<IpAddr>> {
            self.0.borrow_mut().drain(..).map(|v| v.addresses).collect()
        }
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().expect("an address")
    }

    fn link(network: u64, addresses: &[&str]) -> NetworkReport {
        NetworkReport::Link {
            network,
            link: Link {
                addresses: addresses.iter().map(|a| ip(a)).collect(),
                ..Link::default()
            },
        }
    }

    fn with_dns(
        network: u64,
        addresses: &[&str],
        dns: &[&str],
        strict: Option<&str>,
    ) -> NetworkReport {
        NetworkReport::Link {
            network,
            link: Link {
                addresses: addresses.iter().map(|a| ip(a)).collect(),
                dns_servers: dns.iter().map(|a| ip(a)).collect(),
                private_dns: strict.map(str::to_owned),
            },
        }
    }

    fn default_is(network: u64) -> NetworkReport {
        NetworkReport::Default {
            network: Some(network),
        }
    }

    fn count(addresses: usize) -> Told {
        Told {
            addresses,
            dns_servers: None,
            strict_private_dns: false,
        }
    }

    fn attached() -> (NetworkSlot<Heard>, Heard) {
        let heard = Heard::default();
        let mut slot = NetworkSlot::default();
        slot.attach(heard.clone());
        (slot, heard)
    }

    #[test]
    fn the_dns_facts_are_the_default_networks_and_unknown_without_one() {
        let heard = Heard::default();
        let mut slot = NetworkSlot::default();
        slot.attach(heard.clone());
        let _ = slot.report(NetworkReport::Watching);
        let _ = slot.report(with_dns(1, &["10.0.0.5"], &["10.0.0.1"], None));
        let _ = slot.report(with_dns(
            2,
            &["10.115.0.9"],
            &["10.115.0.1", "10.115.0.2"],
            Some("dns.example"),
        ));
        let views = heard.0.borrow_mut().drain(..).collect::<Vec<_>>();
        assert!(
            views
                .iter()
                .all(|v| v.dns_servers.is_none() && v.private_dns.is_none()),
            "no default named: the DNS facts are unknown, {views:?}"
        );

        let _ = slot.report(default_is(2));
        let view = heard
            .0
            .borrow_mut()
            .pop()
            .expect("the default named: a new view");
        assert_eq!(
            view.addresses,
            vec![ip("10.0.0.5"), ip("10.115.0.9")],
            "every network's addresses"
        );
        assert_eq!(
            view.dns_servers,
            Some(vec![ip("10.115.0.1"), ip("10.115.0.2")]),
            "the default's, in order"
        );
        assert_eq!(view.private_dns.as_deref(), Some("dns.example"));

        // A DNS-only change on the default is told; one on another network
        // is not, since it changes nothing the view carries.
        assert_eq!(
            slot.report(with_dns(2, &["10.115.0.9"], &["10.115.0.3"], None)),
            Some(Told {
                addresses: 2,
                dns_servers: Some(1),
                strict_private_dns: false
            })
        );
        assert_eq!(
            slot.report(with_dns(1, &["10.0.0.5"], &["10.0.0.7"], None)),
            None
        );

        // The default lost: unknown DNS again, the other network's
        // addresses kept.
        let _ = slot.report(NetworkReport::Lost { network: 2 });
        let _ = slot.report(NetworkReport::Default { network: None });
        let view = heard.0.borrow_mut().pop().expect("a view");
        assert_eq!(view.addresses, vec![ip("10.0.0.5")]);
        assert!(view.dns_servers.is_none() && view.private_dns.is_none());
    }

    /// `CONNECTIVITY.md` section 14 (A 2026-10-20): an empty DNS view is
    /// either unobserved or unavailable, and the aggregator says which.
    /// Unobserved with no default named and with the default lost;
    /// unavailable for a default with addresses that names no resolver.
    #[test]
    fn dns_is_unobserved_without_a_default_and_unavailable_for_a_default_naming_none() {
        let (mut slot, heard) = attached();
        let _ = slot.report(NetworkReport::Watching);
        let _ = slot.report(link(1, &["10.0.0.5"]));
        let view = heard.0.borrow_mut().pop().expect("a view");
        assert_eq!(view.dns_servers, None, "no default named: unobserved");

        // A default named before its link is reported: still unobserved.
        let _ = slot.report(default_is(2));
        let view = heard.0.borrow_mut().pop().expect("a view");
        assert_eq!(view.dns_servers, None, "a default not reported: unobserved");

        // Its link reported with no address yet: still unobserved, though
        // it names a resolver.
        let _ = slot.report(with_dns(2, &[], &["10.115.0.1"], None));
        if let Some(view) = heard.0.borrow_mut().pop() {
            assert_eq!(
                view.dns_servers, None,
                "a default with no address: unobserved"
            );
        }

        assert_eq!(
            slot.report(link(2, &["10.115.0.9"])),
            Some(Told {
                addresses: 2,
                dns_servers: Some(0),
                strict_private_dns: false
            }),
            "a default with addresses and no resolver: unavailable"
        );
        let view = heard.0.borrow_mut().pop().expect("a view");
        assert_eq!(view.dns_servers, Some(Vec::new()));

        let _ = slot.report(NetworkReport::Lost { network: 2 });
        let view = heard.0.borrow_mut().pop().expect("a view");
        assert_eq!(view.dns_servers, None, "the default lost: unobserved");
    }

    #[test]
    fn the_wire_gives_each_address_by_its_length_and_stops_at_a_malformed_one() {
        let v6 = ip("2001:db8::7");
        let IpAddr::V6(v6_octets) = v6 else {
            unreachable!()
        };
        let mut wire = vec![4, 10, 0, 0, 5, 16];
        wire.extend_from_slice(&v6_octets.octets());
        assert_eq!(addresses_from_wire(&wire), vec![ip("10.0.0.5"), v6]);
        assert!(
            addresses_from_wire(&[]).is_empty(),
            "no address: offline for that network"
        );

        let mut cut = wire.clone();
        cut.truncate(wire.len() - 1);
        assert_eq!(
            addresses_from_wire(&cut),
            vec![ip("10.0.0.5")],
            "a cut v6 is dropped"
        );
        assert_eq!(
            addresses_from_wire(&[4, 10, 0, 0, 5, 6, 1, 2, 3, 4, 5, 6, 4, 10, 0, 0, 6]),
            vec![ip("10.0.0.5")],
            "an unknown length ends the reading: nothing after it is guessed"
        );
    }

    #[test]
    fn the_view_is_every_address_of_every_network_once_and_sorted() {
        let (mut slot, heard) = attached();
        let _ = slot.report(NetworkReport::Watching);
        let _ = slot.report(link(7, &["192.168.1.20", "fe80::1"]));
        let _ = slot.report(link(9, &["10.0.0.5", "192.168.1.20"]));
        assert_eq!(
            heard.take().last().cloned(),
            Some(vec![ip("10.0.0.5"), ip("192.168.1.20"), ip("fe80::1")]),
            "the union of both networks, the shared address once"
        );
    }

    #[test]
    fn a_registered_callback_with_no_network_is_offline_and_a_lost_last_network_is_too() {
        let (mut slot, heard) = attached();
        assert_eq!(slot.report(NetworkReport::Watching), Some(count(0)));
        assert_eq!(
            heard.take(),
            vec![Vec::<IpAddr>::new()],
            "registered, nothing yet: offline"
        );
        assert_eq!(slot.report(link(3, &["10.0.0.5"])), Some(count(1)));
        assert_eq!(
            slot.report(link(3, &["10.0.0.5"])),
            None,
            "told nothing new"
        );
        assert_eq!(
            slot.report(NetworkReport::Lost { network: 3 }),
            Some(count(0))
        );
        assert_eq!(heard.take(), vec![vec![ip("10.0.0.5")], vec![]]);
    }

    #[test]
    fn a_report_that_leaves_the_addresses_as_they_were_tells_the_runtime_nothing() {
        let (mut slot, heard) = attached();
        let _ = slot.report(NetworkReport::Watching);
        let _ = slot.report(link(1, &["10.0.0.5", "10.0.0.6"]));
        heard.take();
        // A Wi-Fi reconnect with the same lease: a new handle, the same
        // addresses, reported in another order.
        let _ = slot.report(link(2, &["10.0.0.6", "10.0.0.5"]));
        let _ = slot.report(NetworkReport::Lost { network: 1 });
        let _ = slot.report(link(2, &["10.0.0.5", "10.0.0.6"]));
        assert!(heard.take().is_empty(), "the addresses never changed");
    }

    #[test]
    fn a_runtime_that_starts_hears_the_networks_reported_before_it_existed() {
        let mut slot = NetworkSlot::default();
        let _ = slot.report(NetworkReport::Watching);
        let _ = slot.report(link(4, &["10.0.0.5"]));
        let heard = Heard::default();
        assert_eq!(slot.told(), None);
        slot.attach(heard.clone());
        assert_eq!(heard.take(), vec![vec![ip("10.0.0.5")]]);
        assert_eq!(
            slot.told(),
            Some(count(1)),
            "what the start told is readable"
        );
        let _ = slot.detach();
        assert_eq!(slot.told(), None);
    }

    #[test]
    fn a_restarted_runtime_hears_the_view_even_when_its_predecessor_was_told_it() {
        let (mut slot, first) = attached();
        let _ = slot.report(NetworkReport::Watching);
        let _ = slot.report(link(4, &["10.0.0.5"]));
        assert_eq!(first.take(), vec![vec![], vec![ip("10.0.0.5")]]);
        assert!(slot.detach().is_some());
        let second = Heard::default();
        slot.attach(second.clone());
        assert_eq!(second.take(), vec![vec![ip("10.0.0.5")]]);
    }

    #[test]
    fn a_detached_slot_holds_no_runtime_and_tells_none() {
        let (mut slot, heard) = attached();
        let _ = slot.report(NetworkReport::Watching);
        heard.take();
        let held = slot.detach();
        assert!(held.is_some());
        assert!(slot.detach().is_none(), "nothing kept after the detach");
        let _ = slot.report(link(5, &["10.0.0.5"]));
        assert!(heard.take().is_empty());
    }

    #[test]
    fn an_unknown_view_is_never_sent_as_offline() {
        let (mut slot, heard) = attached();
        // Before registration, and a report racing the unregistration.
        let _ = slot.report(link(1, &["10.0.0.5"]));
        assert!(heard.take().is_empty(), "no callback registered: unknown");
        let _ = slot.report(NetworkReport::Watching);
        let _ = slot.report(link(1, &["10.0.0.5"]));
        heard.take();
        let _ = slot.report(NetworkReport::Unwatched);
        let _ = slot.report(NetworkReport::Lost { network: 1 });
        assert!(
            heard.take().is_empty(),
            "unregistered is unknown, not offline"
        );
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The platform's networks, as the runtime is told them (plan section 20
//! step 5; human-client-android.md, "Android network lifecycle").
//!
//! The Service's `ConnectivityManager` callback reports one network at a
//! time -- its addresses as its `LinkProperties` change, then its loss --
//! and the runtime takes one thing: a SNAPSHOT of every address of every
//! usable network, empty when there is none ([`NetworkView`]). The union
//! is made here rather than in Kotlin so it is tested on the host. A
//! network's handle is the bridge's key and stops here: the view carries
//! addresses alone (ADR-0001's layering).
//!
//! The runtime is told only when the snapshot differs from the last one
//! it was told, and a runtime that starts is told the current one at once:
//! the callback reported every network when it was registered, before
//! that runtime existed, and does not report them again.

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

/// Every network the callback reports usable, by its handle, and the
/// snapshot they make.
#[derive(Debug, Default)]
pub struct Links {
    /// `None` while no callback is registered: the platform's view is
    /// unknown, which is not the same as offline.
    links: Option<BTreeMap<u64, Vec<IpAddr>>>,
}

impl Links {
    /// The snapshot: every address of every network, sorted, a duplicate
    /// once. `None` while no callback is registered.
    #[must_use]
    pub fn view(&self) -> Option<NetworkView> {
        let links = self.links.as_ref()?;
        let mut addresses: Vec<IpAddr> = links.values().flatten().copied().collect();
        addresses.sort_unstable();
        addresses.dedup();
        Some(NetworkView { addresses })
    }

    /// The callback was registered: no network is known yet, which is
    /// offline until it reports one.
    fn watch(&mut self) {
        self.links = Some(BTreeMap::new());
    }

    /// The callback was unregistered: the view is unknown again.
    fn unwatch(&mut self) {
        self.links = None;
    }

    /// `network`'s addresses are now `addresses`, the whole set. Ignored
    /// while no callback is registered: a report racing the
    /// unregistration says nothing about a later registration.
    fn link(&mut self, network: u64, addresses: Vec<IpAddr>) {
        if let Some(links) = self.links.as_mut() {
            links.insert(network, addresses);
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
/// the callback must not wait.
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

/// One report from the platform's callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkReport {
    /// The callback was registered.
    Watching,
    /// A network's addresses, the whole set.
    Link {
        /// The platform's handle for the network.
        network: u64,
        /// Every address it has now.
        addresses: Vec<IpAddr>,
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
    /// moved: how many addresses it was told, `None` when it was told
    /// nothing. A count, so a log of it names no address.
    pub fn report(&mut self, report: NetworkReport) -> Option<usize> {
        match report {
            NetworkReport::Watching => self.links.watch(),
            NetworkReport::Link { network, addresses } => self.links.link(network, addresses),
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

    /// How many addresses the attached runtime was last told; `None`
    /// when it was told nothing yet, or none is attached.
    #[must_use]
    pub fn told(&self) -> Option<usize> {
        self.told.as_ref().map(|view| view.addresses.len())
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
    fn tell(&mut self) -> Option<usize> {
        let (Some(sink), Some(view)) = (self.sink.as_ref(), self.links.view()) else {
            return None;
        };
        if self.told.as_ref() == Some(&view) {
            return None;
        }
        let count = view.addresses.len();
        sink.network_changed(view.clone());
        self.told = Some(view);
        Some(count)
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
            addresses: addresses.iter().map(|a| ip(a)).collect(),
        }
    }

    fn attached() -> (NetworkSlot<Heard>, Heard) {
        let heard = Heard::default();
        let mut slot = NetworkSlot::default();
        slot.attach(heard.clone());
        (slot, heard)
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
        assert_eq!(slot.report(NetworkReport::Watching), Some(0));
        assert_eq!(
            heard.take(),
            vec![Vec::<IpAddr>::new()],
            "registered, nothing yet: offline"
        );
        assert_eq!(slot.report(link(3, &["10.0.0.5"])), Some(1));
        assert_eq!(
            slot.report(link(3, &["10.0.0.5"])),
            None,
            "told nothing new"
        );
        assert_eq!(slot.report(NetworkReport::Lost { network: 3 }), Some(0));
        assert_eq!(heard.take(), vec![vec![ip("10.0.0.5")], vec![]]);
    }

    #[test]
    fn a_report_that_leaves_the_addresses_as_they_were_tells_the_runtime_nothing() {
        let (mut slot, heard) = attached();
        let _ = slot.report(NetworkReport::Watching);
        let _ = slot.report(link(1, &["10.0.0.5"]));
        heard.take();
        // A Wi-Fi reconnect with the same lease: a new handle, the same
        // address; and the same set in another order.
        let _ = slot.report(link(2, &["10.0.0.5"]));
        let _ = slot.report(NetworkReport::Lost { network: 1 });
        let _ = slot.report(link(2, &["10.0.0.5"]));
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
        assert_eq!(slot.told(), Some(1), "what the start told is readable");
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

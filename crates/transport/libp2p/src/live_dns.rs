// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The DNS transport, with a resolver the runtime replaces while the
//! Swarm runs (`CONNECTIVITY.md` §14, A 2026-10-14).
//!
//! libp2p's builder takes the resolver configuration once
//! (`with_dns_config`), and `libp2p-dns 0.45.0` gives no way to change
//! it afterwards: its `Transport` keeps the resolver in a private field,
//! and its only constructors build a hickory `TokioResolver` from a
//! configuration (`src/lib.rs:79`). On Android the host read is gated off
//! (#277), so the configuration has to come later, from the platform's
//! view, and change when the network does.
//!
//! So this wrapper owns the base transport behind a shared handle
//! ([`Shared`]) and holds one `libp2p_dns::tokio::Transport` over it. A
//! new configuration, handed in through [`ResolverSwitch`], rebuilds that
//! DNS transport over the SAME shared base: listeners, their events and
//! the base's state are untouched, and only the resolver changes. A
//! lookup already in flight owns a clone of the old resolver and finishes
//! on it; the next dial uses the new one.
//!
//! The DNS transport itself is still libp2p-dns's, so the error shape
//! (`dns::Error<T::Error>`) and what it resolves (`/dns`, `/dns4`,
//! `/dns6`, `/dnsaddr`) are what `dialing.rs` already classifies, and
//! `tests/dns_transport.rs` still reads the resolver's own failure.

use std::net::IpAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll};

use libp2p::Multiaddr;
use libp2p::core::transport::{DialOpts, ListenerId, Transport, TransportError, TransportEvent};
use libp2p::dns::{ResolverConfig, ResolverOpts};

/// The base transport, shared between the DNS transport in use and the
/// one that replaces it.
///
/// libp2p-dns takes its inner transport by value; this is the value, and
/// a clone of it is the same transport. The lock is shared: a dial the
/// replaced DNS transport started takes it after resolving, beside the
/// Swarm's own polls. Each holder takes libp2p-dns's lock first and this
/// one second, so a rebuild adds no lock order.
pub struct Shared<T>(Arc<Mutex<T>>);

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Shared<T> {
    fn lock(&self) -> std::sync::MutexGuard<'_, T> {
        // A panic while the base transport was held leaves it as the
        // panic found it; libp2p-dns's own lock (parking_lot) does not
        // poison either.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<T> Transport for Shared<T>
where
    T: Transport + Unpin,
{
    type Output = T::Output;
    type Error = T::Error;
    type ListenerUpgrade = T::ListenerUpgrade;
    type Dial = T::Dial;

    fn listen_on(
        &mut self,
        id: ListenerId,
        addr: Multiaddr,
    ) -> Result<(), TransportError<Self::Error>> {
        self.lock().listen_on(id, addr)
    }

    fn remove_listener(&mut self, id: ListenerId) -> bool {
        self.lock().remove_listener(id)
    }

    fn dial(
        &mut self,
        addr: Multiaddr,
        opts: DialOpts,
    ) -> Result<Self::Dial, TransportError<Self::Error>> {
        self.lock().dial(addr, opts)
    }

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<TransportEvent<Self::ListenerUpgrade, Self::Error>> {
        Pin::new(&mut *self.lock()).poll(cx)
    }
}

/// Where the runtime hands a new resolver configuration to the transport.
///
/// One slot, the latest configuration winning: a view that arrives
/// before the transport took the previous one replaces it, so the slot
/// never holds more than one.
#[derive(Clone, Default)]
pub struct ResolverSwitch(Arc<Mutex<Option<(ResolverConfig, ResolverOpts)>>>);

impl ResolverSwitch {
    /// Replace the resolver at the transport's next dial or poll.
    pub fn install(&self, config: ResolverConfig, opts: ResolverOpts) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some((config, opts));
    }

    fn take(&self) -> Option<(ResolverConfig, ResolverOpts)> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

/// The DNS transport whose resolver [`ResolverSwitch`] replaces.
pub struct LiveDns<T> {
    base: Shared<T>,
    current: libp2p::dns::tokio::Transport<Shared<T>>,
    switch: ResolverSwitch,
}

impl<T> LiveDns<T> {
    /// Wrap `inner` in the DNS transport, resolving with `config` until
    /// `switch` is handed another.
    pub fn new(
        inner: T,
        config: ResolverConfig,
        opts: ResolverOpts,
        switch: ResolverSwitch,
    ) -> Self {
        let base = Shared(Arc::new(Mutex::new(inner)));
        let current = libp2p::dns::tokio::Transport::custom(base.clone(), config, opts);
        Self {
            base,
            current,
            switch,
        }
    }

    /// Take a configuration the switch holds, if any, and rebuild the DNS
    /// transport over the same base with it. The old one is dropped here;
    /// a lookup it started owns its own resolver clone.
    fn refresh(&mut self) {
        if let Some((config, opts)) = self.switch.take() {
            self.current = libp2p::dns::tokio::Transport::custom(self.base.clone(), config, opts);
        }
    }
}

impl<T> Transport for LiveDns<T>
where
    T: Transport + Send + Unpin + 'static,
    T::Error: Send,
    T::Dial: Send,
{
    type Output = <libp2p::dns::tokio::Transport<Shared<T>> as Transport>::Output;
    type Error = <libp2p::dns::tokio::Transport<Shared<T>> as Transport>::Error;
    type ListenerUpgrade = <libp2p::dns::tokio::Transport<Shared<T>> as Transport>::ListenerUpgrade;
    type Dial = <libp2p::dns::tokio::Transport<Shared<T>> as Transport>::Dial;

    fn listen_on(
        &mut self,
        id: ListenerId,
        addr: Multiaddr,
    ) -> Result<(), TransportError<Self::Error>> {
        self.current.listen_on(id, addr)
    }

    fn remove_listener(&mut self, id: ListenerId) -> bool {
        self.current.remove_listener(id)
    }

    fn dial(
        &mut self,
        addr: Multiaddr,
        opts: DialOpts,
    ) -> Result<Self::Dial, TransportError<Self::Error>> {
        self.refresh();
        self.current.dial(addr, opts)
    }

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<TransportEvent<Self::ListenerUpgrade, Self::Error>> {
        // Polled on every Swarm turn, so a replaced resolver -- and the
        // sockets its pool holds -- goes as soon as the next turn, not at
        // the next dial.
        self.refresh();
        Pin::new(&mut self.current).poll(cx)
    }
}

/// At most this many of the platform's nameservers are used, in its
/// order. Android reports a handful (an IPv4 and an IPv6 pair is usual);
/// the bound keeps a misbehaving caller from choosing how many
/// connections a lookup fans out to.
pub const MAX_PLATFORM_NAMESERVERS: usize = 8;

/// The plain-DNS port, the only one a platform view names.
pub const DNS_PORT: u16 = 53;

/// What the platform's DNS facts mean for the resolver, decided from the
/// view alone (`CONNECTIVITY.md` §14, A 2026-10-14 and A 2026-10-20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformDns {
    /// The view carries no DNS facts and no strict Private DNS: the
    /// platform has not said (no default network, or its link not yet
    /// reported), so the resolver in use is kept.
    Unobserved,
    /// The active network names no usable nameserver and Private DNS is
    /// not strict: names are not resolved, as under strict mode, and the
    /// resolver the previous network named is not kept -- it would send
    /// names to a server the person has left.
    Unavailable,
    /// Resolve over plain DNS through these nameservers, deduplicated,
    /// in the platform's order, at most [`MAX_PLATFORM_NAMESERVERS`].
    Plain(Vec<IpAddr>),
    /// The platform's Private DNS is in strict mode. This build has no
    /// DNS-over-TLS, and plain DNS would break the user's setting, so no
    /// name is resolved (architect-cto's ruling, `GZCoord` 01a13bb7).
    PrivateDnsStrict,
}

impl PlatformDns {
    /// Read the view's DNS facts: `dns_servers` is `None` when the
    /// platform has not said, `Some` of what the active network names.
    ///
    /// Strict Private DNS decides first, whatever servers are named or
    /// whether any were observed: those are the network's plain
    /// resolvers, which strict mode does not permit. An unspecified
    /// address (`0.0.0.0`, `::`) names no server and is skipped, so a
    /// network naming only those is unavailable.
    #[must_use]
    pub fn from_view(dns_servers: Option<&[IpAddr]>, private_dns: Option<&str>) -> Self {
        if private_dns.is_some() {
            return Self::PrivateDnsStrict;
        }
        let Some(dns_servers) = dns_servers else {
            return Self::Unobserved;
        };
        let mut servers: Vec<IpAddr> = Vec::new();
        for ip in dns_servers {
            if servers.len() == MAX_PLATFORM_NAMESERVERS {
                break;
            }
            if !ip.is_unspecified() && !servers.contains(ip) {
                servers.push(*ip);
            }
        }
        if servers.is_empty() {
            Self::Unavailable
        } else {
            Self::Plain(servers)
        }
    }

    /// The resolver configuration this decision builds, `None` for
    /// [`Self::Unobserved`] (nothing is built). No search list and no
    /// local domain: an operator's name is resolved as given (ADR-0052
    /// rule 2). Strict mode and an unavailable resolver build the empty
    /// configuration, which resolves nothing.
    #[must_use]
    pub fn resolver_config(&self, port: u16) -> Option<(ResolverConfig, ResolverOpts)> {
        let servers = match self {
            Self::Unobserved => return None,
            Self::Plain(servers) => servers
                .iter()
                .map(|ip| {
                    let mut server = hickory_resolver::config::NameServerConfig::udp_and_tcp(*ip);
                    for connection in &mut server.connections {
                        connection.port = port;
                    }
                    server
                })
                .collect(),
            Self::PrivateDnsStrict | Self::Unavailable => Vec::new(),
        };
        Some((
            ResolverConfig::from_parts(None, Vec::new(), servers),
            ResolverOpts::default(),
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{MAX_PLATFORM_NAMESERVERS, PlatformDns};
    use std::net::IpAddr;

    fn ip(s: &str) -> IpAddr {
        s.parse().expect("ip")
    }

    #[test]
    fn no_dns_facts_and_no_strict_mode_is_unobserved_and_builds_nothing() {
        assert_eq!(PlatformDns::from_view(None, None), PlatformDns::Unobserved);
        assert!(PlatformDns::Unobserved.resolver_config(53).is_none());
    }

    #[test]
    fn a_network_naming_no_server_is_unavailable_and_builds_the_empty_resolver() {
        assert_eq!(
            PlatformDns::from_view(Some(&[]), None),
            PlatformDns::Unavailable
        );
        assert_eq!(
            PlatformDns::from_view(Some(&[ip("0.0.0.0"), ip("::")]), None),
            PlatformDns::Unavailable,
            "an unspecified address names no server"
        );
        let (config, _) = PlatformDns::Unavailable
            .resolver_config(53)
            .expect("a configuration: the previous resolver is replaced");
        assert!(
            config.name_servers().is_empty(),
            "the replacement asks no server"
        );
    }

    #[test]
    fn strict_private_dns_refuses_plain_dns_whatever_servers_are_named() {
        let strict = PlatformDns::from_view(Some(&[ip("192.0.2.53")]), Some("dns.example"));
        assert_eq!(strict, PlatformDns::PrivateDnsStrict);
        assert_eq!(
            PlatformDns::from_view(None, Some("dns.example")),
            PlatformDns::PrivateDnsStrict,
            "strict mode decides before whether servers were observed"
        );
        let (config, _) = strict.resolver_config(53).expect("a configuration");
        assert!(
            config.name_servers().is_empty(),
            "strict mode builds a resolver that asks no server"
        );
        // The control: the same servers without strict mode are used.
        assert_eq!(
            PlatformDns::from_view(Some(&[ip("192.0.2.53")]), None),
            PlatformDns::Plain(vec![ip("192.0.2.53")])
        );
    }

    #[test]
    fn the_nameservers_are_deduplicated_in_order_and_bounded() {
        let mut named: Vec<IpAddr> = (1..=20u8).map(|n| IpAddr::from([192, 0, 2, n])).collect();
        named.insert(1, IpAddr::from([192, 0, 2, 1]));
        let PlatformDns::Plain(servers) = PlatformDns::from_view(Some(&named), None) else {
            panic!("plain");
        };
        assert_eq!(servers.len(), MAX_PLATFORM_NAMESERVERS);
        assert_eq!(servers[0], ip("192.0.2.1"));
        assert_eq!(servers[1], ip("192.0.2.2"), "the duplicate is one");
    }

    #[test]
    fn a_plain_configuration_names_each_server_at_the_port_with_no_search_list() {
        let plain = PlatformDns::Plain(vec![ip("192.0.2.53"), ip("2001:db8::53")]);
        let (config, _) = plain.resolver_config(5353).expect("a configuration");
        assert!(config.search().is_empty() && config.domain().is_none());
        let servers = config.name_servers();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].ip, ip("192.0.2.53"));
        assert!(
            servers
                .iter()
                .flat_map(|s| s.connections.iter())
                .all(|c| c.port == 5353)
        );
    }
}

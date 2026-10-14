// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A listen-port dial the kernel refuses as `EADDRINUSE` is made again
//! from a fresh port (`CONNECTIVITY.md` §12, "Which local port a dial
//! uses").
//!
//! A `PortUse::Reuse` dial binds the listen port before it connects.
//! libp2p-tcp 0.45 retries such a dial on a fresh port when the connect
//! answers `EADDRNOTAVAIL` (its `src/lib.rs`, the `AddrNotAvailable`
//! arm) and on nothing else. On GitHub's ubuntu-24.04 runner the same
//! dial has failed with `EADDRINUSE` (CI run 38557166369, the AutoNAT
//! client's first dial from its listen port), and a Kademlia walk's dial,
//! a relay's control dial or an AutoNAT dial failed with it where a
//! fresh port connects. Why that kernel refused the bind is not
//! established; this host's kernel was not made to refuse it.
//!
//! Since Kademlia and AutoNAT dial from a fresh port (§12, A 2026-10-14),
//! the listen-port dials left are the relay's -- the reservation's
//! control connection and a circuit's dial to its relay -- and a hole
//! punch. A relay dial is redialled. A punch is not: its value is the
//! listen port, which the far end and the NAT's mapping both name, so a
//! fresh-port punch is not a punch; it fails at once, and is counted.
//!
//! Two counts, because two is what this layer can tell apart: libp2p's
//! `DialOpts` carries a role and a port use and no origin, so a
//! reservation's fallback and a circuit's are one count, and the local
//! port a connection was made from is reported nowhere.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use futures::FutureExt as _;
use futures::future::BoxFuture;
use libp2p::Multiaddr;
use libp2p::core::Endpoint;
use libp2p::core::transport::{
    DialOpts, ListenerId, PortUse, Transport, TransportError, TransportEvent,
};

/// The two counts, shared with whoever reads them.
#[derive(Debug, Clone, Default)]
pub struct PortFallbacks(Arc<[AtomicU64; 2]>);

/// The counts at one instant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PortFallbackCounts {
    /// Relay dials refused `EADDRINUSE` on the listen port and made
    /// again from a fresh port.
    pub relay_fell_back: u64,
    /// Hole-punch dials refused `EADDRINUSE` on the listen port, failed
    /// as they came.
    pub punch_refused: u64,
}

impl PortFallbacks {
    /// The counts so far.
    #[must_use]
    pub fn snapshot(&self) -> PortFallbackCounts {
        PortFallbackCounts {
            relay_fell_back: self.0[0].load(Ordering::Relaxed),
            punch_refused: self.0[1].load(Ordering::Relaxed),
        }
    }

    fn relay_fell_back(&self) {
        self.0[0].fetch_add(1, Ordering::Relaxed);
    }

    fn punch_refused(&self) {
        self.0[1].fetch_add(1, Ordering::Relaxed);
    }
}

/// `inner`, with a refused listen-port dial made again from a fresh
/// port. Everything else passes through.
#[derive(Debug)]
pub struct PortFallback<T> {
    inner: T,
    fallbacks: PortFallbacks,
}

impl<T> PortFallback<T> {
    /// Wrap `inner`, counting each fallback into `fallbacks`.
    pub const fn new(inner: T, fallbacks: PortFallbacks) -> Self {
        Self { inner, fallbacks }
    }
}

/// What a dial is to this wrapper. A hole punch is the dial libp2p
/// makes as the connection's listener (`DialOpts::role`).
enum Kind {
    /// From the listen port, and not a punch: redialled when refused.
    ListenPort,
    /// A punch from the listen port: counted when refused, never
    /// redialled.
    Punch,
    /// From a fresh port already: passed through.
    Fresh,
}

const fn kind(opts: DialOpts) -> Kind {
    match (opts.port_use, opts.role) {
        (PortUse::Reuse, Endpoint::Dialer) => Kind::ListenPort,
        (PortUse::Reuse, Endpoint::Listener) => Kind::Punch,
        (PortUse::New, _) => Kind::Fresh,
    }
}

impl<T> Transport for PortFallback<T>
where
    T: Transport<Error = io::Error> + Unpin,
    T::Dial: Send + 'static,
    T::Output: Send + 'static,
{
    type Output = T::Output;
    type Error = io::Error;
    type ListenerUpgrade = T::ListenerUpgrade;
    type Dial = BoxFuture<'static, Result<T::Output, io::Error>>;

    fn listen_on(
        &mut self,
        id: ListenerId,
        addr: Multiaddr,
    ) -> Result<(), TransportError<Self::Error>> {
        self.inner.listen_on(id, addr)
    }

    fn remove_listener(&mut self, id: ListenerId) -> bool {
        self.inner.remove_listener(id)
    }

    fn dial(
        &mut self,
        addr: Multiaddr,
        opts: DialOpts,
    ) -> Result<Self::Dial, TransportError<Self::Error>> {
        match kind(opts) {
            Kind::Fresh => Ok(self.inner.dial(addr, opts)?.boxed()),
            Kind::Punch => {
                let punch = self.inner.dial(addr, opts)?;
                let fallbacks = self.fallbacks.clone();
                Ok(async move {
                    let answer = punch.await;
                    if matches!(&answer, Err(e) if e.kind() == io::ErrorKind::AddrInUse) {
                        fallbacks.punch_refused();
                        tracing::info!(
                            "a hole punch's listen port was refused (EADDRINUSE); the punch fails"
                        );
                    }
                    answer
                }
                .boxed())
            }
            Kind::ListenPort => {
                let first = self.inner.dial(addr.clone(), opts)?;
                // Made now, polled only if the first is refused: `dial`
                // cannot reach the transport again from inside the
                // future, and an unpolled TCP dial is a socket that has
                // not connected, closed when the future is dropped.
                let fresh = self.inner.dial(
                    addr,
                    DialOpts {
                        port_use: PortUse::New,
                        ..opts
                    },
                )?;
                let fallbacks = self.fallbacks.clone();
                Ok(async move {
                    match first.await {
                        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                            fallbacks.relay_fell_back();
                            tracing::info!(error = %e, "a listen-port dial was refused; dialling from a fresh port");
                            fresh.await
                        }
                        answer => answer,
                    }
                }
                .boxed())
            }
        }
    }

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<TransportEvent<Self::ListenerUpgrade, Self::Error>> {
        Pin::new(&mut self.inner).poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use futures::future::{Ready, ready};

    use super::*;

    /// A transport whose dials answer from a script, recording each
    /// dial's port use.
    #[derive(Default)]
    struct Scripted {
        answers: VecDeque<Result<u8, io::ErrorKind>>,
        dialled: Vec<PortUse>,
    }

    impl Transport for Scripted {
        type Output = u8;
        type Error = io::Error;
        type ListenerUpgrade = Ready<Result<u8, io::Error>>;
        type Dial = Ready<Result<u8, io::Error>>;

        fn listen_on(
            &mut self,
            _: ListenerId,
            a: Multiaddr,
        ) -> Result<(), TransportError<io::Error>> {
            Err(TransportError::MultiaddrNotSupported(a))
        }

        fn remove_listener(&mut self, _: ListenerId) -> bool {
            false
        }

        fn dial(
            &mut self,
            _: Multiaddr,
            opts: DialOpts,
        ) -> Result<Self::Dial, TransportError<io::Error>> {
            self.dialled.push(opts.port_use);
            let answer = self.answers.pop_front().expect("a scripted answer");
            Ok(ready(answer.map_err(io::Error::from)))
        }

        fn poll(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<TransportEvent<Self::ListenerUpgrade, io::Error>> {
            Poll::Pending
        }
    }

    fn opts(port_use: PortUse, role: Endpoint) -> DialOpts {
        DialOpts { role, port_use }
    }

    fn addr() -> Multiaddr {
        "/ip4/127.0.0.1/tcp/4001".parse().expect("valid")
    }

    fn wrapped(answers: &[Result<u8, io::ErrorKind>]) -> PortFallback<Scripted> {
        PortFallback::new(
            Scripted {
                answers: answers.iter().copied().collect(),
                dialled: Vec::new(),
            },
            PortFallbacks::default(),
        )
    }

    /// A listen-port dial refused `EADDRINUSE` is answered by the fresh
    /// one, and counted; one that connects is the control: its answer,
    /// nothing counted.
    #[tokio::test]
    async fn a_refused_listen_port_dial_is_made_again_from_a_fresh_port() {
        let mut t = wrapped(&[Err(io::ErrorKind::AddrInUse), Ok(7)]);
        let dial = t
            .dial(addr(), opts(PortUse::Reuse, Endpoint::Dialer))
            .expect("dials");
        assert_eq!(dial.await.expect("the fresh port connects"), 7);
        assert_eq!(t.inner.dialled, [PortUse::Reuse, PortUse::New]);
        assert_eq!(
            t.fallbacks.snapshot(),
            PortFallbackCounts {
                relay_fell_back: 1,
                punch_refused: 0
            }
        );

        let mut t = wrapped(&[Ok(3), Ok(9)]);
        let dial = t
            .dial(addr(), opts(PortUse::Reuse, Endpoint::Dialer))
            .expect("dials");
        assert_eq!(
            dial.await.expect("connects"),
            3,
            "the listen-port dial's own answer"
        );
        assert_eq!(t.fallbacks.snapshot(), PortFallbackCounts::default());
    }

    /// Only `EADDRINUSE` falls back: any other refusal is the answer,
    /// and the fresh dial is never awaited.
    #[tokio::test]
    async fn another_refusal_is_returned_as_it_came() {
        let mut t = wrapped(&[Err(io::ErrorKind::ConnectionRefused), Ok(7)]);
        let dial = t
            .dial(addr(), opts(PortUse::Reuse, Endpoint::Dialer))
            .expect("dials");
        let refused = dial.await.expect_err("refused");
        assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        assert_eq!(t.fallbacks.snapshot(), PortFallbackCounts::default());
    }

    /// A hole punch -- a listen-port dial made as the connection's
    /// listener -- refused `EADDRINUSE` fails as it came and is counted
    /// as a punch; a fresh-port dial is passed through. Neither makes a
    /// second dial.
    #[tokio::test]
    async fn a_hole_punch_and_a_fresh_port_dial_are_dialled_once() {
        let mut t = wrapped(&[Err(io::ErrorKind::AddrInUse)]);
        let dial = t
            .dial(addr(), opts(PortUse::Reuse, Endpoint::Listener))
            .expect("dials");
        assert_eq!(
            dial.await.expect_err("the punch fails").kind(),
            io::ErrorKind::AddrInUse
        );
        assert_eq!(t.inner.dialled, [PortUse::Reuse]);
        assert_eq!(
            t.fallbacks.snapshot(),
            PortFallbackCounts {
                relay_fell_back: 0,
                punch_refused: 1
            }
        );

        let mut t = wrapped(&[Err(io::ErrorKind::AddrInUse)]);
        let dial = t
            .dial(addr(), opts(PortUse::New, Endpoint::Dialer))
            .expect("dials");
        assert_eq!(
            dial.await.expect_err("refused").kind(),
            io::ErrorKind::AddrInUse
        );
        assert_eq!(t.inner.dialled, [PortUse::New]);
        assert_eq!(t.fallbacks.snapshot(), PortFallbackCounts::default());
    }

    /// The real TCP transport inside, over real sockets: a listen-port
    /// dial connects FROM the listen port (the control -- nothing
    /// refused it, nothing counted), and the same dial whose listen-port
    /// attempt is refused `EADDRINUSE` at the seam connects from a fresh
    /// port and is counted. The kernel refusal itself is not reproducible
    /// on a 6.17 kernel (§12, A 2026-10-14); the seam stands in for it.
    #[tokio::test]
    async fn over_real_sockets_the_listen_port_is_kept_and_a_refusal_moves_off_it() {
        use libp2p::tcp;

        /// The real transport, its listen-port dials refused as the
        /// runner's kernel refused them.
        struct RefusingReuse(tcp::tokio::Transport);
        impl Transport for RefusingReuse {
            type Output = tcp::tokio::TcpStream;
            type Error = io::Error;
            type ListenerUpgrade = <tcp::tokio::Transport as Transport>::ListenerUpgrade;
            type Dial = BoxFuture<'static, Result<Self::Output, io::Error>>;
            fn listen_on(
                &mut self,
                id: ListenerId,
                a: Multiaddr,
            ) -> Result<(), TransportError<io::Error>> {
                self.0.listen_on(id, a)
            }
            fn remove_listener(&mut self, id: ListenerId) -> bool {
                self.0.remove_listener(id)
            }
            fn dial(
                &mut self,
                a: Multiaddr,
                opts: DialOpts,
            ) -> Result<Self::Dial, TransportError<io::Error>> {
                let dial = self.0.dial(a, opts)?;
                Ok(match opts.port_use {
                    PortUse::Reuse => {
                        async { Err(io::Error::from(io::ErrorKind::AddrInUse)) }.boxed()
                    }
                    PortUse::New => dial,
                })
            }
            fn poll(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<TransportEvent<Self::ListenerUpgrade, io::Error>> {
                Pin::new(&mut self.0).poll(cx)
            }
        }

        /// Listen on loopback, then dial `far` from the listen port and
        /// answer the source port `far` saw beside the listen port.
        async fn dial_through<T>(mut t: PortFallback<T>) -> (u16, u16, PortFallbackCounts)
        where
            T: Transport<Error = io::Error> + Unpin,
            T::Dial: Send + 'static,
            T::Output: Send + 'static,
        {
            t.listen_on(
                ListenerId::next(),
                "/ip4/127.0.0.1/tcp/0".parse().expect("valid"),
            )
            .expect("listens");
            let listen_port = loop {
                if let TransportEvent::NewAddress { listen_addr, .. } =
                    futures::future::poll_fn(|cx| Pin::new(&mut t).poll(cx)).await
                {
                    break listen_addr
                        .iter()
                        .find_map(|p| match p {
                            libp2p::multiaddr::Protocol::Tcp(port) => Some(port),
                            _ => None,
                        })
                        .expect("a tcp port");
                }
            };
            let far = tokio::net::TcpListener::bind(("127.0.0.1", 0))
                .await
                .expect("binds");
            let far_addr: Multiaddr = format!(
                "/ip4/127.0.0.1/tcp/{}",
                far.local_addr().expect("addr").port()
            )
            .parse()
            .expect("valid");
            let dial = t
                .dial(far_addr, opts(PortUse::Reuse, Endpoint::Dialer))
                .expect("dials");
            // The kernel completes the handshake into `far`'s backlog, so
            // the dial is awaited alone: a refused one fails here rather
            // than leaving `accept` waiting.
            let _stream = dial.await.expect("connects");
            let (_far_end, source) = far.accept().await.expect("accepted");
            (source.port(), listen_port, t.fallbacks.snapshot())
        }

        let (source, listen, counts) = dial_through(PortFallback::new(
            tcp::tokio::Transport::new(tcp::Config::default()),
            PortFallbacks::default(),
        ))
        .await;
        assert_eq!(source, listen, "unrefused, the dial keeps the listen port");
        assert_eq!(counts, PortFallbackCounts::default());

        let (source, listen, counts) = dial_through(PortFallback::new(
            RefusingReuse(tcp::tokio::Transport::new(tcp::Config::default())),
            PortFallbacks::default(),
        ))
        .await;
        assert_ne!(source, listen, "refused, it connects from a fresh port");
        assert_eq!(
            counts,
            PortFallbackCounts {
                relay_fell_back: 1,
                punch_refused: 0
            }
        );
    }
}

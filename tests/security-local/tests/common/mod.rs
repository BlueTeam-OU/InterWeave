// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What more than one test file here shares: two shipped daemons that
//! trust each other, and the session helpers around them.

// Each test binary includes this module and uses a part of it.
#![allow(dead_code)]

use std::time::Duration;

use interweave_local_client_api::{DataSessionPort, SessionEvent};
use interweave_test_support::e2e::{Daemon, Home, example, free_port};
use interweave_transport_api::{MediaType, Payload, TransportIdentity};

/// Two daemons that trust each other, A holding a static route to B.
pub(crate) struct Pair {
    pub(crate) a: Home,
    pub(crate) b: Home,
    pub(crate) a_daemon: Daemon,
    pub(crate) b_daemon: Daemon,
    pub(crate) a_peer: TransportIdentity,
    pub(crate) b_peer: TransportIdentity,
}

impl Pair {
    pub(crate) async fn start() -> Self {
        Self::start_with(Home::write_key, None).await
    }

    /// A's key put in place by `key`, which returns its `PeerId`, once
    /// A's profile is written -- a restore through `transportctl` reads
    /// it -- and before B's, which trusts what `key` returned; A's daemon
    /// started after `a_preamble` when one is given
    /// ([`Home::start_after`]).
    pub(crate) async fn start_with(
        key: impl FnOnce(&Home) -> TransportIdentity,
        a_preamble: Option<&str>,
    ) -> Self {
        let ip = interweave_test_support::net::require_private_interface_v4();
        let (a, b) = (Home::new("human-desktop"), Home::new("human-desktop"));
        let b_peer = b.write_key();
        let (a_port, b_port) = (free_port(ip), free_port(ip));
        let at = |port: u16| format!("/ip4/{ip}/tcp/{port}");
        let route = format!("{}/p2p/{}", at(b_port), b_peer.as_str());
        a.write_config(&example(
            "human-desktop.yaml",
            &b_peer,
            &at(a_port),
            Some(&route),
        ));
        let a_peer = key(&a);
        b.write_config(&example("human-desktop.yaml", &a_peer, &at(b_port), None));
        let mut b_daemon = b.start(&[]);
        b_daemon.serving(&b).await;
        let mut a_daemon = match a_preamble {
            Some(preamble) => a.start_after(preamble, &[]),
            None => a.start(&[]),
        };
        a_daemon.serving(&a).await;
        Self {
            a,
            b,
            a_daemon,
            b_daemon,
            a_peer,
            b_peer,
        }
    }

    pub(crate) fn logs(&self) -> String {
        format!("A:\n{}\nB:\n{}", self.a_daemon.log(), self.b_daemon.log())
    }

    pub(crate) async fn stop(mut self) {
        assert!(self.a_daemon.terminate().await.success(), "{}", self.logs());
        assert!(self.b_daemon.terminate().await.success(), "{}", self.logs());
    }
}

pub(crate) fn payload(bytes: &[u8]) -> Payload {
    Payload::at_ceiling(
        Some(MediaType::parse("application/json").expect("a media type")),
        bytes.to_vec(),
    )
    .expect("a payload")
}

/// Everything `session` receives within `window`.
pub(crate) async fn arriving<S: DataSessionPort>(
    session: &S,
    window: Duration,
) -> Vec<SessionEvent> {
    let deadline = tokio::time::Instant::now() + window;
    let mut got = Vec::new();
    while tokio::time::Instant::now() < deadline {
        got.extend(session.events(64).await.expect("events"));
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    got
}

/// Everything `session` receives within `window`, cut short at
/// `deadline`, for a poll loop that asserts its deadline at the top of
/// each pass. With the whole window, a pass started just before the
/// deadline could gather the last event up to one window after it and
/// end the loop with no assert left to read the clock (#459's review,
/// F1). Clipped, each `events` call starts before the deadline, which
/// leaves the one call in flight at the deadline as the lateness a pass
/// can still accept; a pass that gathers nothing in time leaves the next
/// pass's assert to fail.
pub(crate) async fn arriving_before<S: DataSessionPort>(
    session: &S,
    window: Duration,
    deadline: tokio::time::Instant,
) -> Vec<SessionEvent> {
    let left = deadline.saturating_duration_since(tokio::time::Instant::now());
    arriving(session, window.min(left)).await
}

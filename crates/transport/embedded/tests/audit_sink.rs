// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The trust-audit sink, the runtime's half (plan §20, carried from §18):
//! a sink filtered with the embedded host's own [`log_admits`] at a
//! profile level STRICTER than INFO still receives a trust change's
//! record on the audit target at INFO -- the record is the contract's
//! (`LOCAL-CLIENT.md` §5), not a diagnostic -- while no other
//! first-party INFO line gets through it. The platform's writer (logcat)
//! is the shell's; what it is handed is this.
//!
//! Its own test binary: a GLOBAL subscriber, since the runtime logs from
//! its own worker threads, which a thread-local default would not see.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt as _;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use interweave_local_client_api::{
    AdminBinding as _, AdminCapability, AdminPort as _, TrustChange,
};
use interweave_profile_config::endpoint_key::mint_filesystem;
use interweave_profile_config::provision::{provision_embedded, provision_identity};
use interweave_profile_config::sections::LogLevel;
use interweave_profile_config::{ProfilePaths, TrustBoundary};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_embedded::{EmbeddedHost, EmbeddedLaunch, log_admits};
use tracing_subscriber::layer::SubscriberExt as _;

/// One event the sink received: its target, level, message and
/// `outcome` field.
#[derive(Debug, Clone)]
struct Seen {
    target: String,
    level: tracing::Level,
    message: String,
    outcome: Option<String>,
}

#[derive(Default)]
struct Fields {
    message: String,
    outcome: Option<String>,
}

impl tracing::field::Visit for Fields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "outcome" {
            self.outcome = Some(value.to_owned());
        }
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}

struct Capture(Arc<Mutex<Vec<Seen>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.0.lock().expect("unpoisoned").push(Seen {
            target: event.metadata().target().to_owned(),
            level: *event.metadata().level(),
            message: fields.message,
            outcome: fields.outcome,
        });
    }
}

#[test]
fn a_trust_change_reaches_a_sink_stricter_than_info() {
    // The profile level the sink is built for: WARN, stricter than INFO.
    const STRICT: LogLevel = LogLevel::Warn;
    let seen = Arc::new(Mutex::new(Vec::new()));
    // A GLOBAL filter, as the Android shell's own layer is, never
    // `with_filter`: a per-layer one drops the next event on a thread
    // after a bare `enabled` check there -- libp2p-identify's handler makes
    // one -- and the audit record is logged from a worker thread
    // (tracing-subscriber 0.3.23; `bare_check_probe` below pins it).
    let admits = tracing_subscriber::filter::filter_fn(|meta| {
        log_admits(meta.target(), *meta.level(), STRICT)
    });
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(admits)
            .with(Capture(Arc::clone(&seen))),
    )
    .expect("the only subscriber in this binary");
    bare_check_probe(&seen);

    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let app = root.path().join("app");
    std::fs::create_dir(&app).expect("mkdir");
    std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    let paths =
        ProfilePaths::resolve_embedded("work", TrustBoundary::new(&app).expect("a boundary"))
            .expect("paths");
    provision_embedded(&paths).expect("provisioned");
    // The first identity, as onboarding seals it: form 1 has none, and a
    // start on it is refused.
    let id = mint_filesystem(&paths).expect("a key minted");
    provision_identity(&paths, &id).expect("the first identity");
    let host = EmbeddedHost::start(EmbeddedLaunch {
        app_data_dir: app.clone(),
        profile: "work".to_owned(),
        identity: ProfileIdentity::generate(),
    })
    .expect("starts");
    let port = host
        .runtime()
        .block_on(
            host.binding()
                .admin(BTreeSet::from([AdminCapability::Trust])),
        )
        .expect("an admin port");
    let peer = ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer");
    assert_eq!(
        host.runtime().block_on(port.set_trust(peer, true)),
        Ok(TrustChange::Set),
        "trusted"
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    let audit = loop {
        let found = seen
            .lock()
            .expect("unpoisoned")
            .iter()
            .find(|e| e.target == "interweave::audit" && e.message == "admin.trust.set")
            .cloned();
        if let Some(found) = found {
            break found;
        }
        assert!(
            Instant::now() < deadline,
            "the trust change's record never reached the sink"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        audit.level,
        tracing::Level::INFO,
        "at INFO, below the sink's WARN"
    );
    assert_eq!(audit.outcome.as_deref(), Some("changed"));
    // THE CONTROL: nothing else at INFO got through the same sink.
    let leaked: Vec<Seen> = seen
        .lock()
        .expect("unpoisoned")
        .iter()
        .filter(|e| e.level >= tracing::Level::INFO && e.target != "interweave::audit")
        .cloned()
        .collect();
    assert!(
        leaked.is_empty(),
        "only the audit record passes at INFO: {leaked:?}"
    );
    drop(port);
    host.stop(Duration::from_secs(1)).expect("stops");
}

/// The sink keeps a first-party event that follows a bare `enabled` check
/// of a refused target on the same thread: what libp2p-identify's handler
/// makes (`tracing::enabled!(Level::DEBUG)`, no event after it) when its
/// callsite's interest is `sometimes`, as a thread losing the callsite's
/// first-registration race is handed. A per-layer filter leaves its
/// refusal's bit set in the thread's filter state, and the event after is
/// dropped.
fn bare_check_probe(seen: &Mutex<Vec<Seen>>) {
    use tracing::callsite::{Callsite, Identifier};
    use tracing::subscriber::Interest;
    struct Foreign;
    static FOREIGN: Foreign = Foreign;
    static META: tracing::Metadata<'static> = tracing::Metadata::new(
        "foreign",
        "libp2p_identify::handler",
        tracing::Level::DEBUG,
        None,
        None,
        None,
        tracing::field::FieldSet::new(&[], Identifier(&FOREIGN)),
        tracing::metadata::Kind::EVENT,
    );
    impl Callsite for Foreign {
        fn set_interest(&self, _: Interest) {}
        fn metadata(&self) -> &tracing::Metadata<'_> {
            &META
        }
    }
    let probes = |seen: &Mutex<Vec<Seen>>| {
        seen.lock()
            .expect("unpoisoned")
            .iter()
            .filter(|e| e.target == "interweave::probe")
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    };
    tracing::warn!(target: "interweave::probe", "before the check");
    tracing::dispatcher::get_default(|dispatch| dispatch.enabled(&META));
    tracing::warn!(target: "interweave::probe", "after the check");
    assert_eq!(
        probes(seen),
        ["before the check", "after the check"],
        "the control first, then the event after the check"
    );
}

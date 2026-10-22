// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The bootstrap-peers overlay through a running composition (ADR-0062,
//! ADR-0028 A 2026-10-21 (ii)): an address added through the admin port
//! is written, dialled and kept across a restart with no provider
//! authored; each refusal is answered by name with nothing written; a
//! removed address leaves without a restart; an authored entry stays;
//! an authored `enabled: false` provider refuses the add and lists what
//! the overlay holds as not dialled; and a composed count past 64 stops
//! the start naming the overlay -- each beside its control.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use interweave_local_client_api::{
    AddressChange, AddressRefusal, AddressSource, AdminBinding as _, AdminCapability,
    AdminPort as _, BootstrapAddress,
};
use interweave_profile_config::ProfileConfig;
use interweave_profile_config::bootstrap_overlay::BOOTSTRAP_OVERLAY_FILE;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{TransportEvent, TransportIdentity, TransportRuntime as _};
use interweave_transport_composition::{ComposedRuntime, CompositionError, CompositionOptions};

const PATIENCE: Duration = Duration::from_secs(30);

fn id() -> (ProfileIdentity, TransportIdentity) {
    let identity = ProfileIdentity::generate();
    let peer = identity.transport_identity().expect("peer id");
    (identity, peer)
}

/// How the profile authors the static-bootstrap provider.
#[derive(Clone, Copy)]
enum Provider<'a> {
    None,
    Enabled(&'a [String]),
    Disabled(&'a [String]),
}

fn profile(trusted: &[&TransportIdentity], provider: Provider<'_>) -> ProfileConfig {
    let list = |items: &mut dyn Iterator<Item = String>| {
        items
            .map(|i| format!("\"{i}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let peers = list(&mut trusted.iter().map(|p| p.as_str().to_owned()));
    let providers = match provider {
        Provider::None => "  providers: []\n".to_owned(),
        Provider::Enabled(entries) | Provider::Disabled(entries) => format!(
            "  providers:
    - type: static-bootstrap
      enabled: {}
      priority: 10
      config:
        peers: [{}]
",
            matches!(provider, Provider::Enabled(_)),
            list(&mut entries.iter().cloned())
        ),
    };
    let doc = format!(
        "schema_version: 2
trust:
  policy: static-allowlist
  allowed_peers: [{peers}]
endpoints:
  entries:
    - id: human
      enabled: true
      advertise: false
discovery:
{providers}"
    );
    serde_norway::from_str(&doc).expect("the document parses")
}

fn state() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    chmod(dir.path(), 0o700);
    let path = dir.path().join(BOOTSTRAP_OVERLAY_FILE);
    (dir, path)
}

fn chmod(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

fn options(overlay: &Path) -> CompositionOptions {
    CompositionOptions {
        bootstrap_overlay_file: Some(overlay.to_path_buf()),
        ..CompositionOptions::default()
    }
}

/// A private-range address for `peer`, distinct for each `port`.
fn at(port: u16, peer: &TransportIdentity) -> String {
    format!("/ip4/192.168.77.1/tcp/{port}/p2p/{}", peer.as_str())
}

async fn add(runtime: &ComposedRuntime, address: &str) -> AddressChange {
    port(runtime)
        .await
        .add_bootstrap_address(address.to_owned())
        .await
        .expect("answered")
}

async fn remove(runtime: &ComposedRuntime, address: &str) -> AddressChange {
    port(runtime)
        .await
        .remove_bootstrap_address(address.to_owned())
        .await
        .expect("answered")
}

async fn rows(runtime: &ComposedRuntime) -> Vec<BootstrapAddress> {
    port(runtime)
        .await
        .bootstrap_addresses()
        .await
        .expect("answered")
}

async fn port(runtime: &ComposedRuntime) -> impl interweave_local_client_api::AdminPort {
    runtime
        .sessions()
        .admin([AdminCapability::Peers].into())
        .await
        .expect("a port")
}

fn refused(change: &AddressChange) -> Option<AddressRefusal> {
    match change {
        AddressChange::Refused { reason, .. } => Some(*reason),
        _ => None,
    }
}

fn row(address: &str, source: AddressSource, dialled: bool) -> BootstrapAddress {
    BootstrapAddress {
        address: address.to_owned(),
        source,
        dialled,
    }
}

async fn wait_connected(runtime: &mut ComposedRuntime, peer: &TransportIdentity) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, runtime.next_event()).await {
            Ok(Some(TransportEvent::PeerConnected { peer: got, .. })) if &got == peer => return,
            Ok(Some(_)) => {}
            Ok(None) => panic!("the runtime stopped"),
            Err(elapsed) => panic!("no PeerConnected within {PATIENCE:?} ({elapsed})"),
        }
    }
}

/// Plan section 29 gate (e)'s first case at the composition: with no
/// static-bootstrap provider authored, an address added for a trusted
/// peer is dialled, and after a restart it is still listed and dialled
/// again. The control for the dial: before the add, nothing connects.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_added_address_is_dialled_and_survives_a_restart_with_no_provider_authored() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let listen = CompositionOptions {
        listen: vec![format!("/ip4/{ip}/tcp/0")],
        ..CompositionOptions::default()
    };
    let (b_id, b) = id();
    let (a_id, a) = id();
    let target = ComposedRuntime::start(&b_id, &profile(&[&a], Provider::None), listen.clone())
        .await
        .expect("b composes");
    let b_addr = format!("{}/p2p/{}", target.listening()[0], b.as_str());

    let (_dir, path) = state();
    let a_options = CompositionOptions {
        bootstrap_overlay_file: Some(path.clone()),
        ..listen
    };
    let mut subject =
        ComposedRuntime::start(&a_id, &profile(&[&b], Provider::None), a_options.clone())
            .await
            .expect("a composes");
    assert!(
        tokio::time::timeout(Duration::from_secs(2), wait_connected(&mut subject, &b))
            .await
            .is_err(),
        "the control: nothing connects before the add"
    );
    assert_eq!(add(&subject, &b_addr).await, AddressChange::Added);
    assert!(path.exists(), "written before the answer");
    wait_connected(&mut subject, &b).await;
    assert_eq!(
        rows(&subject).await,
        vec![row(&b_addr, AddressSource::Added, true)]
    );
    subject.shutdown().await.expect("clean shutdown");

    let mut restarted = ComposedRuntime::start(&a_id, &profile(&[&b], Provider::None), a_options)
        .await
        .expect("a composes again");
    assert_eq!(
        rows(&restarted).await,
        vec![row(&b_addr, AddressSource::Added, true)],
        "kept across the restart"
    );
    wait_connected(&mut restarted, &b).await;
    restarted.shutdown().await.expect("clean shutdown");
    target.shutdown().await.expect("clean shutdown");
}

/// Each refusal is an answer naming its rule, and nothing is written;
/// an address already present changes nothing. The control: a
/// private-range address is added.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_refusal_is_named_and_writes_nothing() {
    let (identity, _) = id();
    let (_, b) = id();
    let authored = vec![at(1, &b)];
    let (_dir, path) = state();
    let runtime = ComposedRuntime::start(
        &identity,
        &profile(&[&b], Provider::Enabled(&authored)),
        options(&path),
    )
    .await
    .expect("composes");

    let peer = b.as_str();
    for (address, reason) in [
        ("not an address".to_owned(), AddressRefusal::Grammar),
        (
            "/ip4/192.168.77.1/tcp/1".to_owned(),
            AddressRefusal::Grammar,
        ),
        (
            format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer}"),
            AddressRefusal::SpecialUse,
        ),
        (
            format!("/ip4/169.254.1.1/tcp/4001/p2p/{peer}"),
            AddressRefusal::SpecialUse,
        ),
        (
            format!("/ip6/::1/tcp/4001/p2p/{peer}"),
            AddressRefusal::SpecialUse,
        ),
        (
            format!("/ip4/224.0.0.251/tcp/4001/p2p/{peer}"),
            AddressRefusal::SpecialUse,
        ),
        (
            format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer}/p2p-circuit/p2p/{peer}"),
            AddressRefusal::SpecialUse,
        ),
    ] {
        let change = add(&runtime, &address).await;
        assert_eq!(refused(&change), Some(reason), "{address}: {change:?}");
    }
    assert!(!path.exists(), "nothing written for a refusal");

    assert_eq!(add(&runtime, &authored[0]).await, AddressChange::Unchanged);
    assert!(!path.exists(), "nothing written for an authored address");
    assert_eq!(
        add(&runtime, &at(2, &b)).await,
        AddressChange::Added,
        "the control"
    );
    assert_eq!(add(&runtime, &at(2, &b)).await, AddressChange::Unchanged);
    assert_eq!(
        rows(&runtime).await,
        vec![
            row(&authored[0], AddressSource::Authored, true),
            row(&at(2, &b), AddressSource::Added, true),
        ]
    );

    // The bound: 64 authored and added together.
    for p in 3..=64 {
        assert_eq!(add(&runtime, &at(p, &b)).await, AddressChange::Added, "{p}");
    }
    assert_eq!(
        refused(&add(&runtime, &at(65, &b)).await),
        Some(AddressRefusal::Bound),
        "the 65th"
    );
    assert_eq!(rows(&runtime).await.len(), 64);
    runtime.shutdown().await.expect("clean shutdown");
}

/// A removal of an authored entry is refused by name; an added one
/// leaves the overlay and the running provider at once; an absent one
/// changes nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_authored_address_cannot_be_removed_and_an_added_one_leaves_at_once() {
    let (identity, _) = id();
    let (_, b) = id();
    let authored = vec![at(1, &b)];
    let (_dir, path) = state();
    let runtime = ComposedRuntime::start(
        &identity,
        &profile(&[&b], Provider::Enabled(&authored)),
        options(&path),
    )
    .await
    .expect("composes");
    assert_eq!(add(&runtime, &at(2, &b)).await, AddressChange::Added);
    assert_eq!(
        refused(&remove(&runtime, &authored[0]).await),
        Some(AddressRefusal::Authored)
    );
    assert_eq!(remove(&runtime, &at(2, &b)).await, AddressChange::Removed);
    assert_eq!(remove(&runtime, &at(2, &b)).await, AddressChange::Unchanged);
    assert_eq!(
        rows(&runtime).await,
        vec![row(&authored[0], AddressSource::Authored, true)]
    );
    let on_disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
    assert_eq!(on_disk["peers"], serde_json::json!([]));
    runtime.shutdown().await.expect("clean shutdown");
}

/// Under an authored `enabled: false` provider an add is refused by
/// name, and what the overlay already holds loads and lists as not
/// dialled (ADR-0062 decision 2). The control: the same overlay under an
/// enabled provider lists as dialled.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_disabled_provider_refuses_the_add_and_lists_the_overlay_as_not_dialled() {
    // The subscriber before the first start, or its diagnostic is lost.
    let _ = logged();
    let (identity, _) = id();
    let (_, b) = id();
    let authored = vec![at(1, &b)];
    let (_dir, path) = state();
    std::fs::write(
        &path,
        serde_json::json!({ "peers": [at(2, &b)] }).to_string(),
    )
    .expect("written");
    chmod(&path, 0o600);

    let runtime = ComposedRuntime::start(
        &identity,
        &profile(&[&b], Provider::Disabled(&authored)),
        options(&path),
    )
    .await
    .expect("composes");
    assert_eq!(
        refused(&add(&runtime, &at(3, &b)).await),
        Some(AddressRefusal::ProviderDisabled)
    );
    assert_eq!(
        rows(&runtime).await,
        vec![
            row(&authored[0], AddressSource::Authored, false),
            row(&at(2, &b), AddressSource::Added, false),
        ]
    );
    runtime.shutdown().await.expect("clean shutdown");
    // THE LOAD-TIME DIAGNOSTIC names the overlay and the disabled
    // provider, once for the one start that loaded entries under it.
    let named = |text: &str| {
        text.lines()
            .filter(|line| {
                line.contains("nothing dials")
                    && line.contains("enabled: false")
                    && line.contains(&path.display().to_string())
            })
            .count()
    };
    assert_eq!(named(&logged()), 1, "{}", logged());

    let runtime = ComposedRuntime::start(
        &identity,
        &profile(&[&b], Provider::Enabled(&authored)),
        options(&path),
    )
    .await
    .expect("composes");
    assert_eq!(
        rows(&runtime).await,
        vec![
            row(&authored[0], AddressSource::Authored, true),
            row(&at(2, &b), AddressSource::Added, true),
        ],
        "the control"
    );
    runtime.shutdown().await.expect("clean shutdown");
    assert_eq!(
        named(&logged()),
        1,
        "the control: no diagnostic when enabled"
    );
}

/// Everything the runtimes in this binary have logged so far: ONE
/// subscriber for the binary, installed once, as `trust_overlay.rs`
/// explains -- per-test scoped ones race on tracing's callsite cache.
fn logged() -> String {
    static LOG: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<Vec<u8>>>> =
        std::sync::OnceLock::new();
    let log = LOG.get_or_init(|| {
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&log);
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || Writer(std::sync::Arc::clone(&sink)))
            .with_ansi(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("the one subscriber");
        log
    });
    String::from_utf8(log.lock().expect("lock").clone()).expect("utf-8")
}

struct Writer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("lock").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Authored and added entries past 64 stop the start, naming the
/// overlay; at 64 exactly it starts (the control).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_composed_count_past_64_stops_the_start_naming_the_overlay() {
    let (identity, _) = id();
    let (_, b) = id();
    let authored: Vec<String> = (1..=32).map(|p| at(p, &b)).collect();
    let (_dir, path) = state();
    let write = |n: u16| {
        let added: Vec<String> = (101..101 + n).map(|p| at(p, &b)).collect();
        std::fs::write(&path, serde_json::json!({ "peers": added }).to_string()).expect("written");
        chmod(&path, 0o600);
    };
    write(32);
    ComposedRuntime::start(
        &identity,
        &profile(&[&b], Provider::Enabled(&authored)),
        options(&path),
    )
    .await
    .expect("64 starts: the control")
    .shutdown()
    .await
    .expect("clean shutdown");

    write(33);
    match ComposedRuntime::start(
        &identity,
        &profile(&[&b], Provider::Enabled(&authored)),
        options(&path),
    )
    .await
    {
        Err(e @ CompositionError::BootstrapOverlay(_)) => {
            assert!(e.to_string().contains("bootstrap overlay"), "{e}");
        }
        Err(other) => panic!("refused naming the overlay: {other}"),
        Ok(_) => panic!("65 refused"),
    }
}

/// Close every connection to `peer` the way that counts no dial failure
/// against it: revoke, then allow again (ADR-0012).
async fn reset_trust(runtime: &ComposedRuntime, peer: &TransportIdentity) {
    let port = runtime
        .sessions()
        .admin([AdminCapability::Trust].into())
        .await
        .expect("a port");
    port.set_trust(peer.clone(), false).await.expect("revoked");
    port.set_trust(peer.clone(), true)
        .await
        .expect("allowed again");
}

/// A REMOVED ADDRESS IS DIALLED NO MORE, without a restart: its
/// connection closed, the reconnect round finds no address for the peer
/// in the provider, the operator's door or the book. The control, first:
/// while the address is held, the same close is followed by a reconnect.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_removed_address_is_dialled_no_more_without_a_restart() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let listen = CompositionOptions {
        listen: vec![format!("/ip4/{ip}/tcp/0")],
        ..CompositionOptions::default()
    };
    let (b_id, b) = id();
    let (a_id, a) = id();
    let target = ComposedRuntime::start(&b_id, &profile(&[&a], Provider::None), listen.clone())
        .await
        .expect("b composes");
    let b_addr = format!("{}/p2p/{}", target.listening()[0], b.as_str());
    let mut subject = ComposedRuntime::start(&a_id, &profile(&[&b], Provider::None), listen)
        .await
        .expect("a composes");
    assert_eq!(add(&subject, &b_addr).await, AddressChange::Added);
    wait_connected(&mut subject, &b).await;

    reset_trust(&subject, &b).await;
    wait_connected(&mut subject, &b).await;

    assert_eq!(remove(&subject, &b_addr).await, AddressChange::Removed);
    reset_trust(&subject, &b).await;
    assert!(
        tokio::time::timeout(Duration::from_secs(5), wait_connected(&mut subject, &b))
            .await
            .is_err(),
        "a removed address was dialled"
    );
    subject.shutdown().await.expect("clean shutdown");
    target.shutdown().await.expect("clean shutdown");
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint set's methods through the composition (IPC 3.0,
//! `LOCAL-IPC.md` §The endpoint set's methods): `admin.endpoints.add`,
//! `remove`, `set_advertise` and `set_alias` against a real runtime, the
//! overlay judged by the bytes on disk and by what the next start loads.
//! Every refusal sits beside the move that is made.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_local_client_api::{
    AdminPort, AliasChange, AliasRefusal, DataCapability, DataSessionBinding, DataSessionPort,
    EndpointAddChange, EndpointAddRefusal, EndpointRemoveChange, EndpointRemoveRefusal,
    EndpointSource, LocalSessionEvent, SessionEvent, SessionRequest, TrustChange,
};
use interweave_profile_config::endpoint_key;
use interweave_transport_api::{
    DirectDestination, DisconnectReason, MessageId, Payload, TransportError, TransportEvent,
    TransportIdentity, TransportRuntime,
};
use interweave_transport_composition::{ComposedRuntime, CompositionOptions};
use interweave_trust_api::{MAX_ENDPOINT_SET_ROWS, PeerTrustPolicy};

mod endpoint_set_common;

use endpoint_set_common::{
    Profile, add_minted, admin, bytes, ep, id, mint, options, paths, profile, profile_with, row_of,
};

const PATIENCE: Duration = Duration::from_secs(20);

async fn next_within(runtime: &mut ComposedRuntime) -> TransportEvent {
    match tokio::time::timeout(PATIENCE, runtime.next_event()).await {
        Ok(Some(event)) => event,
        Ok(None) => panic!("the runtime stopped"),
        Err(elapsed) => panic!("nothing within {PATIENCE:?} ({elapsed})"),
    }
}

async fn start(p: &Profile) -> ComposedRuntime {
    let (identity, _) = id();
    ComposedRuntime::start(&identity, &profile(), options(p))
        .await
        .expect("composes")
}

fn lease_request(endpoint: &interweave_transport_api::EndpointId) -> SessionRequest {
    SessionRequest::new(
        "human-client",
        Some(endpoint.clone()),
        [DataCapability::Events],
    )
    .expect("in bounds")
}

/// An add is registered and leasable from its answer, with no restart,
/// and listed as the runtime's: its alias, `added`, unadvertised. After a
/// restart of the runtime over the same state the endpoint is there and
/// leasable again: the add was written, not only published.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_add_is_leasable_at_once_and_after_a_restart() {
    let p = paths();
    let (identity, _) = id();
    let runtime = ComposedRuntime::start(&identity, &profile(), options(&p))
        .await
        .expect("composes");
    let port = admin(&runtime).await;
    let (work, key) = mint(&p);
    // The control: before the add, the id is no endpoint.
    assert_eq!(
        runtime.sessions().open(lease_request(&work)).await.err(),
        Some(TransportError::EndpointUnknown)
    );
    assert_eq!(
        port.add_endpoint(key, "work".into(), vec!["human-client".into()])
            .await,
        Ok(EndpointAddChange::Added(work.clone()))
    );
    let row = row_of(&port, &work).await.expect("listed");
    assert_eq!(row.alias.as_deref(), Some("work"));
    assert_eq!(row.source, EndpointSource::Added);
    assert!(row.enabled && !row.advertise && !row.default && row.persisted);
    assert_eq!(
        row_of(&port, &ep("human")).await.expect("listed").source,
        EndpointSource::Authored
    );
    let session = runtime
        .sessions()
        .open(lease_request(&work))
        .await
        .expect("leasable from the answer on");
    // Its client kinds are the entry's.
    assert_eq!(
        runtime
            .sessions()
            .open(
                SessionRequest::new("cli", Some(work.clone()), [DataCapability::Events])
                    .expect("in bounds")
            )
            .await
            .err(),
        Some(TransportError::EndpointClientKindDenied)
    );
    drop(session);
    drop(port);
    runtime.stop().await.expect("stops");

    let again = ComposedRuntime::start(&identity, &profile(), options(&p))
        .await
        .expect("restarts");
    let port = admin(&again).await;
    let row = row_of(&port, &work).await.expect("still there");
    assert_eq!(row.alias.as_deref(), Some("work"));
    assert_eq!(row.source, EndpointSource::Added);
    again
        .sessions()
        .open(lease_request(&work))
        .await
        .expect("leasable after the restart");
    drop(port);
    again.stop().await.expect("stops");
}

/// Every refusal of the add by name, nothing written: the overlay's bytes
/// are the same before and after each. A retry of an add made is
/// `unchanged`, nothing written. The control is the add that is made,
/// which changes the bytes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_add_refusal_writes_nothing_and_a_retry_is_unchanged() {
    let p = paths();
    let runtime = start(&p).await;
    let port = admin(&runtime).await;
    let first = add_minted(&p, &port, "taken").await;
    let (fresh, key) = mint(&p);
    let other = mint(&p).0;
    let keys = p.paths.state_dir().join("endpoint-keys");
    std::fs::copy(
        keys.join(endpoint_key::key_file_name(&other)),
        keys.join(endpoint_key::key_file_name(&ep("forged"))),
    )
    .expect("a header that derives another id");
    let cases = [
        ("not-a-key-file", "ok", EndpointAddRefusal::IdNotDerived),
        (
            key.as_str(),
            "Not An Alias",
            EndpointAddRefusal::AliasInvalid,
        ),
        (
            &endpoint_key::key_file_name(&ep("never-minted")),
            "ok",
            EndpointAddRefusal::KeyMissing,
        ),
        (
            &endpoint_key::key_file_name(&ep("forged")),
            "ok",
            EndpointAddRefusal::KeyMismatch,
        ),
        (key.as_str(), "taken", EndpointAddRefusal::AliasTaken),
        (key.as_str(), "agent", EndpointAddRefusal::AliasTaken),
    ];
    for (key, alias, want) in cases {
        let before = bytes(&p);
        let answer = port
            .add_endpoint(key.to_owned(), alias.to_owned(), vec![])
            .await
            .expect("answered");
        assert!(
            matches!(&answer, EndpointAddChange::Refused { reason, .. } if *reason == want),
            "{key} {alias}: {answer:?}"
        );
        assert_eq!(bytes(&p), before, "{want:?} wrote nothing");
    }
    // `exists`: the same id as the first add, under another alias.
    let before = bytes(&p);
    let first_key = endpoint_key::key_file_name(&first);
    assert!(matches!(
        port.add_endpoint(first_key.clone(), "elsewhere".into(), vec![])
            .await,
        Ok(EndpointAddChange::Refused {
            reason: EndpointAddRefusal::Exists,
            ..
        })
    ));
    assert_eq!(bytes(&p), before);
    // `unchanged`: the same add again, nothing written.
    assert_eq!(
        port.add_endpoint(first_key, "taken".into(), vec![]).await,
        Ok(EndpointAddChange::Unchanged(first.clone()))
    );
    assert_eq!(bytes(&p), before);
    // The control: the fresh key under a free alias is added and written.
    assert_eq!(
        port.add_endpoint(key, "free".into(), vec![]).await,
        Ok(EndpointAddChange::Added(fresh))
    );
    assert_ne!(bytes(&p), before);
    drop(port);
    runtime.stop().await.expect("stops");
}

/// Sixty-four endpoints, authored and added together, is the bound: the
/// 64th is added, the 65th refused `bound` with nothing written.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_sixty_fifth_endpoint_is_refused_bound() {
    let p = paths();
    let runtime = start(&p).await;
    let port = admin(&runtime).await;
    for i in 0..62 {
        add_minted(&p, &port, &format!("e{i}")).await;
    }
    assert_eq!(port.leases().await.expect("the list").len(), 64);
    let before = bytes(&p);
    let (_, key) = mint(&p);
    assert!(matches!(
        port.add_endpoint(key, "last".into(), vec![]).await,
        Ok(EndpointAddChange::Refused {
            reason: EndpointAddRefusal::Bound,
            ..
        })
    ));
    assert_eq!(bytes(&p), before);
    assert_eq!(port.leases().await.expect("the list").len(), 64);
    drop(port);
    runtime.stop().await.expect("stops");
}

/// An added endpoint admits nobody until its set names a peer: its two
/// sets are written empty, so a peer the operator's list trusts is
/// answered `no_route` at it, while the authored endpoint that inherits
/// the list accepts the same peer -- the control. Once the endpoint's set
/// names the peer, it accepts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_added_endpoint_admits_nobody_until_its_set_names_a_peer() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let p = paths();
    let (r_id, r) = id();
    let (s_id, s) = id();
    let listen = |base: CompositionOptions| CompositionOptions {
        listen: vec![format!("/ip4/{ip}/tcp/0")],
        ..base
    };
    let mut receiver = ComposedRuntime::start(
        &r_id,
        &profile_with(&[&s], &[], &[], 8),
        listen(options(&p)),
    )
    .await
    .expect("r composes");
    let port = admin(&receiver).await;
    let work = add_minted(&p, &port, "work").await;
    let _at_work = receiver
        .sessions()
        .open(lease_request(&work))
        .await
        .expect("leases work");
    let _at_human = receiver
        .sessions()
        .open(lease_request(&ep("human")))
        .await
        .expect("leases human");
    let r_addr = format!("{}/p2p/{}", receiver.listening()[0], r.as_str());
    let sender = ComposedRuntime::start(
        &s_id,
        &profile_with(&[&r], &[r_addr], &[], 8),
        listen(CompositionOptions::default()),
    )
    .await
    .expect("s composes");
    while !matches!(next_within(&mut receiver).await, TransportEvent::PeerConnected { peer, .. } if peer == s)
    {
    }
    let sending = sender
        .sessions()
        .open(
            SessionRequest::new(
                "human-client",
                Some(ep("human")),
                [DataCapability::Commands],
            )
            .expect("in bounds"),
        )
        .await
        .expect("s leases");
    let send = |to: &interweave_transport_api::EndpointId, tag: u8| {
        sending.send_direct(
            DirectDestination {
                peer: r.clone(),
                endpoint: Some(to.clone()),
            },
            MessageId::from_bytes([tag; 16]),
            Payload::at_ceiling(None, vec![tag]).expect("within the ceiling"),
        )
    };
    assert_eq!(
        tokio::time::timeout(PATIENCE, send(&ep("human"), 1))
            .await
            .expect("answered"),
        Ok(ep("human")),
        "the inheriting endpoint accepts the operator's peer"
    );
    assert_eq!(
        tokio::time::timeout(PATIENCE, send(&work, 2))
            .await
            .expect("answered"),
        Err(TransportError::RemoteEndpointUnavailable),
        "the added endpoint's own set is empty"
    );
    assert_eq!(
        port.set_endpoint_trust(work.clone(), s.clone(), true).await,
        Ok(TrustChange::Set)
    );
    assert_eq!(
        tokio::time::timeout(PATIENCE, send(&work, 3))
            .await
            .expect("answered"),
        Ok(work.clone())
    );
    drop((sending, port));
}

/// A remove revokes the live lease -- its holder told the epoch the answer
/// names -- and closes, as `policy`, a peer only the removed endpoint's set
/// held; a peer another set still holds stays connected (the control).
/// `authored` and `default` are refused, nothing written; an id no entry
/// holds is `EndpointUnknown`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remove_revokes_the_lease_and_closes_only_the_peer_it_held() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let p = paths();
    let (a_id, a) = id();
    let (b_id, b) = id();
    let (c_id, c) = id();
    let listen = |base: CompositionOptions| CompositionOptions {
        listen: vec![format!("/ip4/{ip}/tcp/0")],
        ..base
    };
    let mut subject = ComposedRuntime::start(&a_id, &profile(), listen(options(&p)))
        .await
        .expect("a composes");
    let port = admin(&subject).await;
    let work = add_minted(&p, &port, "work").await;
    // B only in work's set; C in work's and human's.
    for (endpoint, peer) in [(&work, &b), (&work, &c), (&ep("human"), &c)] {
        assert_eq!(
            port.set_endpoint_trust(endpoint.clone(), peer.clone(), true)
                .await,
            Ok(TrustChange::Set)
        );
    }
    let a_addr = format!("{}/p2p/{}", subject.listening()[0], a.as_str());
    let peer_b = ComposedRuntime::start(
        &b_id,
        &profile_with(&[&a], std::slice::from_ref(&a_addr), &[], 8),
        listen(CompositionOptions::default()),
    )
    .await
    .expect("b composes");
    let peer_c = ComposedRuntime::start(
        &c_id,
        &profile_with(&[&a], &[a_addr], &[], 8),
        listen(CompositionOptions::default()),
    )
    .await
    .expect("c composes");
    let mut connected = std::collections::BTreeSet::new();
    while connected.len() < 2 {
        if let TransportEvent::PeerConnected { peer, .. } = next_within(&mut subject).await {
            connected.insert(peer);
        }
    }
    let holder = subject
        .sessions()
        .open(lease_request(&work))
        .await
        .expect("leases work");
    let epoch = holder
        .session()
        .endpoint_lease()
        .expect("a lease")
        .epoch
        .clone();

    // The refusals first, nothing written.
    let before = bytes(&p);
    assert!(matches!(
        port.remove_endpoint(ep("human")).await,
        Ok(EndpointRemoveChange::Refused {
            reason: EndpointRemoveRefusal::Authored,
            ..
        })
    ));
    assert_eq!(
        port.remove_endpoint(ep("nowhere")).await,
        Err(TransportError::EndpointUnknown)
    );
    assert_eq!(bytes(&p), before);
    port.set_default_endpoint(Some(work.clone()))
        .await
        .expect("work is the default");
    let defaulted = bytes(&p);
    assert!(matches!(
        port.remove_endpoint(work.clone()).await,
        Ok(EndpointRemoveChange::Refused {
            reason: EndpointRemoveRefusal::Default,
            ..
        })
    ));
    assert_eq!(bytes(&p), defaulted);
    port.set_default_endpoint(Some(ep("human")))
        .await
        .expect("the default moved back");

    assert_eq!(
        port.remove_endpoint(work.clone()).await,
        Ok(EndpointRemoveChange::Removed {
            revoked_epoch: Some(epoch.clone())
        })
    );
    tokio::time::timeout(PATIENCE, holder.ready())
        .await
        .expect("the holder is owed its notice")
        .expect("ready");
    let notices = holder.events(16).await.expect("reads");
    assert!(
        notices.iter().any(|e| matches!(
            e,
            SessionEvent::Local(LocalSessionEvent::EndpointLeaseChanged { endpoint, revoked_epoch })
                if *endpoint == work && *revoked_epoch == epoch
        )),
        "{notices:?}"
    );
    let reason = loop {
        if let TransportEvent::PeerDisconnected {
            peer, reason_class, ..
        } = next_within(&mut subject).await
        {
            assert_ne!(peer, c, "human's set still holds C");
            if peer == b {
                break reason_class;
            }
        }
    };
    assert_eq!(reason, DisconnectReason::Policy);
    let quiet = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(TransportEvent::PeerDisconnected { peer, .. }) = subject.next_event().await
                && peer == c
            {
                return;
            }
        }
    })
    .await;
    assert!(quiet.is_err(), "C stays: another set holds it");
    assert!(row_of(&port, &work).await.is_none(), "gone from the list");
    drop((holder, port, peer_b, peer_c));
}

/// `count` distinct synthetic peers (each a generated identity's).
fn peers(count: usize) -> Vec<TransportIdentity> {
    (0..count).map(|_| id().1).collect()
}

/// The remove drops the endpoint's own sets IN THE SAME WRITE: read from
/// the file after the answer, the id is nowhere in it. And it frees their
/// rows: with the overlay's sets at `MAX_ENDPOINT_SET_ROWS`, a write that
/// would add rows is refused; after removing the endpoint that held half
/// of them, the same write is made.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remove_drops_its_sets_in_the_same_write_and_frees_their_rows() {
    let p = paths();
    let full = peers(PeerTrustPolicy::MAX_ALLOWED_PEERS);
    let list: Vec<&str> = full.iter().map(TransportIdentity::as_str).collect();
    let (work, work_key) = mint(&p);
    let sets = serde_json::json!({
        ep("human").as_str(): list,
        work.as_str(): list,
    });
    let file = serde_json::json!({
        "enabled": {},
        "added": [{"key": work_key, "alias": "work", "enabled": true, "advertise": false}],
        "inbound": sets,
        "outbound": sets,
    });
    std::fs::write(&p.overlay, file.to_string()).expect("written");
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&p.overlay, std::fs::Permissions::from_mode(0o600))
            .expect("private");
    }
    assert_eq!(
        4 * PeerTrustPolicy::MAX_ALLOWED_PEERS,
        MAX_ENDPOINT_SET_ROWS
    );
    let runtime = start(&p).await;
    let port = admin(&runtime).await;
    // `agent` has no own set: a first write gives it one, two rows past
    // the bound. The peer is already in the admission set, so the rows
    // alone refuse it.
    assert_eq!(
        port.set_endpoint_trust(ep("agent"), full[0].clone(), true)
            .await,
        Err(TransportError::InvalidArgument)
    );
    assert!(matches!(
        port.remove_endpoint(work.clone()).await,
        Ok(EndpointRemoveChange::Removed { .. })
    ));
    let text = std::fs::read_to_string(&p.overlay).expect("read");
    assert!(
        !text.contains(work.as_str()),
        "no member keyed by the id is left"
    );
    assert!(text.contains(ep("human").as_str()), "human's sets stay");
    assert_eq!(
        port.set_endpoint_trust(ep("agent"), full[0].clone(), true)
            .await,
        Ok(TrustChange::Set),
        "the freed rows take the write"
    );
    drop(port);
    runtime.stop().await.expect("stops");
}

/// `set_advertise` changes the directory's answer BEFORE it returns: a
/// querier that asks after the answer, and never asked before, is listed
/// the endpoint; one that asked before it was advertised was not (the
/// control). The authored delta is kept in the file while it differs and
/// dropped once it equals the authored value again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_advertise_is_in_the_directory_before_it_answers() {
    let ip = interweave_test_support::net::require_private_interface_v4();
    let p = paths();
    let (a_id, a) = id();
    let (q1_id, q1) = id();
    let (q2_id, q2) = id();
    let listen = |base: CompositionOptions| CompositionOptions {
        listen: vec![format!("/ip4/{ip}/tcp/0")],
        ..base
    };
    let mut subject = ComposedRuntime::start(
        &a_id,
        &profile_with(&[&q1, &q2], &[], &[], 8),
        listen(options(&p)),
    )
    .await
    .expect("a composes");
    let port = admin(&subject).await;
    let _leased = subject
        .sessions()
        .open(lease_request(&ep("human")))
        .await
        .expect("leases human");
    let a_addr = format!("{}/p2p/{}", subject.listening()[0], a.as_str());
    let querier_profile = profile_with(&[&a], &[a_addr], &[], 8);
    let first = ComposedRuntime::start(
        &q1_id,
        &querier_profile,
        listen(CompositionOptions::default()),
    )
    .await
    .expect("q1 composes");
    let second = ComposedRuntime::start(
        &q2_id,
        &querier_profile,
        listen(CompositionOptions::default()),
    )
    .await
    .expect("q2 composes");
    let mut connected = std::collections::BTreeSet::new();
    while connected.len() < 2 {
        if let TransportEvent::PeerConnected { peer, .. } = next_within(&mut subject).await {
            connected.insert(peer);
        }
    }
    let query = |runtime: &ComposedRuntime| {
        let sessions = runtime.sessions();
        let a = a.clone();
        async move {
            let session = sessions
                .open(
                    SessionRequest::new("human-client", None, [DataCapability::EndpointsQuery])
                        .expect("in bounds"),
                )
                .await
                .expect("a session");
            tokio::time::timeout(PATIENCE, session.query_endpoints(a))
                .await
                .expect("answered")
                .expect("a directory")
                .endpoints
        }
    };
    assert_eq!(query(&first).await, vec![], "not advertised yet");
    assert_eq!(port.set_endpoint_advertise(ep("human"), true).await, Ok(()));
    let file: serde_json::Value =
        serde_json::from_slice(&bytes(&p).expect("written")).expect("json");
    assert_eq!(
        file["advertise"][ep("human").as_str()],
        serde_json::json!(true)
    );
    assert_eq!(
        query(&second).await,
        vec![ep("human")],
        "in force when the answer returned"
    );
    assert!(
        row_of(&port, &ep("human")).await.expect("listed").advertise,
        "the list reads the effective value"
    );
    assert_eq!(
        port.set_endpoint_advertise(ep("human"), false).await,
        Ok(())
    );
    let file: serde_json::Value =
        serde_json::from_slice(&bytes(&p).expect("written")).expect("json");
    assert!(
        file.get("advertise").is_none(),
        "a delta equal to the authored value is not kept: {file}"
    );
    drop((port, first, second));
}

/// A value the composed profile would refuse -- past
/// `directory.max_advertised` -- is `InvalidArgument`, nothing written;
/// a value already in effect writes nothing; an id no entry holds is
/// `EndpointUnknown`. The control: once the advertised one is withdrawn,
/// the same set is made. An added endpoint's value is its entry's.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_advertise_refuses_what_the_profile_would_refuse() {
    let p = paths();
    let (identity, _) = id();
    let runtime = ComposedRuntime::start(
        &identity,
        &profile_with(&[], &[], &["human"], 1),
        options(&p),
    )
    .await
    .expect("composes");
    let port = admin(&runtime).await;
    let before = bytes(&p);
    assert_eq!(
        port.set_endpoint_advertise(ep("agent"), true).await,
        Err(TransportError::InvalidArgument)
    );
    assert_eq!(
        port.set_endpoint_advertise(ep("human"), true).await,
        Ok(()),
        "already in effect"
    );
    assert_eq!(
        port.set_endpoint_advertise(ep("nowhere"), true).await,
        Err(TransportError::EndpointUnknown)
    );
    assert_eq!(bytes(&p), before, "nothing written");
    assert_eq!(
        port.set_endpoint_advertise(ep("human"), false).await,
        Ok(())
    );
    assert_eq!(port.set_endpoint_advertise(ep("agent"), true).await, Ok(()));
    let work = add_minted(&p, &port, "work").await;
    assert_eq!(
        port.set_endpoint_advertise(work.clone(), true).await,
        Err(TransportError::InvalidArgument),
        "the cap counts added endpoints too"
    );
    assert_eq!(
        port.set_endpoint_advertise(ep("agent"), false).await,
        Ok(())
    );
    assert_eq!(
        port.set_endpoint_advertise(work.clone(), true).await,
        Ok(())
    );
    let file: serde_json::Value =
        serde_json::from_slice(&bytes(&p).expect("written")).expect("json");
    assert_eq!(file["added"][0]["advertise"], serde_json::json!(true));
    assert!(row_of(&port, &work).await.expect("listed").advertise);
    drop(port);
    runtime.stop().await.expect("stops");
}

/// `set_alias`: `authored` before an invalid alias, invalid before taken;
/// taken against an added and an authored entry; `unchanged` for the
/// entry's own; each writing nothing. The rename is written and the
/// runtime's list reads it; an id no entry holds is `EndpointUnknown`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_alias_renames_an_added_entry_and_refuses_in_order() {
    let p = paths();
    let runtime = start(&p).await;
    let port = admin(&runtime).await;
    let work = add_minted(&p, &port, "work").await;
    add_minted(&p, &port, "home").await;
    let before = bytes(&p);
    for (endpoint, alias, want) in [
        (ep("human"), "Not An Alias", AliasRefusal::Authored),
        (work.clone(), "Not An Alias", AliasRefusal::AliasInvalid),
        (work.clone(), "home", AliasRefusal::AliasTaken),
        (work.clone(), "agent", AliasRefusal::AliasTaken),
    ] {
        let answer = port
            .set_endpoint_alias(endpoint, alias.to_owned())
            .await
            .expect("answered");
        assert!(
            matches!(&answer, AliasChange::Refused { reason, .. } if *reason == want),
            "{alias}: {answer:?}"
        );
    }
    assert_eq!(
        port.set_endpoint_alias(work.clone(), "work".into()).await,
        Ok(AliasChange::Unchanged)
    );
    assert_eq!(
        port.set_endpoint_alias(ep("nowhere"), "x".into()).await,
        Err(TransportError::EndpointUnknown)
    );
    assert_eq!(bytes(&p), before, "nothing written");
    assert_eq!(
        port.set_endpoint_alias(work.clone(), "office".into()).await,
        Ok(AliasChange::Renamed)
    );
    assert_ne!(bytes(&p), before);
    assert_eq!(
        row_of(&port, &work).await.expect("listed").alias.as_deref(),
        Some("office")
    );
    drop(port);
    runtime.stop().await.expect("stops");
}

/// A runtime with no state directory (a test construction) reads no key
/// file, so a well-formed key name is `key_missing`; the name's own check
/// still comes first, so a name that derives no id is `id_not_derived`,
/// the contract's first refusal, as with a state directory.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_state_directory_the_names_check_still_comes_first() {
    let (identity, _) = id();
    let runtime = ComposedRuntime::start(&identity, &profile(), CompositionOptions::default())
        .await
        .expect("composes");
    let port = admin(&runtime).await;
    let refused = |answer: Result<EndpointAddChange, TransportError>| match answer {
        Ok(EndpointAddChange::Refused { reason, .. }) => reason,
        other => panic!("not refused: {other:?}"),
    };
    assert_eq!(
        refused(
            port.add_endpoint("not-a-key".to_owned(), "work".to_owned(), vec![])
                .await
        ),
        EndpointAddRefusal::IdNotDerived
    );
    // The control: a name that derives an id, whose file this runtime
    // cannot read.
    let (_, key) = mint(&paths());
    assert_eq!(
        refused(port.add_endpoint(key, "work".to_owned(), vec![]).await),
        EndpointAddRefusal::KeyMissing
    );
    drop(port);
    runtime.stop().await.expect("stops");
}

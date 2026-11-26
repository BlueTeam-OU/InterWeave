// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint set's methods are answered one at a time (`LOCAL-IPC.md`
//! §The endpoint set's methods, ADR-0061 A 2026-11-26 (ii)): an add's key
//! read, its overlay write and its publish are one answer of the driver,
//! the key file read in that answer's turn and not when the request is
//! accepted, so an add and a remove of one id never overlap, whichever
//! admin connection each came on.
//!
//! What would break it compiles and passes every other endpoint suite: an
//! answer run beside another (a driver that spawned `answer` per request,
//! or polled the request channel while an answer awaited its publish), or
//! the key read moved to where the request is accepted. Each test here
//! holds an add's publish open (`ComposedRuntime::hold_next_add_publish`)
//! and puts the other requests in the driver's channel meanwhile, each
//! from its own admin port.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use interweave_local_client_api::{
    AdminPort, EndpointAddChange, EndpointAddRefusal, EndpointRemoveChange,
};
use interweave_profile_config::endpoint_key;
use interweave_transport_api::TransportError;
use interweave_transport_composition::ComposedRuntime;

mod endpoint_set_common;

use endpoint_set_common::{Profile, admin, bytes, id, mint, options, paths, profile, row_of};

/// How long a request queued behind a held add is watched for an answer
/// it must not give. The hold is not timed -- the driver waits on it --
/// so this bounds only how long the test gives a broken driver to answer.
const QUEUED: Duration = Duration::from_millis(500);

const PATIENCE: Duration = Duration::from_secs(20);

async fn start(p: &Profile) -> ComposedRuntime {
    let (identity, _) = id();
    ComposedRuntime::start(&identity, &profile(), options(p))
        .await
        .expect("composes")
}

/// Poll `f` once without a waker: an admin call's first poll sends its
/// request into the driver's channel (room for 64), so after a `Pending`
/// the request is queued -- accepted -- and the call only waits for its
/// answer.
fn poll_once<T>(f: &mut Pin<Box<impl Future<Output = T> + ?Sized>>) -> Poll<T> {
    f.as_mut().poll(&mut Context::from_waker(Waker::noop()))
}

async fn within<T>(f: impl Future<Output = T>) -> T {
    tokio::time::timeout(PATIENCE, f)
        .await
        .expect("answered within the patience")
}

/// A remove sent while an add of the same id is in its publish waits for
/// the add's answer and then answers `removed`: the entry it removes is
/// the one the add wrote, which the driver holds only once the add has
/// answered (`set_endpoints_noting` adopts the overlay last). A remove
/// answered beside the add would find no entry and answer
/// `EndpointUnknown` -- the control below shows that answer is what the
/// same remove gets with no add in flight.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remove_beside_a_held_add_answers_removed_after_the_add() {
    let p = paths();
    let runtime = start(&p).await;
    let adding = admin(&runtime).await;
    let removing = admin(&runtime).await;
    let (work, key) = mint(&p);

    let mut hold = runtime.hold_next_add_publish().await.expect("held");
    let mut add = Box::pin(adding.add_endpoint(key, "work".to_owned(), vec![]));
    assert!(poll_once(&mut add).is_pending(), "the add is queued");
    within(hold.reached())
        .await
        .expect("the add reached its publish");
    // Its overlay is written before its publish: the hold is inside it.
    assert!(
        String::from_utf8(bytes(&p).expect("the overlay written"))
            .expect("text")
            .contains(work.as_str())
    );

    let mut remove = Box::pin(removing.remove_endpoint(work.clone()));
    assert!(poll_once(&mut remove).is_pending(), "the remove is queued");
    // The add is held; the remove behind it must not answer meanwhile.
    assert!(
        tokio::time::timeout(QUEUED, &mut remove).await.is_err(),
        "the remove answered while the add was in flight"
    );
    assert!(poll_once(&mut add).is_pending(), "the add is still held");

    hold.release();
    assert_eq!(
        within(&mut add).await,
        Ok(EndpointAddChange::Added(work.clone()))
    );
    match within(&mut remove).await {
        Ok(EndpointRemoveChange::Removed { .. }) => {}
        other => panic!("not removed: {other:?}"),
    }
    assert!(row_of(&adding, &work).await.is_none(), "the id is gone");
    assert!(
        !String::from_utf8(bytes(&p).expect("the overlay"))
            .expect("text")
            .contains(work.as_str())
    );
    drop((add, remove));
    drop((adding, removing));
    runtime.stop().await.expect("stops");
}

/// The control: the same remove, for a minted key no add was sent for,
/// answers `EndpointUnknown` and writes nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_remove_with_no_add_in_flight_is_endpoint_unknown() {
    let p = paths();
    let runtime = start(&p).await;
    let removing = admin(&runtime).await;
    let (work, _key) = mint(&p);
    let before = bytes(&p);
    assert_eq!(
        within(removing.remove_endpoint(work.clone())).await,
        Err(TransportError::EndpointUnknown)
    );
    assert_eq!(bytes(&p), before, "nothing written");
    drop(removing);
    runtime.stop().await.expect("stops");
}

/// The client's discard (ADR-0061 decision 7): it empties the key path,
/// then sends the remove. An add of that key accepted BEFORE the path was
/// emptied, and one sent after the remove, both answered after it, read
/// the key path as it is at their turn: each is refused `key_missing`,
/// the remove answers `EndpointUnknown`, and nothing of the id is
/// written. The held add of another key, which keeps them all queued, is
/// added: the positive control that an add accepted the same way, its
/// key file in place, is written.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_add_answered_after_its_key_path_was_emptied_is_key_missing() {
    let p = paths();
    let runtime = start(&p).await;
    let first_port = admin(&runtime).await;
    let early_port = admin(&runtime).await;
    let removing = admin(&runtime).await;
    let late_port = admin(&runtime).await;
    let (first, first_key) = mint(&p);
    let (gone, gone_key) = mint(&p);

    let mut hold = runtime.hold_next_add_publish().await.expect("held");
    let mut add_first = Box::pin(first_port.add_endpoint(first_key, "first".to_owned(), vec![]));
    assert!(poll_once(&mut add_first).is_pending());
    within(hold.reached()).await.expect("the first add is held");

    // Accepted while the key file is there.
    let mut early = Box::pin(early_port.add_endpoint(gone_key.clone(), "gone".to_owned(), vec![]));
    assert!(
        poll_once(&mut early).is_pending(),
        "the early add is queued"
    );
    // The discard: the key path emptied, then the remove.
    std::fs::remove_file(endpoint_key::key_path(&p.paths, &gone)).expect("the key path emptied");
    let mut remove = Box::pin(removing.remove_endpoint(gone.clone()));
    assert!(poll_once(&mut remove).is_pending(), "the remove is queued");
    let mut late = Box::pin(late_port.add_endpoint(gone_key, "gone".to_owned(), vec![]));
    assert!(poll_once(&mut late).is_pending(), "the late add is queued");

    hold.release();
    assert_eq!(
        within(&mut add_first).await,
        Ok(EndpointAddChange::Added(first.clone()))
    );
    for (which, answer) in [
        ("early", within(&mut early).await),
        ("late", within(&mut late).await),
    ] {
        match answer {
            Ok(EndpointAddChange::Refused { reason, .. }) => {
                assert_eq!(reason, EndpointAddRefusal::KeyMissing, "the {which} add");
            }
            other => panic!("the {which} add was not refused: {other:?}"),
        }
    }
    assert_eq!(
        within(&mut remove).await,
        Err(TransportError::EndpointUnknown)
    );
    let overlay = String::from_utf8(bytes(&p).expect("the overlay")).expect("text");
    assert!(overlay.contains(first.as_str()), "the held add was written");
    assert!(
        !overlay.contains(gone.as_str()),
        "nothing of the emptied id"
    );
    assert!(row_of(&first_port, &gone).await.is_none());
    drop((add_first, early, remove, late));
    drop((first_port, early_port, removing, late_port));
    runtime.stop().await.expect("stops");
}

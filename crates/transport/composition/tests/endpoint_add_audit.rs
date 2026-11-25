// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The audit line of a refused `admin.endpoints.add` (LOCAL-IPC.md: the
//! endpoint set's methods are audited as `admin.trust.set` is) names the
//! endpoint whenever the key's name derives one: every refusal after
//! `id_not_derived` carries the id, and `id_not_derived` none.
//!
//! Its own test binary: a GLOBAL subscriber, since the driver logs from
//! the runtime's worker threads.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use interweave_local_client_api::{AdminPort as _, EndpointAddChange, EndpointAddRefusal};
use interweave_transport_composition::ComposedRuntime;

mod endpoint_set_common;

use endpoint_set_common::{admin, id, mint, options, paths, profile};

/// A refusal past `id_not_derived` -- `alias_invalid`, the next one,
/// for a minted key -- is audited with the id the key's name derives; an
/// `id_not_derived` one with none, since there is none to name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_add_is_audited_with_the_id_its_key_derives() {
    let _ = logged();
    let p = paths();
    let (identity, _) = id();
    let runtime = ComposedRuntime::start(&identity, &profile(), options(&p))
        .await
        .expect("composes");
    let port = admin(&runtime).await;
    let (work, key) = mint(&p);
    let refused = |change| match change {
        Ok(EndpointAddChange::Refused { reason, .. }) => reason,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        refused(
            port.add_endpoint(key.clone(), "Bad Alias".to_owned(), vec![])
                .await
        ),
        EndpointAddRefusal::AliasInvalid
    );
    assert_eq!(
        refused(
            port.add_endpoint("not-a-key".to_owned(), "Bad Alias".to_owned(), vec![])
                .await
        ),
        EndpointAddRefusal::IdNotDerived
    );
    drop(port);
    runtime.stop().await.expect("stops");

    let text = logged();
    let audit: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("admin.endpoints.add") && l.contains("outcome=\"refused\""))
        .collect();
    assert_eq!(audit.len(), 2, "{text}");
    assert!(
        audit[0].contains("refused=\"alias_invalid\"")
            && audit[0].contains(&format!("endpoint=\"{}\"", work.as_str())),
        "{}",
        audit[0]
    );
    assert!(
        audit[1].contains("refused=\"id_not_derived\"") && audit[1].contains("endpoint=\"\""),
        "{}",
        audit[1]
    );
}

/// Everything the runtimes in this binary have logged so far. ONE
/// subscriber for the whole binary, installed globally once: per-test
/// scoped subscribers on two threads raced on tracing's callsite interest
/// cache and an audit line went missing under `--test-threads 2`.
fn logged() -> String {
    static LOG: std::sync::OnceLock<Arc<Mutex<Vec<u8>>>> = std::sync::OnceLock::new();
    let log = LOG.get_or_init(|| {
        let log = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || Writer(Arc::clone(&sink)))
            .with_ansi(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("the one subscriber");
        log
    });
    String::from_utf8(log.lock().expect("lock").clone()).expect("utf-8")
}

struct Writer(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("lock").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

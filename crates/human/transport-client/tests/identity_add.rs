// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The add an identity's form asks (`design/2026-11-04-identities.md`
//! §6.2; ADR-0061 decision 7, A 2026-11-26 and A 2026-11-27), over a
//! scripted admin port, a real store's pending-add record behind a
//! wrapper that can fail one write, and a real `endpoint-keys/` under a
//! private directory. Everything the client did, on the port, in the
//! store and on the key, goes to one log in the order it was done.
//! Isolation only: what the daemon's add does is proved beside it
//! (`crates/transport/composition/tests/endpoint_set.rs`).

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::collections::{BTreeSet, VecDeque};
use std::future::Future;
use std::os::unix::fs::PermissionsExt as _;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;

use interweave_human_store::identity_key::{self, CipherFailure, Custody, KeyCipher};
use interweave_human_store::{HumanStore, StoreError, StoreOptions};
use interweave_human_transport_client::{
    ADD_WAIT_MS, AddOutcome, Ended, FirstSeed, FormAdd, KeptBecause, Left, Local, PendingAddStore,
    SeedAsk, Settle, Settled,
};
use interweave_local_client_api::{
    AdminBinding, AdminCapability, AdminPort, AdminStatus, EndpointAddChange, EndpointAddRefusal,
    EndpointAdminView, EndpointRemoveChange, EndpointRemoveRefusal, EndpointSource, Generation,
    LocalAdminPort, TrustAdminView, TrustChange, TrustSeed,
};
use interweave_profile_config::endpoint_key;
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_config::{AliasError, EndpointAlias, TrustBoundary};
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};
use zeroize::Zeroizing;

/// One thing the client did, in the order it did it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    /// An admin connection opened, holding these.
    Opened(BTreeSet<AdminCapability>),
    /// An admin connection closed.
    Closed,
    /// `admin.endpoints.list` asked.
    Listed,
    /// `admin.trust.seed` asked for this endpoint.
    Seeded(EndpointId),
    /// `admin.endpoints.add` asked, and whether the key file stood at
    /// its key path when it was.
    Added {
        key: String,
        alias: String,
        kinds: Vec<String>,
        key_file: bool,
    },
    /// `admin.endpoints.remove` asked, and whether the key file stood at
    /// its key path when it was.
    Removed {
        endpoint: EndpointId,
        key_file: bool,
    },
    /// The record was written for this id.
    Recorded(EndpointId),
    /// The record's write for this id was refused.
    RecordRefused(EndpointId),
    /// The record's row for this id was deleted, and whether the key
    /// file stood at its key path when it was.
    Forgot {
        endpoint: EndpointId,
        key_file: bool,
    },
    /// A key was sealed under this platform alias.
    Sealed(String),
    /// The platform key under this alias was retired.
    Retired(String),
}

/// How the next call of a kind is answered.
#[derive(Debug, Clone)]
enum Reply<T> {
    Now(T),
    /// Not answered at the poll that sends it, and answered at the next:
    /// an answer that waits while something else runs.
    Later(T),
    /// Never answered: the call stays pending until it is dropped.
    Holds,
}

#[derive(Default)]
struct Script {
    steps: Vec<Step>,
    /// Every connection refuses to open with this.
    open_fails: Option<TransportError>,
    /// The next lists' answers; once empty, `listed`.
    lists: VecDeque<Reply<Result<(), TransportError>>>,
    /// The next adds' answers; once empty, `Added` of the key's id.
    adds: VecDeque<AddReply>,
    /// The next removes' answers; once empty, `Removed`.
    removes: VecDeque<Reply<Result<EndpointRemoveChange, TransportError>>>,
    /// The next seeds' answers; once empty, `seeded`.
    seeds: VecDeque<Reply<Result<TrustSeed, TransportError>>>,
    /// What a list answers.
    listed: Vec<EndpointAdminView>,
    /// What the next lists answer, each in place of `listed`.
    views: VecDeque<Vec<EndpointAdminView>>,
    /// Where key files are, for the steps that say whether one stood.
    paths: Option<ProfilePaths>,
    /// The next this many record writes are refused.
    refuse_writes: usize,
    /// Every record delete fails while this is set.
    refuse_forgets: bool,
    /// Every record read fails while this is set.
    refuse_reads: bool,
    /// Every platform key retire fails while this is set.
    refuse_retires: bool,
}

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Script>>);

impl Log {
    fn with<T>(&self, f: impl FnOnce(&mut Script) -> T) -> T {
        f(&mut self.0.lock().unwrap())
    }

    fn steps(&self) -> Vec<Step> {
        self.with(|s| s.steps.clone())
    }

    fn take(&self) -> Vec<Step> {
        self.with(|s| std::mem::take(&mut s.steps))
    }

    fn push(&self, step: Step) {
        self.with(|s| s.steps.push(step));
    }

    fn key_file(&self, endpoint: &EndpointId) -> bool {
        self.with(|s| {
            s.paths
                .as_ref()
                .is_some_and(|p| identity_key::key_path(p, endpoint).exists())
        })
    }

    fn adds(&self) -> Vec<Step> {
        self.steps()
            .into_iter()
            .filter(|s| matches!(s, Step::Added { .. }))
            .collect()
    }

    fn count(&self, f: impl Fn(&Step) -> bool) -> usize {
        self.steps().iter().filter(|s| f(s)).count()
    }
}

/// The admin side: connections that answer list, add and remove as
/// scripted.
#[derive(Clone)]
struct Admins(Log);

struct ScriptedPort {
    log: Log,
    port: LocalAdminPort,
}

impl Drop for ScriptedPort {
    fn drop(&mut self) {
        self.log.push(Step::Closed);
    }
}

impl AdminBinding for Admins {
    type Admin = ScriptedPort;

    fn admin(
        &self,
        capabilities: BTreeSet<AdminCapability>,
    ) -> impl Future<Output = Result<ScriptedPort, TransportError>> + Send {
        let log = self.0.clone();
        let opened = log.with(|s| {
            if let Some(error) = s.open_fails {
                return Err(error);
            }
            s.steps.push(Step::Opened(capabilities.clone()));
            Ok(())
        });
        std::future::ready(opened.map(|()| ScriptedPort {
            log,
            port: LocalAdminPort::new(
                Generation::parse("identity-add-admin-port").unwrap(),
                capabilities,
            ),
        }))
    }
}

/// A reply as a future: one answered now, or one never answered.
fn reply<T: Send + 'static>(reply: Reply<T>) -> impl Future<Output = T> + Send + 'static {
    let mut held = Some(reply);
    std::future::poll_fn(move |_| match held.take() {
        Some(Reply::Now(answer)) => Poll::Ready(answer),
        Some(Reply::Later(answer)) => {
            held = Some(Reply::Now(answer));
            Poll::Pending
        }
        Some(Reply::Holds) | None => {
            held = Some(Reply::Holds);
            Poll::Pending
        }
    })
}

impl AdminPort for ScriptedPort {
    fn port(&self) -> &LocalAdminPort {
        &self.port
    }
    fn leases(
        &self,
    ) -> impl Future<Output = Result<Vec<EndpointAdminView>, TransportError>> + Send {
        let answer = self.log.with(|s| {
            s.steps.push(Step::Listed);
            let views = s.views.pop_front().unwrap_or_else(|| s.listed.clone());
            match s.lists.pop_front().unwrap_or(Reply::Now(Ok(()))) {
                Reply::Now(Ok(())) => Reply::Now(Ok(views)),
                Reply::Later(Ok(())) => Reply::Later(Ok(views)),
                Reply::Now(Err(e)) => Reply::Now(Err(e)),
                Reply::Later(Err(e)) => Reply::Later(Err(e)),
                Reply::Holds => Reply::Holds,
            }
        });
        reply(answer)
    }
    fn add_endpoint(
        &self,
        key: String,
        alias: String,
        allowed_client_kinds: Vec<String>,
    ) -> impl Future<Output = Result<EndpointAddChange, TransportError>> + Send {
        let id = endpoint_key::parse_key_file_name(&key).expect("a key file's name");
        let key_file = self.log.key_file(&id);
        let answer = self.log.with(|s| {
            s.steps.push(Step::Added {
                key,
                alias,
                kinds: allowed_client_kinds,
                key_file,
            });
            s.adds
                .pop_front()
                .unwrap_or(Reply::Now(Ok(EndpointAddChange::Added(id))))
        });
        reply(answer)
    }
    fn remove_endpoint(
        &self,
        endpoint: EndpointId,
    ) -> impl Future<Output = Result<EndpointRemoveChange, TransportError>> + Send {
        let key_file = self.log.key_file(&endpoint);
        let answer = self.log.with(|s| {
            s.steps.push(Step::Removed { endpoint, key_file });
            s.removes
                .pop_front()
                .unwrap_or(Reply::Now(Ok(EndpointRemoveChange::Removed {
                    revoked_epoch: None,
                })))
        });
        reply(answer)
    }
    fn seed_endpoint_trust(
        &self,
        endpoint: EndpointId,
    ) -> impl Future<Output = Result<TrustSeed, TransportError>> + Send {
        let answer = self.log.with(|s| {
            s.steps.push(Step::Seeded(endpoint));
            s.seeds
                .pop_front()
                .unwrap_or(Reply::Now(Ok(TrustSeed::Seeded)))
        });
        reply(answer)
    }
    fn status(&self) -> impl Future<Output = Result<AdminStatus, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn revoke_endpoint(
        &self,
        _: EndpointId,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn set_endpoint_enabled(
        &self,
        _: EndpointId,
        _: bool,
    ) -> impl Future<Output = Result<Option<Generation>, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn set_default_endpoint(
        &self,
        _: Option<EndpointId>,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn trust(&self) -> impl Future<Output = Result<TrustAdminView, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn set_trust(
        &self,
        _: TransportIdentity,
        _: bool,
    ) -> impl Future<Output = Result<TrustChange, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn shutdown(&self, _: Duration) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
}

/// The real store's record, each call written to the log, a write or a
/// delete failed when the script says so.
struct Records {
    store: HumanStore,
    log: Log,
}

impl PendingAddStore for Records {
    fn add_pending_add(&mut self, endpoint: &EndpointId) -> Result<(), StoreError> {
        let refused = self.log.with(|s| {
            let refused = s.refuse_writes > 0;
            s.refuse_writes = s.refuse_writes.saturating_sub(1);
            refused
        });
        if refused {
            self.log.push(Step::RecordRefused(endpoint.clone()));
            return Err(StoreError::Degraded);
        }
        let written = self.store.add_pending_add(endpoint);
        self.log.push(if written.is_ok() {
            Step::Recorded(endpoint.clone())
        } else {
            Step::RecordRefused(endpoint.clone())
        });
        written
    }

    fn pending_adds(&self) -> Result<Vec<EndpointId>, StoreError> {
        if self.log.with(|s| s.refuse_reads) {
            return Err(StoreError::Degraded);
        }
        self.store.pending_adds()
    }

    fn forget_pending_add(&mut self, endpoint: &EndpointId) -> Result<bool, StoreError> {
        if self.log.with(|s| s.refuse_forgets) {
            return Err(StoreError::Degraded);
        }
        let key_file = self.log.key_file(endpoint);
        let forgot = self.store.forget_pending_add(endpoint)?;
        self.log.push(Step::Forgot {
            endpoint: endpoint.clone(),
            key_file,
        });
        Ok(forgot)
    }
}

/// The desktop's custody, its seal and retire written to the log.
struct Keys(Log);

impl KeyCipher for Keys {
    fn custody(&self) -> Custody {
        Custody::FilesystemOnly
    }
    fn seal(&self, alias: &str, _: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure> {
        self.0.push(Step::Sealed(alias.to_owned()));
        Ok(secret.to_vec())
    }
    fn open(&self, _: &str, _: &[u8], sealed: &[u8]) -> Result<Zeroizing<[u8; 32]>, CipherFailure> {
        let secret: [u8; 32] = sealed.try_into().map_err(|_| CipherFailure::Unauthentic)?;
        Ok(Zeroizing::new(secret))
    }
    fn retire(&self, alias: &str) -> Result<(), CipherFailure> {
        if self.0.with(|s| s.refuse_retires) {
            return Err(CipherFailure::KeyUnavailable);
        }
        self.0.push(Step::Retired(alias.to_owned()));
        Ok(())
    }
}

/// A test's alias rule: the name lowercased, `-2`, `-3` … past the
/// aliases in use. app-core's rule is proved beside it.
fn rule(name: &str, in_use: &[EndpointAlias]) -> Result<EndpointAlias, AliasError> {
    let base = name.to_ascii_lowercase();
    let mut n = 1;
    loop {
        let candidate = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        if !in_use.iter().any(|a| a.as_str() == candidate) {
            return EndpointAlias::parse(candidate);
        }
        n += 1;
    }
}

/// One test's device: a private state root, a store, the log.
struct Device {
    _dir: tempfile::TempDir,
    paths: ProfilePaths,
    records: Records,
    keys: Keys,
    log: Log,
}

impl Device {
    fn new() -> Self {
        let dir = common::private_tempdir();
        let boundary = TrustBoundary::new(dir.path()).expect("a boundary");
        let paths = ProfilePaths::resolve_embedded("default", boundary).expect("paths");
        let log = Log::default();
        log.with(|s| s.paths = Some(paths.clone()));
        let store = HumanStore::open_in_memory(&endpoint_id("human"), StoreOptions::default())
            .expect("store");
        Self {
            _dir: dir,
            paths,
            records: Records {
                store,
                log: log.clone(),
            },
            keys: Keys(log.clone()),
            log,
        }
    }

    fn local(&mut self) -> Local<'_, Records> {
        Local {
            store: &mut self.records,
            paths: &self.paths,
            cipher: &self.keys,
        }
    }

    fn form(&self) -> FormAdd<Admins> {
        FormAdd::new(
            Arc::new(Admins(self.log.clone())),
            vec!["human-client".to_owned()],
            rule,
        )
    }

    /// Poll `form` at `now`.
    fn poll(&mut self, form: &mut FormAdd<Admins>, now: u64) -> Option<AddOutcome> {
        self.poll_ended(form, now).map(|ended| ended.outcome)
    }

    fn poll_ended(&mut self, form: &mut FormAdd<Admins>, now: u64) -> Option<Ended> {
        let mut local = self.local();
        form.poll(now, &mut local)
    }

    fn admins(&self) -> Arc<Admins> {
        Arc::new(Admins(self.log.clone()))
    }

    /// A record and its key file, as a form whose add ended unknown left
    /// them: the key sealed under the device's custody.
    fn left_by_a_form(&mut self) -> EndpointId {
        let id = identity_key::create(&self.paths, &self.keys)
            .expect("a key")
            .endpoint()
            .clone();
        self.records.store.add_pending_add(&id).expect("a record");
        id
    }

    /// A settle of every record but `open`'s, polled at each of `times`
    /// until it ends.
    fn settle(&mut self, open: &[EndpointId], times: &[u64]) -> Option<Vec<Settled>> {
        let admins = self.admins();
        let mut local = self.local();
        let mut settle = Settle::of(&admins, &local, open).expect("the records read");
        times.iter().find_map(|now| settle.poll(*now, &mut local))
    }

    /// Start a Create of `name` and poll it once, expecting it to end.
    fn create(&mut self, form: &mut FormAdd<Admins>, name: &str, now: u64) -> AddOutcome {
        assert!(form.create(name, &[]), "a Create starts");
        self.poll(form, now)
            .expect("the Create ends without waiting")
    }

    fn leave(&mut self, form: FormAdd<Admins>) -> Left {
        let mut local = self.local();
        form.leave(&mut local)
    }

    fn records(&self) -> Vec<EndpointId> {
        self.records.store.pending_adds().expect("the record reads")
    }

    fn key_file(&self, endpoint: &EndpointId) -> bool {
        identity_key::key_path(&self.paths, endpoint).exists()
    }

    fn script(&self, f: impl FnOnce(&mut Script)) {
        self.log.with(f);
    }
}

fn view(endpoint: &EndpointId, alias: &str) -> EndpointAdminView {
    EndpointAdminView {
        endpoint: endpoint.clone(),
        enabled: true,
        default: false,
        persisted: true,
        lease: None,
        alias: Some(alias.to_owned()),
        source: EndpointSource::Added,
        advertise: false,
    }
}

/// How the next add is answered.
type AddReply = Reply<Result<EndpointAddChange, TransportError>>;

fn refused(reason: EndpointAddRefusal) -> AddReply {
    Reply::Now(Ok(EndpointAddChange::Refused {
        reason,
        detail: String::new(),
    }))
}

fn error(e: TransportError) -> AddReply {
    Reply::Now(Err(e))
}

fn position(steps: &[Step], f: impl Fn(&Step) -> bool) -> usize {
    steps.iter().position(f).expect("the step was taken")
}

/// Whether a list was read after the add.
fn listed_after_add(steps: &[Step]) -> bool {
    let added = position(steps, |s| matches!(s, Step::Added { .. }));
    steps[added..].contains(&Step::Listed)
}

#[test]
fn the_record_is_written_before_the_key_is_sealed_and_the_key_before_the_add() {
    let mut device = Device::new();
    let mut form = device.form();
    let outcome = device.create(&mut form, "Anna", 0);
    let id = form.endpoint().cloned().expect("a key was made");
    assert!(matches!(outcome, AddOutcome::Created { .. }), "{outcome:?}");
    let steps = device.log.steps();
    let recorded = position(&steps, |s| *s == Step::Recorded(id.clone()));
    let sealed = position(&steps, |s| matches!(s, Step::Sealed(_)));
    let added = position(&steps, |s| matches!(s, Step::Added { .. }));
    assert!(recorded < sealed && sealed < added, "{steps:?}");
    assert!(
        matches!(&steps[added], Step::Added { key_file: true, key, .. }
            if *key == endpoint_key::key_file_name(&id)),
        "the add names the key file, which stands: {steps:?}"
    );

    // A record that cannot be written seals nothing and sends no add.
    let mut device = Device::new();
    device.script(|s| s.refuse_writes = 1);
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Failed);
    let id = form.endpoint().cloned().expect("a key was made");
    assert_eq!(
        device.log.count(|s| matches!(s, Step::Sealed(_))),
        0,
        "nothing sealed"
    );
    assert!(device.log.adds().is_empty(), "no add");
    assert!(!device.key_file(&id), "no key file");
}

#[test]
fn every_call_opens_a_connection_of_its_own_holding_endpoints_alone() {
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(error(TransportError::Internal)));
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Unknown);
    let steps = device.log.steps();
    let opened: Vec<_> = steps
        .iter()
        .filter_map(|s| match s {
            Step::Opened(caps) => Some(caps.clone()),
            _ => None,
        })
        .collect();
    // The list before, the add, the list after.
    assert_eq!(
        opened,
        vec![BTreeSet::from([AdminCapability::Endpoints]); 3],
        "{steps:?}"
    );
    assert_eq!(
        device.log.count(|s| *s == Step::Closed),
        3,
        "each closed with its answer: {steps:?}"
    );
}

#[test]
fn each_answer_of_a_first_add_is_read_as_decided() {
    use EndpointAddRefusal as R;
    let cases: Vec<(&str, AddReply, AddOutcome)> = vec![
        ("alias_taken", refused(R::AliasTaken), AddOutcome::Failed),
        (
            "alias_invalid",
            refused(R::AliasInvalid),
            AddOutcome::Failed,
        ),
        (
            "id_not_derived",
            refused(R::IdNotDerived),
            AddOutcome::Failed,
        ),
        ("key_mismatch", refused(R::KeyMismatch), AddOutcome::Failed),
        ("bound", refused(R::Bound), AddOutcome::Limit),
        ("key_missing", refused(R::KeyMissing), AddOutcome::KeyGone),
        (
            "InvalidArgument",
            error(TransportError::InvalidArgument),
            AddOutcome::Failed,
        ),
        (
            "Internal",
            error(TransportError::Internal),
            AddOutcome::Unknown,
        ),
        (
            "BackendUnavailable",
            error(TransportError::BackendUnavailable),
            AddOutcome::Unknown,
        ),
        (
            "Timeout",
            error(TransportError::Timeout),
            AddOutcome::Unknown,
        ),
        (
            "ProtocolUnsupported",
            error(TransportError::ProtocolUnsupported),
            AddOutcome::Unknown,
        ),
    ];
    for (name, answer, want) in cases {
        let mut device = Device::new();
        device.script(|s| s.adds.push_back(answer));
        let mut form = device.form();
        let got = device.create(&mut form, "Anna", 0);
        assert_eq!(got, want, "{name}");
        let id = form.endpoint().cloned().unwrap();
        let steps = device.log.steps();
        assert_eq!(
            listed_after_add(&steps),
            want == AddOutcome::Unknown,
            "{name}: a list follows an unknown outcome and nothing else: {steps:?}"
        );
        assert_eq!(
            device.records(),
            vec![id.clone()],
            "{name}: the record stays"
        );
        assert!(
            device.key_file(&id),
            "{name}: the key stays until the form is left"
        );
    }

    // added and unchanged are the identity: the record goes.
    for answer in [
        EndpointAddChange::Added as fn(EndpointId) -> EndpointAddChange,
        EndpointAddChange::Unchanged,
    ] {
        let mut device = Device::new();
        let mut form = device.form();
        // The id the client takes is its own key's, whatever the answer
        // names.
        let other = endpoint_id("other");
        device.script(|s| s.adds.push_back(Reply::Now(Ok(answer(other)))));
        let got = device.create(&mut form, "Anna", 0);
        let id = form.endpoint().cloned().unwrap();
        assert_eq!(
            got,
            AddOutcome::Created {
                endpoint: id,
                alias: Some("anna".to_owned())
            }
        );
        assert!(device.records().is_empty(), "the record is deleted");
    }
}

#[test]
fn after_an_unknown_add_every_refusal_but_bound_and_key_missing_is_unknown() {
    use EndpointAddRefusal as R;
    let cases: Vec<(&str, AddReply, AddOutcome)> = vec![
        ("alias_taken", refused(R::AliasTaken), AddOutcome::Unknown),
        (
            "alias_invalid",
            refused(R::AliasInvalid),
            AddOutcome::Unknown,
        ),
        (
            "id_not_derived",
            refused(R::IdNotDerived),
            AddOutcome::Unknown,
        ),
        ("key_mismatch", refused(R::KeyMismatch), AddOutcome::Unknown),
        ("exists", refused(R::Exists), AddOutcome::Unknown),
        (
            "InvalidArgument",
            error(TransportError::InvalidArgument),
            AddOutcome::Unknown,
        ),
        ("bound", refused(R::Bound), AddOutcome::Limit),
        ("key_missing", refused(R::KeyMissing), AddOutcome::KeyGone),
    ];
    for (name, answer, want) in cases {
        let mut device = Device::new();
        device.script(|s| {
            s.adds.push_back(error(TransportError::Internal));
            s.adds.push_back(answer);
        });
        let mut form = device.form();
        assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Unknown);
        device.log.take();
        assert_eq!(device.create(&mut form, "Anna", 1), want, "{name}");
        let steps = device.log.steps();
        assert_eq!(
            listed_after_add(&steps),
            want == AddOutcome::Unknown,
            "{name}: {steps:?}"
        );
        // Leaving keeps both: an add of the key ended unknown.
        let id = form.endpoint().cloned().unwrap();
        assert_eq!(device.leave(form), Left::Kept(id.clone()), "{name}");
        assert!(device.key_file(&id), "{name}");
        assert_eq!(device.records(), vec![id], "{name}");
    }
}

#[test]
fn an_add_unanswered_for_the_wait_is_unknown_and_no_longer_waited_for() {
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(Reply::Holds));
    let mut form = device.form();
    let start = 1_000;
    assert!(form.create("Anna", &[]));
    assert_eq!(device.poll(&mut form, start), None, "the add is waited for");
    assert!(form.is_running());
    // The list before the add closed; the add's connection stays open.
    assert_eq!(device.log.count(|s| *s == Step::Closed), 1);
    assert_eq!(device.poll(&mut form, start + ADD_WAIT_MS - 1), None);
    assert_eq!(
        device.log.count(|s| *s == Step::Closed),
        1,
        "still waited for"
    );
    let added = device.log.steps().len();
    assert_eq!(
        device.poll(&mut form, start + ADD_WAIT_MS),
        Some(AddOutcome::Unknown),
        "no answer in the wait is an outcome not known"
    );
    let steps = device.log.steps();
    assert_eq!(
        steps[added],
        Step::Closed,
        "the add is dropped, closing its connection, before the list: {steps:?}"
    );
    assert_eq!(
        steps[added + 1],
        Step::Opened(BTreeSet::from([AdminCapability::Endpoints]))
    );
    assert!(!form.is_running());
}

#[test]
fn after_unknown_create_reads_the_list_and_sends_the_first_unknown_adds_alias() {
    let mut device = Device::new();
    device.script(|s| {
        s.adds.push_back(error(TransportError::Internal));
        s.adds.push_back(error(TransportError::Internal));
    });
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Unknown);
    device.log.take();
    // The person changed the name: the add sends the first unknown add's
    // alias all the same, after a list read.
    assert_eq!(device.create(&mut form, "Bruno", 1), AddOutcome::Unknown);
    let steps = device.log.steps();
    let listed = position(&steps, |s| *s == Step::Listed);
    let added = position(&steps, |s| matches!(s, Step::Added { .. }));
    assert!(listed < added, "the list first: {steps:?}");
    assert!(
        matches!(&steps[added], Step::Added { alias, .. } if alias == "anna"),
        "{steps:?}"
    );
    device.log.take();
    assert_eq!(
        device.create(&mut form, "Carla", 2),
        AddOutcome::Created {
            endpoint: form.endpoint().cloned().unwrap(),
            alias: Some("anna".to_owned()),
        }
    );
    assert!(
        matches!(&device.log.adds()[..], [Step::Added { alias, .. }] if alias == "anna"),
        "{:?}",
        device.log.steps()
    );
}

#[test]
fn found_is_the_id_listed_with_its_key_file_there() {
    for key_there in [true, false] {
        let mut device = Device::new();
        device.script(|s| s.adds.push_back(Reply::Holds));
        let mut form = device.form();
        assert!(form.create("Anna", &[]));
        assert_eq!(device.poll(&mut form, 0), None);
        let id = form.endpoint().cloned().unwrap();
        device.script(|s| s.listed = vec![view(&id, "anna-listed")]);
        if !key_there {
            std::fs::remove_file(identity_key::key_path(&device.paths, &id)).unwrap();
        }
        let got = device.poll(&mut form, ADD_WAIT_MS);
        if key_there {
            assert_eq!(
                got,
                Some(AddOutcome::Created {
                    endpoint: id.clone(),
                    alias: Some("anna-listed".to_owned()),
                })
            );
            assert!(device.records().is_empty(), "Found deletes the record");
        } else {
            assert_eq!(
                got,
                Some(AddOutcome::KeyGone),
                "listed without its key file"
            );
            assert_eq!(device.records(), vec![id], "the record stays");
        }
    }
}

#[test]
fn one_form_makes_one_key() {
    let mut device = Device::new();
    device.script(|s| {
        s.adds.push_back(error(TransportError::Internal));
        s.adds.push_back(refused(EndpointAddRefusal::Exists));
        s.adds.push_back(error(TransportError::BackendUnavailable));
    });
    let mut form = device.form();
    for now in 0..3 {
        assert_eq!(device.create(&mut form, "Anna", now), AddOutcome::Unknown);
    }
    let keys: BTreeSet<String> = device
        .log
        .adds()
        .into_iter()
        .map(|s| match s {
            Step::Added { key, .. } => key,
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(keys.len(), 1, "every add sends the one key: {keys:?}");
    assert_eq!(device.log.count(|s| matches!(s, Step::Sealed(_))), 1);
    assert_eq!(
        identity_key::sealed(&device.paths).unwrap(),
        vec![form.endpoint().cloned().unwrap()],
        "one key file"
    );
    assert_eq!(device.records().len(), 1, "one record");
}

#[test]
fn a_record_whose_delete_fails_ends_not_finished_and_the_next_create_sends_no_add() {
    let mut device = Device::new();
    device.script(|s| s.refuse_forgets = true);
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::NotFinished);
    let id = form.endpoint().cloned().unwrap();
    assert_eq!(
        device.records(),
        vec![id.clone()],
        "the id is no identity yet"
    );
    assert_eq!(device.log.adds().len(), 1);

    // The next Create reads the list and sends no add; the delete fails
    // again.
    device.script(|s| s.listed = vec![view(&id, "anna")]);
    device.log.take();
    assert_eq!(device.create(&mut form, "Anna", 1), AddOutcome::NotFinished);
    assert_eq!(
        device.log.steps().first(),
        Some(&Step::Opened(BTreeSet::from([AdminCapability::Endpoints])))
    );
    assert!(device.log.steps().contains(&Step::Listed));
    assert!(device.log.adds().is_empty(), "no add");

    // A list that fails changes nothing.
    device.script(|s| {
        s.lists
            .push_back(Reply::Now(Err(TransportError::BackendUnavailable)));
    });
    assert_eq!(device.create(&mut form, "Anna", 2), AddOutcome::NotFinished);

    // Then the delete lands: Done.
    device.script(|s| s.refuse_forgets = false);
    assert_eq!(
        device.create(&mut form, "Anna", 3),
        AddOutcome::Created {
            endpoint: id.clone(),
            alias: Some("anna".to_owned())
        }
    );
    assert!(device.log.adds().is_empty(), "still no add");
    assert!(device.records().is_empty());
    assert!(!form.create("Anna", &[]), "no Create after Done");

    // Not listed: the endpoint may have gone since, an outcome not known,
    // and the Create after it sends an add again.
    let mut device = Device::new();
    device.script(|s| s.refuse_forgets = true);
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::NotFinished);
    device.script(|s| s.refuse_forgets = false);
    device.log.take();
    assert_eq!(device.create(&mut form, "Anna", 1), AddOutcome::Unknown);
    assert!(device.log.adds().is_empty());
    assert!(matches!(
        device.create(&mut form, "Anna", 2),
        AddOutcome::Created { .. }
    ));
    assert_eq!(device.log.adds().len(), 1);
}

#[test]
fn a_first_add_answered_exists_is_read_from_the_list() {
    // Not listed: an add that ended unknown, and leaving keeps both.
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(refused(EndpointAddRefusal::Exists)));
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Unknown);
    let steps = device.log.steps();
    assert!(listed_after_add(&steps), "the list is read: {steps:?}");
    let id = form.endpoint().cloned().unwrap();
    // And the next add sends the alias that add sent.
    device.log.take();
    device.script(|s| s.adds.push_back(error(TransportError::Internal)));
    assert_eq!(device.create(&mut form, "Bruno", 1), AddOutcome::Unknown);
    assert!(
        matches!(&device.log.adds()[..], [Step::Added { alias, .. }] if alias == "anna"),
        "{:?}",
        device.log.steps()
    );
    assert_eq!(device.leave(form), Left::Kept(id.clone()));
    assert!(device.key_file(&id), "the key is kept");
    assert_eq!(device.records(), vec![id], "the record is kept");

    // Listed with its key file there: the identity.
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(Reply::Holds));
    let mut form = device.form();
    assert!(form.create("Anna", &[]));
    assert_eq!(device.poll(&mut form, 0), None);
    let id = form.endpoint().cloned().unwrap();
    // The held add times out unknown; the Create after it is answered
    // `exists` and finds the id.
    device.script(|s| s.lists.push_back(Reply::Now(Err(TransportError::Internal))));
    assert_eq!(
        device.poll(&mut form, ADD_WAIT_MS),
        Some(AddOutcome::Unknown)
    );
    device.script(|s| {
        s.lists.push_back(Reply::Now(Err(TransportError::Internal)));
        s.adds.push_back(refused(EndpointAddRefusal::Exists));
        s.listed = vec![view(&id, "anna")];
    });
    assert_eq!(
        device.create(&mut form, "Anna", ADD_WAIT_MS + 1),
        AddOutcome::Created {
            endpoint: id,
            alias: Some("anna".to_owned())
        }
    );
}

#[test]
fn an_add_whose_connection_did_not_open_was_not_sent() {
    let mut device = Device::new();
    device.script(|s| s.open_fails = Some(TransportError::BackendUnavailable));
    let mut form = device.form();
    assert_eq!(
        device.create(&mut form, "Anna", 0),
        AddOutcome::Failed,
        "an add never sent created nothing"
    );
    assert!(device.log.adds().is_empty());
    let id = form.endpoint().cloned().unwrap();
    assert_eq!(device.leave(form), Left::Deleted);
    assert!(!device.key_file(&id));
    assert!(device.records().is_empty());
}

#[test]
fn an_alias_refused_alias_taken_is_not_sent_again_by_the_form() {
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(refused(EndpointAddRefusal::AliasTaken)));
    let mut form = device.form();
    // The list lacks it (an entry left ahead holds it): the rule alone
    // would choose it again.
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Failed);
    assert!(matches!(
        device.create(&mut form, "Anna", 1),
        AddOutcome::Created { .. }
    ));
    let aliases: Vec<String> = device
        .log
        .adds()
        .into_iter()
        .map(|s| match s {
            Step::Added { alias, .. } => alias,
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(aliases, ["anna", "anna-2"]);
    assert_eq!(
        form.refused_aliases(),
        [EndpointAlias::parse("anna").unwrap()]
    );
    // And an alias the list holds is skipped too.
    let mut device = Device::new();
    device.script(|s| s.listed = vec![view(&endpoint_id("other"), "anna")]);
    let mut form = device.form();
    assert!(
        matches!(device.create(&mut form, "Anna", 0), AddOutcome::Created { alias: Some(a), .. } if a == "anna-2")
    );
}

#[test]
fn a_form_keeps_at_most_sixty_four_refused_aliases() {
    let mut device = Device::new();
    let mut form = device.form();
    for n in 0..70 {
        device.script(|s| s.adds.push_back(refused(EndpointAddRefusal::AliasTaken)));
        assert_eq!(device.create(&mut form, "Anna", n), AddOutcome::Failed);
    }
    assert_eq!(form.refused_aliases().len(), 64);
}

#[test]
fn leaving_after_a_known_refusal_deletes_the_key_then_the_record_and_sends_no_remove() {
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(refused(EndpointAddRefusal::AliasInvalid)));
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Failed);
    let id = form.endpoint().cloned().unwrap();
    device.log.take();
    assert_eq!(device.leave(form), Left::Deleted);
    let alias = identity_key::alias("default", &id);
    assert_eq!(
        device.log.steps(),
        vec![
            Step::Retired(alias),
            Step::Forgot {
                endpoint: id.clone(),
                key_file: false
            },
        ],
        "the key file, the platform key, then the record, and no remove"
    );
    assert!(!device.key_file(&id));
    assert!(device.records().is_empty());
}

#[test]
fn leaving_after_an_unknown_or_a_created_add_keeps_the_key_and_the_record() {
    // Unknown.
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(error(TransportError::Internal)));
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Unknown);
    let id = form.endpoint().cloned().unwrap();
    device.log.take();
    assert_eq!(device.leave(form), Left::Kept(id.clone()));
    assert!(
        device.log.steps().is_empty(),
        "nothing deleted, nothing sent"
    );
    assert!(device.key_file(&id));
    assert_eq!(device.records(), vec![id]);

    // Created, not finished.
    let mut device = Device::new();
    device.script(|s| s.refuse_forgets = true);
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::NotFinished);
    let id = form.endpoint().cloned().unwrap();
    device.script(|s| s.refuse_forgets = false);
    assert_eq!(device.leave(form), Left::Kept(id.clone()));
    assert!(device.key_file(&id));
    assert_eq!(device.records(), vec![id]);

    // A Create still running.
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(Reply::Holds));
    let mut form = device.form();
    assert!(form.create("Anna", &[]));
    assert_eq!(device.poll(&mut form, 0), None);
    let id = form.endpoint().cloned().unwrap();
    assert_eq!(device.leave(form), Left::Kept(id.clone()));
    assert!(device.key_file(&id));
    assert_eq!(device.records(), vec![id]);
}

#[test]
fn a_key_that_cannot_be_deleted_on_leaving_keeps_the_record() {
    let mut device = Device::new();
    device.script(|s| s.adds.push_back(refused(EndpointAddRefusal::AliasInvalid)));
    let mut form = device.form();
    assert_eq!(device.create(&mut form, "Anna", 0), AddOutcome::Failed);
    let id = form.endpoint().cloned().unwrap();
    device.script(|s| s.refuse_retires = true);
    assert_eq!(device.leave(form), Left::DeleteFailed(id.clone()));
    assert_eq!(
        device.records(),
        vec![id],
        "the record is kept for the settle"
    );
}

fn kept(endpoint: &EndpointId, because: KeptBecause) -> Settled {
    Settled::Kept {
        endpoint: endpoint.clone(),
        because,
    }
}

fn remove_steps(device: &Device) -> Vec<Step> {
    device
        .log
        .steps()
        .into_iter()
        .filter(|s| matches!(s, Step::Removed { .. }))
        .collect()
}

#[test]
fn a_failed_list_keeps_the_record_and_deletes_nothing() {
    for (failure, times) in [
        (
            Reply::Now(Err(TransportError::BackendUnavailable)),
            &[0][..],
        ),
        (Reply::Holds, &[0, ADD_WAIT_MS - 1, ADD_WAIT_MS][..]),
    ] {
        let mut device = Device::new();
        let id = device.left_by_a_form();
        device.log.take();
        device.script(|s| s.lists.push_back(failure));
        assert_eq!(
            device.settle(&[], times),
            Some(vec![kept(&id, KeptBecause::ListFailed)])
        );
        assert_eq!(device.records(), vec![id.clone()], "the record is kept");
        assert!(device.key_file(&id), "the key file is kept");
        assert!(
            !device.log.steps().iter().any(|s| matches!(
                s,
                Step::Removed { .. } | Step::Retired(_) | Step::Forgot { .. }
            )),
            "nothing deleted, no remove: {:?}",
            device.log.steps()
        );
    }
}

#[test]
fn a_listed_id_with_its_key_file_becomes_an_identity_at_the_records_delete() {
    let mut device = Device::new();
    let id = device.left_by_a_form();
    device.script(|s| s.listed = vec![view(&id, "anna")]);
    device.log.take();
    assert_eq!(
        device.settle(&[], &[0]),
        Some(vec![Settled::Identity {
            endpoint: id.clone(),
            alias: Some("anna".to_owned()),
        }])
    );
    assert!(device.records().is_empty(), "the record is deleted");
    assert!(device.key_file(&id), "the key is the identity's");
    assert!(remove_steps(&device).is_empty(), "no remove");

    // The delete fails: no identity yet, and nothing discarded.
    let mut device = Device::new();
    let id = device.left_by_a_form();
    device.script(|s| {
        s.listed = vec![view(&id, "anna")];
        s.refuse_forgets = true;
    });
    assert_eq!(
        device.settle(&[], &[0]),
        Some(vec![kept(&id, KeptBecause::RecordNotDeleted)])
    );
    assert_eq!(device.records(), vec![id.clone()]);
    assert!(device.key_file(&id));
    assert!(remove_steps(&device).is_empty());
}

#[test]
fn an_unlisted_id_is_discarded_key_path_platform_key_remove_then_record() {
    for answer in [
        Ok(EndpointRemoveChange::Removed {
            revoked_epoch: None,
        }),
        Err(TransportError::EndpointUnknown),
    ] {
        let mut device = Device::new();
        let id = device.left_by_a_form();
        device.log.take();
        device.script(|s| s.removes.push_back(Reply::Now(answer.clone())));
        assert_eq!(
            device.settle(&[], &[0]),
            Some(vec![Settled::Discarded(id.clone())]),
            "{answer:?}"
        );
        let steps: Vec<Step> = device
            .log
            .steps()
            .into_iter()
            .filter(|s| {
                matches!(
                    s,
                    Step::Retired(_) | Step::Removed { .. } | Step::Forgot { .. }
                )
            })
            .collect();
        assert_eq!(
            steps,
            vec![
                Step::Retired(identity_key::alias("default", &id)),
                Step::Removed {
                    endpoint: id.clone(),
                    key_file: false,
                },
                Step::Forgot {
                    endpoint: id.clone(),
                    key_file: false,
                },
            ],
            "the key path, the platform key, the remove, then the record: {answer:?}"
        );
        assert!(device.records().is_empty());
    }
}

#[test]
fn a_listed_id_whose_key_path_is_empty_is_discarded() {
    let mut device = Device::new();
    let id = device.left_by_a_form();
    std::fs::remove_file(identity_key::key_path(&device.paths, &id)).unwrap();
    device.script(|s| s.listed = vec![view(&id, "anna")]);
    assert_eq!(
        device.settle(&[], &[0]),
        Some(vec![Settled::Discarded(id.clone())])
    );
    assert_eq!(remove_steps(&device).len(), 1, "the remove is sent");
    assert!(device.records().is_empty());
}

#[test]
fn a_key_path_that_cannot_be_emptied_sends_no_remove_and_keeps_the_record() {
    // The platform key.
    let mut device = Device::new();
    let id = device.left_by_a_form();
    device.script(|s| s.refuse_retires = true);
    assert_eq!(
        device.settle(&[], &[0]),
        Some(vec![kept(&id, KeptBecause::KeyNotEmptied)])
    );
    assert!(remove_steps(&device).is_empty(), "no remove");
    assert_eq!(device.records(), vec![id]);

    // The key file, in a directory nobody may delete from.
    let mut device = Device::new();
    let id = device.left_by_a_form();
    let dir = device
        .paths
        .state_dir()
        .join(endpoint_key::ENDPOINT_KEYS_DIR);
    let mode = |m| std::fs::Permissions::from_mode(m);
    std::fs::set_permissions(&dir, mode(0o500)).unwrap();
    let settled = device.settle(&[], &[0]);
    std::fs::set_permissions(&dir, mode(0o700)).unwrap();
    assert_eq!(settled, Some(vec![kept(&id, KeptBecause::KeyNotEmptied)]));
    assert!(device.key_file(&id), "the file stands");
    assert!(remove_steps(&device).is_empty(), "no remove");
    assert_eq!(device.records(), vec![id]);
}

#[test]
fn any_other_answer_of_the_remove_keeps_the_record() {
    let refused = |reason| {
        Reply::Now(Ok(EndpointRemoveChange::Refused {
            reason,
            detail: String::new(),
        }))
    };
    for answer in [
        Reply::Now(Err(TransportError::Internal)),
        Reply::Now(Err(TransportError::BackendUnavailable)),
        refused(EndpointRemoveRefusal::Authored),
        refused(EndpointRemoveRefusal::Default),
        Reply::Holds,
    ] {
        let mut device = Device::new();
        let id = device.left_by_a_form();
        let shown = format!("{answer:?}");
        device.script(|s| s.removes.push_back(answer));
        assert_eq!(
            device.settle(&[], &[0, ADD_WAIT_MS]),
            Some(vec![kept(&id, KeptBecause::RemoveNotDone)]),
            "{shown}"
        );
        assert_eq!(remove_steps(&device).len(), 1, "{shown}");
        assert_eq!(device.records(), vec![id], "{shown}");
    }
}

#[test]
fn a_key_file_no_record_names_is_never_touched() {
    let mut device = Device::new();
    // A key a deployment minted, or another program's add in progress:
    // in `endpoint-keys/`, unlisted, and named by no record.
    let stranger = identity_key::create(&device.paths, &device.keys)
        .unwrap()
        .endpoint()
        .clone();
    let id = device.left_by_a_form();
    device.log.take();
    assert_eq!(
        device.settle(&[], &[0]),
        Some(vec![Settled::Discarded(id.clone())])
    );
    assert!(device.key_file(&stranger), "the stranger's key stands");
    assert_eq!(
        device
            .log
            .count(|s| *s == Step::Retired(identity_key::alias("default", &stranger))),
        0
    );
    assert_eq!(
        remove_steps(&device),
        vec![Step::Removed {
            endpoint: id,
            key_file: false
        }]
    );
}

#[test]
fn the_settle_skips_the_open_forms_record_and_no_other() {
    let mut device = Device::new();
    let open = device.left_by_a_form();
    let others = [device.left_by_a_form(), device.left_by_a_form()];
    device.log.take();
    assert_eq!(
        device.settle(std::slice::from_ref(&open), &[0]),
        Some(others.iter().cloned().map(Settled::Discarded).collect())
    );
    assert_eq!(device.records(), vec![open.clone()]);
    assert!(device.key_file(&open), "the open form's key stands");
    assert_eq!(
        device.log.count(|s| *s == Step::Listed),
        2,
        "one list per record settled"
    );
}

/// Two settles over one record, its list or its remove answered to each
/// at its own time -- the roster's own settle and a form's at the bound
/// consumed in one poll -- in the order that would do harm: the settle
/// holding a stale answer acts after the other settled the record. It
/// reads the record again and does nothing (A 2026-11-26 (ii)'s settle,
/// held to the record it reads). Mutations: no re-read before the list's
/// act -- the found identity's key path is emptied and its remove sent;
/// no re-read before the remove's record delete -- the record is
/// reported discarded twice.
#[test]
fn a_record_another_settle_settled_first_is_left_with_no_act() {
    // Found by one, a stale Absent in the other.
    let mut device = Device::new();
    let id = device.left_by_a_form();
    device.log.take();
    device.script(|s| {
        s.lists.push_back(Reply::Later(Ok(())));
        s.views.push_back(Vec::new());
        s.views.push_back(vec![view(&id, "anna")]);
    });
    let admins = device.admins();
    let mut stale = Settle::new(Arc::clone(&admins), vec![id.clone()]);
    let mut fresh = Settle::new(admins, vec![id.clone()]);
    let mut local = device.local();
    assert_eq!(stale.poll(0, &mut local), None, "its list answers later");
    assert_eq!(
        fresh.poll(0, &mut local),
        Some(vec![Settled::Identity {
            endpoint: id.clone(),
            alias: Some("anna".to_owned()),
        }])
    );
    assert_eq!(
        stale.poll(1, &mut local),
        Some(vec![Settled::Gone(id.clone())]),
        "the stale Absent is not acted on"
    );
    assert!(device.key_file(&id), "the identity's key file stands");
    assert!(device.records().is_empty());
    assert!(remove_steps(&device).is_empty(), "no remove");
    assert_eq!(device.log.count(|s| matches!(s, Step::Retired(_))), 0);

    // Both discard: the remove answered later to one.
    let mut device = Device::new();
    let id = device.left_by_a_form();
    device.log.take();
    device.script(|s| {
        s.removes
            .push_back(Reply::Later(Ok(EndpointRemoveChange::Removed {
                revoked_epoch: None,
            })));
    });
    let admins = device.admins();
    let mut stale = Settle::new(Arc::clone(&admins), vec![id.clone()]);
    let mut fresh = Settle::new(admins, vec![id.clone()]);
    let mut local = device.local();
    assert_eq!(stale.poll(0, &mut local), None, "its remove answers later");
    assert_eq!(
        fresh.poll(0, &mut local),
        Some(vec![Settled::Discarded(id.clone())])
    );
    assert_eq!(
        stale.poll(1, &mut local),
        Some(vec![Settled::Gone(id.clone())])
    );
    assert!(device.records().is_empty());
    assert_eq!(
        device.log.count(|s| matches!(s, Step::Forgot { .. })),
        1,
        "the record deleted once: {:?}",
        device.log.steps()
    );
}

/// The record unread before an act: the settle keeps it and does
/// nothing. Mutation: an unread record read as naming the id -- the key
/// path is emptied.
#[test]
fn a_record_unread_before_an_act_is_kept_with_no_act() {
    let mut device = Device::new();
    let id = device.left_by_a_form();
    device.log.take();
    device.script(|s| s.lists.push_back(Reply::Later(Ok(()))));
    let mut settle = Settle::new(device.admins(), vec![id.clone()]);
    let mut local = device.local();
    assert_eq!(settle.poll(0, &mut local), None);
    device.script(|s| s.refuse_reads = true);
    let mut local = device.local();
    assert_eq!(
        settle.poll(1, &mut local),
        Some(vec![kept(&id, KeptBecause::RecordUnread)])
    );
    device.script(|s| s.refuse_reads = false);
    assert_eq!(device.records(), vec![id.clone()]);
    assert!(device.key_file(&id));
    assert!(remove_steps(&device).is_empty());
}

#[test]
fn a_write_refused_at_the_bound_settles_once_and_is_made_once_more() {
    // Run apart, so that a settle that never ends fails the test rather
    // than holding the suite.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        at_the_bound();
        tx.send(()).ok();
    });
    rx.recv_timeout(Duration::from_secs(120))
        .expect("each Create ends");
}

fn at_the_bound() {
    let full = |device: &mut Device| -> Vec<EndpointId> {
        let ids: Vec<EndpointId> = (0..64).map(|_| device.left_by_a_form()).collect();
        device.log.take();
        ids
    };

    // The daemon answers: the settle frees the records, the write lands,
    // the add is sent.
    let mut device = Device::new();
    let ids = full(&mut device);
    let mut form = device.form();
    assert!(form.create("Anna", &[]));
    let ended = device.poll_ended(&mut form, 0).expect("ends");
    let id = form.endpoint().cloned().unwrap();
    assert!(
        matches!(ended.outcome, AddOutcome::Created { .. }),
        "{ended:?}"
    );
    assert_eq!(
        ended.settled,
        ids.iter()
            .cloned()
            .map(Settled::Discarded)
            .collect::<Vec<_>>()
    );
    let steps = device.log.steps();
    assert_eq!(steps.first(), Some(&Step::RecordRefused(id.clone())));
    let recorded = position(&steps, |s| *s == Step::Recorded(id.clone()));
    let last_remove = steps
        .iter()
        .rposition(|s| matches!(s, Step::Removed { .. }))
        .unwrap();
    assert!(last_remove < recorded, "settled, then written once more");
    assert!(
        position(&steps, |s| matches!(s, Step::Sealed(_))) > recorded,
        "and only then sealed"
    );
    assert_eq!(device.log.adds().len(), 1);

    // The daemon is down: every record is kept, the second refusal seals
    // nothing and sends no add, and each Create settles once.
    let mut device = Device::new();
    let ids = full(&mut device);
    device.script(|s| s.open_fails = Some(TransportError::BackendUnavailable));
    let mut form = device.form();
    for now in 0..2 {
        assert!(form.create("Anna", &[]));
        let ended = device.poll_ended(&mut form, now).expect("ends");
        assert_eq!(ended.outcome, AddOutcome::Failed);
        assert_eq!(
            ended.settled,
            ids.iter()
                .map(|id| kept(id, KeptBecause::ListFailed))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            device.log.count(|s| matches!(s, Step::RecordRefused(_))),
            2,
            "refused, settled, refused again"
        );
        assert_eq!(device.log.count(|s| matches!(s, Step::Sealed(_))), 0);
        assert!(device.log.adds().is_empty());
        device.log.take();
    }
    assert_eq!(device.records(), ids);
}

#[test]
fn a_create_settles_every_record_but_those_of_forms_open_elsewhere() {
    let mut device = Device::new();
    let ids: Vec<EndpointId> = (0..64).map(|_| device.left_by_a_form()).collect();
    let elsewhere = ids[0].clone();
    let mut form = device.form();
    assert!(form.create("Anna", std::slice::from_ref(&elsewhere)));
    let ended = device.poll_ended(&mut form, 0).expect("ends");
    assert!(
        matches!(ended.outcome, AddOutcome::Created { .. }),
        "{ended:?}"
    );
    assert_eq!(
        ended.settled,
        ids[1..]
            .iter()
            .cloned()
            .map(Settled::Discarded)
            .collect::<Vec<_>>()
    );
    assert_eq!(device.records(), vec![elsewhere.clone()]);
    assert!(device.key_file(&elsewhere), "the other form's key stands");
}

/// A seed the form asks goes on a connection of its own holding
/// `admin.trust` alone, names the first identity, and its answer --
/// `seeded` or `present` -- is the run's: every clone of the seed reads
/// it answered. The connection is closed as the ask ends, and a poll
/// after that answers the flag again. Mutation: `SeedAsk::poll` not
/// recording the answer -- the clone reads it unanswered.
#[test]
fn a_seed_the_form_asks_is_recorded_for_the_run_and_its_connection_closed() {
    for answer in [TrustSeed::Seeded, TrustSeed::Present] {
        let device = Device::new();
        device.script(|s| s.seeds.push_back(Reply::Now(Ok(answer))));
        let seed = FirstSeed::new(endpoint_id("human"));
        let run = seed.clone();
        let mut ask = SeedAsk::new(&device.admins(), &seed, 0);
        assert!(!run.answered(), "nothing asked before the poll");
        assert_eq!(ask.poll(0), Some(true), "{answer:?}");
        assert!(run.answered(), "the run's flag: {answer:?}");
        assert_eq!(
            device.log.steps(),
            [
                Step::Opened(BTreeSet::from([AdminCapability::Trust])),
                Step::Seeded(endpoint_id("human")),
                Step::Closed,
            ]
        );
        assert_eq!(ask.poll(1), Some(true));
    }
}

/// A seed refused, a connection that does not open, and a seed not
/// answered within [`ADD_WAIT_MS`] each leave the run unseeded; the
/// unanswered one is waited for until the wait is out and then dropped,
/// which closes its connection. Mutations: `poll` recording any end as
/// answered -- the flag is set; `Call::poll` waiting past the wait -- the
/// ask never ends.
#[test]
fn a_seed_refused_or_unanswered_in_the_wait_leaves_the_run_unseeded() {
    let device = Device::new();
    device.script(|s| {
        s.seeds
            .push_back(Reply::Now(Err(TransportError::BackendUnavailable)));
    });
    let seed = FirstSeed::new(endpoint_id("human"));
    let mut ask = SeedAsk::new(&device.admins(), &seed, 0);
    assert_eq!(ask.poll(0), Some(false));
    assert!(!seed.answered());

    let device = Device::new();
    device.script(|s| s.open_fails = Some(TransportError::Overloaded));
    let mut ask = SeedAsk::new(&device.admins(), &seed, 0);
    assert_eq!(ask.poll(0), Some(false));
    assert!(!seed.answered());
    assert!(device.log.steps().is_empty(), "nothing was sent");

    let device = Device::new();
    device.script(|s| s.seeds.push_back(Reply::Holds));
    let mut ask = SeedAsk::new(&device.admins(), &seed, 100);
    assert_eq!(ask.poll(100), None);
    assert_eq!(ask.poll(100 + ADD_WAIT_MS - 1), None, "still waited for");
    assert!(!device.log.steps().contains(&Step::Closed));
    assert_eq!(ask.poll(100 + ADD_WAIT_MS), Some(false), "the wait is out");
    assert_eq!(device.log.steps().last(), Some(&Step::Closed), "dropped");
    assert!(!seed.answered());
}

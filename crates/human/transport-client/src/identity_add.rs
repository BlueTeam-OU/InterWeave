// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The add an identity's form asks (`design/2026-11-04-identities.md`
//! §6.2; ADR-0061 decision 7, A 2026-11-26 and A 2026-11-27;
//! `LOCAL-IPC.md`, `admin.endpoints.add` and `list`).
//!
//! [`FormAdd`] is one form's add: it makes ONE key for the form's life,
//! writes the store's pending-add record naming the key's id before the
//! key is sealed, seals the key before the add is sent, and reads each
//! answer as the design does. An answer the client cannot read as
//! "created" or "nothing created" is an outcome it does not know, and
//! the key and the record are then kept for the settle to decide.
//!
//! Every call to the daemon goes over an administrative connection of
//! its own, holding `admin.endpoints` alone and opened for that one act,
//! as a trust act's is (`trust.rs`), and closed when its answer is read.
//! Calls are polled with the caller's clock and not awaited, as the
//! advertise assertion is (`client.rs`): a call the daemon has not
//! answered within [`ADD_WAIT_MS`] is dropped, which closes its
//! connection, and read as no answer
//! (`an_add_unanswered_for_the_wait_is_unknown_and_no_longer_waited_for`).
//!
//! [`Settle`] settles the records whose forms are gone: an id listed with
//! its key file there becomes an identity at its record's delete, and
//! any other id a list that answered says something of is discarded --
//! its key path, its platform key, the administrative remove, then the
//! record. It reaches the ids its caller gives it, which are the
//! record's.
//!
//! What the add writes on this device -- the record and the key file --
//! is reached through [`Local`], lent to each call and not kept: the
//! store behind [`PendingAddStore`], so a test can fail one write, and
//! the key's place and custody.

use std::collections::{BTreeSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::FutureExt as _;

use interweave_human_store::identity_key::{self, IdentityKey, KeyCipher};
use interweave_human_store::{HumanStore, StoreError};
use interweave_local_client_api::{
    AdminCapability, AdminPort as _, EndpointAddChange, EndpointAddRefusal, EndpointAdminView,
    EndpointRemoveChange,
};
use interweave_profile_config::endpoint_key;
use interweave_profile_config::paths::ProfilePaths;
use interweave_profile_config::{AliasError, EndpointAlias, MAX_ENDPOINTS};
use interweave_transport_api::{EndpointId, TransportError};

use crate::client::AssertingBinding;

/// How long the client waits for the answer to one call of an add or of
/// a settle, in the caller's milliseconds: the design's add wait, a
/// hypothesis to be measured (§6.2, "The outcome is unknown").
pub const ADD_WAIT_MS: u64 = 30_000;

/// The most aliases a form remembers it was refused `alias_taken` for:
/// the profile's bound on endpoints (§6.2, "The alias"). A refusal past
/// it is not kept.
const REFUSED_ALIASES: usize = MAX_ENDPOINTS;

/// The store's pending-add record, as the add and the settle use it:
/// [`HumanStore`]'s three calls, behind a trait so a test can fail one.
pub trait PendingAddStore {
    /// Write `endpoint` to the record ([`HumanStore::add_pending_add`]).
    ///
    /// # Errors
    /// The store's, [`StoreError::PendingAddBound`] at the bound.
    fn add_pending_add(&mut self, endpoint: &EndpointId) -> Result<(), StoreError>;

    /// Every id the record names ([`HumanStore::pending_adds`]).
    ///
    /// # Errors
    /// The store's.
    fn pending_adds(&self) -> Result<Vec<EndpointId>, StoreError>;

    /// Delete `endpoint`'s row ([`HumanStore::forget_pending_add`]).
    ///
    /// # Errors
    /// The store's.
    fn forget_pending_add(&mut self, endpoint: &EndpointId) -> Result<bool, StoreError>;
}

impl PendingAddStore for HumanStore {
    fn add_pending_add(&mut self, endpoint: &EndpointId) -> Result<(), StoreError> {
        Self::add_pending_add(self, endpoint)
    }

    fn pending_adds(&self) -> Result<Vec<EndpointId>, StoreError> {
        Self::pending_adds(self)
    }

    fn forget_pending_add(&mut self, endpoint: &EndpointId) -> Result<bool, StoreError> {
        Self::forget_pending_add(self, endpoint)
    }
}

/// What an add writes on this device, lent for one call: the store that
/// holds the pending-add record, and where an identity's key file is
/// kept and under whose custody (ADR-0063; `identity_key`).
pub struct Local<'a, S: PendingAddStore + ?Sized> {
    /// The store holding the pending-add record.
    pub store: &'a mut S,
    /// The profile whose `endpoint-keys/` holds the key file.
    pub paths: &'a ProfilePaths,
    /// The platform's key custody: the desktop's filesystem, Android's
    /// Keystore.
    pub cipher: &'a dyn KeyCipher,
}

/// How a form makes an alias from the name the person typed and the
/// aliases already in use: app-core's rule (`alias_for`), given by the
/// caller since this crate is beneath it.
pub type AliasRule = fn(&str, &[EndpointAlias]) -> Result<EndpointAlias, AliasError>;

/// How one Create ended (§6.2, Create and the outcomes other than Done).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOutcome {
    /// Done: the add answered `added` or `unchanged`, or a list holds the
    /// id while its key file is at its key path, and the record is
    /// deleted, so the id is an identity from now on.
    Created {
        /// The new identity.
        endpoint: EndpointId,
        /// Its alias: the one the add sent, or the list's row's.
        alias: Option<String>,
    },
    /// "Created, not finished": the endpoint exists and the record could
    /// not be deleted, so the id is no identity yet. The next Create
    /// sends no add.
    NotFinished,
    /// "Refused, with nothing created": the client knows that nothing
    /// was created.
    Failed,
    /// The outcome is unknown: the key and the record are kept.
    Unknown,
    /// The add was refused `bound`: Create is not offered any more.
    Limit,
    /// The form's key is gone: Create is not offered any more.
    KeyGone,
}

/// What leaving the form did (§6.2, "The form is left"). Leaving is
/// [`FormAdd::leave`]; the app's end is dropping the form, which deletes
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Left {
    /// No key was made, or the id became an identity: nothing to delete.
    NothingToDelete,
    /// Nothing was created: the key file, the platform key and then the
    /// record were deleted.
    Deleted,
    /// An add may have landed, or did: the key and the record are kept
    /// for the settle.
    Kept(EndpointId),
    /// Nothing was created, but a delete failed: the record is kept for
    /// the settle.
    DeleteFailed(EndpointId),
}

/// A call to the daemon on its way, and when it was sent by the
/// caller's clock.
struct Call<T> {
    answer: Pin<Box<dyn Future<Output = T> + Send>>,
    since: u64,
}

/// What a polled call came to.
enum Answered<T> {
    Answer(T),
    /// No answer within [`ADD_WAIT_MS`]: the call was dropped.
    NoAnswer,
}

impl<T> Call<T> {
    fn start(answer: impl Future<Output = T> + Send + 'static, now: u64) -> Self {
        Self {
            answer: Box::pin(answer),
            since: now,
        }
    }

    /// The answer if there is one, no answer once the wait is out, and
    /// `None` while it is still waited for.
    fn poll(&mut self, now: u64) -> Option<Answered<T>> {
        if let Some(answer) = self.answer.as_mut().now_or_never() {
            return Some(Answered::Answer(answer));
        }
        (now.saturating_sub(self.since) >= ADD_WAIT_MS).then_some(Answered::NoAnswer)
    }
}

type Listing = Result<Vec<EndpointAdminView>, TransportError>;

/// Why an add ended without an answer the daemon gave.
enum AddFailure {
    /// The connection did not open, so the add was not sent
    /// (`an_add_whose_connection_did_not_open_was_not_sent`).
    NotSent,
    /// The add was sent and answered this error.
    Error(TransportError),
}

type Adding = Result<EndpointAddChange, AddFailure>;

type Removing = Result<EndpointRemoveChange, TransportError>;

/// The endpoint list, read on a connection of its own.
fn list<A: AssertingBinding>(binding: &Arc<A>, now: u64) -> Call<Listing> {
    let binding = Arc::clone(binding);
    Call::start(
        async move {
            let port = binding
                .admin(BTreeSet::from([AdminCapability::Endpoints]))
                .await?;
            port.leases().await
        },
        now,
    )
}

/// The add of `key`'s file under `alias`, on a connection of its own.
fn add<A: AssertingBinding>(
    binding: &Arc<A>,
    key: String,
    alias: String,
    kinds: Vec<String>,
    now: u64,
) -> Call<Adding> {
    let binding = Arc::clone(binding);
    Call::start(
        async move {
            let port = binding
                .admin(BTreeSet::from([AdminCapability::Endpoints]))
                .await
                .map_err(|_| AddFailure::NotSent)?;
            port.add_endpoint(key, alias, kinds)
                .await
                .map_err(AddFailure::Error)
        },
        now,
    )
}

/// The administrative remove of `endpoint`, on a connection of its own.
fn remove<A: AssertingBinding>(binding: &Arc<A>, endpoint: EndpointId, now: u64) -> Call<Removing> {
    let binding = Arc::clone(binding);
    Call::start(
        async move {
            let port = binding
                .admin(BTreeSet::from([AdminCapability::Endpoints]))
                .await?;
            port.remove_endpoint(endpoint).await
        },
        now,
    )
}

/// What a list that answered says of one id (ADR-0061 decision 7, A
/// 2026-11-26: Found is the id listed while its key file is at its key
/// path).
enum Seen {
    /// Listed, its key file there; the row's alias.
    Found(Option<String>),
    /// Listed, its key path empty.
    KeyGone,
    /// Not listed.
    Absent,
    /// Listed, and whether a file is at its key path could not be read.
    Unsure,
}

fn seen(views: &[EndpointAdminView], endpoint: &EndpointId, paths: &ProfilePaths) -> Seen {
    let Some(view) = views.iter().find(|v| v.endpoint == *endpoint) else {
        return Seen::Absent;
    };
    match identity_key::present(paths, endpoint) {
        Ok(true) => Seen::Found(view.alias.clone()),
        Ok(false) => Seen::KeyGone,
        Err(_) => Seen::Unsure,
    }
}

/// Where a running Create is.
enum Running<A> {
    /// The steps on this device: the key, the record, the seal.
    Start,
    /// The settle of the other records, after the record's write was
    /// refused at the bound and before it is made once more.
    Settling(Settle<A>),
    /// The list read before the add, for Found and for the aliases in use.
    Before(Call<Listing>),
    /// The add, and the alias it sent.
    Adding(Call<Adding>, String),
    /// The list read after an add whose outcome is not known.
    After(Call<Listing>),
    /// The list read of a Create on "Created, not finished", which sends
    /// no add.
    Finishing(Call<Listing>),
}

/// What one step of a Create leads to.
enum Next<A> {
    /// Waiting on a call.
    Wait(Running<A>),
    /// Go on in this poll.
    Go(Running<A>),
    /// The Create ended.
    End(AddOutcome),
}

/// One add form's add: its one key, its record, and its Creates.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is one fact of the design's form, read apart"
)]
pub struct FormAdd<A> {
    binding: Arc<A>,
    kinds: Vec<String>,
    alias_rule: AliasRule,
    /// Made at the first Create and kept while the form is open: every
    /// Create sends the same key, so one form makes one identity
    /// (`one_form_makes_one_key`).
    key: Option<IdentityKey>,
    /// The record naming the key's id is written and not deleted.
    recorded: bool,
    /// The key file is sealed.
    sealed: bool,
    /// The alias of the first add of the key that ended unknown, which
    /// every later add sends; `Some` is "an add of the form's key ended
    /// unknown".
    unknown_alias: Option<String>,
    /// An add answered `added` or `unchanged`, or a list read the id as
    /// Found: leaving keeps the key and the record.
    landed: bool,
    /// "Created, not finished": the next Create reads the list and sends
    /// no add.
    finishing: bool,
    /// The record was deleted at Done: the id is an identity.
    done: bool,
    /// The aliases an add of this form was refused `alias_taken` for.
    refused: Vec<EndpointAlias>,
    /// The name the running Create took its alias from.
    name: String,
    /// The records of the other forms open now, which the running
    /// Create's settle leaves alone.
    open_elsewhere: Vec<EndpointId>,
    /// The running Create settled once: a second refusal at the bound
    /// ends it.
    settled_once: bool,
    /// What the running Create's settle did.
    settled: Vec<Settled>,
    running: Option<Running<A>>,
}

impl<A: AssertingBinding> FormAdd<A> {
    /// A form's add, opening nothing yet: `binding` opens each call's
    /// connection, `kinds` are the client kinds of an identity, and
    /// `alias_rule` makes an alias from a name.
    #[must_use]
    pub fn new(binding: Arc<A>, kinds: Vec<String>, alias_rule: AliasRule) -> Self {
        Self {
            binding,
            kinds,
            alias_rule,
            key: None,
            recorded: false,
            sealed: false,
            unknown_alias: None,
            landed: false,
            finishing: false,
            done: false,
            refused: Vec::new(),
            name: String::new(),
            open_elsewhere: Vec::new(),
            settled_once: false,
            settled: Vec::new(),
            running: None,
        }
    }

    /// The id the form's key derives, once a Create made the key: what
    /// its record names, and what a settle leaves alone while the form is
    /// open.
    #[must_use]
    pub fn endpoint(&self) -> Option<&EndpointId> {
        self.key.as_ref().map(IdentityKey::endpoint)
    }

    /// Whether a Create is running: Back and Escape do nothing then.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// The aliases this form was refused `alias_taken` for, which no
    /// later add of it sends.
    #[must_use]
    pub fn refused_aliases(&self) -> &[EndpointAlias] {
        &self.refused
    }

    /// Start a Create for the card named `name`. `open_elsewhere` are
    /// the records of the other add forms open now, which a settle at the
    /// bound leaves alone (none where one form is open at a time).
    /// Nothing runs until [`Self::poll`]. Whether it started: not while
    /// one runs, and not after Done.
    pub fn create(&mut self, name: &str, open_elsewhere: &[EndpointId]) -> bool {
        if self.running.is_some() || self.done {
            return false;
        }
        name.clone_into(&mut self.name);
        open_elsewhere.clone_into(&mut self.open_elsewhere);
        self.settled_once = false;
        self.settled.clear();
        self.running = Some(Running::Start);
        true
    }

    /// Run the Create as far as it goes without waiting, at `now` by the
    /// caller's clock: how it ended, with what its settle at the bound
    /// did, or `None` while a call is waited for (and when no Create
    /// runs).
    pub fn poll<S: PendingAddStore + ?Sized>(
        &mut self,
        now: u64,
        local: &mut Local<'_, S>,
    ) -> Option<Ended> {
        loop {
            let running = self.running.take()?;
            match self.step(running, now, local) {
                Next::Wait(running) => {
                    self.running = Some(running);
                    return None;
                }
                Next::Go(running) => self.running = Some(running),
                Next::End(outcome) => {
                    return Some(Ended {
                        outcome,
                        settled: std::mem::take(&mut self.settled),
                    });
                }
            }
        }
    }

    /// Leave the form (Back, or any other way a person leaves it).
    /// Known nothing created, it deletes the key file and the platform
    /// key, then the record, and sends no remove; otherwise, and while a
    /// Create runs, it keeps both for the settle.
    pub fn leave<S: PendingAddStore + ?Sized>(self, local: &mut Local<'_, S>) -> Left {
        let Some(endpoint) = self.endpoint().cloned() else {
            return Left::NothingToDelete;
        };
        if self.done {
            return Left::NothingToDelete;
        }
        if self.running.is_some() || self.unknown_alias.is_some() || self.landed {
            return Left::Kept(endpoint);
        }
        if identity_key::remove(local.paths, &endpoint, local.cipher).is_err() {
            return Left::DeleteFailed(endpoint);
        }
        // Forgotten even when no write was known to land: a write that
        // failed may still have committed, and forgetting no row is no
        // error.
        match local.store.forget_pending_add(&endpoint) {
            Ok(_) => Left::Deleted,
            Err(_) => Left::DeleteFailed(endpoint),
        }
    }

    fn step<S: PendingAddStore + ?Sized>(
        &mut self,
        running: Running<A>,
        now: u64,
        local: &mut Local<'_, S>,
    ) -> Next<A> {
        match running {
            Running::Start => self.start(now, local),
            Running::Settling(mut settle) => match settle.poll(now, local) {
                None => Next::Wait(Running::Settling(settle)),
                Some(settled) => {
                    self.settled.extend(settled);
                    Next::Go(Running::Start)
                }
            },
            Running::Before(mut call) => match call.poll(now) {
                None => Next::Wait(Running::Before(call)),
                Some(answer) => {
                    // Dropped first: a call no longer waited for closes
                    // its connection before the next one opens.
                    drop(call);
                    self.before(answer, now, local)
                }
            },
            Running::Adding(mut call, alias) => match call.poll(now) {
                None => Next::Wait(Running::Adding(call, alias)),
                Some(answer) => {
                    // Dropped first: a call no longer waited for closes
                    // its connection before the next one opens.
                    drop(call);
                    self.added(&answer, alias, now, local)
                }
            },
            Running::After(mut call) => match call.poll(now) {
                None => Next::Wait(Running::After(call)),
                Some(answer) => {
                    // Dropped first: a call no longer waited for closes
                    // its connection before the next one opens.
                    drop(call);
                    self.after(answer, local)
                }
            },
            Running::Finishing(mut call) => match call.poll(now) {
                None => Next::Wait(Running::Finishing(call)),
                Some(answer) => {
                    // Dropped first: a call no longer waited for closes
                    // its connection before the next one opens.
                    drop(call);
                    self.finished(answer, local)
                }
            },
        }
    }

    /// The key, the record, then the seal, each made once for the form:
    /// a record that cannot be written seals nothing, and a key that
    /// cannot be sealed is sent in no add
    /// (`the_record_is_written_before_the_key_is_sealed_and_the_key_before_the_add`).
    fn start<S: PendingAddStore + ?Sized>(
        &mut self,
        now: u64,
        local: &mut Local<'_, S>,
    ) -> Next<A> {
        if self.finishing {
            return Next::Go(Running::Finishing(list(&self.binding, now)));
        }
        if self.key.is_none() {
            match IdentityKey::generate() {
                Ok(key) => self.key = Some(key),
                Err(_) => return self.stopped(now),
            }
        }
        let Some(key) = self.key.as_ref() else {
            return self.stopped(now);
        };
        if !self.recorded {
            match local.store.add_pending_add(key.endpoint()) {
                Ok(()) => self.recorded = true,
                // Refused at the bound: the other records are settled
                // first, once, and the write is made once more; a second
                // refusal is a record that cannot be written (A
                // 2026-11-26 (iii);
                // `a_write_refused_at_the_bound_settles_once_and_is_made_once_more`).
                Err(StoreError::PendingAddBound) if !self.settled_once => {
                    self.settled_once = true;
                    return match Settle::of(&self.binding, local, &self.open_elsewhere) {
                        Ok(settle) => Next::Go(Running::Settling(settle)),
                        Err(_) => self.stopped(now),
                    };
                }
                Err(_) => return self.stopped(now),
            }
        }
        if !self.sealed {
            if identity_key::install(local.paths, key, local.cipher).is_err() {
                return self.stopped(now);
            }
            self.sealed = true;
        }
        Next::Go(Running::Before(list(&self.binding, now)))
    }

    /// The list before the add: Found or the key gone end the Create
    /// there; otherwise, and on a list that failed, the add is sent.
    fn before<S: PendingAddStore + ?Sized>(
        &mut self,
        answer: Answered<Listing>,
        now: u64,
        local: &mut Local<'_, S>,
    ) -> Next<A> {
        // A list that failed reads as an empty one here: it ends no
        // Create, and the add is safe to send.
        let views = if let Answered::Answer(Ok(views)) = answer {
            views
        } else {
            Vec::new()
        };
        let Some(endpoint) = self.endpoint().cloned() else {
            return self.stopped(now);
        };
        match seen(&views, &endpoint, local.paths) {
            Seen::Found(alias) => return self.found(endpoint, alias, local),
            Seen::KeyGone => return Next::End(AddOutcome::KeyGone),
            Seen::Absent | Seen::Unsure => {}
        }
        let alias = if let Some(alias) = self.unknown_alias.clone() {
            alias
        } else {
            let mut in_use: Vec<EndpointAlias> = views
                .iter()
                .filter_map(|v| v.alias.as_deref())
                .filter_map(|a| EndpointAlias::parse(a).ok())
                .collect();
            in_use.extend(self.refused.iter().cloned());
            match (self.alias_rule)(&self.name, &in_use) {
                Ok(alias) => alias.as_str().to_owned(),
                Err(_) => return self.stopped(now),
            }
        };
        let call = add(
            &self.binding,
            endpoint_key::key_file_name(&endpoint),
            alias.clone(),
            self.kinds.clone(),
            now,
        );
        Next::Go(Running::Adding(call, alias))
    }

    /// The add's answer, read as the design reads it.
    fn added<S: PendingAddStore + ?Sized>(
        &mut self,
        answer: &Answered<Adding>,
        alias: String,
        now: u64,
        local: &mut Local<'_, S>,
    ) -> Next<A> {
        let known = self.unknown_alias.is_none();
        let Some(endpoint) = self.endpoint().cloned() else {
            return self.stopped(now);
        };
        match answer {
            Answered::Answer(Ok(EndpointAddChange::Added(_) | EndpointAddChange::Unchanged(_))) => {
                self.landed = true;
                self.found(endpoint, Some(alias), local)
            }
            Answered::Answer(Ok(EndpointAddChange::Refused { reason, .. })) => match reason {
                // Decided after `unchanged`, so the overlay does not hold
                // the id: no list is read, whatever ended unknown before.
                EndpointAddRefusal::Bound => Next::End(AddOutcome::Limit),
                EndpointAddRefusal::KeyMissing => Next::End(AddOutcome::KeyGone),
                // An entry holds the id: no refusal that says nothing was
                // created (A 2026-11-27). The list decides, and anything
                // but Found is an add that ended unknown.
                EndpointAddRefusal::Exists => self.unknown(alias, now),
                EndpointAddRefusal::AliasTaken => {
                    self.refuse(&alias);
                    if known {
                        Next::End(AddOutcome::Failed)
                    } else {
                        self.unknown(alias, now)
                    }
                }
                EndpointAddRefusal::IdNotDerived
                | EndpointAddRefusal::AliasInvalid
                | EndpointAddRefusal::KeyMismatch => {
                    if known {
                        Next::End(AddOutcome::Failed)
                    } else {
                        self.unknown(alias, now)
                    }
                }
            },
            Answered::Answer(Err(AddFailure::NotSent)) => self.stopped(now),
            Answered::Answer(Err(AddFailure::Error(TransportError::InvalidArgument))) if known => {
                Next::End(AddOutcome::Failed)
            }
            Answered::Answer(Err(AddFailure::Error(_))) | Answered::NoAnswer => {
                self.unknown(alias, now)
            }
        }
    }

    /// The list after an add whose outcome is not known.
    fn after<S: PendingAddStore + ?Sized>(
        &mut self,
        answer: Answered<Listing>,
        local: &mut Local<'_, S>,
    ) -> Next<A> {
        let Some(endpoint) = self.endpoint().cloned() else {
            return Next::End(AddOutcome::Unknown);
        };
        match answer {
            Answered::Answer(Ok(views)) => match seen(&views, &endpoint, local.paths) {
                Seen::Found(alias) => self.found(endpoint, alias, local),
                Seen::KeyGone => Next::End(AddOutcome::KeyGone),
                Seen::Absent | Seen::Unsure => Next::End(AddOutcome::Unknown),
            },
            Answered::Answer(Err(_)) | Answered::NoAnswer => Next::End(AddOutcome::Unknown),
        }
    }

    /// The list of a Create on "Created, not finished".
    fn finished<S: PendingAddStore + ?Sized>(
        &mut self,
        answer: Answered<Listing>,
        local: &mut Local<'_, S>,
    ) -> Next<A> {
        let Some(endpoint) = self.endpoint().cloned() else {
            return Next::End(AddOutcome::NotFinished);
        };
        match answer {
            Answered::Answer(Ok(views)) => match seen(&views, &endpoint, local.paths) {
                Seen::Found(alias) => self.found(endpoint, alias, local),
                Seen::KeyGone => {
                    self.finishing = false;
                    Next::End(AddOutcome::KeyGone)
                }
                // The endpoint may have been removed since its add: a
                // list read, which ends no add, so it is not an add that
                // ended unknown.
                Seen::Absent => {
                    self.finishing = false;
                    Next::End(AddOutcome::Unknown)
                }
                Seen::Unsure => Next::End(AddOutcome::NotFinished),
            },
            Answered::Answer(Err(_)) | Answered::NoAnswer => Next::End(AddOutcome::NotFinished),
        }
    }

    /// The id is an identity once its record is deleted, and not before.
    fn found<S: PendingAddStore + ?Sized>(
        &mut self,
        endpoint: EndpointId,
        alias: Option<String>,
        local: &mut Local<'_, S>,
    ) -> Next<A> {
        self.landed = true;
        if local.store.forget_pending_add(&endpoint).is_err() {
            self.finishing = true;
            return Next::End(AddOutcome::NotFinished);
        }
        self.recorded = false;
        self.finishing = false;
        self.done = true;
        Next::End(AddOutcome::Created { endpoint, alias })
    }

    /// An add of the key ended unknown: its alias is the one every later
    /// add sends, and the list is read.
    fn unknown(&mut self, alias: String, now: u64) -> Next<A> {
        self.unknown_alias.get_or_insert(alias);
        Next::Go(Running::After(list(&self.binding, now)))
    }

    /// The Create stopped before its add: nothing created, unless an
    /// earlier add of the key ended unknown, which the list then reads.
    fn stopped(&self, now: u64) -> Next<A> {
        if self.unknown_alias.is_some() {
            Next::Go(Running::After(list(&self.binding, now)))
        } else {
            Next::End(AddOutcome::Failed)
        }
    }

    fn refuse(&mut self, alias: &str) {
        let Ok(alias) = EndpointAlias::parse(alias) else {
            return;
        };
        if self.refused.len() < REFUSED_ALIASES && !self.refused.contains(&alias) {
            self.refused.push(alias);
        }
    }
}

/// What [`FormAdd::poll`] gives when a Create ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    /// How it ended.
    pub outcome: AddOutcome,
    /// What its settle did, when its record was refused at the bound: an
    /// identity found there is one from now on, as one a settle at the
    /// start finds.
    pub settled: Vec<Settled>,
}

/// How the settle of one record ended (§6.2, "A pending-add record is
/// settled").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    /// Listed with its key file at its key path: the record is deleted,
    /// and the id is an identity from now on, named by its alias until it
    /// has a card.
    Identity {
        /// The identity.
        endpoint: EndpointId,
        /// The list's row's alias.
        alias: Option<String>,
    },
    /// Discarded: its key path and platform key emptied, its remove
    /// answered `removed` or `EndpointUnknown`, and its record deleted.
    Discarded(EndpointId),
    /// Kept, to be settled again at the next moment.
    Kept {
        /// The record's id.
        endpoint: EndpointId,
        /// Why.
        because: KeptBecause,
    },
}

/// Why a settle kept a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptBecause {
    /// The list failed, or did not answer within the wait: nothing was
    /// deleted.
    ListFailed,
    /// The id is listed, and whether a file is at its key path could not
    /// be read: neither an identity nor a discard.
    KeyPathUnread,
    /// The key path or the platform key could not be emptied: no remove
    /// was sent.
    KeyNotEmptied,
    /// The remove answered something other than `removed` or
    /// `EndpointUnknown`, or nothing within the wait.
    RemoveNotDone,
    /// The record's delete failed.
    RecordNotDeleted,
}

/// Where the settle of the record in hand is.
enum Settling {
    Listing(Call<Listing>),
    Removing(Call<Removing>),
}

/// One settle of pending-add records whose forms are gone (§6.2; ADR-0061
/// decision 7, A 2026-11-26 (ii)): one list per record, then the
/// record's delete or its discard. The moments it runs at -- the start,
/// the list's opening, a form's closing -- are the caller's; the fourth,
/// a write refused at the bound, is [`FormAdd`]'s own.
pub struct Settle<A> {
    binding: Arc<A>,
    records: VecDeque<EndpointId>,
    current: Option<(EndpointId, Settling)>,
    report: Vec<Settled>,
}

impl<A: AssertingBinding> Settle<A> {
    /// A settle of `records`, which are every record but those of the add
    /// forms open now (`the_settle_skips_the_open_forms_record_and_no_other`):
    /// it touches no key file a record does not name
    /// (`a_key_file_no_record_names_is_never_touched`).
    #[must_use]
    pub fn new(binding: Arc<A>, records: Vec<EndpointId>) -> Self {
        Self {
            binding,
            records: records.into(),
            current: None,
            report: Vec::new(),
        }
    }

    /// A settle of every record the store holds but `open`'s, the records
    /// of the add forms open now.
    ///
    /// # Errors
    /// The store's, reading the record.
    pub fn of<S: PendingAddStore + ?Sized>(
        binding: &Arc<A>,
        local: &Local<'_, S>,
        open: &[EndpointId],
    ) -> Result<Self, StoreError> {
        let records = local
            .store
            .pending_adds()?
            .into_iter()
            .filter(|id| !open.contains(id))
            .collect();
        Ok(Self::new(Arc::clone(binding), records))
    }

    /// Settle as far as it goes without waiting, at `now` by the caller's
    /// clock: what was done for each record once every record is
    /// settled, or `None` while a call is waited for. Polled again after
    /// that, it has nothing left and gives an empty list.
    pub fn poll<S: PendingAddStore + ?Sized>(
        &mut self,
        now: u64,
        local: &mut Local<'_, S>,
    ) -> Option<Vec<Settled>> {
        loop {
            let (endpoint, settling) = match self.current.take() {
                Some(current) => current,
                None => match self.records.pop_front() {
                    Some(endpoint) => (endpoint, Settling::Listing(list(&self.binding, now))),
                    None => return Some(std::mem::take(&mut self.report)),
                },
            };
            match settling {
                Settling::Listing(mut call) => match call.poll(now) {
                    None => {
                        self.current = Some((endpoint, Settling::Listing(call)));
                        return None;
                    }
                    Some(answer) => {
                        drop(call);
                        self.listed(endpoint, answer, now, local);
                    }
                },
                Settling::Removing(mut call) => match call.poll(now) {
                    None => {
                        self.current = Some((endpoint, Settling::Removing(call)));
                        return None;
                    }
                    Some(answer) => {
                        drop(call);
                        self.removed(endpoint, &answer, local);
                    }
                },
            }
        }
    }

    /// The list's answer for one record.
    fn listed<S: PendingAddStore + ?Sized>(
        &mut self,
        endpoint: EndpointId,
        answer: Answered<Listing>,
        now: u64,
        local: &mut Local<'_, S>,
    ) {
        let Answered::Answer(Ok(views)) = answer else {
            return self.kept(endpoint, KeptBecause::ListFailed);
        };
        match seen(&views, &endpoint, local.paths) {
            Seen::Found(alias) => match local.store.forget_pending_add(&endpoint) {
                Ok(_) => self.report.push(Settled::Identity { endpoint, alias }),
                Err(_) => self.kept(endpoint, KeptBecause::RecordNotDeleted),
            },
            Seen::Unsure => self.kept(endpoint, KeptBecause::KeyPathUnread),
            // The key path first, then the platform key, and the remove
            // after them, so an add applied late is refused `key_missing`
            // (`an_unlisted_id_is_discarded_key_path_platform_key_remove_then_record`).
            Seen::KeyGone | Seen::Absent => {
                if identity_key::remove(local.paths, &endpoint, local.cipher).is_err() {
                    return self.kept(endpoint, KeptBecause::KeyNotEmptied);
                }
                let call = remove(&self.binding, endpoint.clone(), now);
                self.current = Some((endpoint, Settling::Removing(call)));
            }
        }
    }

    /// The remove's answer for one record being discarded.
    fn removed<S: PendingAddStore + ?Sized>(
        &mut self,
        endpoint: EndpointId,
        answer: &Answered<Removing>,
        local: &mut Local<'_, S>,
    ) {
        match answer {
            Answered::Answer(
                Ok(EndpointRemoveChange::Removed { .. }) | Err(TransportError::EndpointUnknown),
            ) => match local.store.forget_pending_add(&endpoint) {
                Ok(_) => self.report.push(Settled::Discarded(endpoint)),
                Err(_) => self.kept(endpoint, KeptBecause::RecordNotDeleted),
            },
            Answered::Answer(Ok(EndpointRemoveChange::Refused { .. }) | Err(_))
            | Answered::NoAnswer => self.kept(endpoint, KeptBecause::RemoveNotDone),
        }
    }

    fn kept(&mut self, endpoint: EndpointId, because: KeptBecause) {
        self.report.push(Settled::Kept { endpoint, because });
    }
}

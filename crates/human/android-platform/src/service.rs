// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Service's hold on the client: the embedded runtime, the message
//! store and the facade over the runtime's in-process binding, started and
//! stopped as one, once per process (human-client-android.md, "Service
//! ownership and local session").
//!
//! The Service calls in from its own threads, never its main one: start,
//! the wait and stop all BLOCK, as the embedded host's do.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use interweave_human_app_core::KeyProblem;
use interweave_human_app_core::{
    HeldEntry, HeldList, KeySource, Roster, RosterConfig, RosterError, held_list,
};
use interweave_human_store::at_rest;
use interweave_human_store::key_custody::{CipherFailure, Custody, KeyCipher};
use interweave_human_store::{HumanStore, StoreError, StoreOptions};
use interweave_profile_config::endpoint_overlay::{EndpointOverlay, EndpointOverlayError};
pub use interweave_profile_config::runtime::AvailabilityMode;
pub use interweave_profile_config::sections::LogLevel;
use interweave_profile_config::{ProfileConfig, ProfilePaths, TrustBoundary};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::MAX_PAYLOAD_BYTES;
use interweave_transport_embedded::{
    EmbeddedHost, EmbeddedLaunch, EmbeddedRefused, Ended as HostEnded, StayReachable,
};

use crate::facade::{FacadeLoop, FacadeNote, SpawnError};
use crate::first_identity::FirstIdentityRefused;
use crate::hub::{Hub, StoreAnswer, StoreGeneration, StoreRefusal};
use crate::identity::UnlockStatus;
use crate::network::{NetworkReport, NetworkSlot, Told};
use crate::offline::offline_embedded;

/// The client kind the session declares, as on the desktop: the profile's
/// endpoint entry must allow it.
pub const CLIENT_KIND: &str = "human-client";

/// How long a stop waits for the facade to commit what its session holds
/// and close it, after the runtime began refusing and before the
/// runtime stops under it.
const FACADE_CLOSE: Duration = Duration::from_secs(2);

/// What the Service starts the client with.
pub struct ServiceLaunch {
    /// The app's data directory as the platform reports it
    /// (`Context.getDataDir()`): the trust boundary.
    pub app_data_dir: PathBuf,
    /// The profile's name; its configuration is in place under the
    /// host's configuration root before the first start.
    pub profile: String,
    /// The profile's identity, unlocked by the caller.
    pub identity: ProfileIdentity,
    /// The platform's custody of the store key (ADR-0060 decision 3): the
    /// Keystore at `background-compatible` under the store's own alias,
    /// whatever the seed's policy is, so the store opens on an unattended
    /// restart.
    pub store_cipher: Box<dyn KeyCipher + Send>,
    /// The unlock policy the identity's start left in force
    /// ([`crate::identity::start_resealed`]); `None` where no start of the
    /// record ran (a test's own identity).
    pub unlock: Option<UnlockStatus>,
}

/// Why the client did not start. Closed, so the Service maps each to its
/// own message; every detail is for the log only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartRefused {
    /// The embedded runtime refused, naming why -- or, with no network
    /// access, the profile or its directories were refused for the same
    /// reasons the runtime would have named.
    Runtime(EmbeddedRefused),
    /// The endpoint the profile names for the Android service
    /// (`runtime.android.endpoint`) is not an enabled entry open to this
    /// client's kind.
    NoHumanEndpoint,
    /// The message store needs recovery: corrupt, from a newer version,
    /// its migration failed, or its key is missing or does not open it
    /// (ADR-0060 decision 6). It was not renamed, moved or deleted; the
    /// recovery screen offers the confirmed Discard and recreate
    /// ([`ServiceHost::plan_store_discard`]).
    StoreNeedsRecovery,
    /// The Keystore has invalidated the store's key (ADR-0060 decision 6):
    /// the client offers Discard and recreate directly. A store condition,
    /// never an identity one.
    StoreKeyInvalidated,
    /// The message store, or its directory, is not private to the app.
    StoreNotPrivate(String),
    /// The message store could not be opened now; trying again may work.
    StoreUnavailable(String),
    /// The facade's thread could not be started.
    Thread(String),
    /// The first identity's key could not be sealed, or form 2 not
    /// written, on a form-1 document beside a sealed seed
    /// ([`crate::first_identity::finish`]); the next start tries again.
    FirstIdentity(FirstIdentityRefused),
}

/// A discard that ran, and the hub's count of start answers when it
/// ended.
#[derive(Debug)]
pub struct StoreDiscarded {
    /// What the discard removed.
    pub discarded: at_rest::Discarded,
    /// The start answers said before the discard ended: the restart's
    /// answer counts past it.
    pub answered: StoreGeneration,
}

/// Why a store action was not taken.
#[derive(Debug)]
pub enum StoreActionRefused {
    /// The client is running and holds the store: stop it first. Also
    /// while a facade left running past its stop still finishes its
    /// commands through a connection of its own: a discard would delete
    /// the files under it (`human-client-android.md`, a facade past its
    /// stop, rule 4).
    Running,
    /// The app's data directory or the profile's paths were refused.
    Profile(String),
    /// The store's own refusal.
    Store(StoreError),
}

fn embedded_paths(
    app_data_dir: &std::path::Path,
    profile: &str,
) -> Result<ProfilePaths, StoreActionRefused> {
    let boundary =
        TrustBoundary::new(app_data_dir).map_err(|e| StoreActionRefused::Profile(e.to_string()))?;
    ProfilePaths::resolve_embedded(profile, boundary)
        .map_err(|e| StoreActionRefused::Profile(e.to_string()))
}

/// Why the wait for the client's end returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// The runtime's owner was asked to stop it -- an admin port's
    /// shutdown, or [`ServiceHost::stop`] from another thread -- within
    /// `grace`. The Service answers by stopping.
    ShutdownRequested {
        /// The grace asked for.
        grace: Duration,
    },
    /// The runtime stopped without being asked, its substrate gone: the
    /// Service stops and reports it; it was no request.
    RuntimeEnded,
    /// The effective availability is no longer the one the client started
    /// in -- the person's Stay-reachable choice changed it -- and the
    /// Service restarts in this mode (ADR-0041 A 2026-10-10).
    AvailabilityChanged(AvailabilityMode),
    /// Nothing runs.
    NotRunning,
}

/// What a stop came to: diagnostics, nothing a person acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stopped {
    /// The facade closed its session -- the lease released -- before the
    /// runtime stopped.
    pub session_closed: bool,
    /// The runtime's own answer: the neutral events it dropped over its
    /// life, or why it did not stop cleanly.
    pub runtime: Result<u64, EmbeddedRefused>,
}

struct Running {
    /// `None` in the network-denied posture: no runtime, no lease.
    host: Option<Arc<EmbeddedHost>>,
    /// The posture's own executor for the facade's turns; `None` when
    /// the host's drives them.
    executor: Option<tokio::runtime::Runtime>,
    /// `None` for the runtime alone (`start_runtime_alone`, the
    /// android-e2e cases'): no store, no facade, no lease.
    facade: Option<FacadeLoop>,
}

/// The process's one client host.
pub struct ServiceHost {
    hub: Arc<Hub>,
    running: Mutex<Option<Running>>,
    /// The platform's networks and the runtime that hears them: a lock of
    /// their own, so the callback never waits on a start
    /// (`tests/service.rs`: `a_network_report_never_waits_on_a_start`).
    network: Mutex<NetworkSlot<Arc<EmbeddedHost>>>,
    /// Why the last start's identity key was not opened or made, if it
    /// was not: the app logs its class, as the desktop prints it (#414 F3).
    key_refused: Arc<Mutex<Option<KeyProblem>>>,
    /// Each identity other than the first whose key was not opened at the
    /// last start, by the problem's kind alone, which names no folder
    /// (`a_key_problems_kind_names_no_folder`): the app logs them as it
    /// logs [`Self::identity_key_refused`].
    others_key_refused: Arc<Mutex<Vec<&'static str>>>,
    /// What the facade said for the platform log and the app has not
    /// taken yet ([`Self::take_facade_notes`]): each class once, however
    /// often it was said before the take
    /// (`a_class_said_twice_is_taken_once`).
    facade_notes: Arc<Mutex<Vec<FacadeNote>>>,
    /// The last start's unlock status: set by a start that ran, never by
    /// one redelivered to a running client, whose reseal found the profile
    /// held and so says nothing of the record.
    unlock: Mutex<Option<UnlockStatus>>,
    /// How many starts' custody steps ran: a start whose reseal ran again
    /// owes the P-up notice again (unlock note section 4).
    unlock_starts: std::sync::atomic::AtomicU64,
    /// How many identities the last start's profile held, set before the
    /// store opens so that it is known when the open was refused: Discard
    /// and recreate deletes the whole profile store, and the store screen's
    /// confirmation names every identity in it ([`Self::held_count`]).
    held_count: std::sync::atomic::AtomicUsize,
    /// Armed by [`refuse_next_store_open`](Self::refuse_next_store_open)
    /// or [`invalidate_next_store_key`](Self::invalidate_next_store_key):
    /// the debug build's stand-in for a store that needs recovery, or
    /// whose key the Keystore invalidated; the last armed wins.
    #[cfg(feature = "dev-stand-ins")]
    refuse_store: Mutex<Option<StartRefused>>,
    /// Told as a stop passes its drain and then its facade's close: a
    /// test orders a send inside the close's wait by them, not by a sleep
    /// ([`Self::stop_stages_for_test`]).
    #[cfg(feature = "test-hooks")]
    stop_stages: Mutex<Option<std::sync::mpsc::Sender<StopStage>>>,
}

/// Where a stop has got to, as a test hears it. TEST BUILDS ONLY.
#[cfg(feature = "test-hooks")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopStage {
    /// The drain returned; the facade's close begins next.
    Drained,
    /// The facade's close returned; the runtime's stop comes next.
    Closed,
}

impl Default for ServiceHost {
    fn default() -> Self {
        Self::new()
    }
}

/// Unix milliseconds, for what the store and the envelope record.
fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

impl ServiceHost {
    /// A host with nothing running. The app uses [`global`](Self::global);
    /// a test makes its own.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hub: Arc::new(Hub::new()),
            running: Mutex::new(None),
            network: Mutex::new(NetworkSlot::default()),
            key_refused: Arc::new(Mutex::new(None)),
            others_key_refused: Arc::new(Mutex::new(Vec::new())),
            facade_notes: Arc::new(Mutex::new(Vec::new())),
            unlock: Mutex::new(None),
            unlock_starts: std::sync::atomic::AtomicU64::new(0),
            held_count: std::sync::atomic::AtomicUsize::new(1),
            #[cfg(feature = "dev-stand-ins")]
            refuse_store: Mutex::new(None),
            #[cfg(feature = "test-hooks")]
            stop_stages: Mutex::new(None),
        }
    }

    /// Refuse the next start's store open as one that needs recovery,
    /// once, leaving the files as they are: the debug build's adb hook
    /// (unlock note section 8, A16's discard), since a store the phone
    /// damages on demand does not exist. The recovery screen then plans
    /// and runs a real discard of the store the files hold, and the
    /// restart after it opens.
    #[cfg(feature = "dev-stand-ins")]
    pub fn refuse_next_store_open(&self) {
        self.arm_store_refusal(StartRefused::StoreNeedsRecovery);
    }

    /// Refuse the next start's store open as one whose key the Keystore
    /// invalidated, once, leaving the files and the key as they are: the
    /// debug build's adb hook for K (identity note section 10 item 15),
    /// since the phone invalidates a key only when its screen lock goes.
    /// K's discard is then a real one, and the restart after it opens a
    /// new store.
    #[cfg(feature = "dev-stand-ins")]
    pub fn invalidate_next_store_key(&self) {
        self.arm_store_refusal(StartRefused::StoreKeyInvalidated);
    }

    #[cfg(feature = "dev-stand-ins")]
    fn arm_store_refusal(&self, refused: StartRefused) {
        *self
            .refuse_store
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(refused);
    }

    fn note_held(&self, held: &Held) {
        self.held_count
            .store(held.held.len(), std::sync::atomic::Ordering::Release);
    }

    /// The profile's store for a start, opened as `identity` -- the
    /// endpoint the service speaks as, whose derived id schema v14 gives
    /// every row an older store holds (STATE.md §Migrations) -- or the
    /// refusal a start answers.
    #[cfg_attr(
        not(feature = "dev-stand-ins"),
        expect(
            clippy::unused_self,
            reason = "the dev-stand-ins build reads the armed refusal"
        )
    )]
    fn open_store(
        &self,
        paths: &ProfilePaths,
        identity: &interweave_transport_api::EndpointId,
        cipher: &dyn KeyCipher,
    ) -> Result<HumanStore, StartRefused> {
        #[cfg(feature = "dev-stand-ins")]
        if let Some(refused) = self
            .refuse_store
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            return Err(refused);
        }
        HumanStore::open_profile(paths, cipher, identity, StoreOptions::default())
            .map_err(|e| classify(&e))
    }

    /// The process's host: one per process, because the profile lock is
    /// (a second embedded host in one process is refused `LockHeld`).
    pub fn global() -> &'static Self {
        static HOST: OnceLock<ServiceHost> = OnceLock::new();
        HOST.get_or_init(Self::new)
    }

    /// The roster made by `make`, each held identity given its key from
    /// its own file under the Keystore at that identity's own alias
    /// (ADR-0063 decision 2, app-core's `KeySource`), one source per
    /// identity (`each_held_identity_opens_its_own_key`): opened, or made
    /// the first time before any share of its card. The store's cipher is
    /// the same bridge, each secret under its own alias namespace. A file
    /// this build refuses, or one gone after a share, is left as it is:
    /// the identity's page says so and offers what it can, and its
    /// identity holds its lease all the same (ADR-0067 decisions 3 and 5).
    /// The first identity's problem is kept for
    /// [`Self::identity_key_refused`]; another's, by kind alone, for
    /// [`Self::other_identity_keys_refused`].
    fn with_key_sources<B, A>(
        &self,
        make: impl FnOnce() -> Result<Roster<B, A>, RosterError> + Send + 'static,
        sources: Vec<KeySource>,
    ) -> impl FnOnce() -> Result<Roster<B, A>, RosterError> + Send + 'static
    where
        B: interweave_human_transport_client::SessionBinding,
        A: interweave_local_client_api::AdminBinding,
    {
        let refused = Arc::clone(&self.key_refused);
        let others = Arc::clone(&self.others_key_refused);
        move || {
            let mut roster = make()?;
            let first = roster.first().clone();
            let mut first_problem = None;
            let mut other_problems = Vec::new();
            for source in sources {
                let endpoint = source.endpoint().clone();
                // Every source names a held identity: the roster holds
                // what the sources were made from.
                let problem = roster.side_mut(&endpoint).and_then(|side| {
                    side.set_key_source(source);
                    side.key_problem().cloned()
                });
                if endpoint == first {
                    first_problem = problem;
                } else if let Some(problem) = problem {
                    other_problems.push(key_problem_kind(&problem));
                }
            }
            *refused.lock().unwrap_or_else(PoisonError::into_inner) = first_problem;
            *others.lock().unwrap_or_else(PoisonError::into_inner) = other_problems;
            Ok(roster)
        }
    }

    /// Where the facade says what the platform log names, by class (no
    /// [`FacadeNote`] variant carries a field, the build's check where it
    /// is declared): kept until the app takes it
    /// ([`Self::take_facade_notes`]; `a_class_said_twice_is_taken_once`).
    fn facade_note(&self) -> impl Fn(FacadeNote) + Send + 'static {
        let notes = Arc::clone(&self.facade_notes);
        move |note| {
            note.keep_in(&mut notes.lock().unwrap_or_else(PoisonError::into_inner));
        }
    }

    /// What the facade said for the platform log since the last take, by
    /// class and never content (no [`FacadeNote`] variant carries a field,
    /// the build's check where it is declared), each once
    /// (`a_class_said_twice_is_taken_once`): the app logs them after a
    /// start, as it logs [`Self::identity_key_refused`].
    #[must_use]
    pub fn take_facade_notes(&self) -> Vec<FacadeNote> {
        std::mem::take(
            &mut *self
                .facade_notes
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }

    /// Why the last start ran with no identity key, if it did: a class for
    /// the platform log, not the key.
    #[must_use]
    pub fn identity_key_refused(&self) -> Option<KeyProblem> {
        self.key_refused
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The kind of each problem the last start met opening the key of an
    /// identity other than the first, in held order: a class for the
    /// platform log, its kind alone (`a_key_problems_kind_names_no_folder`).
    #[must_use]
    pub fn other_identity_keys_refused(&self) -> Vec<&'static str> {
        self.others_key_refused
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Where the view attaches and the platform reads its notices.
    #[must_use]
    pub fn hub(&self) -> &Arc<Hub> {
        &self.hub
    }

    /// How many identities the last start's profile held, readable when
    /// that start's store open was refused (one before any start): the
    /// count the store recovery screen's confirmation says its discard
    /// deletes (`the_held_count_is_known_when_the_store_open_is_refused`).
    #[must_use]
    pub fn held_count(&self) -> usize {
        self.held_count.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Whether the client runs now.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.lock().is_some()
    }

    /// The store at rest, as the recovery screen and diagnostics show it
    /// (ADR-0060): the custody, the key's presence and the files. Reads
    /// only; a running client is no obstacle.
    ///
    /// # Errors
    /// [`StoreActionRefused`].
    pub fn store_health(
        &self,
        app_data_dir: &std::path::Path,
        profile: &str,
        store_cipher: &dyn KeyCipher,
    ) -> Result<at_rest::AtRestReport, StoreActionRefused> {
        let paths = embedded_paths(app_data_dir, profile)?;
        at_rest::inspect(&paths, store_cipher).map_err(StoreActionRefused::Store)
    }

    /// What Discard and recreate would remove now, for the screen that asks
    /// the person to confirm it (ADR-0060 decision 6). Writes nothing.
    ///
    /// # Errors
    /// [`StoreActionRefused::Running`] while the client holds the store.
    pub fn plan_store_discard(
        &self,
        app_data_dir: &std::path::Path,
        profile: &str,
        store_cipher: &dyn KeyCipher,
    ) -> Result<at_rest::DiscardPlan, StoreActionRefused> {
        if self.lock().is_some() || self.hub.facade_alive() {
            return Err(StoreActionRefused::Running);
        }
        let paths = embedded_paths(app_data_dir, profile)?;
        at_rest::plan_discard(&paths, store_cipher).map_err(StoreActionRefused::Store)
    }

    /// Run the Discard the person confirmed; the next start recreates the
    /// store. Held under the host's lock so no start opens the store while
    /// it goes, and the hub's count of start answers is read under it: an
    /// answer counted past [`StoreDiscarded::answered`] is a later start's.
    ///
    /// # Errors
    /// [`StoreActionRefused`]; [`StoreError::DiscardChanged`] inside it
    /// when the store moved since the plan, nothing removed.
    pub fn discard_store(
        &self,
        app_data_dir: &std::path::Path,
        profile: &str,
        store_cipher: &dyn KeyCipher,
        confirmed: at_rest::ConfirmedDiscard,
    ) -> Result<StoreDiscarded, StoreActionRefused> {
        let running = self.lock();
        // A facade's thread is alive only while a start made one, and
        // none starts under this lock: what is read here holds until the
        // discard ends.
        if running.is_some() || self.hub.facade_alive() {
            return Err(StoreActionRefused::Running);
        }
        let paths = embedded_paths(app_data_dir, profile)?;
        let discarded =
            at_rest::discard(&paths, store_cipher, confirmed).map_err(StoreActionRefused::Store)?;
        Ok(StoreDiscarded {
            discarded,
            answered: self.hub.store_generation(),
        })
    }

    /// A start the platform side refused before it reached this host -- no
    /// profile, no identity, no Keystore -- said to the view as a refusal
    /// no store screen shows, under the start lock as every answer is.
    /// Nothing while the client runs: that start changed nothing.
    pub fn refused_before_start(&self) {
        let running = self.lock();
        if running.is_none() {
            self.hub.set_store_answer(StoreAnswer::OtherRefusal);
        }
    }

    /// [`refused_before_start`](Self::refused_before_start) by a start
    /// whose custody step ran, with what it learnt of the unlock policy: a
    /// start refused `UserNotAuthenticated` has
    /// [`crate::identity::locked_status`], one refused after its reseal
    /// the reseal's status, any other refusal of the record none. Where
    /// the custody step did not run, the last start's status stands: the
    /// record is as it left it. Nothing while the client runs.
    pub fn refused_before_start_with(&self, unlock: Option<UnlockStatus>) {
        let running = self.lock();
        if running.is_none() {
            *self.unlock_slot() = unlock;
            self.unlock_start_ran();
            self.hub.set_store_answer(StoreAnswer::OtherRefusal);
        }
    }

    /// [`refused_before_start_with`](Self::refused_before_start_with)
    /// when `custody_ran`, else
    /// [`refused_before_start`](Self::refused_before_start), by a start
    /// that compared the identity it unlocks with C4's `PeerId` and was
    /// refused for it -- another `PeerId`, or a refusal that says nothing
    /// of the record: the view shows `Refused` with Try again where it
    /// would show no service running (design
    /// 2026-10-19-android-identity.md section 6, An answer after the
    /// bound). Nothing while the client runs.
    pub fn refused_compared(&self, unlock: Option<UnlockStatus>, custody_ran: bool) {
        let running = self.lock();
        if running.is_none() {
            if custody_ran {
                *self.unlock_slot() = unlock;
                self.unlock_start_ran();
            }
            self.hub.set_compared_refusal();
        }
    }

    /// [`refused_before_start_with`](Self::refused_before_start_with) by a
    /// start refused `UserNotAuthenticated`, with its
    /// [`crate::identity::locked_status`]: answered as
    /// [`StoreAnswer::Locked`], since the start the person's Unlock runs
    /// may still open the store. Nothing while the client runs.
    pub fn refused_locked(&self, unlock: Option<UnlockStatus>) {
        let running = self.lock();
        if running.is_none() {
            *self.unlock_slot() = unlock;
            self.hub.set_store_answer(StoreAnswer::Locked);
        }
    }

    /// The Activity's check found that no start can run until the person
    /// authenticates, and asked them before any start
    /// ([`crate::identity::locked_status`]): what the locked view shows,
    /// set while nothing runs, with no start answered -- none was
    /// refused. Nothing while the client runs.
    pub fn locked_before_start(&self, unlock: Option<UnlockStatus>) {
        let running = self.lock();
        if running.is_none() {
            *self.unlock_slot() = unlock;
        }
    }

    fn unlock_slot(&self) -> std::sync::MutexGuard<'_, Option<UnlockStatus>> {
        // Replaced whole.
        self.unlock.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The unlock policy the last start left in force, the configured one
    /// and a reseal left pending: what the person's view shows of a
    /// changed `key_unlock_policy` (ADR-0042 A 2026-11-05). Kept after a
    /// stop, since the record has not changed; `None` before any start
    /// learnt it.
    #[must_use]
    pub fn unlock_status(&self) -> Option<UnlockStatus> {
        *self.unlock_slot()
    }

    /// A start's custody step ran: its reseal was tried again. A start
    /// refused for the person, or the Activity's locked check, tried
    /// nothing and counts no start.
    fn unlock_start_ran(&self) {
        self.unlock_starts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Say the unlock note's four facts to the view (section 3): the last
    /// start's status and
    /// [`restart_requires_user_authentication`](Self::restart_requires_user_authentication)
    /// for `profile` under
    /// `app_data_dir`, with the count of starts whose custody ran. Called
    /// by the platform after every start, refusal and locked check, since
    /// the mode at rest is read from the profile it names.
    pub fn say_unlock(&self, app_data_dir: &std::path::Path, profile: &str) {
        let facts = self.unlock_status().map(|status| {
            crate::identity::unlock_facts(
                status,
                self.restart_requires_user_authentication(app_data_dir, profile),
            )
        });
        self.hub.set_unlock(
            facts,
            self.unlock_starts.load(std::sync::atomic::Ordering::SeqCst),
        );
    }

    /// `background_restart_requires_user_authentication` for `profile`
    /// under `app_data_dir`: the policy in force by the last start's status,
    /// under the running host's effective availability -- or, while no
    /// runtime runs (locked, network denied, stopped), the mode read from
    /// disk ([`crate::identity::restart_requires_user_authentication_at_rest`]).
    /// `None` while no start learnt the policy, or the mode cannot be read.
    #[must_use]
    pub fn restart_requires_user_authentication(
        &self,
        app_data_dir: &std::path::Path,
        profile: &str,
    ) -> Option<bool> {
        let unlock = self.unlock_status()?;
        match self.availability() {
            Some(mode) => Some(unlock.restart_requires_user_authentication(mode)),
            None => crate::identity::restart_requires_user_authentication_at_rest(
                app_data_dir,
                profile,
                &unlock,
            ),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Running>> {
        // Each field is replaced whole, so a panic that poisoned the lock
        // left nothing half-written.
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn network(&self) -> std::sync::MutexGuard<'_, NetworkSlot<Arc<EmbeddedHost>>> {
        // A report is taken whole or not at all.
        self.network.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Start the runtime, open the store under the runtime's paths and run
    /// the facade over the runtime's binding. A start while the client
    /// runs does nothing and succeeds: the platform delivers a start
    /// command again to a Service it restarts.
    ///
    /// BLOCKS; call it off the Service's main thread.
    ///
    /// # Errors
    /// [`StartRefused`]; nothing is left running.
    pub fn start(&self, launch: ServiceLaunch) -> Result<(), StartRefused> {
        self.start_with(launch, EmbeddedHost::binding)
    }

    /// [`start`](Self::start), with the facade's data binding wrapped in a
    /// recorder of every payload the runtime hands it, for the android-e2e
    /// cases that check what crossed (`interweave-android-e2e-cases`'
    /// `human_chat`). Compiled under the `e2e-cases` feature alone, off by
    /// default; the app's debug build turns it on for its instrumentation
    /// (apps/human-android/src/e2e.rs), its release build never does.
    ///
    /// # Errors
    /// [`StartRefused`], as for [`start`](Self::start).
    #[cfg(feature = "e2e-cases")]
    pub fn start_recording(
        &self,
        launch: ServiceLaunch,
        tap: interweave_android_e2e_cases::Tap,
    ) -> Result<(), StartRefused> {
        self.start_with(launch, move |host| {
            interweave_android_e2e_cases::Recording::new(host.binding(), tap)
        })
    }

    /// Start the runtime ALONE, as [`start`](Self::start) starts it -- the
    /// same launch, identity and paths, the one host this process holds --
    /// with no store, no facade and so no endpoint lease: what the
    /// android-e2e cases of `NEED_A_RUNTIME` run on, leasing `human`
    /// themselves (the seam agreed with p2p-network-dev-02, 01a128a6/
    /// 01a128a8). A start while anything runs does nothing and succeeds.
    /// TEST BUILDS ONLY, under `e2e-cases`.
    ///
    /// # Errors
    /// [`StartRefused::Runtime`].
    #[cfg(feature = "e2e-cases")]
    pub fn start_runtime_alone(&self, launch: ServiceLaunch) -> Result<(), StartRefused> {
        let mut running = self.lock();
        if running.is_some() {
            return Ok(());
        }
        *self.unlock_slot() = launch.unlock;
        self.unlock_start_ran();
        first_identity(&launch)?;
        let host = EmbeddedHost::start(EmbeddedLaunch {
            app_data_dir: launch.app_data_dir,
            profile: launch.profile,
            identity: launch.identity,
        })
        .map_err(StartRefused::Runtime)?;
        *running = Some(Running {
            host: Some(Arc::new(host)),
            executor: None,
            facade: None,
        });
        Ok(())
    }

    /// The binding of the runtime now serving, for an android-e2e case to
    /// open its own sessions on; `None` while no runtime runs. Under
    /// `e2e-cases`, never a second host.
    #[cfg(feature = "e2e-cases")]
    #[must_use]
    pub fn runtime_binding(
        &self,
    ) -> Option<
        impl interweave_local_client_api::DataSessionBinding
        + interweave_local_client_api::AdminBinding
        + Clone
        + Send
        + 'static,
    > {
        self.lock()
            .as_ref()
            .and_then(|running| running.host.as_ref())
            .map(|host| host.binding())
    }

    /// The start, with the facade's data binding made by `data` from the
    /// runtime's host; the admin binding is the runtime's own either way.
    fn start_with<B>(
        &self,
        launch: ServiceLaunch,
        data: impl FnOnce(&EmbeddedHost) -> B,
    ) -> Result<(), StartRefused>
    where
        B: interweave_human_transport_client::SessionBinding + Clone + Send + 'static,
    {
        let mut running = self.lock();
        if running.is_some() {
            return Ok(());
        }
        // Whatever the rest of the start comes to, its custody step ran.
        *self.unlock_slot() = launch.unlock;
        self.unlock_start_ran();
        // The debug build's hook: refused as the runtime refuses a profile
        // another host holds, answered on the path every refusal takes.
        #[cfg(any(test, feature = "dev-stand-ins"))]
        if busy_hook::take() {
            let started = Err(StartRefused::Runtime(EmbeddedRefused::LockHeld(
                "the busy-start hook".to_owned(),
            )));
            self.answer(&started);
            return started;
        }
        let started = self.start_locked(&mut running, launch, data);
        self.answer(&started);
        started
    }

    /// The debug build's busy-start hook: each of the next `starts`
    /// service starts is refused with the profile in use
    /// (`EmbeddedRefused::LockHeld`) after its custody step, as a profile
    /// another host holds refuses it, so `IdStartBusy` can be shown on a
    /// phone where nothing else holds the lock (identity note §9,
    /// acceptance 12 and 15). Debug builds and this crate's tests only.
    #[cfg(any(test, feature = "dev-stand-ins"))]
    pub fn refuse_next_starts_busy(&self, starts: u32) {
        busy_hook::arm(starts);
    }

    /// Say a start's answer to the hub, while the caller holds the start
    /// lock: a discard reads the hub's count under it too, so every answer
    /// counted past the discard's came from a start after it (#377 review
    /// r3, risk 2).
    fn answer(&self, started: &Result<(), StartRefused>) {
        let answer = match started {
            Ok(()) => Some(StoreAnswer::Opened),
            Err(why) => store_answer(why),
        };
        if let Some(answer) = answer {
            self.hub.set_store_answer(answer);
        }
    }

    /// [`start_with`](Self::start_with) once the lock is held and nothing
    /// runs; the caller says the answer.
    fn start_locked<B>(
        &self,
        running: &mut Option<Running>,
        launch: ServiceLaunch,
        data: impl FnOnce(&EmbeddedHost) -> B,
    ) -> Result<(), StartRefused>
    where
        B: interweave_human_transport_client::SessionBinding + Clone + Send + 'static,
    {
        first_identity(&launch)?;
        let host = EmbeddedHost::start(EmbeddedLaunch {
            app_data_dir: launch.app_data_dir,
            profile: launch.profile,
            identity: launch.identity,
        })
        .map_err(StartRefused::Runtime)?;
        // From here a refusal stops the host before it answers.
        let refuse = |host: EmbeddedHost, why: StartRefused| {
            let _ = host.stop(Duration::ZERO);
            Err(why)
        };
        let config = match ProfileConfig::load(host.paths()) {
            Ok(config) => config,
            Err(e) => {
                return refuse(
                    host,
                    StartRefused::Runtime(EmbeddedRefused::ProfileInvalid(e.to_string())),
                );
            }
        };
        // The endpoint the Android profile names for its service
        // (`runtime.android.endpoint`, `human` by default), which must be
        // an enabled entry open to this client's kind.
        let Some(endpoint) = human_endpoint(&config) else {
            return refuse(host, StartRefused::NoHumanEndpoint);
        };
        // Read before the store opens: a refusal opens nothing.
        let held = match held_identities(host.paths(), &config, endpoint.clone()) {
            Ok(held) => held,
            Err(why) => return refuse(host, StartRefused::Runtime(why)),
        };
        self.note_held(&held);
        let store = match self.open_store(host.paths(), &endpoint, &*launch.store_cipher) {
            Ok(store) => store,
            Err(why) => return refuse(host, why),
        };
        let cipher = SharedCipher::new(launch.store_cipher);
        let sources = key_sources(host.paths(), &held, || Box::new(cipher.clone()));
        let (data, admin) = (data(&host), host.binding());
        let roster_config = roster_config(&config, &held);
        let make = self.with_key_sources(
            move || {
                Roster::new(
                    roster_config,
                    store,
                    move |_| (data.clone(), admin.clone()),
                    wall_ms,
                    0,
                )
            },
            sources,
        );
        let facade = match FacadeLoop::spawn(host.runtime(), make, &self.hub, self.facade_note()) {
            Ok(facade) => facade,
            Err(e) => return refuse(host, spawn_refused(e)),
        };
        self.hub.set_network_access(true);
        let host = Arc::new(host);
        // The callback reported the networks before this runtime existed
        // and does not report them again: it hears them now.
        self.network().attach(Arc::clone(&host));
        *running = Some(Running {
            host: Some(host),
            executor: None,
            facade: Some(facade),
        });
        Ok(())
    }

    /// Start the network-denied posture (human-client-android.md,
    /// "Runtime permissions and the network-denied state"): the platform
    /// withholds network access, so no runtime starts and no lease is
    /// held, but the profile's store opens and the facade runs over a
    /// binding with nothing behind it ([`crate::offline`]). Reading, Keep
    /// and Unkeep go on as the store defines them; nothing is sent or
    /// joined. The view is told access is withheld. A start while the
    /// client runs, in either posture, does nothing and succeeds; the
    /// Service stops the posture before a full [`start`](Self::start).
    ///
    /// BLOCKS; call it off the Service's main thread.
    ///
    /// # Errors
    /// [`StartRefused`]; nothing is left running.
    pub fn start_without_network(
        &self,
        app_data_dir: &std::path::Path,
        profile: &str,
        store_cipher: Arc<dyn KeyCipher + Send + Sync>,
    ) -> Result<(), StartRefused> {
        let mut running = self.lock();
        if running.is_some() {
            return Ok(());
        }
        let started = self.start_offline_locked(&mut running, app_data_dir, profile, store_cipher);
        self.answer(&started);
        started
    }

    /// [`start_without_network`](Self::start_without_network) once the
    /// lock is held and nothing runs; the caller says the answer.
    fn start_offline_locked(
        &self,
        running: &mut Option<Running>,
        app_data_dir: &std::path::Path,
        profile: &str,
        store_cipher: Arc<dyn KeyCipher + Send + Sync>,
    ) -> Result<(), StartRefused> {
        crate::first_identity::at_start(app_data_dir, profile, &*store_cipher)
            .map_err(StartRefused::FirstIdentity)?;
        let refused = |e: EmbeddedRefused| StartRefused::Runtime(e);
        let boundary = TrustBoundary::new(app_data_dir).map_err(|e| refused(e.into()))?;
        let paths =
            ProfilePaths::resolve_embedded(profile, boundary).map_err(|e| refused(e.into()))?;
        // This posture reaches no host, so the host's half of M3 (form 1
        // beside one sealed first key -> form 2) runs here too, under the
        // profile lock taken without waiting; another holder is left to
        // the steps below to answer
        // (`a_start_without_network_finishes_m3_beside_one_sealed_key`).
        if let Ok(_held) = interweave_profile_config::ProfileLock::acquire(&paths, Duration::ZERO) {
            interweave_transport_embedded::finish_first_identity(&paths).map_err(refused)?;
        }
        let config = ProfileConfig::load(&paths).map_err(|e| refused(e.into()))?;
        let endpoint = human_endpoint(&config).ok_or(StartRefused::NoHumanEndpoint)?;
        // Read before the store opens, as a start with network reads it.
        let held = held_identities(&paths, &config, endpoint.clone()).map_err(refused)?;
        self.note_held(&held);
        let store = self.open_store(&paths, &endpoint, &*store_cipher)?;
        let executor = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("interweave-offline")
            .enable_time()
            .build()
            .map_err(|e| StartRefused::Thread(e.to_string()))?;
        let sources = key_sources(&paths, &held, move || Box::new(Arc::clone(&store_cipher)));
        let roster_config = roster_config(&config, &held);
        let make = self.with_key_sources(
            move || {
                Roster::new(
                    roster_config,
                    store,
                    |_| (offline_embedded(), offline_embedded()),
                    wall_ms,
                    0,
                )
            },
            sources,
        );
        // Said before the facade runs, so a view never shows this posture
        // as a service that can send.
        self.hub.set_network_access(false);
        let facade = FacadeLoop::spawn(
            executor.handle().clone(),
            make,
            &self.hub,
            self.facade_note(),
        )
        .map_err(spawn_refused)?;
        *running = Some(Running {
            host: None,
            executor: Some(executor),
            facade: Some(facade),
        });
        Ok(())
    }

    /// Whether the client runs in the network-denied posture.
    #[must_use]
    pub fn is_network_denied(&self) -> bool {
        self.lock().as_ref().is_some_and(|r| r.host.is_none())
    }

    /// The profile's effective availability while the client runs:
    /// whether the Service keeps it reachable as a foreground service
    /// (ADR-0041) -- the person's Stay-reachable choice where one is
    /// persisted, else the profile's authored default (ADR-0041 A
    /// 2026-10-10), as the embedded host reads it. `None` while nothing
    /// runs.
    #[must_use]
    pub fn availability(&self) -> Option<AvailabilityMode> {
        self.lock()
            .as_ref()
            .and_then(|r| r.host.as_ref())
            .map(|host| host.availability())
    }

    /// The log level the running runtime loaded from its profile: the
    /// level the app's platform log follows around every start while one
    /// runs, since a start redelivered then starts nothing and the file
    /// may have changed since. `None` while no runtime runs.
    #[must_use]
    pub fn log_level(&self) -> Option<LogLevel> {
        self.lock()
            .as_ref()
            .and_then(|r| r.host.as_ref())
            .map(|host| host.log_level())
    }

    /// Record the person's Stay-reachable choice: `Some` turns it on,
    /// `None` removes it and the authored default applies again. The
    /// running client's wait then answers [`Ended::AvailabilityChanged`]
    /// when the effective mode moved. Call it only on the person's act.
    ///
    /// BLOCKS; call it off the Service's main thread.
    ///
    /// # Errors
    /// The host's, when the choice could not be written (the entry is
    /// then as it was); [`EmbeddedRefused::Internal`] when nothing runs.
    pub fn set_availability(&self, choice: Option<StayReachable>) -> Result<(), EmbeddedRefused> {
        let Some(host) = self.lock().as_ref().and_then(|r| r.host.clone()) else {
            return Err(EmbeddedRefused::Internal(
                "the network service is not running".to_owned(),
            ));
        };
        host.set_availability(choice)
    }

    /// End the runtime as if its substrate had gone, for a test of
    /// [`Ended::RuntimeEnded`]. TEST BUILDS ONLY.
    #[cfg(feature = "test-hooks")]
    pub fn end_runtime_for_test(&self) {
        if let Some(host) = self.lock().as_ref().and_then(|r| r.host.as_ref()) {
            host.end_runtime_for_test();
        }
    }

    /// Told each stage a stop with a runtime passes ([`StopStage`]), in
    /// order; a stop with no runtime reports none. TEST BUILDS ONLY.
    #[cfg(feature = "test-hooks")]
    pub fn stop_stages_for_test(&self) -> std::sync::mpsc::Receiver<StopStage> {
        let (tell, stages) = std::sync::mpsc::channel();
        *self
            .stop_stages
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(tell);
        stages
    }

    #[cfg(feature = "test-hooks")]
    fn tell_stop_stage(&self, stage: StopStage) {
        if let Some(tell) = self
            .stop_stages
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            let _ = tell.send(stage);
        }
    }

    /// Wait until the client is asked to stop. Returns
    /// [`Ended::NotRunning`] at once when no runtime runs: nothing, or
    /// the network-denied posture, which only a stop ends.
    ///
    /// BLOCKS; call it from a thread of the Service's own.
    #[must_use]
    pub fn wait_ended(&self) -> Ended {
        let Some(host) = self.lock().as_ref().and_then(|r| r.host.clone()) else {
            return Ended::NotRunning;
        };
        match host.wait_shutdown_requested() {
            HostEnded::ShutdownRequested(request) => Ended::ShutdownRequested {
                grace: request.grace,
            },
            HostEnded::RuntimeEnded => Ended::RuntimeEnded,
            HostEnded::AvailabilityChanged(mode) => Ended::AvailabilityChanged(mode),
        }
    }

    /// One report from the Service's network callback (plan section 20
    /// step 5): the runtime hears the snapshot the reports make, when it
    /// moved, and a runtime that starts later hears it at its start
    /// ([`crate::network`]). Does not wait, whatever a start or a stop is
    /// doing. What the runtime was told, by count, `None` for nothing.
    pub fn network_report(&self, report: NetworkReport) -> Option<Told> {
        self.network().report(report)
    }

    /// What the running runtime was last told of the platform's networks,
    /// by count -- at its start, when the callback reported before it
    /// existed; `None` while it was told nothing.
    #[must_use]
    pub fn network_told(&self) -> Option<Told> {
        self.network().told()
    }

    /// Stop the client: the runtime refuses inbound direct first, the
    /// facade commits what its session's queue holds unread and closes
    /// the session (releasing the lease), then the runtime stops within
    /// what is left of `grace` and releases the profile lock. `None` when
    /// nothing ran.
    ///
    /// BLOCKS; call it off the Service's main thread.
    #[must_use]
    pub fn stop(&self, grace: Duration) -> Option<Stopped> {
        let Running {
            host,
            executor,
            facade,
        } = self.lock().take()?;
        // Before the stop: the slot's hold would keep the host from it.
        drop(self.network().detach());
        let Some(host) = host else {
            // The network-denied posture: the facade and its executor are
            // all there is. Access is said again at the next start.
            let session_closed = facade.is_none_or(|f| f.close(FACADE_CLOSE));
            if let Some(executor) = executor {
                executor.shutdown_timeout(FACADE_CLOSE);
            }
            return Some(Stopped {
                session_closed,
                runtime: Ok(0),
            });
        };
        let began = Instant::now();
        // Releases a thread parked in `wait_ended` with this request: it
        // holds the host until it returns. It refuses nothing.
        let _ = host.request_shutdown(grace);
        // The daemon's order (`DIRECT.md` §Graceful shutdown, A
        // 2026-11-17): the runtime refuses inbound direct from here, so
        // the facade's close below reads its session's queue for the last
        // time and commits it unread before the session closes. With the
        // close first, a message admitted between the close and the stop
        // went with the session unread, its sender told `AcceptedV2`
        // (`stop_drain.rs`: `a_send_while_the_close_waits_is_refused`
        // holds the refusal before the close,
        // `a_message_admitted_before_the_stop_is_unread_after_it` the
        // close's commit). An error is a runtime already ended: nothing
        // more can arrive.
        let _ = host.drain();
        #[cfg(feature = "test-hooks")]
        self.tell_stop_stage(StopStage::Drained);
        let session_closed = facade.is_none_or(|f| f.close(FACADE_CLOSE));
        #[cfg(feature = "test-hooks")]
        self.tell_stop_stage(StopStage::Closed);
        let deadline = Instant::now() + FACADE_CLOSE;
        let mut host = host;
        let runtime = loop {
            match Arc::try_unwrap(host) {
                // What is left of the grace: the hand-over is inside it.
                Ok(owned) => break owned.stop(grace.saturating_sub(began.elapsed())),
                Err(shared) if Instant::now() < deadline => {
                    host = shared;
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(shared) => {
                    // A waiter that did not let go: the host stops when
                    // its last holder drops it, without waiting for
                    // exchanges in flight (the host's own fallback).
                    drop(shared);
                    break Err(EmbeddedRefused::Internal(
                        "a waiter still held the runtime; it stops when that waiter returns"
                            .to_owned(),
                    ));
                }
            }
        };
        Some(Stopped {
            session_closed,
            runtime,
        })
    }
}

/// The log level `profile`'s `config.yaml` names under `app_data_dir`
/// (`observability.log_level`), read as a start reads the profile -- so
/// the app's platform log can be filtered at it before the runtime logs
/// its first line (plan section 20, the audit sink carried from section
/// 18), and wherever no runtime runs to say which level it loaded.
/// `None` when the profile cannot be read: a start refuses and says why.
#[must_use]
pub fn profile_log_level(app_data_dir: &std::path::Path, profile: &str) -> Option<LogLevel> {
    let boundary = TrustBoundary::new(app_data_dir).ok()?;
    let paths = ProfilePaths::resolve_embedded(profile, boundary).ok()?;
    Some(ProfileConfig::load(&paths).ok()?.observability.log_level)
}

/// The endpoint the Android profile names for its service
/// (`runtime.android.endpoint`, `human` by default), if it is an enabled
/// entry open to this client's kind.
fn human_endpoint(config: &ProfileConfig) -> Option<interweave_transport_api::EndpointId> {
    config.runtime.android.endpoint().filter(|id| {
        config.endpoints.entries.iter().any(|e| {
            &e.id == id
                && e.enabled
                && e.allowed_client_kinds
                    .iter()
                    .any(|k| k.as_str() == CLIENT_KIND)
        })
    })
}

/// The identities the Android client holds (plan section 30 step 3; the
/// note of 2026-11-26 (ii)): the endpoint its profile names for the
/// service first, then every entry the endpoint overlay added that admits
/// this client's kind, in app-core's [`held_list`] order, each with its
/// entry's alias; and the overlay's effective default when it is held.
struct Held {
    /// The first first, so the list holds at least it
    /// (`androids_first_stays_first_and_only_the_added_follow`).
    held: Vec<HeldEntry>,
    default: Option<interweave_transport_api::EndpointId>,
}

/// [`Held`] for `config` under `paths`, `first` being
/// [`human_endpoint`]'s answer, which stays the first whatever app-core's
/// first is: [`held_list`] takes the first AUTHORED entry admitting the
/// kind, and an Android profile may name another for its service
/// (`runtime.android.endpoint`). Only the added entries are taken from
/// it (`androids_first_stays_first_and_only_the_added_follow`).
///
/// # Errors
/// A present overlay that cannot be read: reachable by another user, a
/// directory refused, as the runtime refuses one at its own start; else
/// the profile invalid (the runtime's start answers a parse as internal:
/// either is a runtime refusal, the same code to the Service and the
/// same notice). Never read as "no added entries", since the client would then show a held list the file may
/// contradict (`tests/held.rs`,
/// `an_overlay_that_does_not_read_refuses_the_start_without_network`).
fn held_identities(
    paths: &ProfilePaths,
    config: &ProfileConfig,
    first: interweave_transport_api::EndpointId,
) -> Result<Held, EmbeddedRefused> {
    let refused = |e: EndpointOverlayError| match e {
        EndpointOverlayError::NotPrivate { .. } => EmbeddedRefused::DirectoryRefused(e.to_string()),
        _ => EmbeddedRefused::ProfileInvalid(e.to_string()),
    };
    let list = held_list(paths, config).map_err(refused)?;
    // `held_list` filters the default against its own list, whose first is
    // the first authored entry, so an Android service named for a later
    // authored entry would lose its default there: the overlay's effective
    // default is read again, unfiltered, and held against Android's list.
    let (overlay, _normalised) = EndpointOverlay::read_within(
        &EndpointOverlay::path_for(paths),
        &config.endpoints,
        paths.boundary(),
    )
    .map_err(refused)?;
    let alias = config
        .endpoints
        .entries
        .iter()
        .find(|e| e.id == first)
        .and_then(|e| e.alias.as_ref().map(ToString::to_string));
    Ok(android_held(
        list,
        HeldEntry {
            endpoint: first,
            alias,
        },
        overlay.effective(&config.endpoints).default,
    ))
}

/// [`held_identities`]' rule on a list already read, `first` being
/// Android's first identity with its entry's alias and `default` the
/// overlay's effective default, not yet held against any list.
fn android_held(
    list: HeldList,
    first: HeldEntry,
    default: Option<interweave_transport_api::EndpointId>,
) -> Held {
    let mut held = vec![first];
    // `held_list`'s own first is the first authored entry, so skipping it
    // drops no added one (app-core's `a_second_authored_human_entry_is_not_held`).
    for entry in list.held.into_iter().skip(1) {
        if !held.iter().any(|h| h.endpoint == entry.endpoint) {
            held.push(entry);
        }
    }
    let default = default.filter(|d| held.iter().any(|h| &h.endpoint == d));
    Held { held, default }
}

/// One key source per held identity, each with its own cipher from
/// `cipher`: a source owns its cipher, and every one is the same bridge.
fn key_sources(
    paths: &ProfilePaths,
    held: &Held,
    mut cipher: impl FnMut() -> Box<dyn KeyCipher + Send>,
) -> Vec<KeySource> {
    held.held
        .iter()
        .map(|h| KeySource::new(paths.clone(), h.endpoint.clone(), cipher()))
        .collect()
}

/// The launch's one store cipher shared by every key source: the start
/// is given one box, and each source owns a cipher of its own.
#[derive(Clone)]
struct SharedCipher(Arc<Mutex<Box<dyn KeyCipher + Send>>>);

impl SharedCipher {
    fn new(cipher: Box<dyn KeyCipher + Send>) -> Self {
        Self(Arc::new(Mutex::new(cipher)))
    }

    fn cipher(&self) -> std::sync::MutexGuard<'_, Box<dyn KeyCipher + Send>> {
        // A cipher call leaves nothing half-written for the next.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl KeyCipher for SharedCipher {
    fn custody(&self) -> Custody {
        self.cipher().custody()
    }

    fn seal(&self, alias: &str, aad: &[u8], secret: &[u8; 32]) -> Result<Vec<u8>, CipherFailure> {
        self.cipher().seal(alias, aad, secret)
    }

    fn open(
        &self,
        alias: &str,
        aad: &[u8],
        sealed: &[u8],
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, CipherFailure> {
        self.cipher().open(alias, aad, sealed)
    }

    fn retire(&self, alias: &str) -> Result<(), CipherFailure> {
        self.cipher().retire(alias)
    }
}

/// A key problem's kind, for the platform log: not the folder a
/// [`KeyProblem::DirUnsafe`] names (`a_key_problems_kind_names_no_folder`).
const fn key_problem_kind(problem: &KeyProblem) -> &'static str {
    match problem {
        KeyProblem::LostOnDevice => "lost on device",
        KeyProblem::BelowDevice => "below device",
        KeyProblem::OtherDevice => "other device",
        KeyProblem::NewerVersion => "newer version",
        KeyProblem::Damaged => "damaged",
        KeyProblem::Exposed => "exposed",
        KeyProblem::DirUnsafe { .. } => "folder unsafe",
        KeyProblem::NotNow => "not now",
        KeyProblem::Missing => "missing",
    }
}

/// The roster of the identities `held` names: the first, whose store
/// this is, then the others, each with its alias where its entry has one
/// (`Roster::held_if_changed` tells them), the default the overlay's
/// effective one when held.
fn roster_config(config: &ProfileConfig, held: &Held) -> RosterConfig {
    let endpoints: Vec<_> = held.held.iter().map(|h| h.endpoint.clone()).collect();
    RosterConfig {
        client_kind: CLIENT_KIND.to_owned(),
        // Every identity held, so each client's lease target is in it.
        endpoints: endpoints.clone(),
        channels: config.channels.desired.clone(),
        max_payload_bytes: MAX_PAYLOAD_BYTES,
        others: endpoints.into_iter().skip(1).collect(),
        default: held.default.clone(),
        aliases: held
            .held
            .iter()
            .filter_map(|h| Some((h.endpoint.clone(), h.alias.clone()?)))
            .collect(),
    }
}

/// A facade that did not start, as the start's refusal. More identities
/// than a client holds -- an overlay that added more human-client entries
/// than the identity cap leaves room for -- is refused as a store that
/// cannot be opened now, the detail for the log.
fn spawn_refused(error: SpawnError) -> StartRefused {
    match error {
        SpawnError::Store(e) => classify(&e),
        SpawnError::Thread(e) => StartRefused::Thread(e.to_string()),
        SpawnError::OverBound => {
            StartRefused::StoreUnavailable(format!("{:?}", RosterError::OverBound))
        }
    }
}

/// A start's half of M3, before the embedded host takes the profile's
/// lock: the host's start refuses form 1, and finishes it beside a sealed
/// key alone (`crate::first_identity`).
fn first_identity(launch: &ServiceLaunch) -> Result<(), StartRefused> {
    crate::first_identity::at_start(&launch.app_data_dir, &launch.profile, &*launch.store_cipher)
        .map(|_| ())
        .map_err(StartRefused::FirstIdentity)
}

/// What a refused start answers of the store, or `None` for a runtime
/// refused network access: the Service then starts the network-denied
/// posture, whose answer is the start's.
const fn store_answer(why: &StartRefused) -> Option<StoreAnswer> {
    match why {
        StartRefused::StoreNeedsRecovery => Some(StoreAnswer::Refused(StoreRefusal::Recovery)),
        StartRefused::StoreKeyInvalidated => Some(StoreAnswer::Refused(StoreRefusal::KeyLost)),
        StartRefused::Runtime(EmbeddedRefused::NetworkDenied(_)) => None,
        StartRefused::Runtime(EmbeddedRefused::LockHeld(_)) => Some(StoreAnswer::ProfileBusy),
        _ => Some(StoreAnswer::OtherRefusal),
    }
}

fn classify(error: &StoreError) -> StartRefused {
    if matches!(error, StoreError::KeyInvalidated) {
        return StartRefused::StoreKeyInvalidated;
    }
    if error.needs_recovery() {
        return StartRefused::StoreNeedsRecovery;
    }
    match error {
        StoreError::NotAFile { .. }
        | StoreError::PermissionsTooOpen { .. }
        | StoreError::DirectoryNotPrivate { .. } => {
            StartRefused::StoreNotPrivate(error.to_string())
        }
        _ => StartRefused::StoreUnavailable(error.to_string()),
    }
}

/// [`ServiceHost::refuse_next_starts_busy`]'s count, taken one start at a
/// time (`the_busy_start_hook_is_taken_one_start_at_a_time`).
#[cfg(any(test, feature = "dev-stand-ins"))]
mod busy_hook {
    use std::sync::atomic::{AtomicU32, Ordering};

    static LEFT: AtomicU32 = AtomicU32::new(0);

    pub(super) fn arm(starts: u32) {
        LEFT.store(starts, Ordering::SeqCst);
    }

    /// Whether this start is to be refused: one of the armed count, spent.
    pub(super) fn take() -> bool {
        LEFT.try_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
            left.checked_sub(1)
        })
        .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the facade says reaches the take once per class, however
    /// often it was said, and a take empties it. Mutation: the closure
    /// `facade_note` returns pushing every note fails the "taken once"
    /// assert.
    #[test]
    fn a_class_said_twice_is_taken_once() {
        let host = ServiceHost::new();
        let note = host.facade_note();
        note(FacadeNote::FrontUnread);
        note(FacadeNote::FrontUnremembered);
        note(FacadeNote::FrontUnread);
        assert_eq!(
            host.take_facade_notes(),
            vec![FacadeNote::FrontUnread, FacadeNote::FrontUnremembered],
            "taken once"
        );
        assert!(host.take_facade_notes().is_empty(), "a take empties it");
    }

    /// Mutation: `take` answering true without spending one of the count
    /// refuses every start after the hook.
    #[test]
    fn the_busy_start_hook_is_taken_one_start_at_a_time() {
        assert!(!busy_hook::take(), "unarmed");
        busy_hook::arm(2);
        assert!(busy_hook::take());
        assert!(busy_hook::take());
        assert!(!busy_hook::take(), "two, then none");
    }

    /// A runtime refused network access answers nothing: the Service goes
    /// on to the network-denied posture, whose answer is the start's, so a
    /// store screen waiting on the restart does not leave on the first
    /// half of it. Every other refusal answers.
    #[test]
    fn only_a_runtime_refused_network_access_answers_nothing() {
        assert_eq!(
            store_answer(&StartRefused::Runtime(EmbeddedRefused::NetworkDenied(
                String::new()
            ))),
            None
        );
        assert_eq!(
            store_answer(&StartRefused::Runtime(EmbeddedRefused::ProfileInvalid(
                String::new()
            ))),
            Some(StoreAnswer::OtherRefusal)
        );
        assert_eq!(
            store_answer(&StartRefused::NoHumanEndpoint),
            Some(StoreAnswer::OtherRefusal)
        );
        assert_eq!(
            store_answer(&StartRefused::Runtime(EmbeddedRefused::LockHeld(
                String::new()
            ))),
            Some(StoreAnswer::ProfileBusy),
            "told apart from the other runtime refusals (identity note Q12)"
        );
        assert_eq!(
            store_answer(&StartRefused::StoreKeyInvalidated),
            Some(StoreAnswer::Refused(StoreRefusal::KeyLost))
        );
        assert_eq!(
            store_answer(&StartRefused::StoreNeedsRecovery),
            Some(StoreAnswer::Refused(StoreRefusal::Recovery))
        );
    }

    fn id(name: &str) -> interweave_transport_api::EndpointId {
        interweave_test_support::endpoint_keys::endpoint_id(name)
    }

    fn entry(name: &str) -> HeldEntry {
        HeldEntry {
            endpoint: id(name),
            alias: Some(name.to_owned()),
        }
    }

    fn ids(held: &Held) -> Vec<interweave_transport_api::EndpointId> {
        held.held.iter().map(|h| h.endpoint.clone()).collect()
    }

    /// Android's first identity stays first, whichever entry app-core's
    /// `held_list` put first, and only the list's added entries follow it,
    /// in its order, Android's first not twice, and held at least when the
    /// list is empty; the default is the one given when Android holds it,
    /// else none. Mutations: the list's own first kept (`skip(1)`
    /// dropped) -- `authored` is held; the duplicate check dropped --
    /// `android` is held twice; the default left unfiltered -- `authored`
    /// is the default; Android's first not seeded -- the order assert
    /// fails.
    #[test]
    fn androids_first_stays_first_and_only_the_added_follow() {
        let list = HeldList {
            held: vec![
                entry("authored"),
                entry("work"),
                entry("android"),
                entry("home"),
            ],
            default: Some(id("work")),
        };
        let held = android_held(list, entry("android"), Some(id("work")));
        assert_eq!(ids(&held), [id("android"), id("work"), id("home")]);
        assert_eq!(held.held[1].alias.as_deref(), Some("work"));
        assert_eq!(held.default, Some(id("work")));
        let list = HeldList {
            held: vec![entry("authored"), entry("work")],
            default: Some(id("authored")),
        };
        let held = android_held(list, entry("android"), Some(id("authored")));
        assert_eq!(ids(&held), [id("android"), id("work")]);
        assert_eq!(held.default, None, "the default Android does not hold");
        let held = android_held(
            HeldList {
                held: Vec::new(),
                default: None,
            },
            entry("android"),
            None,
        );
        assert_eq!(ids(&held), [id("android")], "held at least the first");
    }

    /// A key problem's kind says what went wrong and not where: the folder
    /// a `DirUnsafe` names is in no kind's text. Mutation: the label of
    /// `DirUnsafe` made a path ("folder/unsafe") -- the separator assert
    /// fails.
    #[test]
    fn a_key_problems_kind_names_no_folder() {
        let folder = "/data/user/0/app/endpoint-keys".to_owned();
        let kind = key_problem_kind(&KeyProblem::DirUnsafe {
            folder: folder.clone(),
        });
        assert!(!kind.contains(&folder), "{kind}");
        assert!(!kind.contains('/'), "{kind}");
    }

    /// A profile of two authored human-client entries, `first` and
    /// `second`, whose default is `second`, and whose Android service is
    /// named for `second` (`runtime.android.endpoint`).
    fn two_authored_android_second() -> ProfileConfig {
        let text = format!(
            "schema_version: 2
profile:
  name: test
runtime:
  deployment: embedded-android
  android:
    endpoint: {second}
ipc:
  enabled: false
transport:
  listen:
    addresses: [\"/ip4/0.0.0.0/tcp/0\"]
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  registration_policy: configured-only
  default_direct_endpoint: {second}
  entries:
    - {{ key: {first}.key, alias: first, enabled: true, advertise: true, allowed_client_kinds: [human-client] }}
    - {{ key: {second}.key, alias: second, enabled: true, advertise: true, allowed_client_kinds: [human-client] }}
channels: {{ desired: [] }}
",
            first = id("first").as_str(),
            second = id("second").as_str(),
        );
        ProfileConfig::parse_yaml(&text).expect("parses")
    }

    /// The effective default is Android's when Android's first is a later
    /// authored entry: `held_list` filters the default against a list
    /// whose first is the first authored entry, so the default naming
    /// Android's own first would be lost there. Mutation: the default
    /// taken from `held_list`'s answer rather than the overlay's, which
    /// answers `None` here (the overlay's default replaced by `None`) --
    /// the final assert fails.
    #[test]
    fn a_default_naming_androids_later_authored_first_is_kept() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("chmod");
        let paths = ProfilePaths::resolve_embedded(
            "default",
            TrustBoundary::new(dir.path()).expect("a boundary"),
        )
        .expect("paths");
        let config = two_authored_android_second();
        let held = held_identities(&paths, &config, id("second")).expect("reads");
        assert_eq!(ids(&held), [id("second")]);
        assert_eq!(held.default, Some(id("second")));
    }
}

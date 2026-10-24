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

use interweave_human_app_core::{FacadeSide, identity_key_under};
use interweave_human_store::at_rest;
use interweave_human_store::identity_key::IdentityKey;
use interweave_human_store::key_custody::KeyCipher;
use interweave_human_store::{HumanStore, StoreError, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_local_client_api::DataSessionBinding;
pub use interweave_profile_config::runtime::AvailabilityMode;
pub use interweave_profile_config::sections::LogLevel;
use interweave_profile_config::{ProfileConfig, ProfilePaths, TrustBoundary};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::MAX_PAYLOAD_BYTES;
use interweave_transport_embedded::{
    EmbeddedHost, EmbeddedLaunch, EmbeddedRefused, Ended as HostEnded, StayReachable,
};

use crate::facade::{FacadeLoop, SpawnError};
use crate::hub::Hub;
use crate::hub::StoreRefusal;
use crate::network::{NetworkReport, NetworkSlot, Told};
use crate::offline::offline_embedded;

/// The client kind the session declares, as on the desktop: the profile's
/// endpoint entry must allow it.
pub const CLIENT_KIND: &str = "human-client";

/// How long a stop waits for the facade to close its session before the
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
}

/// Why a store action was not taken.
#[derive(Debug)]
pub enum StoreActionRefused {
    /// The client is running and holds the store: stop it first.
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
        }
    }

    /// The process's host: one per process, because the profile lock is
    /// (a second embedded host in one process is refused `LockHeld`).
    pub fn global() -> &'static Self {
        static HOST: OnceLock<ServiceHost> = OnceLock::new();
        HOST.get_or_init(Self::new)
    }

    /// Where the view attaches and the platform reads its notices.
    #[must_use]
    pub fn hub(&self) -> &Arc<Hub> {
        &self.hub
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
        if self.lock().is_some() {
            return Err(StoreActionRefused::Running);
        }
        let paths = embedded_paths(app_data_dir, profile)?;
        at_rest::plan_discard(&paths, store_cipher).map_err(StoreActionRefused::Store)
    }

    /// Run the Discard the person confirmed; the next start recreates the
    /// store. Held under the host's lock so no start opens the store while
    /// it goes.
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
    ) -> Result<at_rest::Discarded, StoreActionRefused> {
        let running = self.lock();
        if running.is_some() {
            return Err(StoreActionRefused::Running);
        }
        let paths = embedded_paths(app_data_dir, profile)?;
        at_rest::discard(&paths, store_cipher, confirmed).map_err(StoreActionRefused::Store)
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
        B: DataSessionBinding + Send + 'static,
    {
        let mut running = self.lock();
        if running.is_some() {
            return Ok(());
        }
        let host = EmbeddedHost::start(EmbeddedLaunch {
            app_data_dir: launch.app_data_dir,
            profile: launch.profile,
            identity: launch.identity,
        })
        .map_err(StartRefused::Runtime)?;
        // From here a refusal stops the host before it answers.
        let refuse = |host: EmbeddedHost, why: StartRefused| {
            let _ = host.stop(Duration::ZERO);
            self.hub.set_store_refused(store_screen(&why));
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
        let store = match HumanStore::open_profile(
            host.paths(),
            &*launch.store_cipher,
            StoreOptions::default(),
        ) {
            Ok(store) => store,
            Err(e) => return refuse(host, classify(&e)),
        };
        let key = own_key(host.paths(), &endpoint, &*launch.store_cipher);
        let (data, admin) = (data(&host), host.binding());
        let client_config = client_config(&config, endpoint);
        let make = move || {
            let client =
                TransportClient::new(data, admin, store, client_config, Box::new(wall_ms), 0)?;
            Ok(with_key(FacadeSide::new(client, wall_ms), key))
        };
        let facade = match FacadeLoop::spawn(host.runtime(), make, &self.hub) {
            Ok(facade) => facade,
            Err(SpawnError::Store(e)) => return refuse(host, classify(&e)),
            Err(SpawnError::Thread(e)) => return refuse(host, StartRefused::Thread(e.to_string())),
        };
        self.hub.set_store_refused(None);
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
        store_cipher: &dyn KeyCipher,
    ) -> Result<(), StartRefused> {
        let mut running = self.lock();
        if running.is_some() {
            return Ok(());
        }
        let refused = |e: EmbeddedRefused| StartRefused::Runtime(e);
        let boundary = TrustBoundary::new(app_data_dir).map_err(|e| refused(e.into()))?;
        let paths =
            ProfilePaths::resolve_embedded(profile, boundary).map_err(|e| refused(e.into()))?;
        let config = ProfileConfig::load(&paths).map_err(|e| refused(e.into()))?;
        let endpoint = human_endpoint(&config).ok_or(StartRefused::NoHumanEndpoint)?;
        let store = HumanStore::open_profile(&paths, store_cipher, StoreOptions::default())
            .map_err(|e| {
                let why = classify(&e);
                self.hub.set_store_refused(store_screen(&why));
                why
            })?;
        let executor = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("interweave-offline")
            .enable_time()
            .build()
            .map_err(|e| StartRefused::Thread(e.to_string()))?;
        let key = own_key(&paths, &endpoint, store_cipher);
        let client_config = client_config(&config, endpoint);
        let make = move || {
            let client = TransportClient::new(
                offline_embedded(),
                offline_embedded(),
                store,
                client_config,
                Box::new(wall_ms),
                0,
            )?;
            Ok(with_key(FacadeSide::new(client, wall_ms), key))
        };
        // Said before the facade runs, so a view never shows this posture
        // as a service that can send.
        self.hub.set_store_refused(None);
        self.hub.set_network_access(false);
        let facade = match FacadeLoop::spawn(executor.handle().clone(), make, &self.hub) {
            Ok(facade) => facade,
            Err(SpawnError::Store(e)) => return Err(classify(&e)),
            Err(SpawnError::Thread(e)) => return Err(StartRefused::Thread(e.to_string())),
        };
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

    /// Stop the client: the facade closes its session (releasing the
    /// lease), then the runtime stops within `grace` and releases the
    /// profile lock. `None` when nothing ran.
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
        // Releases a thread parked in `wait_ended` with this request: it
        // holds the host until it returns.
        let _ = host.request_shutdown(grace);
        let session_closed = facade.is_none_or(|f| f.close(FACADE_CLOSE));
        let deadline = Instant::now() + FACADE_CLOSE;
        let mut host = host;
        let runtime = loop {
            match Arc::try_unwrap(host) {
                Ok(owned) => break owned.stop(grace),
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

/// The identity's own key under the Keystore, at the identity's own alias
/// (ADR-0063 decision 2, `identity_key::alias`): opened from its file, or
/// made the first time the service runs, by app-core's rule
/// (`identity_key_under`, whose tests hold it). The store's cipher is the
/// same bridge, each secret under its own alias namespace. A file this
/// build refuses is left as it is and the client runs with no key: Share
/// says so, and nothing is signed.
fn own_key(
    paths: &ProfilePaths,
    endpoint: &interweave_transport_api::EndpointId,
    cipher: &dyn KeyCipher,
) -> Option<IdentityKey> {
    identity_key_under(paths, std::slice::from_ref(endpoint), cipher)
        .ok()
        .flatten()
}

fn with_key<B: DataSessionBinding, A: interweave_local_client_api::AdminBinding>(
    mut facade: FacadeSide<B, A>,
    key: Option<IdentityKey>,
) -> FacadeSide<B, A> {
    if let Some(key) = key {
        facade.set_identity_key(key);
    }
    facade
}

fn client_config(
    config: &ProfileConfig,
    endpoint: interweave_transport_api::EndpointId,
) -> ClientConfig {
    ClientConfig {
        client_kind: CLIENT_KIND.to_owned(),
        endpoints: vec![endpoint],
        channels: config.channels.desired.clone(),
        max_payload_bytes: MAX_PAYLOAD_BYTES,
    }
}

/// The screen a store refusal shows in place of the chats (ADR-0060
/// decision 6), or none for any other refusal.
const fn store_screen(why: &StartRefused) -> Option<StoreRefusal> {
    match why {
        StartRefused::StoreNeedsRecovery => Some(StoreRefusal::Recovery),
        StartRefused::StoreKeyInvalidated => Some(StoreRefusal::KeyLost),
        _ => None,
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

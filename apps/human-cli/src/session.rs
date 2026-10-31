// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The command line's root: the facade side and the model side of
//! `app-core`, pumped on one thread as the desktop's two threads pump them
//! (`apps/human-desktop/src/facade_thread.rs` and `app.rs`), with a
//! scripted surface in the window's place. A verb is a script of view
//! events the model resolves into commands: the command line asks the
//! facade nothing the model would not, which is what makes a verb's test
//! at the model level a test of the verb.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use interweave_human_app_core::{FacadeSide, ModelSide, Opener, Surface, Update};
use interweave_human_client_api::{ClientEvent, SessionState};
use interweave_human_ui_model::{UiModel, ViewEvent};
use interweave_local_client_api::{AdminBinding, DataSessionBinding};

use crate::Failure;

/// How long the session sleeps between turns while it waits.
pub(crate) const STEP: Duration = Duration::from_millis(20);

/// How often whether a daemon serves the profile is asked again, as the
/// desktop asks it.
const DAEMON_PROBE: Duration = Duration::from_secs(1);

/// How long nothing has to happen before what the daemon had queued for
/// this client counts as handed over.
const QUIET: Duration = Duration::from_millis(200);

/// One take of the scripted surface: resolved against the model as it is
/// when the model side takes it, as a person acts on what is on screen.
type Batch = Box<dyn FnOnce(&UiModel) -> Vec<ViewEvent>>;

/// The surface in the window's place: renders nothing, and hands over the
/// batches a verb scripted, one per take.
#[derive(Default)]
pub(crate) struct Script {
    batches: VecDeque<Batch>,
}

impl Surface for Script {
    fn render(&mut self, _model: &UiModel) {}

    fn take_events(&mut self, model: &UiModel) -> Vec<ViewEvent> {
        self.batches
            .pop_front()
            .map_or_else(Vec::new, |batch| batch(model))
    }
}

/// No link is opened, and no platform prompt exists on a command line:
/// the model raises neither without a person activating it, and no verb
/// scripts one.
pub(crate) struct NoOpener;

impl Opener for NoOpener {
    fn open(&mut self, _destination: &str) {}
    fn ask_network_access(&mut self) {}
    fn authenticate(&mut self) {}
}

/// How opening the session ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Opening {
    /// Ready.
    Ready,
    /// No daemon serves the profile.
    NoDaemon,
    /// The facade is still re-opening at the deadline.
    NotYet,
    /// Opening was refused, as the facade says.
    Refused(SessionState),
    /// The store cannot hold unread content: the facade released the lease.
    StorageDegraded,
}

impl Opening {
    /// The failure a verb that needs the session returns.
    pub(crate) fn failure(&self, profile: &str) -> Option<Failure> {
        match self {
            Self::Ready => None,
            Self::NoDaemon => Some(Failure::Unreachable(format!(
                "no daemon is running for profile {profile:?}"
            ))),
            Self::NotYet => Some(Failure::Unreachable(format!(
                "the daemon for profile {profile:?} did not open a session in time"
            ))),
            Self::Refused(SessionState::Refused { problem }) => Some(Failure::Refused(format!(
                "the daemon refused the session: {}",
                crate::output::session_problem(*problem)
            ))),
            Self::Refused(_) => Some(Failure::Refused(
                "the daemon refused the session".to_owned(),
            )),
            Self::StorageDegraded => Some(Failure::Store(
                "the message store cannot hold new messages now".to_owned(),
            )),
        }
    }
}

/// The root.
pub(crate) struct Session<B: DataSessionBinding, A: AdminBinding> {
    facade: FacadeSide<B, A>,
    model: ModelSide<Script, NoOpener>,
    state: Option<SessionState>,
    start: Instant,
    daemon: Box<dyn FnMut() -> Option<bool>>,
    probed: Option<Instant>,
    /// What the facade side reported in the last step: replaced at each,
    /// so a verb that never takes them holds one step's worth, not every
    /// message the run received
    /// (`what_a_step_reported_is_held_for_that_step_only`).
    updates: Vec<Update>,
}

impl<B: DataSessionBinding, A: AdminBinding> Session<B, A> {
    /// A root over `facade`, started as the desktop starts: from the
    /// store's summaries, a conversation's rows read only when it is
    /// opened -- a verb that prints one walks it ([`crate::walk`]).
    /// `daemon` says whether a daemon serves the profile, `None` when it
    /// cannot tell.
    ///
    /// # Errors
    /// [`Failure::Store`] when the store cannot be read.
    pub(crate) fn new(
        mut facade: FacadeSide<B, A>,
        daemon: impl FnMut() -> Option<bool> + 'static,
    ) -> Result<Self, Failure> {
        let start = facade
            .start()
            .map_err(|e| Failure::Store(format!("the message store could not be read: {e}")))?;
        let mut model = ModelSide::new(Script::default(), NoOpener);
        model.apply(Update::Started(Box::new(start)));
        for update in facade.outbound_now() {
            model.apply(update);
        }
        Ok(Self {
            facade,
            model,
            state: None,
            start: Instant::now(),
            daemon: Box::new(daemon),
            probed: None,
            updates: Vec::new(),
        })
    }

    fn now(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// The model.
    pub(crate) const fn model(&self) -> &UiModel {
        self.model.model()
    }

    /// The model side, for what a verb reads off it beyond the model.
    pub(crate) const fn side(&mut self) -> &mut ModelSide<Script, NoOpener> {
        &mut self.model
    }

    /// The session as the facade last said it.
    pub(crate) const fn state(&self) -> Option<&SessionState> {
        self.state.as_ref()
    }

    /// What the facade side reported in the last step, in order.
    pub(crate) fn take_updates(&mut self) -> Vec<Update> {
        std::mem::take(&mut self.updates)
    }

    fn apply(&mut self, update: Update) {
        if let Update::Client(ClientEvent::Session(state)) = &update {
            self.state = Some(state.clone());
        }
        self.updates.push(update.clone());
        self.model.apply(update);
    }

    /// Script one take of the surface, for the next [`Self::step`].
    pub(crate) fn will(&mut self, batch: impl FnOnce(&UiModel) -> Vec<ViewEvent> + 'static) {
        self.model.surface_mut().batches.push_back(Box::new(batch));
    }

    /// One turn of both sides: the facade's, the daemon probed when due,
    /// the model's with every command it asks carried out, and the
    /// facade's again when there were any, as a send is attempted there.
    /// True when anything other than a counter changed.
    pub(crate) async fn step(&mut self) -> bool {
        self.updates.clear();
        let now = self.now();
        for update in self.facade.turn(now).await {
            self.apply(update);
        }
        if self.probed.is_none_or(|at| at.elapsed() >= DAEMON_PROBE) {
            self.probed = Some(Instant::now());
            let seen = (self.daemon)();
            self.model.daemon_seen(seen);
        }
        let commands = self.model.turn();
        let any = !commands.is_empty();
        for command in commands {
            let now = self.now();
            for update in self.facade.execute(command, now).await {
                self.apply(update);
            }
        }
        if any {
            let now = self.now();
            for update in self.facade.turn(now).await {
                self.apply(update);
            }
        }
        self.updates
            .iter()
            .any(|u| !matches!(u, Update::Diagnostics(_)))
    }

    /// Turn until the session is ready, cannot be, or `until`.
    pub(crate) async fn open(&mut self, until: Instant) -> Opening {
        loop {
            self.step().await;
            match &self.state {
                Some(SessionState::Ready { .. }) => return Opening::Ready,
                Some(state @ SessionState::Refused { .. }) => {
                    return Opening::Refused(state.clone());
                }
                Some(SessionState::StorageDegraded) => return Opening::StorageDegraded,
                _ => {}
            }
            // Asked at the first step: an absent daemon is said at once,
            // not after the whole wait.
            if self.model().session_notice()
                == Some(interweave_human_ui_model::SessionNotice::NoDaemon)
            {
                return Opening::NoDaemon;
            }
            if Instant::now() >= until {
                return Opening::NotYet;
            }
            tokio::time::sleep(STEP).await;
        }
    }

    /// Turn until nothing has happened for a while, or `until`: what the
    /// daemon held for this client's endpoint is handed over in the first
    /// turns after the session opens.
    pub(crate) async fn settle(&mut self, until: Instant) {
        let mut quiet_since = Instant::now();
        while Instant::now() < until && quiet_since.elapsed() < QUIET {
            if self.step().await {
                quiet_since = Instant::now();
            }
            tokio::time::sleep(STEP).await;
        }
    }

    /// Close the session: the lease is released, the daemon keeps running.
    pub(crate) async fn close(mut self) {
        self.facade.close().await;
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade side on a thread of its own, with its own runtime.
//!
//! Why not the window's thread: a session whose events go undrained stops
//! answering keepalive pings and loses its lease (`ipc-client`), so the
//! facade turns on its own schedule, whatever the window is doing. And
//! why a multi-thread runtime with one worker: the IPC client's reader
//! task then runs on that worker, so a store call that blocks this thread
//! (an fsync) cannot starve the pings either.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use interweave_human_app_core::{
    Command, Crossing, Facade, RosterError, Routed, SwitchOutcome, Update,
};
use interweave_human_store::StoreError;
use interweave_transport_api::EndpointId;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

/// How many messages wait for the window at most. When the window falls
/// that far behind, the facade thread waits for it before it hands over
/// more: it then stops draining, so what is still arriving stays in the
/// daemon's bounded endpoint queue, and what was drained is in the store
/// already. A window that stalls costs the process this many messages of
/// memory, not one per message sent to it -- and, stalled past the
/// keepalive miss threshold under sustained traffic, its lease, as any
/// session that leaves its events undrained (LOCAL-IPC.md); it then
/// reconnects. Bounded memory is the side of that trade AGENTS.md
/// section 6 takes.
pub const TO_WINDOW: usize = 256;

/// How long the facade waits for a command before it turns anyway.
const POLL: Duration = Duration::from_millis(100);

/// How often whether a daemon serves the profile is asked again.
const DAEMON_PROBE: Duration = Duration::from_secs(1);

/// What the window's side sends.
#[derive(Debug)]
pub enum ToFacade {
    /// Carry out `command` as `identity`; `None` as the identity in
    /// front, which the window sends only before it was told the
    /// identities held ([`FacadeThread::held`]): no switch can come before
    /// that, so the identity in front is then the one its model shows.
    Command {
        /// The identity it is for, when the window knows it.
        identity: Option<EndpointId>,
        /// What to carry out.
        command: Command,
    },
    /// Bring this identity in front: sent after every command of the
    /// identity it replaces, which therefore runs first
    /// (`interweave_human_app_core::ModelSide::switch`).
    Front(EndpointId),
    /// Close the session and stop. The lease is released; the daemon is
    /// not asked to stop (ADR-0040).
    Close,
}

/// The identities a facade holds, as the window is told them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    /// In held order.
    pub held: Vec<EndpointId>,
    /// In front at start.
    pub front: EndpointId,
}

/// Why a facade could not be built: a class, never content, so the window
/// says what happened and not what the store holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacadeFault {
    /// The store could not be read: its pending rows, or a card's
    /// creation time.
    Store,
    /// More identities than a client holds (`RosterError::OverBound`).
    OverBound,
}

impl From<StoreError> for FacadeFault {
    fn from(_: StoreError) -> Self {
        Self::Store
    }
}

impl From<RosterError> for FacadeFault {
    fn from(e: RosterError) -> Self {
        match e {
            RosterError::Store(_) => Self::Store,
            RosterError::OverBound => Self::OverBound,
        }
    }
}

/// What the facade's side sends back.
#[derive(Debug)]
pub enum FromFacade {
    /// An update for the model side, for the identity it names (boxed:
    /// it is large, the others are not).
    Update(EndpointId, Box<Update>),
    /// Whether a daemon serves the profile changed.
    /// `None` when the lock cannot tell.
    Daemon(Option<bool>),
    /// The store could not be listed at start: what it holds stays
    /// unshown this session. A class, never content.
    ListingFailed,
    /// The facade could not be built, for the class it carries. Nothing
    /// is sent or received this session.
    FacadeFailed(FacadeFault),
    /// The session is closed and the thread is ending.
    Closed,
}

/// The window's handle on the facade thread.
pub struct FacadeThread {
    // Not bounded by a number. What fills it: the person's actions, at
    // most one take of the view's bounded queue per turn of the window;
    // and a focused window's MarkRead for each unread row it shows, which
    // inbound traffic drives. ModelSide sends a command once while it is
    // in flight, so what can wait is at most one per action and per
    // unread row the model holds -- rows the store holds, bounded by its
    // quota, or the disk when none is set. The facade takes every command
    // waiting at each iteration, which is latency, not the bound.
    to_facade: UnboundedSender<ToFacade>,
    from_facade: mpsc::Receiver<FromFacade>,
    /// A wake is asked of the window and not yet answered by a take: one
    /// at a time, however many messages wait.
    woken: Arc<AtomicBool>,
    /// The identities held and the one in front at start, set once the
    /// facade is built and before its first message is sent. Beside the
    /// queue rather than in it: the queue's bound counts what the window
    /// has not taken of the facade's output, as it did with one identity,
    /// and a facade waiting on a full queue must still have turned once
    /// (`closing_while_the_facade_waits_on_a_full_queue_completes`).
    held: Arc<OnceLock<Held>>,
    thread: Option<JoinHandle<()>>,
}

impl FacadeThread {
    /// Start the facade `make` builds -- the roster, or a facade side
    /// alone -- on a thread of its own (`make` fails with the class of
    /// what could not be built, [`FacadeFault`]).
    /// `daemon_present` asks whether a daemon serves the profile (`Err`
    /// when it cannot tell: that is logged once per change, never read as
    /// "absent").
    /// `notify` is called, from the facade thread, when there is something
    /// to take and no wake is outstanding: the window schedules a turn
    /// with it, and its next [`take`](Self::take) answers it. `report`
    /// takes the thread's log lines (stderr, in the app).
    ///
    /// # Errors
    /// When the thread or its runtime cannot be started.
    pub fn spawn<F: Facade + 'static>(
        make: impl FnOnce() -> Result<F, FacadeFault> + Send + 'static,
        daemon_present: impl Fn() -> Result<bool, String> + Send + 'static,
        notify: Arc<dyn Fn() + Send + Sync>,
        report: fn(&str),
    ) -> std::io::Result<Self> {
        Self::spawn_bounded(make, daemon_present, notify, report, TO_WINDOW)
    }

    /// [`spawn`](Self::spawn), with at most `capacity` messages waiting
    /// for the window instead of [`TO_WINDOW`].
    ///
    /// # Errors
    /// When the thread or its runtime cannot be started.
    pub fn spawn_bounded<F: Facade + 'static>(
        make: impl FnOnce() -> Result<F, FacadeFault> + Send + 'static,
        daemon_present: impl Fn() -> Result<bool, String> + Send + 'static,
        notify: Arc<dyn Fn() + Send + Sync>,
        report: fn(&str),
        capacity: usize,
    ) -> std::io::Result<Self> {
        let (to_facade, mut commands) = unbounded_channel();
        let (send, from_facade) = mpsc::sync_channel(capacity);
        let woken = Arc::new(AtomicBool::new(false));
        let wake = Arc::clone(&woken);
        let held = Arc::new(OnceLock::new());
        let named = Arc::clone(&held);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        let thread = std::thread::Builder::new()
            .name("human-facade".to_owned())
            .spawn(move || {
                let start = Instant::now();
                let now = || u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
                let out = |message: FromFacade| {
                    // Waits while the window holds `capacity` untaken: the
                    // backpressure the bound is for. A window that closes
                    // drains it; one that is gone drops the receiver, and
                    // the send fails instead -- nothing to tell then.
                    if send.send(message).is_err() {
                        return;
                    }
                    // Sent before the flag is read, and the take clears
                    // the flag before it reads: a message is either in
                    // that take or asks a wake of its own.
                    if !wake.swap(true, Ordering::SeqCst) {
                        notify();
                    }
                };
                runtime.block_on(async move {
                    let mut side = match make() {
                        Ok(side) => side,
                        Err(fault) => {
                            out(FromFacade::FacadeFailed(fault));
                            return;
                        }
                    };
                    if side.front_unread() {
                        // The class only: the row names an identity. The
                        // line names no fallback: the roster chose the
                        // default or the first, and this thread does not
                        // know which.
                        report(
                            "the identity last in front could not be read; \
                             the start went on without it",
                        );
                    }
                    // Before any message: a take that holds an update
                    // finds the identities named.
                    let _ = named.set(Held {
                        held: side.held(),
                        front: side.front(),
                    });
                    // The windowed start (design/2026-10-27-start-window.md):
                    // each conversation's summary and the contacts without
                    // their photos, no backlog; a conversation's rows are
                    // read a page at a time as its window asks
                    // (`the_desktop_starts_windowed_and_an_opened_conversation_reads_one_page`).
                    for (identity, start) in side.start() {
                        match start {
                            Ok(start) => out(FromFacade::Update(
                                identity,
                                Box::new(Update::Started(Box::new(start))),
                            )),
                            Err(_) => out(FromFacade::ListingFailed),
                        }
                    }
                    let routed = |routed: Routed| {
                        for (identity, updates) in routed {
                            for update in updates {
                                out(FromFacade::Update(identity.clone(), Box::new(update)));
                            }
                        }
                    };
                    let mut daemon: Option<Option<bool>> = None;
                    let mut cannot_tell: Option<String> = None;
                    let mut probed: Option<Instant> = None;
                    loop {
                        let first = tokio::select! {
                            message = commands.recv() => Some(message),
                            () = tokio::time::sleep(POLL) => None,
                        };
                        // Every command waiting now rather than one per
                        // turn: a burst -- a focused window marking each
                        // unread row it shows -- is carried out before the
                        // next turn instead of a turn apart each. A
                        // latency choice: one per turn would carry out the
                        // same commands in the same order, later.
                        let mut waiting: Vec<Option<ToFacade>> = first.into_iter().collect();
                        while let Ok(more) = commands.try_recv() {
                            waiting.push(Some(more));
                        }
                        let mut closing = false;
                        for message in waiting {
                            // In the order sent: a switch's commands of
                            // the old front run before the switch itself.
                            match message {
                                Some(ToFacade::Command { identity, command }) => {
                                    let identity = identity.unwrap_or_else(|| side.front());
                                    if let Some(answer) =
                                        side.execute(&identity, command, now()).await
                                    {
                                        routed(answer);
                                    }
                                }
                                Some(ToFacade::Front(to)) => {
                                    // A row that could not be written leaves
                                    // the next start in front of what it
                                    // said before; the switch stands.
                                    if let SwitchOutcome::Switched { remembered: Err(_) } =
                                        side.set_front(&to)
                                    {
                                        report("the identity in front could not be remembered");
                                    }
                                }
                                Some(ToFacade::Close) | None => {
                                    closing = true;
                                    break;
                                }
                            }
                        }
                        if closing {
                            side.close().await;
                            break;
                        }
                        routed(side.turn(now()).await);
                        if probed.is_none_or(|at| at.elapsed() >= DAEMON_PROBE) {
                            probed = Some(Instant::now());
                            let answer = daemon_present();
                            let seen = answer.as_ref().ok().copied();
                            // The window hears every change, cannot-tell
                            // included: an earlier "no daemon" must not
                            // outlive a lock that can no longer answer.
                            if daemon != Some(seen) {
                                daemon = Some(seen);
                                out(FromFacade::Daemon(seen));
                            }
                            match answer {
                                Ok(_) => cannot_tell = None,
                                // Said once, and again only when the reason
                                // changes.
                                Err(why) => {
                                    if cannot_tell.as_ref() != Some(&why) {
                                        report(&format!(
                                            "whether the transport daemon runs cannot be told: \
                                             {why}"
                                        ));
                                        cannot_tell = Some(why);
                                    }
                                }
                            }
                        }
                    }
                });
                out(FromFacade::Closed);
            })?;
        Ok(Self {
            to_facade,
            from_facade,
            woken,
            held,
            thread: Some(thread),
        })
    }

    /// Ask the facade to carry out `command` as `identity`, or as the
    /// identity in front for `None` ([`ToFacade::Command`]). False once the
    /// facade thread has ended.
    #[must_use]
    pub fn command(&self, identity: Option<EndpointId>, command: Command) -> bool {
        self.to_facade
            .send(ToFacade::Command { identity, command })
            .is_ok()
    }

    /// Hand over what the model side sends across: a command as the
    /// identity it names, or the switch. False once the facade thread has
    /// ended.
    #[must_use]
    pub fn cross(&self, crossing: Crossing) -> bool {
        let message = match crossing {
            Crossing::Command { identity, command } => ToFacade::Command {
                identity: Some(identity),
                command,
            },
            Crossing::Front(to) => ToFacade::Front(to),
        };
        self.to_facade.send(message).is_ok()
    }

    /// The identities held, in held order, and the one in front at start:
    /// `None` until the facade is built, and set before its first message,
    /// so a [`take`](Self::take) that returns an update finds it set.
    #[must_use]
    pub fn held(&self) -> Option<&Held> {
        self.held.get()
    }

    /// What the facade side sent since the last call, in order. Answers
    /// the outstanding wake, so the next message asks for another.
    #[must_use]
    pub fn take(&self) -> Vec<FromFacade> {
        self.woken.store(false, Ordering::SeqCst);
        self.from_facade.try_iter().collect()
    }

    /// Close the session and wait for the thread, at most `wait`. The
    /// lease is released; the daemon keeps running. True when the thread
    /// ended in time.
    #[must_use]
    pub fn close(mut self, wait: Duration) -> bool {
        let _ = self.to_facade.send(ToFacade::Close);
        let deadline = Instant::now() + wait;
        while Instant::now() < deadline {
            match self.from_facade.recv_timeout(Duration::from_millis(20)) {
                // Closed now, or ended already: a thread that ended before
                // the window asked -- its facade could not be built, its
                // Closed taken by a pump -- has dropped its sender.
                Ok(FromFacade::Closed) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    if let Some(thread) = self.thread.take() {
                        let _ = thread.join();
                    }
                    return true;
                }
                Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
        // Past the deadline the process exits anyway: the OS closes the
        // socket, and the daemon releases the lease on its own.
        false
    }
}

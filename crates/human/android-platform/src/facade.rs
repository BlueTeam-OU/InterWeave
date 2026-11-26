// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade on the Service's side: one thread for the process, turning
//! on the embedded runtime's own executor ([`EmbeddedHost::runtime`]), so
//! the process runs one executor, not two.
//!
//! Why its own thread, as on the desktop: a session whose events go
//! undrained loses its lease, so the facade turns on its own schedule
//! whatever the Activity is doing -- and here, whether there is an
//! Activity at all.
//!
//! [`EmbeddedHost::runtime`]: interweave_transport_embedded::EmbeddedHost::runtime

use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use interweave_human_app_core::{
    Facade, FacadeSide, Roster, RosterError, Routed, SwitchOutcome, Update,
};
use interweave_human_store::StoreError;
use interweave_human_transport_client::{ClientEvent, SessionBinding};
use interweave_local_client_api::AdminBinding;
use interweave_transport_api::EndpointId;
use tokio::sync::{mpsc, oneshot};

use crate::hub::{Hub, ToFacade, ToView};

/// How long the loop waits for a command before it turns anyway.
const POLL: Duration = Duration::from_millis(100);

/// What a view that attaches late is told of a held identity's state
/// now, beside the store's rows. A trait of this crate's rather than a
/// method of app-core's `Facade`: the snapshot reads each identity's
/// client, which that trait does not expose.
pub trait HeldState {
    /// `identity`'s state now, as the updates that would have set it;
    /// nothing for an identity not held.
    fn state_now(&self, identity: &EndpointId) -> Vec<Update>;
}

/// Each identity's own client, session and counters; the connectivity
/// is the first identity's, as the roster tells every identity in a turn:
/// the others read no status.
impl<B: SessionBinding, A: AdminBinding> HeldState for Roster<B, A> {
    fn state_now(&self, identity: &EndpointId) -> Vec<Update> {
        match (self.side(identity), self.side(self.first())) {
            (Some(side), Some(first)) => snapshot(side, first),
            _ => Vec::new(),
        }
    }
}

/// `side`'s state now, its connectivity read from `status`'s client.
fn snapshot<B: SessionBinding, A: AdminBinding>(
    side: &FacadeSide<B, A>,
    status: &FacadeSide<B, A>,
) -> Vec<Update> {
    let client = side.client();
    // This node's PeerId, which a Ready brings and the late view missed:
    // Peers reads this device's own row by it (design section 5.2).
    let mut now: Vec<Update> = client
        .local_peer()
        .cloned()
        .map(Update::ThisDevice)
        .into_iter()
        .collect();
    now.extend([
        Update::Client(ClientEvent::Session(client.session_state().clone())),
        Update::Client(ClientEvent::Connectivity(status.client().connectivity())),
        Update::Diagnostics(client.diagnostics()),
    ]);
    now.extend(side.outbound_now());
    now
}

/// Why the facade did not start.
#[derive(Debug)]
pub enum SpawnError {
    /// Its thread could not be started.
    Thread(std::io::Error),
    /// Its store could not be read: the pending rows, or a card's
    /// creation time.
    Store(StoreError),
    /// More identities than a client holds (`RosterError::OverBound`).
    OverBound,
}

impl From<RosterError> for SpawnError {
    fn from(e: RosterError) -> Self {
        match e {
            RosterError::Store(e) => Self::Store(e),
            RosterError::OverBound => Self::OverBound,
        }
    }
}

/// What the facade says for the platform log: a class, never what the
/// store holds, since a row names an identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacadeNote {
    /// The row naming the identity last in front could not be read: the
    /// start went on without it, the default or the first in front.
    FrontUnread,
    /// The identity in front changed, and the row remembering it could not
    /// be written: the next start is in front of what the row said before.
    FrontUnremembered,
}

/// Carry out `message` on `facade`: a command as the identity it names,
/// or as the one in front when it names none; a command naming an
/// identity not held is not carried out, `execute` answering `None`. A
/// switch brings `to` in front and answers nothing; a row it could not
/// write is told to `note`, once per switch
/// (`a_command_runs_as_the_identity_it_names_else_the_front`,
/// `a_switch_answers_nothing_and_says_an_unwritten_row`).
async fn carry<F: Facade>(
    facade: &mut F,
    message: ToFacade,
    now: u64,
    note: &dyn Fn(FacadeNote),
) -> Routed {
    match message {
        ToFacade::Command { identity, command } => {
            let identity = identity.unwrap_or_else(|| facade.front());
            facade
                .execute(&identity, command, now)
                .await
                .unwrap_or_default()
        }
        ToFacade::Front(to) => {
            if let SwitchOutcome::Switched { remembered: Err(_) } = facade.set_front(&to) {
                note(FacadeNote::FrontUnremembered);
            }
            Vec::new()
        }
    }
}

/// Why the loop woke.
enum Woke {
    /// A command or a switch arrived.
    Message(ToFacade),
    /// The loop is asked to close, or nothing can command it any more.
    Closed,
    /// Nothing arrived within [`POLL`]: a turn is due anyway.
    Poll,
}

/// A facade closing: withdrawn from the hub, so a command sent from now
/// on is handed back to the view to fail, then every command the hub
/// accepted before, for the loop to carry out. Withdrawn first, so none
/// can arrive after the drain (owner's finding P2-2;
/// `closing_hands_back_every_command_accepted_before_it`).
fn closing(
    hub: &Hub,
    generation: u64,
    waiting: &mut mpsc::UnboundedReceiver<ToFacade>,
) -> Vec<ToFacade> {
    hub.withdraw(generation);
    let mut accepted = Vec::new();
    while let Ok(command) = waiting.try_recv() {
        accepted.push(command);
    }
    accepted
}

/// The running facade loop.
pub struct FacadeLoop {
    close: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    hub: Arc<Hub>,
    generation: u64,
}

/// Says the facade's thread ended to the hub however it ends, a panic
/// included: a thread counted alive forever would refuse every store
/// discard.
struct Ends {
    hub: Arc<Hub>,
    generation: u64,
}

impl Drop for Ends {
    fn drop(&mut self) {
        self.hub.ended(self.generation);
    }
}

/// What the facade's thread holds. Fields drop in order, so the facade --
/// its store connection with it, the WAL truncated and closed -- goes
/// before the hub hears the thread ended, on every path, an unwind
/// included: until then a store discard is refused (#495 review F3,
/// `the_side_drops_before_the_hub_hears_the_thread_ended`).
struct Held<S> {
    side: S,
    _ends: Ends,
}

impl FacadeLoop {
    /// Build the facade with `make` -- the roster the Service composes --
    /// on a thread of its own (a session is not `Send`, so it lives where
    /// it is made) and run it there, its futures driven on `executor`, its
    /// output to `hub`, each update tagged with the identity it is for.
    /// What it says for the log goes to `note`. The hub hears the facade
    /// running before this returns.
    ///
    /// # Errors
    /// When the thread cannot be started, or `make` fails: the store could
    /// not be read, or more identities were asked than a client holds.
    /// Nothing runs then.
    pub fn spawn<F>(
        executor: tokio::runtime::Handle,
        make: impl FnOnce() -> Result<F, RosterError> + Send + 'static,
        hub: &Arc<Hub>,
        note: impl Fn(FacadeNote) + Send + 'static,
    ) -> Result<Self, SpawnError>
    where
        F: Facade + HeldState + 'static,
    {
        // Not bounded by a number, as the desktop's is not
        // (apps/human-desktop/src/facade_thread.rs). Every command comes
        // through a view's ModelSide (the window's, or a test build's e2e
        // client), which drops a command while the same one is in flight:
        // one send per conversation, and each other command by its value,
        // a MarkRead per unread row (app-core model_side.rs `InFlight`;
        // tests/root.rs
        // `a_second_send_while_one_is_in_flight_is_not_sent_twice`). What can
        // wait is at most one per action and per unread row the model
        // holds: rows the store holds, bounded by its quota; and a switch,
        // one per the person's choice in the switcher. The loop below
        // takes every waiting command each turn, which is latency, not the
        // bound. The other direction is bounded (hub.rs `TO_VIEW`).
        let (commands, mut waiting) = mpsc::unbounded_channel();
        let (close, mut closed) = oneshot::channel();
        let (report, reported) = std::sync::mpsc::sync_channel(1);
        let on_thread = Arc::clone(hub);
        let spawned = std::thread::Builder::new()
            .name("human-facade".to_owned())
            .spawn(move || {
                let hub = on_thread;
                let facade = match make() {
                    Ok(facade) => facade,
                    Err(e) => {
                        let _ = report.send(Err(SpawnError::from(e)));
                        return;
                    }
                };
                // Said once, by class, before the start returns: the row
                // names an identity.
                if facade.front_unread() {
                    note(FacadeNote::FrontUnread);
                }
                // Named before the hub says the facade runs, so a view
                // hears whose updates follow before the first of them.
                let generation = hub.started(
                    commands,
                    crate::hub::Held {
                        held: facade.held(),
                        front: facade.front(),
                    },
                );
                let mut held = Held {
                    side: facade,
                    _ends: Ends {
                        hub: Arc::clone(&hub),
                        generation,
                    },
                };
                let facade = &mut held.side;
                let _ = report.send(Ok(generation));
                let start = Instant::now();
                let now = || u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
                // Every update goes out under the identity the facade
                // answered it for: the listing's snapshot, a command's
                // answer, a turn.
                let publish = |routed: Routed| {
                    for (identity, updates) in routed {
                        for update in updates {
                            hub.update(generation, &identity, update);
                        }
                    }
                };
                executor.block_on(async {
                    loop {
                        if let Some(relists) = hub.wants_listing(generation) {
                            // The windowed start, as on the desktop
                            // (design/2026-10-27-start-window.md): the
                            // summaries and the contacts, no backlog; the
                            // open conversation reads its rows a page at a
                            // time. A view started already re-bases on it
                            // (ui-model `started`). One per identity held,
                            // each naming its identity.
                            let listing: Vec<ToView> = facade
                                .start()
                                .into_iter()
                                .map(|(identity, start)| match start {
                                    Ok(start) => ToView::Update(
                                        identity,
                                        Box::new(Update::Started(Box::new(start))),
                                    ),
                                    Err(_) => ToView::ListingFailed,
                                })
                                .collect();
                            hub.listed(generation, relists, listing);
                            // A view attaching to a facade that has run
                            // for a while missed the events that set its
                            // state: they went out with no view to take
                            // them. It is told, for each identity held,
                            // this node's PeerId, the session,
                            // connectivity, counters and each pending
                            // row's status as they are now. NOT a peer's
                            // path: the facade keeps no current path to
                            // replay, so a route indicator shows nothing
                            // until the path next changes (carried to the
                            // Android client's batch 2).
                            let states: Routed = facade
                                .held()
                                .into_iter()
                                .map(|identity| {
                                    let state = facade.state_now(&identity);
                                    (identity, state)
                                })
                                .collect();
                            publish(states);
                        }
                        // A close is taken first when both are ready: what
                        // was accepted before it is carried out by the
                        // close's own drain below, never dropped.
                        let woke = tokio::select! {
                            biased;
                            _ = &mut closed => Woke::Closed,
                            message = waiting.recv() => message.map_or(Woke::Closed, Woke::Message),
                            () = tokio::time::sleep(POLL) => Woke::Poll,
                        };
                        // Every command waiting now rather than one per
                        // turn, as the desktop does: a burst of reads is
                        // carried out before the next turn.
                        let (mut batch, closing_now) = match woke {
                            // Every command accepted before the close is
                            // carried out, never dropped (owner's finding
                            // P2-2); past the stop's grace its answer is
                            // refused at the retired hub.
                            Woke::Closed => (closing(&hub, generation, &mut waiting), true),
                            Woke::Message(message) => (vec![message], false),
                            Woke::Poll => (Vec::new(), false),
                        };
                        if !closing_now {
                            while let Ok(more) = waiting.try_recv() {
                                batch.push(more);
                            }
                        }
                        // In the order sent: a switch's commands of the old
                        // front come before the switch itself.
                        for message in batch {
                            if matches!(message, ToFacade::Command { .. }) {
                                hub.before_command();
                            }
                            publish(carry(facade, message, now(), &note).await);
                        }
                        if closing_now {
                            break;
                        }
                        if hub.turns_held() {
                            continue;
                        }
                        publish(facade.turn(now()).await);
                    }
                    facade.close().await;
                });
                // Its own generation only: a facade that outlived its close
                // never stops one started after it, and owes it a listing.
                drop(held);
            });
        let thread = spawned.map_err(SpawnError::Thread)?;
        match reported.recv() {
            Ok(Ok(generation)) => Ok(Self {
                close: Some(close),
                thread: Some(thread),
                hub: Arc::clone(hub),
                generation,
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            // The thread ended without a word: it panicked in `make`.
            Err(_) => {
                let _ = thread.join();
                Err(SpawnError::Thread(std::io::Error::other(
                    "the facade's thread ended while it was built",
                )))
            }
        }
    }

    /// Close the session -- the lease is released -- and wait for the
    /// thread, at most `wait`; then retire the facade at the hub, whether
    /// or not it ended. True when it ended in time.
    ///
    /// Left running, it finishes the commands it accepted against the
    /// store and publishes nothing (`human-client-android.md`, a facade
    /// past its stop): it is not waited for, since a command that never
    /// returns would hold the service.
    #[must_use]
    pub fn close(mut self, wait: Duration) -> bool {
        let ended = self.wait_closed(wait);
        self.hub.retire(self.generation);
        ended
    }

    fn wait_closed(&mut self, wait: Duration) -> bool {
        if let Some(close) = self.close.take() {
            let _ = close.send(());
        }
        let Some(thread) = self.thread.take() else {
            return true;
        };
        let deadline = Instant::now() + wait;
        while !thread.is_finished() {
            if Instant::now() >= deadline {
                // Left running: the runtime's stop closes the session
                // under it, and the process may end before it does.
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = thread.join();
        true
    }
}

#[cfg(test)]
mod closing_tests {
    use super::{FacadeNote, carry, closing};
    use crate::hub::{Held, Hub, ToFacade};
    use interweave_human_app_core::{Command, Facade, Routed, Start, SwitchOutcome, Update};
    use interweave_human_store::StoreError;
    use interweave_transport_api::EndpointId;
    use std::cell::RefCell;

    fn me() -> EndpointId {
        interweave_test_support::endpoint_keys::endpoint_id("human")
    }

    fn other() -> EndpointId {
        interweave_test_support::endpoint_keys::endpoint_id("other")
    }

    fn held() -> Held {
        Held {
            held: vec![me()],
            front: me(),
        }
    }

    /// A facade holding `held`, `front` in front, that records what it is
    /// asked: the roster's answers by its contract (`Facade::execute`
    /// answers `None` for an identity not held; `set_front` is `Unchanged`
    /// for the front or one not held), without a store behind it.
    struct Recorder {
        held: Vec<EndpointId>,
        front: EndpointId,
        /// Whether a switch's row write fails.
        unwritable: bool,
        ran: Vec<(EndpointId, Command)>,
        fronted: Vec<EndpointId>,
    }

    impl Recorder {
        fn new(held: Vec<EndpointId>, front: EndpointId) -> Self {
            Self {
                held,
                front,
                unwritable: false,
                ran: Vec::new(),
                fronted: Vec::new(),
            }
        }
    }

    impl Facade for Recorder {
        fn held(&self) -> Vec<EndpointId> {
            self.held.clone()
        }
        fn front(&self) -> EndpointId {
            self.front.clone()
        }
        fn start(&mut self) -> Vec<(EndpointId, Result<Start, StoreError>)> {
            Vec::new()
        }
        fn turn(&mut self, _now: u64) -> impl Future<Output = Routed> {
            std::future::ready(Vec::new())
        }
        fn execute(
            &mut self,
            identity: &EndpointId,
            command: Command,
            _now: u64,
        ) -> impl Future<Output = Option<Routed>> {
            let answer = self.held.contains(identity).then(|| {
                self.ran.push((identity.clone(), command.clone()));
                vec![(identity.clone(), vec![Update::Done(command)])]
            });
            std::future::ready(answer)
        }
        fn set_front(&mut self, to: &EndpointId) -> SwitchOutcome {
            self.fronted.push(to.clone());
            if !self.held.contains(to) || to == &self.front {
                return SwitchOutcome::Unchanged;
            }
            self.front = to.clone();
            SwitchOutcome::Switched {
                remembered: if self.unwritable {
                    Err(StoreError::Degraded)
                } else {
                    Ok(())
                },
            }
        }
        fn close(&mut self) -> impl Future<Output = ()> {
            std::future::ready(())
        }
    }

    fn run<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("a runtime")
            .block_on(future)
    }

    fn command(identity: Option<EndpointId>) -> ToFacade {
        ToFacade::Command {
            identity,
            command: Command::Reopen,
        }
    }

    /// A command naming no identity runs as the one in front and is
    /// answered under it; one naming a held identity runs as that one,
    /// whatever is in front; one naming an identity not held is not run
    /// and answers nothing. Mutations: `None` run as the first held
    /// rather than the front fails the "as the front" assert; an unheld
    /// command run as the front fails the "not held: nothing answered"
    /// and "not held: not run" asserts.
    #[test]
    fn a_command_runs_as_the_identity_it_names_else_the_front() {
        let mut facade = Recorder::new(vec![me(), other()], other());
        let said = RefCell::new(Vec::new());
        let note = |n| said.borrow_mut().push(n);
        let answered = run(carry(&mut facade, command(None), 0, &note));
        assert_eq!(
            answered,
            vec![(other(), vec![Update::Done(Command::Reopen)])],
            "as the front"
        );
        let answered = run(carry(&mut facade, command(Some(me())), 0, &note));
        assert_eq!(answered, vec![(me(), vec![Update::Done(Command::Reopen)])]);
        let stranger = interweave_test_support::endpoint_keys::endpoint_id("stranger");
        let answered = run(carry(&mut facade, command(Some(stranger)), 0, &note));
        assert!(answered.is_empty(), "not held: nothing answered");
        assert_eq!(
            facade.ran,
            vec![(other(), Command::Reopen), (me(), Command::Reopen)],
            "not held: not run"
        );
        assert!(said.borrow().is_empty());
    }

    /// A switch asks the facade to bring `to` in front and answers
    /// nothing; an unchanged front says nothing, and a row that could not
    /// be written is said once, by class. Mutations: a switch not passed
    /// to `set_front` fails the `fronted` assert; the unwritten row not
    /// said, or said for an unchanged front, fails a `said` assert.
    #[test]
    fn a_switch_answers_nothing_and_says_an_unwritten_row() {
        let mut facade = Recorder::new(vec![me(), other()], me());
        facade.unwritable = true;
        let said = RefCell::new(Vec::new());
        let note = |n| said.borrow_mut().push(n);
        assert!(run(carry(&mut facade, ToFacade::Front(me()), 0, &note)).is_empty());
        assert!(said.borrow().is_empty(), "unchanged: nothing said");
        assert!(run(carry(&mut facade, ToFacade::Front(other()), 0, &note)).is_empty());
        assert_eq!(facade.fronted, vec![me(), other()]);
        assert_eq!(facade.front, other());
        assert_eq!(*said.borrow(), vec![FacadeNote::FrontUnremembered]);
        assert!(facade.ran.is_empty(), "a switch runs no command");
    }

    /// Every command the hub accepted before the close is handed to the
    /// loop to carry out, and none is accepted after it. Mutation: the
    /// close not draining what waits.
    #[test]
    fn closing_hands_back_every_command_accepted_before_it() {
        let hub = Hub::new();
        let (sender, mut waiting) = tokio::sync::mpsc::unbounded_channel();
        let generation = hub.started(sender, held());
        for _ in 0..3 {
            assert_eq!(hub.command(None, Command::Reopen), Ok(()));
        }
        let accepted = closing(&hub, generation, &mut waiting);
        assert_eq!(
            accepted,
            vec![
                ToFacade::Command {
                    identity: None,
                    command: Command::Reopen
                };
                3
            ]
        );
        assert_eq!(hub.command(None, Command::Reopen), Err(Command::Reopen));
    }

    /// #495 review F3: the side drops while the facade still counts as
    /// alive, and the hub hears it ended only after, on a plain drop and on
    /// an unwind alike. Mutation: `Held`'s fields in the other order.
    #[test]
    fn the_side_drops_before_the_hub_hears_the_thread_ended() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        struct Probe {
            hub: Arc<Hub>,
            alive_at_drop: Arc<AtomicBool>,
        }
        impl Drop for Probe {
            fn drop(&mut self) {
                self.alive_at_drop
                    .store(self.hub.facade_alive(), Ordering::SeqCst);
            }
        }
        for unwind in [false, true] {
            let hub = Arc::new(Hub::new());
            let (sender, _waiting) = tokio::sync::mpsc::unbounded_channel();
            let generation = hub.started(sender, held());
            let seen = Arc::new(AtomicBool::new(false));
            let held = super::Held {
                side: Probe {
                    hub: Arc::clone(&hub),
                    alive_at_drop: Arc::clone(&seen),
                },
                _ends: super::Ends {
                    hub: Arc::clone(&hub),
                    generation,
                },
            };
            if unwind {
                let ended = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    let _held = held;
                    panic!("the facade's thread unwinds");
                }));
                assert!(ended.is_err());
            } else {
                drop(held);
            }
            assert!(
                seen.load(Ordering::SeqCst),
                "unwind {unwind}: alive as the side dropped"
            );
            assert!(!hub.facade_alive(), "unwind {unwind}: ended after");
        }
    }
}

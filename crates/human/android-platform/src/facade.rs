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

use interweave_human_app_core::{Command, FacadeSide, Update};
use interweave_human_store::StoreError;
use interweave_human_transport_client::ClientEvent;
use interweave_local_client_api::{AdminBinding, DataSessionBinding};
use tokio::sync::{mpsc, oneshot};

use crate::hub::{Hub, ToView};

/// How long the loop waits for a command before it turns anyway.
const POLL: Duration = Duration::from_millis(100);

/// The facade's state now, as the updates that would have set it: what a
/// view that attaches late needs beside the store's rows.
fn snapshot<B: DataSessionBinding, A: AdminBinding>(side: &FacadeSide<B, A>) -> Vec<Update> {
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
        Update::Client(ClientEvent::Connectivity(client.connectivity())),
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
    /// Its store's pending rows could not be read.
    Store(StoreError),
}

/// Why the loop woke.
enum Woke {
    /// A command arrived.
    Command(Command),
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
    waiting: &mut mpsc::UnboundedReceiver<Command>,
) -> Vec<Command> {
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

/// What the facade's thread holds. Fields drop in order, so the side --
/// its store connection with it, the WAL truncated and closed -- goes
/// before the hub hears the thread ended, on every path, an unwind
/// included: until then a store discard is refused (#495 review F3,
/// `the_side_drops_before_the_hub_hears_the_thread_ended`).
struct Held<S> {
    side: S,
    _ends: Ends,
}

impl FacadeLoop {
    /// Build the facade side with `make` on a thread of its own -- a
    /// session is not `Send`, so it lives where it is made -- and run it
    /// there, its futures driven on `executor`, its output to `hub`. The
    /// hub hears the facade running before this returns.
    ///
    /// # Errors
    /// When the thread cannot be started, or `make` fails: the store's
    /// pending rows could not be read. Nothing runs then.
    pub fn spawn<B, A>(
        executor: tokio::runtime::Handle,
        make: impl FnOnce() -> Result<FacadeSide<B, A>, StoreError> + Send + 'static,
        hub: &Arc<Hub>,
    ) -> Result<Self, SpawnError>
    where
        B: DataSessionBinding + 'static,
        A: AdminBinding + 'static,
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
        // holds: rows the store holds, bounded by its quota. The loop below
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
                let side = match make() {
                    Ok(side) => side,
                    Err(e) => {
                        let _ = report.send(Err(e));
                        return;
                    }
                };
                let generation = hub.started(commands);
                let mut held = Held {
                    side,
                    _ends: Ends {
                        hub: Arc::clone(&hub),
                        generation,
                    },
                };
                let side = &mut held.side;
                let _ = report.send(Ok(generation));
                let start = Instant::now();
                let now = || u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
                executor.block_on(async {
                    loop {
                        if let Some(relists) = hub.wants_listing(generation) {
                            // The windowed start, as on the desktop
                            // (design/2026-10-27-start-window.md): the
                            // summaries and the contacts, no backlog; the
                            // open conversation reads its rows a page at a
                            // time. A view started already re-bases on it
                            // (ui-model `started`).
                            hub.listed(
                                generation,
                                relists,
                                match side.start() {
                                    Ok(start) => {
                                        ToView::Update(Box::new(Update::Started(Box::new(start))))
                                    }
                                    Err(_) => ToView::ListingFailed,
                                },
                            );
                            // A view attaching to a facade that has run
                            // for a while missed the events that set its
                            // state: they went out with no view to take
                            // them. It is told this node's PeerId, the
                            // session, connectivity, counters and each
                            // pending row's status as they are now. NOT a peer's
                            // path: the facade keeps no current path to
                            // replay, so a route indicator shows nothing
                            // until the path next changes (carried to the
                            // Android client's batch 2).
                            for update in snapshot(side) {
                                hub.update(generation, update);
                            }
                        }
                        // A close is taken first when both are ready: what
                        // was accepted before it is carried out by the
                        // close's own drain below, never dropped.
                        let woke = tokio::select! {
                            biased;
                            _ = &mut closed => Woke::Closed,
                            command = waiting.recv() => command.map_or(Woke::Closed, Woke::Command),
                            () = tokio::time::sleep(POLL) => Woke::Poll,
                        };
                        // Every command waiting now rather than one per
                        // turn, as the desktop does: a burst of reads is
                        // carried out before the next turn.
                        let mut batch = match woke {
                            Woke::Closed => {
                                // Every command accepted before the close
                                // is carried out, never dropped (owner's
                                // finding P2-2); past the stop's grace its
                                // answer is refused at the retired hub.
                                for command in closing(&hub, generation, &mut waiting) {
                                    hub.before_command();
                                    for update in side.execute(command, now()).await {
                                        hub.update(generation, update);
                                    }
                                }
                                break;
                            }
                            Woke::Command(command) => vec![command],
                            Woke::Poll => Vec::new(),
                        };
                        while let Ok(more) = waiting.try_recv() {
                            batch.push(more);
                        }
                        for command in batch {
                            hub.before_command();
                            for update in side.execute(command, now()).await {
                                hub.update(generation, update);
                            }
                        }
                        for update in side.turn(now()).await {
                            hub.update(generation, update);
                        }
                    }
                    side.close().await;
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
                Err(SpawnError::Store(e))
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
    use super::closing;
    use crate::hub::Hub;
    use interweave_human_app_core::Command;

    /// Every command the hub accepted before the close is handed to the
    /// loop to carry out, and none is accepted after it. Mutation: the
    /// close not draining what waits.
    #[test]
    fn closing_hands_back_every_command_accepted_before_it() {
        let hub = Hub::new();
        let (sender, mut waiting) = tokio::sync::mpsc::unbounded_channel();
        let generation = hub.started(sender);
        for _ in 0..3 {
            assert_eq!(hub.command(Command::Reopen), Ok(()));
        }
        let accepted = closing(&hub, generation, &mut waiting);
        assert_eq!(accepted, vec![Command::Reopen; 3]);
        assert_eq!(hub.command(Command::Reopen), Err(Command::Reopen));
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
            let generation = hub.started(sender);
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

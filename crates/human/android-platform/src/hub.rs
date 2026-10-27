// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Where the facade's output meets the Activity, which comes and goes
//! while the Service holds the session (human-client-android.md, "Service
//! ownership and local session": an Activity destroyed or rotated does
//! not release the lease).
//!
//! At most one view is attached. A view that attaches is sent whether
//! the network service runs, then -- once the facade runs -- the store's
//! listing before any update, so it never applies an update to rows it
//! has not listed. With no view attached, updates are not kept: the store
//! holds every row a view will list when it attaches, and what a person
//! must hear while no view is open is a notice, not an update.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use interweave_human_app_core::{Command, Update};

use crate::notice::Notices;

/// How many messages wait for the view at most. A view that falls that
/// far behind is detached rather than waited for: the facade then keeps
/// draining, so notices keep coming, and the view lists the store again
/// when it re-attaches ([`ViewLink::lost`]). Bounded memory, as the
/// desktop's window queue is (CLAUDE.md section 6).
pub const TO_VIEW: usize = 256;

/// What a view is sent.
#[derive(Debug)]
pub enum ToView {
    /// Whether the network service runs: the Android counterpart of the
    /// desktop's "a daemon serves this profile". A view seeing `false`
    /// shows that, and holds no rows until a listing arrives.
    Running(bool),
    /// Whether the platform gives this app network access: `false` in
    /// the network-denied posture, where the store is shown and nothing
    /// is sent. Sent at an attach and at every change; it outlives a
    /// change of [`ToView::Running`].
    NetworkAccess(bool),
    /// Whether nothing can run until the person authenticates: a profile
    /// whose key needs them, outside the window of their last
    /// authentication. Sent at an attach and at every change; it outlives
    /// a change of [`ToView::Running`], as access does.
    AuthenticationNeeded(bool),
    /// An update for the model side, the first after an attach or a start
    /// being `Update::Listed`.
    Update(Box<Update>),
    /// The store could not be listed for this view: what it holds stays
    /// unshown until the view attaches again. A class, never content.
    ListingFailed,
    /// A start's answer as the message store saw it (ADR-0060 decision
    /// 6), with the hub's count of answers so far: sent at every answer,
    /// and at an attach once a start has answered. The count is the
    /// hub's, so a view attached again still tells an answer that came
    /// after a discard from one before it (#377 review r3, risk 1).
    Store(StoreAnswer, StoreGeneration),
}

/// What a start's answer says of the message store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreAnswer {
    /// The store opened.
    Opened,
    /// The store refused: the screen the view shows in place of the chats.
    Refused(StoreRefusal),
    /// The start was refused for a reason no store screen shows -- the
    /// identity, the runtime, the profile, the endpoint -- so nothing says
    /// the store opened (#377 review r3, pre-existing).
    OtherRefusal,
}

/// How many start answers the hub has said: an answer said later counts
/// higher. A discard reads it under the start lock, so an answer counted
/// past the discard's is a start's that came after it (#377 review r3,
/// risk 2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoreGeneration(u64);

impl StoreGeneration {
    /// The count, for a decision that compares two.
    #[must_use]
    pub const fn count(self) -> u64 {
        self.0
    }
}

struct Viewer {
    send: SyncSender<ToView>,
    woken: Arc<AtomicBool>,
    notify: Arc<dyn Fn() + Send + Sync>,
    /// The listing went out to this viewer: updates may follow.
    listed: bool,
}

impl Viewer {
    /// Send `message`; false when the view is gone or too far behind,
    /// either way to be detached.
    fn send(&self, message: ToView) -> bool {
        match self.send.try_send(message) {
            Ok(()) => {
                // Sent before the flag is read, and a take clears the flag
                // before it reads: a message is in that take or asks a
                // wake of its own.
                if !self.woken.swap(true, Ordering::SeqCst) {
                    (self.notify)();
                }
                true
            }
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
        }
    }
}

struct Slot {
    viewer: Option<Viewer>,
    running: bool,
    /// Whether the platform gives this app network access, as the
    /// Service last said; access is assumed until it says otherwise.
    network: bool,
    /// Whether the person must authenticate before anything runs, as the
    /// Activity last said.
    authentication: bool,
    /// The last start's answer, if one answered, and the answers so far.
    store: Option<StoreAnswer>,
    generation: StoreGeneration,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            viewer: None,
            running: false,
            network: true,
            authentication: false,
            store: None,
            generation: StoreGeneration::default(),
        }
    }
}

/// Why the message store refused the last start (ADR-0060 decision 6):
/// the screen the view shows in place of the chats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreRefusal {
    /// Recovery mode: the key missing or not opening the store, a damaged
    /// file, a newer build's, a failed migration.
    Recovery,
    /// The platform invalidated the store key: Discard and recreate is
    /// offered directly.
    KeyLost,
}

/// The running facade's generation and where its commands go.
#[derive(Default)]
struct Commands {
    generation: Option<u64>,
    sender: Option<tokio::sync::mpsc::UnboundedSender<Command>>,
}

/// The process's one meeting point between the facade and the view.
pub struct Hub {
    slot: Mutex<Slot>,
    /// The running facade's generation and, while it takes commands, where
    /// they go: withdrawn before it closes, so a command is never accepted
    /// by a facade that will not carry it out (owner's finding P2-2).
    commands: Mutex<Commands>,
    generations: std::sync::atomic::AtomicU64,
    focused: AtomicBool,
    notices: Notices,
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

impl Hub {
    /// No view, no facade.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slot: Mutex::new(Slot::default()),
            commands: Mutex::new(Commands::default()),
            generations: std::sync::atomic::AtomicU64::new(0),
            focused: AtomicBool::new(false),
            notices: Notices::default(),
        }
    }

    fn slot(&self) -> MutexGuard<'_, Slot> {
        // A panic while the slot was held left nothing half-written that
        // a reader could misread: each field is replaced whole.
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Attach a view, replacing any attached before (whose link reports
    /// [`ViewLink::lost`]). `notify` is called, from whichever thread has
    /// something for the view, when a message waits and no wake is
    /// outstanding: the view schedules a turn with it.
    pub fn attach(&self, notify: Arc<dyn Fn() + Send + Sync>) -> ViewLink {
        let (send, from) = mpsc::sync_channel(TO_VIEW);
        let woken = Arc::new(AtomicBool::new(false));
        let viewer = Viewer {
            send,
            woken: Arc::clone(&woken),
            notify,
            listed: false,
        };
        let mut slot = self.slot();
        // A new viewer starts from what runs now; the listing follows from
        // the facade's next turn.
        viewer.send(ToView::Running(slot.running));
        viewer.send(ToView::NetworkAccess(slot.network));
        viewer.send(ToView::AuthenticationNeeded(slot.authentication));
        if let Some(answer) = slot.store {
            viewer.send(ToView::Store(answer, slot.generation));
        }
        slot.viewer = Some(viewer);
        ViewLink {
            from,
            woken,
            lost: false,
        }
    }

    /// The view's window gained or lost the person's focus: while it has
    /// it, an arrival is read there and asks no notice.
    pub fn set_focused(&self, focused: bool) {
        self.focused.store(focused, Ordering::SeqCst);
        if focused {
            self.notices.clear();
        }
    }

    /// Whether a view's window has the person's focus now.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.focused.load(Ordering::SeqCst)
    }

    /// The notices waiting for the platform.
    #[must_use]
    pub const fn notices(&self) -> &Notices {
        &self.notices
    }

    /// Carry out `command` on the facade, or hand it back while no facade
    /// takes commands -- none runs, or the one that runs is closing -- for
    /// the view to fail it explicitly (owner's finding P2-2). A command
    /// accepted here is carried out: the facade withdraws before it
    /// closes and runs every command it accepted first.
    ///
    /// # Errors
    /// The command, when no facade takes it.
    pub fn command(&self, command: Command) -> Result<(), Command> {
        let commands = self.commands.lock().unwrap_or_else(PoisonError::into_inner);
        match commands.sender.as_ref() {
            Some(sender) => sender.send(command).map_err(|e| e.0),
            None => Err(command),
        }
    }

    // --- the facade's side -------------------------------------------------

    /// The facade started (`Some`, with where its commands go) or stopped:
    /// the latest generation, whichever it is. The tests' shorthand; a
    /// facade starts, withdraws and stops by its own generation.
    #[cfg(test)]
    pub(crate) fn set_running(
        &self,
        commands: Option<tokio::sync::mpsc::UnboundedSender<Command>>,
    ) {
        if let Some(sender) = commands {
            self.started(sender);
            return;
        }
        let current = self
            .commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .generation;
        if let Some(generation) = current {
            self.stopped(generation);
        }
    }

    /// A facade started, taking commands at `sender`: its generation, which
    /// alone may withdraw or stop it.
    pub(crate) fn started(&self, sender: tokio::sync::mpsc::UnboundedSender<Command>) -> u64 {
        let generation = self
            .generations
            .fetch_add(1, Ordering::SeqCst)
            .wrapping_add(1);
        *self.commands.lock().unwrap_or_else(PoisonError::into_inner) = Commands {
            generation: Some(generation),
            sender: Some(sender),
        };
        self.say_running(true);
        generation
    }

    /// Facade `generation` is closing: it takes no more commands. A later
    /// generation is left as it is.
    pub(crate) fn withdraw(&self, generation: u64) {
        let mut commands = self.commands.lock().unwrap_or_else(PoisonError::into_inner);
        if commands.generation == Some(generation) {
            commands.sender = None;
        }
    }

    /// Facade `generation` stopped: said, unless a later one runs -- an old
    /// facade's late stop never clears a new one.
    pub(crate) fn stopped(&self, generation: u64) {
        {
            let mut commands = self.commands.lock().unwrap_or_else(PoisonError::into_inner);
            if commands.generation != Some(generation) {
                return;
            }
            *commands = Commands::default();
        }
        self.say_running(false);
    }

    fn say_running(&self, running: bool) {
        let mut slot = self.slot();
        slot.running = running;
        if let Some(viewer) = slot.viewer.as_mut() {
            // Listed again from the new facade's first turn: rows the
            // last one showed may have changed while none ran.
            viewer.listed = false;
            if !viewer.send(ToView::Running(running)) {
                slot.viewer = None;
            }
        }
    }

    /// Whether the platform gives this app network access, as the
    /// Service sees it: said to the attached view, and to each later one.
    pub(crate) fn set_network_access(&self, allowed: bool) {
        let mut slot = self.slot();
        if slot.network == allowed {
            return;
        }
        slot.network = allowed;
        if let Some(viewer) = slot.viewer.as_ref()
            && !viewer.send(ToView::NetworkAccess(allowed))
        {
            slot.viewer = None;
        }
    }

    /// Whether the person must authenticate before anything runs, as the
    /// Activity learns it: said to the attached view, and to each later
    /// one.
    pub fn set_authentication_needed(&self, needed: bool) {
        let mut slot = self.slot();
        if slot.authentication == needed {
            return;
        }
        slot.authentication = needed;
        if let Some(viewer) = slot.viewer.as_ref()
            && !viewer.send(ToView::AuthenticationNeeded(needed))
        {
            slot.viewer = None;
        }
    }

    /// A start's answer: said to the attached view, and to each later
    /// one. EVERY ANSWER IS SAID AND COUNTED, an unchanged one included:
    /// each answers a start, and a view that asked for a restart learns
    /// from it how the restart went (#377 review F3). The caller says it
    /// under the start lock (`ServiceHost`), so the count orders answers
    /// against a discard.
    pub(crate) fn set_store_answer(&self, answer: StoreAnswer) {
        let mut slot = self.slot();
        slot.store = Some(answer);
        slot.generation = StoreGeneration(slot.generation.0 + 1);
        let generation = slot.generation;
        if let Some(viewer) = slot.viewer.as_ref()
            && !viewer.send(ToView::Store(answer, generation))
        {
            slot.viewer = None;
        }
    }

    /// The start answers said so far.
    pub(crate) fn store_generation(&self) -> StoreGeneration {
        self.slot().generation
    }

    /// Whether the attached view, if any, still waits for its listing
    /// from a running facade.
    pub(crate) fn wants_listing(&self) -> bool {
        let slot = self.slot();
        slot.running && slot.viewer.as_ref().is_some_and(|v| !v.listed)
    }

    /// Hand the listing (or its failure) to the view that waits for it.
    pub(crate) fn listed(&self, listing: ToView) {
        let mut slot = self.slot();
        if let Some(viewer) = slot.viewer.as_mut() {
            viewer.listed = true;
            if !viewer.send(listing) {
                slot.viewer = None;
            }
        }
    }

    /// Hand `update` to a listed view; with none, it is dropped (the
    /// store keeps the rows) after a received message has been counted
    /// for a notice when no window has the person's focus.
    pub(crate) fn update(&self, update: Update) {
        if matches!(update, Update::Received(_)) && !self.focused() {
            self.notices.received();
        }
        let mut slot = self.slot();
        if let Some(viewer) = slot.viewer.as_ref().filter(|v| v.listed)
            && !viewer.send(ToView::Update(Box::new(update)))
        {
            slot.viewer = None;
        }
    }
}

/// A view's end of the hub.
pub struct ViewLink {
    from: Receiver<ToView>,
    woken: Arc<AtomicBool>,
    lost: bool,
}

impl ViewLink {
    /// What was sent since the last call, in order. Answers the
    /// outstanding wake, so the next message asks for another.
    #[must_use]
    pub fn take(&mut self) -> Vec<ToView> {
        self.woken.store(false, Ordering::SeqCst);
        let mut taken = Vec::new();
        loop {
            match self.from.try_recv() {
                Ok(message) => taken.push(message),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.lost = true;
                    break;
                }
            }
        }
        taken
    }

    /// The hub detached this view -- another attached, or it fell
    /// [`TO_VIEW`] behind -- and sends it nothing more. A view that is
    /// still shown attaches again, and starts again from a listing.
    #[must_use]
    pub const fn lost(&self) -> bool {
        self.lost
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use interweave_human_app_core::Listing;
    use interweave_human_client_api::Diagnostics;

    use super::*;

    fn counting() -> (Arc<AtomicUsize>, Arc<dyn Fn() + Send + Sync>) {
        let count = Arc::new(AtomicUsize::new(0));
        let inner = Arc::clone(&count);
        (
            count,
            Arc::new(move || {
                inner.fetch_add(1, Ordering::SeqCst);
            }),
        )
    }

    fn listed() -> ToView {
        ToView::Update(Box::new(Update::Listed(Listing::default())))
    }

    #[test]
    fn a_view_hears_whether_the_service_runs_then_its_listing_then_updates() {
        let hub = Hub::new();
        let (_, wake) = counting();
        let mut link = hub.attach(wake);
        assert!(matches!(
            link.take()[..],
            [
                ToView::Running(false),
                ToView::NetworkAccess(true),
                ToView::AuthenticationNeeded(false)
            ]
        ));
        assert!(!hub.wants_listing(), "nothing to list while none runs");

        let (commands, _rx) = tokio::sync::mpsc::unbounded_channel();
        hub.set_running(Some(commands));
        assert!(hub.wants_listing());
        // An update before the listing never reaches the view.
        hub.update(Update::Diagnostics(Diagnostics::default()));
        hub.listed(listed());
        hub.update(Update::Diagnostics(Diagnostics::default()));
        let taken = link.take();
        assert!(matches!(taken[0], ToView::Running(true)), "{taken:?}");
        assert!(
            matches!(&taken[1], ToView::Update(u) if matches!(**u, Update::Listed(_))),
            "{taken:?}"
        );
        assert!(
            matches!(&taken[2], ToView::Update(u) if matches!(**u, Update::Diagnostics(_))),
            "{taken:?}"
        );
        assert_eq!(taken.len(), 3, "the pre-listing update was not sent");
    }

    #[test]
    fn one_wake_per_take_however_many_messages_wait() {
        let hub = Hub::new();
        let (woken, wake) = counting();
        let mut link = hub.attach(wake);
        let (commands, _rx) = tokio::sync::mpsc::unbounded_channel();
        hub.set_running(Some(commands));
        hub.listed(listed());
        assert_eq!(
            woken.load(Ordering::SeqCst),
            1,
            "Running(false), then nothing taken"
        );
        let _ = link.take();
        hub.update(Update::Diagnostics(Diagnostics::default()));
        hub.update(Update::Diagnostics(Diagnostics::default()));
        assert_eq!(woken.load(Ordering::SeqCst), 2, "one wake for both");
    }

    #[test]
    fn a_view_that_falls_behind_is_detached_and_told() {
        let hub = Hub::new();
        let (_, wake) = counting();
        let mut link = hub.attach(wake);
        let (commands, _rx) = tokio::sync::mpsc::unbounded_channel();
        hub.set_running(Some(commands));
        hub.listed(listed());
        // Running(false), Running(true) and the listing are queued already.
        for _ in 0..TO_VIEW {
            hub.update(Update::Diagnostics(Diagnostics::default()));
        }
        let taken = link.take();
        assert_eq!(taken.len(), TO_VIEW, "the queue held exactly its bound");
        assert!(link.lost(), "and the view was dropped, not waited for");
        assert!(!hub.wants_listing(), "no viewer remains");
    }

    #[test]
    fn a_view_hears_a_change_of_network_access_once_and_a_later_view_hears_it_too() {
        let hub = Hub::new();
        let (_, wake) = counting();
        let mut link = hub.attach(wake);
        let _ = link.take();
        hub.set_network_access(false);
        hub.set_network_access(false);
        assert!(
            matches!(link.take()[..], [ToView::NetworkAccess(false)]),
            "said once, at the change"
        );
        let (_, wake) = counting();
        let mut later = hub.attach(wake);
        assert!(matches!(
            later.take()[..],
            [
                ToView::Running(false),
                ToView::NetworkAccess(false),
                ToView::AuthenticationNeeded(false)
            ]
        ));
    }

    /// The person's authentication, needed or not, is said at the change
    /// and to every later view, as access is.
    #[test]
    fn a_view_hears_that_authentication_is_needed_once_and_a_later_view_hears_it_too() {
        let hub = Hub::new();
        let (_, wake) = counting();
        let mut link = hub.attach(wake);
        let _ = link.take();
        hub.set_authentication_needed(true);
        hub.set_authentication_needed(true);
        assert!(
            matches!(link.take()[..], [ToView::AuthenticationNeeded(true)]),
            "said once, at the change"
        );
        let (_, wake) = counting();
        let mut later = hub.attach(wake);
        assert!(matches!(
            later.take()[..],
            [
                ToView::Running(false),
                ToView::NetworkAccess(true),
                ToView::AuthenticationNeeded(true)
            ]
        ));
    }

    #[test]
    fn a_second_view_replaces_the_first() {
        let hub = Hub::new();
        let (_, wake) = counting();
        let mut first = hub.attach(Arc::clone(&wake));
        let mut second = hub.attach(wake);
        let _ = first.take();
        assert!(first.lost());
        assert!(matches!(
            second.take()[..],
            [
                ToView::Running(false),
                ToView::NetworkAccess(true),
                ToView::AuthenticationNeeded(false)
            ]
        ));
        assert!(!second.lost());
    }

    fn arrival() -> Update {
        use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
        use interweave_human_client_api::{Origin, Received};
        let peer = interweave_profile_identity::ProfileIdentity::generate()
            .transport_identity()
            .expect("a peer id");
        Update::Received(Received {
            row: interweave_human_core::RowId::from_stored(1),
            origin: Origin::Direct {
                peer,
                endpoint: interweave_transport_api::EndpointId::parse("human")
                    .expect("an endpoint"),
            },
            envelope: HumanChatV2 {
                v: 2,
                kind: MessageKind::Text,
                app_message_id: format!("{:032x}", 1),
                text: "hello".to_owned(),
                reply_to: None,
                sent_at_ms: None,
                from_endpoint: None,
            },
            received_at: 1,
        })
    }

    /// human-client-android.md, "Notifications": an arrival no focused
    /// window reads asks a notice; one a focused window reads does not;
    /// and focus withdraws what waited (review F4).
    #[test]
    fn an_arrival_asks_a_notice_only_while_no_window_has_focus() {
        let now = std::time::Duration::ZERO;
        let hub = Hub::new();
        hub.update(arrival());
        assert_eq!(
            hub.notices().wait_change(now),
            Some(1),
            "unfocused: noticed"
        );

        hub.set_focused(true);
        assert_eq!(
            hub.notices().wait_change(now),
            Some(0),
            "focus withdraws it"
        );
        hub.update(arrival());
        assert_eq!(hub.notices().wait_change(now), None, "focused: read there");

        hub.set_focused(false);
        hub.update(Update::Diagnostics(Diagnostics::default()));
        assert_eq!(
            hub.notices().wait_change(now),
            None,
            "the control: an update that is no arrival asks nothing"
        );
    }

    #[test]
    fn a_command_goes_nowhere_while_no_facade_runs() {
        let hub = Hub::new();
        assert_eq!(hub.command(Command::Reopen), Err(Command::Reopen));
        let (commands, mut rx) = tokio::sync::mpsc::unbounded_channel();
        hub.set_running(Some(commands));
        assert_eq!(hub.command(Command::Reopen), Ok(()));
        assert!(matches!(rx.try_recv(), Ok(Command::Reopen)));
        hub.set_running(None);
        assert_eq!(hub.command(Command::Reopen), Err(Command::Reopen));
    }

    /// Owner's finding P2-2: a closing facade takes no command -- it is
    /// handed back for the view to fail -- and an old facade's withdraw or
    /// late stop never touches a newer one. Mutations: withdraw leaving the
    /// sender; a stop of any generation clearing the current one.
    #[test]
    fn a_closing_facade_takes_no_command_and_an_old_one_never_stops_a_new_one() {
        let hub = Hub::new();
        let (first, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let old = hub.started(first);
        hub.withdraw(old);
        assert_eq!(
            hub.command(Command::Reopen),
            Err(Command::Reopen),
            "closing"
        );
        let (second, mut rx2) = tokio::sync::mpsc::unbounded_channel();
        let new = hub.started(second);
        assert_ne!(old, new);
        hub.withdraw(old);
        hub.stopped(old);
        assert_eq!(
            hub.command(Command::Reopen),
            Ok(()),
            "the new one still takes it"
        );
        assert!(matches!(rx2.try_recv(), Ok(Command::Reopen)));
        hub.stopped(new);
        assert_eq!(hub.command(Command::Reopen), Err(Command::Reopen));
    }

    /// Every answer is said and counted, an unchanged one included: a
    /// view that asked for a restart learns it was refused again (#377
    /// review F3); and a view attached later hears the last answer with
    /// the hub's count, never a count of its own from zero (#377 review
    /// r3, risk 1).
    #[test]
    fn every_answer_is_said_with_the_hubs_count_and_an_attach_hears_the_last() {
        let hub = Hub::new();
        let (_, wake) = counting();
        let mut link = hub.attach(wake);
        let _ = link.take();
        let answers = |link: &mut ViewLink| {
            link.take()
                .into_iter()
                .filter_map(|m| match m {
                    ToView::Store(answer, generation) => Some((answer, generation.0)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let key_lost = StoreAnswer::Refused(StoreRefusal::KeyLost);
        hub.set_store_answer(key_lost);
        hub.set_store_answer(key_lost);
        assert_eq!(answers(&mut link), [(key_lost, 1), (key_lost, 2)]);
        hub.set_store_answer(StoreAnswer::OtherRefusal);
        hub.set_store_answer(StoreAnswer::Opened);
        hub.set_store_answer(StoreAnswer::Opened);
        assert_eq!(
            answers(&mut link),
            [
                (StoreAnswer::OtherRefusal, 3),
                (StoreAnswer::Opened, 4),
                (StoreAnswer::Opened, 5)
            ]
        );
        assert_eq!(hub.store_generation(), StoreGeneration(5));

        let (_, wake) = counting();
        let mut again = hub.attach(wake);
        assert_eq!(
            answers(&mut again),
            [(StoreAnswer::Opened, 5)],
            "the last answer, at the hub's count"
        );
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Add-a-peer acceptance 10's superseded cases, those the tests below name
//! (`design/2026-10-22-add-a-peer.md` §5.7 and acceptance 10), against a
//! real daemon: the app's change held on its open admin connection before
//! it is sent, `transportctl` writing the same address or peer
//! meanwhile, then the held change sent.
//!
//! The client is its two root halves -- `app-core`'s model side and
//! facade side over the human client's facade, on the daemon's IPC
//! sockets -- as `codes.rs` runs them. The hold is [`Holding`], an admin
//! binding that opens the real IPC admin port, so the daemon stamps the
//! connection at its admission (`LOCAL-IPC.md` §Cancellation mapping and
//! request concurrency), and holds each write on that port until the case
//! releases it. `transportctl`'s connection is admitted after the app's,
//! so its write for the same key makes the app's held write a write from
//! an older connection: refused `superseded`, nothing written. The hold
//! is this crate's: nothing that ships is built with it.
//!
//! Each case asserts the daemon's audit line naming the app's write
//! refused `superseded` (the only witness for the cases whose page would
//! read the same had the write been applied: an add read back as added,
//! a removal as removed, a Trust as trusted, an Undo as listed), what the
//! page shows, that the change was sent once and the list (or trust set)
//! read once, and in most what `transportctl` reads afterwards. The
//! control that the hold is what makes them superseded: with the hold moved before the port is opened, so the
//! app's connection is admitted after `transportctl`'s, every case
//! fails at its audit assertion (run 2026-11-16, recorded on the pull
//! request that brought them).
//!
//! The read back "made to fail" is the hold's: armed so, the first read
//! of the list or the trust set after the held write is answered
//! `BackendUnavailable` without reaching the daemon, the write itself
//! still the daemon's. So is a lost answer: armed so, the write reaches
//! the daemon and the app hears `BackendUnavailable` in place of its
//! answer, which is how an earlier add is left `AddUnknown`.
//!
//! Every case asserts where focus is asked to go after its outcome
//! (§8's table), on the model, since this client has no view. Also run:
//! a superseded add carrying a line an earlier `AddUnknown` left
//! unsettled, the line settled only by the refusal's successful read
//! back; Trust again after `TrustStepSupersededUnread` writing nothing,
//! which says its own class -- `TrustUnavailable`, A's daemon stopped so
//! the trust connection cannot open; and a superseded Trust left unread,
//! then decided by a later read of the trust set (the settings opened),
//! focus not moved by it.
//!
//! NOT run here, and why. The `Internal` class of that Trust again
//! (`TrustFailed`): the facade reports a write that writes nothing as
//! `Internal` only for an error the daemon does not answer to
//! `admin.trust.set` (its own `Internal` there may have reached it, so
//! it is unconfirmed), so only the hold inventing an answer would give
//! it, and `TrustUnavailable` holds the same rule from the real daemon.
//! The announcement after the later read, and the two snackbars without
//! an action staying 10 seconds, under focus or the pointer, or for
//! Android's accessibility timeout: the view's (`ui-slint`'s
//! `NO_ACTION_MIN_MS` and its announcer), after the outcome this file
//! holds, which no daemon enters. `ui-model`'s and `ui-slint`'s
//! `tests/peers.rs` hold those on the model and the rendered views. Nor
//! the window, or Android, whose facade is the same crate in process.

use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
use std::future::Future;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use interweave_human_app_core::{Command, FacadeSide, ModelSide, Opener, Surface, Update};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{
    AddressOrigin, ClientConfig, TransportClient, TrustProblem,
};
use interweave_human_ui_model::{
    AddNote, AddResult, BlockHeld, BlockProblem, BlockState, BlockView, Peers, PeersFocus,
    PeersInput, PeersSnack, TrustInput, TrustSettings, UiModel, ViewEvent,
};
use interweave_ipc_client::IpcBinding;
use interweave_local_client_api::{
    AddressChange, AddressesChange, AdminBinding, AdminCapability, AdminPort, AdminStatus,
    BootstrapAddress, EndpointAdminView, Generation, LocalAdminPort, OwnAddresses, PeerGateView,
    TrustAdminView, TrustChange,
};
use interweave_transport_api::{EndpointId, MAX_PAYLOAD_BYTES, TransportError, TransportIdentity};
use tokio::sync::oneshot;

use crate::common::{PATIENCE, human};
use crate::package::transportctl_ok;
use crate::world::{World, two_daemons};

// --- the hold ------------------------------------------------------------

/// What an arming does besides holding the write: whether the read
/// after it fails, and whether the write's own answer is lost.
#[derive(Clone, Copy, Default)]
struct Arming {
    /// The first read of the list or the trust set after the write is
    /// answered `BackendUnavailable` without being sent.
    fail_read_back: bool,
    /// The write is sent and the daemon's answer to it dropped, the call
    /// answered `BackendUnavailable` in its place: the answer lost after
    /// the request left, which the facade reports unconfirmed.
    lose_answer: bool,
}

/// One armed hold: who to tell the write is held, what releases it, and
/// what else it does.
type Armed = (oneshot::Sender<&'static str>, oneshot::Receiver<()>, Arming);

/// The next write on an admin port [`Holding`] opened waits here once
/// armed; one write per arming. Armed to fail the read back, the first
/// read of the list or the trust set after that write, on any port the
/// binding opened, is answered `BackendUnavailable` without being sent:
/// the read back "made to fail" of acceptance 10, the write itself still
/// the daemon's. Armed to lose the answer, the write reaches the daemon
/// and the app hears `BackendUnavailable` instead of its answer.
#[derive(Clone, Default)]
struct Hold {
    armed: Arc<Mutex<Option<Armed>>>,
    read_fails: Arc<AtomicBool>,
}

impl Hold {
    /// Hold the next write: the receiver hears its method once it is
    /// held, and the sender lets it go.
    fn arm(&self, arming: Arming) -> (oneshot::Receiver<&'static str>, oneshot::Sender<()>) {
        let (held, heard) = oneshot::channel();
        let (release, released) = oneshot::channel();
        *self.armed.lock().unwrap_or_else(PoisonError::into_inner) = Some((held, released, arming));
        (heard, release)
    }

    /// Wait here if armed: after the port was opened, before `method` is
    /// sent on it. Answers whether the write's answer is to be lost.
    async fn wait(&self, method: &'static str) -> bool {
        let armed = self
            .armed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let Some((held, released, arming)) = armed else {
            return false;
        };
        let _ = held.send(method);
        let _ = released.await;
        self.read_fails
            .store(arming.fail_read_back, Ordering::SeqCst);
        arming.lose_answer
    }

    /// Whether this read is the one made to fail; once.
    fn read_fails(&self) -> bool {
        self.read_fails.swap(false, Ordering::SeqCst)
    }
}

/// `answer` as the app hears it: lost after the daemon gave it, when
/// `lost`.
fn heard<T>(answer: Result<T, TransportError>, lost: bool) -> Result<T, TransportError> {
    if lost {
        drop(answer);
        Err(TransportError::BackendUnavailable)
    } else {
        answer
    }
}

/// The app's admin binding over the real IPC binding, its writes held by
/// [`Hold`] on the port they were opened on.
#[derive(Clone)]
struct Holding {
    inner: IpcBinding,
    hold: Hold,
}

impl AdminBinding for Holding {
    type Admin = HeldPort<<IpcBinding as AdminBinding>::Admin>;

    fn admin(
        &self,
        capabilities: BTreeSet<AdminCapability>,
    ) -> impl Future<Output = Result<Self::Admin, TransportError>> + Send {
        let open = self.inner.admin(capabilities);
        let hold = self.hold.clone();
        async move {
            Ok(HeldPort {
                port: open.await?,
                hold,
            })
        }
    }
}

/// An open admin port whose writes wait on the hold before they are sent.
struct HeldPort<P> {
    port: P,
    hold: Hold,
}

impl<P: AdminPort + Sync> AdminPort for HeldPort<P> {
    fn port(&self) -> &LocalAdminPort {
        self.port.port()
    }

    fn status(&self) -> impl Future<Output = Result<AdminStatus, TransportError>> + Send {
        self.port.status()
    }

    fn leases(
        &self,
    ) -> impl Future<Output = Result<Vec<EndpointAdminView>, TransportError>> + Send {
        self.port.leases()
    }

    fn revoke_endpoint(
        &self,
        endpoint: EndpointId,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        self.port.revoke_endpoint(endpoint)
    }

    fn set_endpoint_enabled(
        &self,
        endpoint: EndpointId,
        enabled: bool,
    ) -> impl Future<Output = Result<Option<Generation>, TransportError>> + Send {
        self.port.set_endpoint_enabled(endpoint, enabled)
    }

    fn set_default_endpoint(
        &self,
        endpoint: Option<EndpointId>,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        self.port.set_default_endpoint(endpoint)
    }

    async fn trust(&self) -> Result<TrustAdminView, TransportError> {
        if self.hold.read_fails() {
            return Err(TransportError::BackendUnavailable);
        }
        self.port.trust().await
    }

    fn peers(&self) -> impl Future<Output = Result<Vec<PeerGateView>, TransportError>> + Send {
        self.port.peers()
    }

    async fn add_bootstrap_address(
        &self,
        address: String,
    ) -> Result<AddressChange, TransportError> {
        let lost = self.hold.wait("admin.peers.add_address").await;
        heard(self.port.add_bootstrap_address(address).await, lost)
    }

    async fn add_bootstrap_addresses(
        &self,
        addresses: Vec<String>,
    ) -> Result<AddressesChange, TransportError> {
        let lost = self.hold.wait("admin.peers.add_addresses").await;
        heard(self.port.add_bootstrap_addresses(addresses).await, lost)
    }

    async fn remove_bootstrap_address(
        &self,
        address: String,
    ) -> Result<AddressChange, TransportError> {
        let lost = self.hold.wait("admin.peers.remove_address").await;
        heard(self.port.remove_bootstrap_address(address).await, lost)
    }

    async fn bootstrap_addresses(&self) -> Result<Vec<BootstrapAddress>, TransportError> {
        if self.hold.read_fails() {
            return Err(TransportError::BackendUnavailable);
        }
        self.port.bootstrap_addresses().await
    }

    fn own_addresses(&self) -> impl Future<Output = Result<OwnAddresses, TransportError>> + Send {
        self.port.own_addresses()
    }

    async fn set_trust(
        &self,
        peer: TransportIdentity,
        allowed: bool,
    ) -> Result<TrustChange, TransportError> {
        let lost = self.hold.wait("admin.trust.set").await;
        heard(self.port.set_trust(peer, allowed).await, lost)
    }

    fn shutdown(&self, grace: Duration) -> impl Future<Output = Result<(), TransportError>> + Send {
        self.port.shutdown(grace)
    }
}

// --- the client ----------------------------------------------------------

/// What the Peers page showed at the last render.
#[derive(Clone, Default)]
struct Seen {
    peers: Peers,
    blocks: Vec<BlockView>,
    trust: TrustSettings,
}

impl Seen {
    /// Where the page asks focus to be, on what is drawn.
    fn focus(&self) -> Option<PeersFocus> {
        self.peers.focus().0
    }

    /// The row A's profile authors, its static route to B: listed after
    /// every row added here.
    fn configured(&self) -> String {
        self.peers
            .list()
            .expect("a list")
            .addresses
            .iter()
            .find(|row| row.origin == AddressOrigin::Configured)
            .expect("the configured route to B")
            .address
            .clone()
    }
}

/// A surface whose person acts from a queue the case fills, keeping what
/// each render showed of Peers.
#[derive(Clone, Default)]
struct Scripted {
    events: Rc<RefCell<VecDeque<Vec<ViewEvent>>>>,
    seen: Rc<RefCell<Seen>>,
}

impl Surface for Scripted {
    fn render(&mut self, model: &UiModel) {
        *self.seen.borrow_mut() = Seen {
            peers: model.peers().clone(),
            blocks: model.peers().blocks(model.trust_settings()),
            trust: model.trust_settings().clone(),
        };
    }

    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        self.events.borrow_mut().pop_front().unwrap_or_default()
    }
}

/// Asked to open nothing in this case.
struct Nothing;

impl Opener for Nothing {
    fn open(&mut self, destination: &str) {
        panic!("asked to open {destination}");
    }

    fn ask_network_access(&mut self) {}

    fn authenticate(&mut self) {}
}

fn wall_ms() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after the epoch")
            .as_millis(),
    )
    .expect("ms")
}

/// The client's two root halves on A's sockets, the admin ones held.
struct App {
    model: ModelSide<Scripted, Nothing>,
    facade: FacadeSide<IpcBinding, Holding>,
    person: Scripted,
    hold: Hold,
    commands: Vec<Command>,
    start: Instant,
    _store: tempfile::TempDir,
}

impl App {
    async fn on(world: &World) -> Self {
        let hold = Hold::default();
        let store_dir = tempfile::tempdir().expect("a store directory");
        let store = HumanStore::open(
            &store_dir.path().join("state/human.sqlite"),
            StoreOptions::default(),
        )
        .expect("a store");
        let client = TransportClient::new(
            world.a.binding(),
            Holding {
                inner: world.a.binding(),
                hold: hold.clone(),
            },
            store,
            ClientConfig {
                client_kind: "human-client".to_owned(),
                endpoints: vec![human()],
                channels: Vec::new(),
                max_payload_bytes: MAX_PAYLOAD_BYTES,
            },
            Box::new(wall_ms),
            0,
        )
        .expect("an empty store's pending rows");
        let mut facade = FacadeSide::new(client, wall_ms);
        let person = Scripted::default();
        let mut model = ModelSide::new(person.clone(), Nothing);
        model.apply(Update::Started(Box::new(
            facade.start().expect("an empty store's start"),
        )));
        let mut app = Self {
            model,
            facade,
            person,
            hold,
            commands: Vec::new(),
            start: Instant::now(),
            _store: store_dir,
        };
        app.act(PeersInput::Opened).await;
        assert!(
            app.seen().peers.list().is_some(),
            "the control: the page read the daemon's list"
        );
        app
    }

    fn seen(&self) -> Seen {
        self.person.seen.borrow().clone()
    }

    /// One pass of the root's loop, as the desktop root runs it; whether
    /// the model side asked for anything.
    async fn pump(&mut self) -> bool {
        let start = self.start;
        let now = || u64::try_from(start.elapsed().as_millis()).expect("ms");
        for update in self.facade.turn(now()).await {
            self.model.apply(update);
        }
        let asked = self.model.turn();
        let any = !asked.is_empty();
        for command in asked {
            self.commands.push(command.clone());
            for update in self.facade.execute(command, now()).await {
                self.model.apply(update);
            }
        }
        any
    }

    /// Run the loop until the model side has asked for nothing three
    /// passes running.
    async fn settle(&mut self) {
        let deadline = Instant::now() + PATIENCE;
        let mut quiet = 0;
        // The deadline is asked before each pass, never after one: the
        // pass that settles it must end the loop, not fail it.
        while quiet < 3 {
            assert!(
                Instant::now() < deadline,
                "the model side never settled: {:?}",
                self.commands
            );
            if self.pump().await {
                quiet = 0;
            } else {
                quiet += 1;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// The person does `input`, and the client settles.
    async fn act(&mut self, input: PeersInput) {
        self.person
            .events
            .borrow_mut()
            .push_back(vec![ViewEvent::Peers(input)]);
        self.settle().await;
    }

    /// The person does `input`; the write it leads to is held on its open
    /// port while `meanwhile` runs, then sent. Answers the commands made
    /// from the input on.
    async fn held_while(&mut self, input: PeersInput, meanwhile: impl FnOnce()) -> Vec<Command> {
        self.held_with(input, Arming::default(), meanwhile).await
    }

    /// [`Self::held_while`], the hold armed as `arming` says.
    async fn held_with(
        &mut self,
        input: PeersInput,
        arming: Arming,
        meanwhile: impl FnOnce(),
    ) -> Vec<Command> {
        let from = self.commands.len();
        let (held, release) = self.hold.arm(arming);
        self.person
            .events
            .borrow_mut()
            .push_back(vec![ViewEvent::Peers(input)]);
        let other = async move {
            let method = tokio::time::timeout(PATIENCE, held)
                .await
                .expect("the control: the app's write was held")
                .expect("the hold");
            meanwhile();
            let _ = release.send(());
            method
        };
        let ((), method) = tokio::join!(self.settle(), other);
        let commands = self.commands[from..].to_vec();
        assert!(
            !commands.is_empty(),
            "{method} held, yet no command was made"
        );
        commands
    }
}

// --- the cases -----------------------------------------------------------

/// The hold with the read back after the write made to fail.
const READ_FAILS: Arming = Arming {
    fail_read_back: true,
    lose_answer: false,
};

/// A fresh peer's address at a private host the add floor admits.
fn fresh_address() -> (TransportIdentity, String) {
    let peer = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    let address = format!("/ip4/192.168.1.20/tcp/4001/p2p/{peer}");
    (TransportIdentity::parse(peer).expect("a PeerId"), address)
}

/// Whether `transportctl`'s list of A's addresses holds `address`.
fn transportctl_lists(world: &World, address: &str) -> bool {
    transportctl_ok(&world.a, &["peers", "addresses"], "")
        .lines()
        .any(|line| line.split_whitespace().next() == Some(address))
}

/// Whether the page's list, as read, holds `address`.
fn page_lists(seen: &Seen, address: &str) -> bool {
    seen.peers
        .list()
        .expect("the list as read back")
        .addresses
        .iter()
        .any(|row| row.address == address)
}

/// Whether A's daemon audited a write of `method` naming `key` as refused
/// `superseded`: the daemon's own word that the app's held write was
/// refused, which an outcome the page would also show for an applied
/// write (an add read back as added, a removal as removed) cannot say.
fn audited_superseded(world: &World, method: &str, key: &str) -> bool {
    world
        .a_daemon
        .log()
        .lines()
        .any(|line| line.contains("superseded") && line.contains(method) && line.contains(key))
}

fn count(commands: &[Command], which: impl Fn(&Command) -> bool) -> usize {
    commands.iter().filter(|c| which(c)).count()
}

/// The change made once, and the list read once: by the change's own read
/// back, never by a command after it.
fn sent_once_read_once(commands: &[Command], change: impl Fn(&Command) -> bool) {
    assert_eq!(count(commands, change), 1, "sent once: {commands:?}");
    assert_eq!(
        count(commands, |c| matches!(c, Command::ReadAddresses)),
        0,
        "the list was read once, by the change: {commands:?}"
    );
}

/// The add page open with `address` typed.
async fn typed(app: &mut App, address: &str) {
    app.act(PeersInput::OpenAdd).await;
    app.act(PeersInput::FieldChanged(address.to_owned())).await;
}

fn add_of(address: &str) -> impl Fn(&Command) -> bool + '_ {
    move |c| matches!(c, Command::AddAddresses(lines) if lines == &[address.to_owned()])
}

fn removal_of(address: &str) -> impl Fn(&Command) -> bool + '_ {
    move |c| matches!(c, Command::RemoveAddress(a) if a == address)
}

/// An add held while `transportctl` removes the address reads
/// `AddSuperseded`, the field kept and no Try again; `transportctl`'s
/// list shows the address absent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_add_held_while_transportctl_removes_the_address_reads_add_superseded() {
    let world = two_daemons().await;
    let (_, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;

    let commands = app
        .held_while(PeersInput::Add, || {
            transportctl_ok(&world.a, &["peers", "remove", &address], "");
        })
        .await;

    assert!(
        audited_superseded(&world, "admin.peers.add_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );
    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::AddButton),
        "Add, the line under it"
    );
    assert_eq!(
        seen.peers.add_page_note(),
        Some(&AddNote::Superseded { lines: 1 })
    );
    assert_eq!(seen.peers.add_result(), None);
    assert_eq!(seen.peers.field(), address, "the field is kept");
    assert!(!page_lists(&seen, &address), "as read back");
    sent_once_read_once(&commands, add_of(&address));
    // No Try again: the line offers none, and Add with the field as it is
    // is the person's own new add, which this case does not make.
    app.act(PeersInput::TryAgain).await;
    assert_eq!(
        count(&app.commands, add_of(&address)),
        1,
        "not sent again: {:?}",
        app.commands
    );
    assert!(!transportctl_lists(&world, &address), "absent");
}

/// An add held while `transportctl` adds the same address reads
/// `AddedChecked` and offers step 2.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_add_held_while_transportctl_adds_it_reads_added_checked_and_offers_step_two() {
    let world = two_daemons().await;
    let (peer, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;

    let commands = app
        .held_while(PeersInput::Add, || {
            transportctl_ok(&world.a, &["peers", "add", &address], "");
        })
        .await;

    assert!(
        audited_superseded(&world, "admin.peers.add_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );
    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::Block(peer.clone())),
        "step 2's title"
    );
    assert_eq!(
        seen.peers.add_result(),
        Some(&AddResult::Checked { lines: 1 })
    );
    assert_eq!(
        seen.blocks.first().map(|b| (&b.peer, b.state)),
        Some((&peer, BlockState::Undecided)),
        "step 2 offers Trust for the peer: {:?}",
        seen.blocks
    );
    sent_once_read_once(&commands, add_of(&address));
    assert!(transportctl_lists(&world, &address), "listed");
}

/// A Remove held while `transportctl` adds the same address (answered
/// unchanged) shows `RemoveSuperseded` with no action, the row listed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remove_held_while_transportctl_adds_it_shows_remove_superseded_the_row_listed() {
    let world = two_daemons().await;
    let (_, address) = fresh_address();
    transportctl_ok(&world.a, &["peers", "add", &address], "");
    let mut app = App::on(&world).await;
    assert!(page_lists(&app.seen(), &address), "the control: listed");

    let mut unchanged = String::new();
    let commands = app
        .held_while(PeersInput::Remove(address.clone()), || {
            unchanged = transportctl_ok(&world.a, &["peers", "add", &address], "");
        })
        .await;

    assert!(
        audited_superseded(&world, "admin.peers.remove_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );
    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::RowMenu(address.clone())),
        "the row's menu button"
    );
    assert_eq!(
        seen.peers.snack().0,
        Some(&PeersSnack::RemoveSuperseded {
            address: address.clone()
        }),
        "transportctl's add said: {unchanged}"
    );
    assert!(page_lists(&seen, &address), "the row listed");
    sent_once_read_once(&commands, removal_of(&address));
    app.act(PeersInput::TryAgain).await;
    app.act(PeersInput::Undo).await;
    assert_eq!(
        count(&app.commands, removal_of(&address)),
        1,
        "no action sends it again: {:?}",
        app.commands
    );
    assert_eq!(count(&app.commands, add_of(&address)), 0, "nor adds it");
    assert!(transportctl_lists(&world, &address), "listed");
}

/// A Remove held while `transportctl` removes the address shows
/// `ConfirmAddressRemoved` with Undo.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remove_held_while_transportctl_removes_it_shows_removed_with_undo() {
    let world = two_daemons().await;
    let (_, address) = fresh_address();
    transportctl_ok(&world.a, &["peers", "add", &address], "");
    let mut app = App::on(&world).await;
    assert!(page_lists(&app.seen(), &address), "the control: listed");

    let commands = app
        .held_while(PeersInput::Remove(address.clone()), || {
            transportctl_ok(&world.a, &["peers", "remove", &address], "");
        })
        .await;

    assert!(
        audited_superseded(&world, "admin.peers.remove_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );
    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::Row(seen.configured())),
        "as Remove done: the next row"
    );
    assert!(
        matches!(
            seen.peers.snack().0,
            Some(PeersSnack::Removed { address: a, undo: true, .. }) if *a == address
        ),
        "{:?}",
        seen.peers.snack().0
    );
    assert!(!page_lists(&seen, &address), "the row gone");
    sent_once_read_once(&commands, removal_of(&address));
    assert!(!transportctl_lists(&world, &address), "absent");
}

/// An Undo held while `transportctl` removes the address (answered
/// unchanged) shows `UndoSuperseded` with no action, the row absent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_undo_held_while_transportctl_removes_it_shows_undo_superseded_the_row_absent() {
    let world = two_daemons().await;
    let (_, address) = fresh_address();
    transportctl_ok(&world.a, &["peers", "add", &address], "");
    let mut app = App::on(&world).await;
    app.act(PeersInput::Remove(address.clone())).await;
    assert!(
        matches!(
            app.seen().peers.snack().0,
            Some(PeersSnack::Removed { undo: true, .. })
        ),
        "the control: removed, Undo offered"
    );

    let commands = app
        .held_while(PeersInput::Undo, || {
            transportctl_ok(&world.a, &["peers", "remove", &address], "");
        })
        .await;

    assert!(
        audited_superseded(&world, "admin.peers.add_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );
    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::Row(seen.configured())),
        "as after Remove: the next row"
    );
    assert_eq!(
        seen.peers.snack().0,
        Some(&PeersSnack::UndoSuperseded {
            address: address.clone()
        })
    );
    assert!(!page_lists(&seen, &address), "the row absent");
    sent_once_read_once(&commands, add_of(&address));
    app.act(PeersInput::TryAgain).await;
    app.act(PeersInput::Undo).await;
    assert_eq!(
        count(&app.commands, add_of(&address)),
        1,
        "no action sends it again: {:?}",
        app.commands
    );
    assert!(!transportctl_lists(&world, &address), "absent");
}

/// A step-2 Trust held while `transportctl` revokes the peer reads
/// `TrustStepSuperseded` with Trust and Not now; the trust set read once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_two_trust_held_while_transportctl_revokes_reads_trust_step_superseded() {
    let world = two_daemons().await;
    let (peer, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;
    app.act(PeersInput::Add).await;
    assert_eq!(
        app.seen().blocks.first().map(|b| (&b.peer, b.state)),
        Some((&peer, BlockState::Undecided)),
        "the control: step 2 offered"
    );

    let commands = app
        .held_while(PeersInput::Trust(peer.clone()), || {
            transportctl_ok(&world.a, &["trust", "revoke", peer.as_str()], "");
        })
        .await;

    assert!(
        audited_superseded(&world, "admin.trust.set", peer.as_str()),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );
    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::BlockProblem(peer.clone())),
        "the line in the block"
    );
    assert_eq!(
        seen.blocks.first(),
        Some(&BlockView {
            peer: peer.clone(),
            state: BlockState::Undecided,
            problem: Some(BlockProblem::Superseded),
        }),
        "Trust and Not now offered, the line saying why"
    );
    let sets = |c: &Command| matches!(c, Command::SetTrust(t) if t.peer == peer);
    assert_eq!(count(&commands, sets), 1, "sent once: {commands:?}");
    assert_eq!(
        count(&commands, |c| matches!(c, Command::ReadTrust)),
        1,
        "the trust set read once: {commands:?}"
    );
    let trusted = transportctl_ok(&world.a, &["trust", "list"], "");
    assert!(
        !trusted.contains(peer.as_str()),
        "the peer is not in transportctl's trust list"
    );
}

/// A superseded Trust read back as trusted reads `TrustStepDone`: a
/// step-2 Trust held while `transportctl` allows the peer. Its one block
/// so decided, the page closes with `PeerTrusted`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_two_trust_held_while_transportctl_allows_reads_trust_step_done() {
    let world = two_daemons().await;
    let (peer, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;
    app.act(PeersInput::Add).await;

    let commands = app
        .held_while(PeersInput::Trust(peer.clone()), || {
            transportctl_ok(&world.a, &["trust", "allow", peer.as_str()], "");
        })
        .await;
    assert!(
        audited_superseded(&world, "admin.trust.set", peer.as_str()),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );

    // Read back as trusted, the one block is decided: the page closes
    // as after any Trust it decides (§5.3), saying the peer is trusted.
    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::Opener),
        "the page closed: the control that opened it"
    );
    assert!(
        !seen.peers.add_open(),
        "every block decided: the page closes"
    );
    assert_eq!(seen.peers.snack().0, Some(&PeersSnack::PeerTrusted));
    let sets = |c: &Command| matches!(c, Command::SetTrust(t) if t.peer == peer);
    assert_eq!(count(&commands, sets), 1, "sent once: {commands:?}");
    assert_eq!(
        count(&commands, |c| matches!(c, Command::ReadTrust)),
        1,
        "the trust set read once: {commands:?}"
    );
}

/// A superseded Undo read back listing the row restores it: an Undo held
/// while `transportctl` adds the address.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_undo_held_while_transportctl_adds_it_restores_the_row() {
    let world = two_daemons().await;
    let (_, address) = fresh_address();
    transportctl_ok(&world.a, &["peers", "add", &address], "");
    let mut app = App::on(&world).await;
    app.act(PeersInput::Remove(address.clone())).await;
    assert!(!page_lists(&app.seen(), &address), "the control: removed");

    let commands = app
        .held_while(PeersInput::Undo, || {
            transportctl_ok(&world.a, &["peers", "add", &address], "");
        })
        .await;
    assert!(
        audited_superseded(&world, "admin.peers.add_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );

    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::Row(address.clone())),
        "the restored row"
    );
    assert!(page_lists(&seen, &address), "the row back");
    assert_eq!(seen.peers.snack().0, None, "as Undo done: no snackbar");
    sent_once_read_once(&commands, add_of(&address));
    assert!(transportctl_lists(&world, &address), "listed");
}

/// With the read back also made to fail, a superseded Remove puts Peers
/// in its unreadable state.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_superseded_remove_whose_read_back_fails_leaves_peers_unreadable() {
    let world = two_daemons().await;
    let (_, address) = fresh_address();
    transportctl_ok(&world.a, &["peers", "add", &address], "");
    let mut app = App::on(&world).await;

    let commands = app
        .held_with(PeersInput::Remove(address.clone()), READ_FAILS, || {
            transportctl_ok(&world.a, &["peers", "remove", &address], "");
        })
        .await;
    assert!(
        audited_superseded(&world, "admin.peers.remove_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );

    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::ListProblem),
        "the problem line"
    );
    assert_eq!(seen.peers.list(), None, "no list claimed");
    assert!(seen.peers.list_problem().is_some(), "the unreadable state");
    assert_eq!(seen.peers.snack().0, None, "no snackbar");
    sent_once_read_once(&commands, removal_of(&address));
}

/// With the read back made to fail, a superseded add reads
/// `AddSuperseded`, never `AddUnknown`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_superseded_add_whose_read_back_fails_reads_add_superseded() {
    let world = two_daemons().await;
    let (_, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;

    let commands = app
        .held_with(PeersInput::Add, READ_FAILS, || {
            transportctl_ok(&world.a, &["peers", "remove", &address], "");
        })
        .await;
    assert!(
        audited_superseded(&world, "admin.peers.add_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );

    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::AddButton),
        "Add, the line under it"
    );
    assert_eq!(
        seen.peers.add_page_note(),
        Some(&AddNote::Superseded { lines: 1 })
    );
    assert_eq!(seen.peers.add_result(), None);
    assert_eq!(seen.peers.field(), address, "the field is kept");
    sent_once_read_once(&commands, add_of(&address));
}

/// With the read back made to fail, a superseded step-2 Trust reads
/// `TrustStepSupersededUnread`, never `TrustStepUnknown`, Trust and Not
/// now kept.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_superseded_trust_whose_read_back_fails_reads_superseded_unread() {
    let world = two_daemons().await;
    let (peer, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;
    app.act(PeersInput::Add).await;

    let commands = app
        .held_with(PeersInput::Trust(peer.clone()), READ_FAILS, || {
            transportctl_ok(&world.a, &["trust", "revoke", peer.as_str()], "");
        })
        .await;
    assert!(
        audited_superseded(&world, "admin.trust.set", peer.as_str()),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );

    let seen = app.seen();
    assert_eq!(
        seen.focus(),
        Some(PeersFocus::BlockProblem(peer.clone())),
        "the line in the block"
    );
    assert_eq!(
        seen.blocks.first(),
        Some(&BlockView {
            peer: peer.clone(),
            state: BlockState::Undecided,
            problem: Some(BlockProblem::SupersededUnread),
        })
    );
    let sets = |c: &Command| matches!(c, Command::SetTrust(t) if t.peer == peer);
    assert_eq!(count(&commands, sets), 1, "sent once: {commands:?}");
    assert_eq!(
        count(&commands, |c| matches!(c, Command::ReadTrust)),
        1,
        "the trust set read once, and it failed: {commands:?}"
    );
}

/// An add left unknown -- its answer lost after the daemon wrote it, its
/// read back failing -- then the same line sent again and refused
/// `superseded` while `transportctl` removes it: `AddSuperseded`, never
/// `AddUnknown`. The line stays unsettled unless that refusal's read
/// back succeeds (§5.7, "unsettled lines"), which the next add, answered
/// unchanged once `transportctl` has added the line, shows: unsettled,
/// `AddedChecked`, since the earlier write may be what stored it;
/// settled by the read, `AddedUnchanged`.
async fn a_superseded_add_of_an_unknown_line(read_back_fails: bool) {
    let world = two_daemons().await;
    let (_, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;

    let lost = Arming {
        fail_read_back: true,
        lose_answer: true,
    };
    let _ = app.held_with(PeersInput::Add, lost, || {}).await;
    assert_eq!(
        app.seen().peers.add_page_note(),
        Some(&AddNote::Unknown { lines: 1 }),
        "the control: the add left unknown"
    );
    assert!(
        transportctl_lists(&world, &address),
        "the control: the answer was lost, not the write"
    );

    let arming = Arming {
        fail_read_back: read_back_fails,
        lose_answer: false,
    };
    let commands = app
        .held_with(PeersInput::Add, arming, || {
            transportctl_ok(&world.a, &["peers", "remove", &address], "");
        })
        .await;
    assert!(
        audited_superseded(&world, "admin.peers.add_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );
    let seen = app.seen();
    assert_eq!(
        seen.peers.add_page_note(),
        Some(&AddNote::Superseded { lines: 1 }),
        "never AddUnknown, read back failing {read_back_fails}"
    );
    assert_eq!(seen.focus(), Some(PeersFocus::AddButton), "Add");
    assert_eq!(seen.peers.field(), address, "the field is kept");
    sent_once_read_once(&commands, add_of(&address));

    transportctl_ok(&world.a, &["peers", "add", &address], "");
    app.act(PeersInput::Add).await;
    let expected = if read_back_fails {
        AddResult::Checked { lines: 1 }
    } else {
        AddResult::Unchanged { lines: 1 }
    };
    assert_eq!(
        app.seen().peers.add_result(),
        Some(&expected),
        "read back failing {read_back_fails}"
    );
}

/// [`a_superseded_add_of_an_unknown_line`], its read back failing: the
/// line still unsettled.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_superseded_add_whose_read_back_fails_leaves_an_unknown_line_unsettled() {
    a_superseded_add_of_an_unknown_line(true).await;
}

/// [`a_superseded_add_of_an_unknown_line`], its read back succeeding: the
/// line settled, not added.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_superseded_add_read_back_settles_an_unknown_line() {
    a_superseded_add_of_an_unknown_line(false).await;
}

/// The step-2 block left `TrustStepSupersededUnread`, a Trust whose
/// trust set could not be read again.
async fn superseded_unread(world: &World, app: &mut App, peer: &TransportIdentity) {
    app.act(PeersInput::Add).await;
    let _ = app
        .held_with(PeersInput::Trust(peer.clone()), READ_FAILS, || {
            transportctl_ok(&world.a, &["trust", "revoke", peer.as_str()], "");
        })
        .await;
    assert_eq!(
        app.seen().blocks.first().and_then(|b| b.problem),
        Some(BlockProblem::SupersededUnread),
        "the control: left unread"
    );
}

/// Trust again on a `TrustStepSupersededUnread` block, writing nothing:
/// no Trust is pending after the refusal, so the answer says its own
/// class (`TrustUnavailable`), never `TrustStepUnknown`, focus on the
/// block's line (§5.3, §5.7). Nothing is written because A's daemon has
/// stopped, so the app's trust connection cannot open.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trust_again_after_a_superseded_unread_trust_writing_nothing_says_its_class() {
    let mut world = two_daemons().await;
    let (peer, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;
    superseded_unread(&world, &mut app, &peer).await;

    let _ = world.a_daemon.terminate().await;
    let from = app.commands.len();
    app.act(PeersInput::Trust(peer.clone())).await;

    let seen = app.seen();
    assert_eq!(
        seen.blocks.first(),
        Some(&BlockView {
            peer: peer.clone(),
            state: BlockState::Undecided,
            problem: Some(BlockProblem::Trust(TrustProblem::Unavailable)),
        })
    );
    assert_eq!(seen.focus(), Some(PeersFocus::BlockProblem(peer.clone())));
    let sets = |c: &Command| matches!(c, Command::SetTrust(t) if t.peer == peer);
    assert_eq!(count(&app.commands[from..], sets), 1, "{:?}", app.commands);
}

/// A `TrustStepSupersededUnread` block decided by a read of the trust
/// set the person did not ask for -- the trust settings opened --
/// started after the refusal (§5.3, §8): trusted when `transportctl`
/// allowed the peer meanwhile, `TrustStepSuperseded` when nothing did;
/// the page stays open, and focus is not moved by the read, the block's
/// new line taking the place of the one focus was on.
async fn a_superseded_unread_trust_decided_by_a_later_read(allowed: bool) {
    let world = two_daemons().await;
    let (peer, address) = fresh_address();
    let mut app = App::on(&world).await;
    typed(&mut app, &address).await;
    superseded_unread(&world, &mut app, &peer).await;
    let asked = app.seen().peers.focus();
    assert_eq!(asked.0, Some(PeersFocus::BlockProblem(peer.clone())));

    if allowed {
        transportctl_ok(&world.a, &["trust", "allow", peer.as_str()], "");
    }
    let from = app.commands.len();
    app.person
        .events
        .borrow_mut()
        .push_back(vec![ViewEvent::Trust(TrustInput::Opened)]);
    app.settle().await;
    assert_eq!(
        count(&app.commands[from..], |c| matches!(c, Command::ReadTrust)),
        1,
        "the control: the settings read the trust set: {:?}",
        app.commands
    );

    let seen = app.seen();
    assert!(seen.peers.add_open(), "the page stays open");
    assert_eq!(seen.peers.focus(), asked, "focus not moved by the read");
    let block = seen.blocks.first().expect("the block");
    if allowed {
        assert_eq!(
            (block.state, block.problem),
            (BlockState::Trusted, None),
            "decided trusted"
        );
        assert_eq!(
            seen.peers
                .focus_after_block_decided(&seen.trust, &peer, BlockHeld::Problem),
            Some(PeersFocus::BlockState(peer.clone())),
            "from its line, the block's new line in its place"
        );
    } else {
        assert_eq!(
            (block.state, block.problem),
            (BlockState::Undecided, Some(BlockProblem::Superseded)),
            "decided not trusted"
        );
    }
}

/// [`a_superseded_unread_trust_decided_by_a_later_read`], trusted.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_superseded_unread_trust_is_decided_trusted_by_a_later_read() {
    a_superseded_unread_trust_decided_by_a_later_read(true).await;
}

/// [`a_superseded_unread_trust_decided_by_a_later_read`], not trusted.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_superseded_unread_trust_is_decided_not_trusted_by_a_later_read() {
    a_superseded_unread_trust_decided_by_a_later_read(false).await;
}

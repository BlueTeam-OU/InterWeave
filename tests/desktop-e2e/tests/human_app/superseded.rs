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
//! still the daemon's.
//!
//! Acceptance 10's superseded cases NOT run here: a superseded add
//! carrying a line of an earlier `AddUnknown`; a Trust again after
//! `TrustStepSupersededUnread` answered `Internal`, showing
//! `TrustFailed`; a superseded Trust left unread then decided by a later
//! read of the trust set, and the focus and announcement rules of that
//! read; the two snackbars without an action staying 10 seconds, under
//! focus or the pointer, or for Android's accessibility timeout; and
//! focus after each outcome. `ui-model`'s and `ui-slint`'s
//! `tests/peers.rs` hold those on the model and the rendered views, not
//! against a daemon. Nor the window, or Android, whose facade is the
//! same crate in process.

use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
use std::future::Future;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use interweave_human_app_core::{Command, FacadeSide, ModelSide, Opener, Surface, Update};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{
    AddNote, AddResult, BlockProblem, BlockState, BlockView, Peers, PeersInput, PeersSnack,
    UiModel, ViewEvent,
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

/// One armed hold: who to tell the write is held, what releases it, and
/// whether the read after it fails.
type Armed = (oneshot::Sender<&'static str>, oneshot::Receiver<()>, bool);

/// The next write on an admin port [`Holding`] opened waits here once
/// armed; one write per arming. Armed to fail the read back, the first
/// read of the list or the trust set after that write, on any port the
/// binding opened, is answered `BackendUnavailable` without being sent:
/// the read back "made to fail" of acceptance 10, the write itself still
/// the daemon's.
#[derive(Clone, Default)]
struct Hold {
    armed: Arc<Mutex<Option<Armed>>>,
    read_fails: Arc<AtomicBool>,
}

impl Hold {
    /// Hold the next write: the receiver hears its method once it is
    /// held, and the sender lets it go.
    fn arm(&self, fail_read_back: bool) -> (oneshot::Receiver<&'static str>, oneshot::Sender<()>) {
        let (held, heard) = oneshot::channel();
        let (release, released) = oneshot::channel();
        *self.armed.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((held, released, fail_read_back));
        (heard, release)
    }

    /// Wait here if armed: after the port was opened, before `method` is
    /// sent on it.
    async fn wait(&self, method: &'static str) {
        let armed = self
            .armed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some((held, released, fail_read_back)) = armed {
            let _ = held.send(method);
            let _ = released.await;
            self.read_fails.store(fail_read_back, Ordering::SeqCst);
        }
    }

    /// Whether this read is the one made to fail; once.
    fn read_fails(&self) -> bool {
        self.read_fails.swap(false, Ordering::SeqCst)
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
        self.hold.wait("admin.peers.add_address").await;
        self.port.add_bootstrap_address(address).await
    }

    async fn add_bootstrap_addresses(
        &self,
        addresses: Vec<String>,
    ) -> Result<AddressesChange, TransportError> {
        self.hold.wait("admin.peers.add_addresses").await;
        self.port.add_bootstrap_addresses(addresses).await
    }

    async fn remove_bootstrap_address(
        &self,
        address: String,
    ) -> Result<AddressChange, TransportError> {
        self.hold.wait("admin.peers.remove_address").await;
        self.port.remove_bootstrap_address(address).await
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
        self.hold.wait("admin.trust.set").await;
        self.port.set_trust(peer, allowed).await
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
        self.held_with(input, false, meanwhile).await
    }

    /// [`Self::held_while`], the read back after the write made to fail
    /// when `fail_read_back`.
    async fn held_with(
        &mut self,
        input: PeersInput,
        fail_read_back: bool,
        meanwhile: impl FnOnce(),
    ) -> Vec<Command> {
        let from = self.commands.len();
        let (held, release) = self.hold.arm(fail_read_back);
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
    assert!(!trusted.contains(peer.as_str()), "not trusted: {trusted}");
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
        .held_with(PeersInput::Remove(address.clone()), true, || {
            transportctl_ok(&world.a, &["peers", "remove", &address], "");
        })
        .await;
    assert!(
        audited_superseded(&world, "admin.peers.remove_address", &address),
        "the daemon refused the app's held write superseded: {}",
        world.a_daemon.log()
    );

    let seen = app.seen();
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
        .held_with(PeersInput::Add, true, || {
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
        .held_with(PeersInput::Trust(peer.clone()), true, || {
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

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Plan §29 (e)'s two code cases against a real daemon, and what the
//! daemon holds afterwards read over its own admin port.
//!
//! - **A hostile code adds its addresses and nothing else.** The text a
//!   hostile party's code decodes to, for a peer nobody trusts, handed to
//!   the client as both shells hand a scan's text over
//!   (`PeersInput::Scanned`), through the client's own root halves --
//!   `app-core`'s model side and facade side over the human client's
//!   facade, on the daemon's IPC sockets: the daemon's address list gains
//!   exactly the code's lines, its trust set is unchanged, and no trust
//!   change was asked of it, by command or on the admin socket. The
//!   controls that each read is live: the trust set read before holds B,
//!   the list afterwards holds the code's lines, and the tap saw the add.
//!   The window is not in this case; the next one is.
//! - **A code's image pasted in the window adds its addresses and nothing
//!   else.** The shipped window, on Linux, reads the clipboard's image
//!   within bounds: text on the clipboard says `ClipboardNoImage` under
//!   the scan controls and adds nothing; a PNG of a hostile code, owned
//!   on the display's clipboard as another program would own it, pressed
//!   in with Paste an image, adds exactly its lines to the daemon's list,
//!   the trust set unchanged. The control: the list before holds none of
//!   them. Open an image… is not driven: it is the XDG portal's dialog,
//!   which this display has no backend for.
//! - **A hostile clipboard owner is given up at the deadline.** An owner
//!   that never answers says `ClipboardUnreadable`; one that announces an
//!   image in incremental chunks and keeps sending bytes says
//!   `ImageUnreadable`. Each within the client's five-second deadline and
//!   a margin, the page still usable: the next paste is read. What this
//!   does not prove: an owner that out-paces the client so its queue never
//!   runs dry, which this display's scheduling does not produce; the
//!   deadline's check before every queued event is `ui-slint`'s unit test
//!   (`a_queue_that_never_runs_dry_is_given_up_at_the_deadline`).
//! - **The shown code is this node's addresses and `PeerId` and nothing
//!   else.** Show my address's code in the shipped window, read from the
//!   window's pixels as a phone's camera reads a screen, decodes to
//!   exactly the lines the daemon answers for its own addresses, each
//!   ending in its `PeerId`, none loopback, unspecified or link-local. The
//!   control: the daemon listens on this host's private address, so the
//!   list is not empty.
//!
//! What this does not prove: the camera's path on Android (the same
//! model input takes its text; `ui-model`'s `peers.rs` holds a scan's
//! refusals), or that the daemon's floor refuses every class -- its own
//! tests do, and here the classes are only those this host's interfaces
//! offer.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use interweave_human_app_core::{Command, FacadeSide, ModelSide, Opener, Surface, Update};
use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{
    PeersInput, ScanControl, UiModel, UiText, ViewEvent, fill, placeholder_en,
};
use interweave_ipc_client::IpcBinding;
use interweave_local_client_api::{
    AddressSource, AdminBinding as _, AdminCapability, AdminPort as _, BootstrapAddress,
};
use interweave_transport_api::MAX_PAYLOAD_BYTES;

use crate::a11y::{Bus, Element};
use crate::clipboard;
use crate::common::{Home, PATIENCE, human};
use crate::daemon_link::{Tap, asked};
use crate::display;
use crate::harness::{self as app, until_lease};
use crate::world::two_daemons;

fn text(key: UiText) -> &'static str {
    placeholder_en::text(key)
}

/// What the daemon's trust policy allows, as strings, in its order.
async fn allowed(home: &Home) -> Vec<String> {
    let port = home
        .binding()
        .admin([AdminCapability::Trust].into())
        .await
        .expect("an admin port");
    let view = port.trust().await.expect("the trust view");
    view.peers().map(|p| p.as_str().to_owned()).collect()
}

/// Every bootstrap address the daemon holds, authored then added.
async fn addresses(home: &Home) -> Vec<BootstrapAddress> {
    let port = home
        .binding()
        .admin([AdminCapability::Peers].into())
        .await
        .expect("an admin port");
    port.bootstrap_addresses().await.expect("the address list")
}

/// A surface whose person acts from a queue the case fills.
#[derive(Clone, Default)]
struct Scripted(Rc<RefCell<VecDeque<Vec<ViewEvent>>>>);

impl Surface for Scripted {
    fn render(&mut self, _model: &UiModel) {}

    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        self.0.borrow_mut().pop_front().unwrap_or_default()
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

/// One pass of the root's loop, as the desktop root runs it: the
/// facade's turn, then each command the model side asks for, its updates
/// applied; `now` the milliseconds since `start`, the monotonic clock the
/// facade was started at 0 on. Whether the model side asked for anything.
async fn pump(
    model: &mut ModelSide<Scripted, Nothing>,
    facade: &mut FacadeSide<IpcBinding, IpcBinding>,
    commands: &mut Vec<Command>,
    start: Instant,
) -> bool {
    let now = || u64::try_from(start.elapsed().as_millis()).expect("ms");
    for update in facade.turn(now()).await {
        model.apply(update);
    }
    let asked = model.turn();
    let any = !asked.is_empty();
    for command in asked {
        commands.push(command.clone());
        for update in facade.execute(command, now()).await {
            model.apply(update);
        }
    }
    any
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hostile_code_scanned_adds_its_addresses_and_nothing_else_the_trust_set_unchanged() {
    let world = two_daemons().await;
    let hostile = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    let lines = vec![
        format!("/ip4/192.168.1.20/tcp/4001/p2p/{hostile}"),
        format!("/ip4/10.0.0.7/tcp/4001/p2p/{hostile}"),
    ];

    let trusted = allowed(&world.a).await;
    assert!(
        trusted.iter().any(|p| p == world.b_peer.as_str()),
        "the control: the trust read is live and holds B: {trusted:?}"
    );
    let before = addresses(&world.a).await;
    assert!(
        !before.iter().any(|r| r.address.contains(&hostile)),
        "{before:?}"
    );

    // The client's root halves over the human client's facade, on A's
    // sockets, as the desktop window composes them.
    let admin = Tap::install(&world.a.admin_socket());
    let store_dir = tempfile::tempdir().expect("a store directory");
    let store = HumanStore::open(
        &store_dir.path().join("state/human.sqlite"),
        StoreOptions::default(),
    )
    .expect("a store");
    let start = Instant::now();
    let client = TransportClient::new(
        world.a.binding(),
        world.a.binding(),
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
    model.apply(Update::Listed(facade.listing().expect("a listing")));
    let mut commands = Vec::new();
    for _ in 0..4 {
        pump(&mut model, &mut facade, &mut commands, start).await;
    }

    // The person opens the add page and scans the hostile code.
    person.0.borrow_mut().extend([
        vec![ViewEvent::Peers(PeersInput::OpenAdd)],
        vec![ViewEvent::Peers(PeersInput::ScanPressed(
            ScanControl::Camera,
        ))],
        vec![ViewEvent::Peers(PeersInput::Scanned(lines.join("\n")))],
    ]);
    let deadline = Instant::now() + PATIENCE;
    loop {
        pump(&mut model, &mut facade, &mut commands, start).await;
        let now = addresses(&world.a).await;
        if now.len() > before.len() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the scanned code was never added: {now:?}; commands {commands:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // The model side acts on the add's answer in the turns after it (a
    // re-read, a next step): run the loop until it has asked for nothing
    // three passes running, so a command the answer leads to is made
    // before what the daemon holds is read.
    let mut quiet = 0;
    while quiet < 3 {
        if pump(&mut model, &mut facade, &mut commands, start).await {
            quiet = 0;
        } else {
            quiet += 1;
        }
        assert!(
            Instant::now() < deadline,
            "the model side never settled after the add: {commands:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let after = addresses(&world.a).await;
    let mut want = before.clone();
    want.extend(lines.iter().map(|line| BootstrapAddress {
        address: line.clone(),
        source: AddressSource::Added,
        dialled: true,
    }));
    assert_eq!(after, want, "its addresses and nothing else");
    assert_eq!(
        allowed(&world.a).await,
        trusted,
        "the trust set unchanged: {hostile} not added to it"
    );
    assert!(
        commands.contains(&Command::AddAddresses(lines.clone())),
        "the control: the scan was one add: {commands:?}"
    );
    assert!(
        !commands.iter().any(|c| matches!(c, Command::SetTrust(_))),
        "no trust change commanded: {commands:?}"
    );
    let methods = asked(&admin.from(i32::try_from(std::process::id()).expect("a pid"))).methods;
    assert!(
        methods.contains("admin.peers.add_addresses"),
        "the control: the tap saw the add: {methods:?}"
    );
    assert!(
        !methods.contains("admin.trust.set"),
        "no trust change asked: {methods:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_shown_code_reads_back_as_this_nodes_own_addresses_and_peer_id_alone() {
    let _focus = display::exclusive().await;
    let world = two_daemons().await;
    let own = world
        .a
        .binding()
        .admin([AdminCapability::Peers].into())
        .await
        .expect("an admin port")
        .own_addresses()
        .await
        .expect("this node's own addresses");
    assert_eq!(own.peer, world.a_peer);
    let lines: Vec<String> = own.addresses.iter().map(|r| r.address.clone()).collect();
    assert!(
        !lines.is_empty(),
        "the control: A listens on a private address"
    );

    let bus = Bus::connect().await;
    let mut client = app::start(&world.a);
    until_lease(&world.a.binding(), true, &client).await;
    let window = bus.window_of(client.child.id(), || client.log()).await;
    display::focus(client.child.id(), || client.log()).await;
    let log = || client.log();
    let press = |name: &'static str| move |e: &Element| e.name == name && e.actions == ["click"];
    for (what, name) in [
        ("the app's options", text(UiText::AppOptions)),
        ("the app's Peers entry", text(UiText::PeersTitle)),
        ("Show my address", text(UiText::ShowMyAddress)),
    ] {
        let control = window.until(what, press(name), log).await;
        window.activate(&control, "click").await;
    }
    let label = if lines.len() == 1 {
        text(UiText::QrLabelOne).to_owned()
    } else {
        fill(
            text(UiText::QrLabelOther),
            &[("n", &lines.len().to_string())],
        )
    };
    window
        .until("the code, labelled", |e: &Element| e.name == label, log)
        .await;

    // Read as a camera reads the screen: the window's pixels, until a
    // frame holds the code (the label is set in the render that draws it,
    // and the frame reaches the display a moment later).
    let pid = client.child.id();
    let deadline = Instant::now() + PATIENCE;
    let read = loop {
        let (width, height, luma) = display::luma_of(pid);
        let mut prepared =
            rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| luma[y * width + x]);
        let decoded: Vec<String> = prepared
            .detect_grids()
            .into_iter()
            .filter_map(|grid| grid.decode().ok().map(|(_, text)| text))
            .collect();
        if !decoded.is_empty() {
            break decoded;
        }
        assert!(
            Instant::now() < deadline,
            "no code in the window's pixels ({width}x{height}):\n{}",
            client.log()
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    assert_eq!(read.len(), 1, "one code on the screen: {read:?}");
    let shown: Vec<&str> = read[0].lines().collect();
    assert_eq!(
        shown, lines,
        "exactly the daemon's own addresses, in its order"
    );
    let suffix = format!("/p2p/{}", world.a_peer.as_str());
    for line in &shown {
        assert!(line.ends_with(&suffix), "{line} names this node's PeerId");
        for refused in [
            "/ip4/127.",
            "/ip4/0.0.0.0/",
            "/ip4/169.254.",
            "/ip6/::1/",
            "/ip6/::/",
            "/ip6/fe80",
        ] {
            assert!(!line.starts_with(refused), "{line} is of a refused class");
        }
    }
    assert!(client.terminate().success(), "{}", client.log());
}

/// A PNG of the code holding `text`, four pixels a module with its quiet
/// zone, as a screenshot of another device's Show would be.
fn code_png(text: &str) -> Vec<u8> {
    const SCALE: u32 = 4;
    const QUIET: u32 = 4;
    let code = qrcode::QrCode::new(text).expect("a code holds it");
    let modules = u32::try_from(code.width()).expect("a side");
    let colors = code.to_colors();
    let side = (modules + 2 * QUIET) * SCALE;
    let picture = image::GrayImage::from_fn(side, side, |x, y| {
        let (mx, my) = (x / SCALE, y / SCALE);
        let dark = (QUIET..QUIET + modules).contains(&mx)
            && (QUIET..QUIET + modules).contains(&my)
            && colors[((my - QUIET) * modules + (mx - QUIET)) as usize] == qrcode::Color::Dark;
        image::Luma([if dark { 0 } else { 255 }])
    });
    let mut png = Vec::new();
    picture
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("encoded");
    png
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_code_image_pasted_in_the_window_adds_its_addresses_and_nothing_else() {
    // The display's focus, and its one clipboard, for this case alone.
    let _focus = display::exclusive().await;
    let world = two_daemons().await;
    let hostile = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    let lines = [
        format!("/ip4/192.168.1.21/tcp/4001/p2p/{hostile}"),
        format!("/ip4/10.0.0.8/tcp/4001/p2p/{hostile}"),
    ];
    let trusted = allowed(&world.a).await;
    let before = addresses(&world.a).await;
    assert!(
        !before.iter().any(|r| r.address.contains(&hostile)),
        "the control: none of the code's lines yet: {before:?}"
    );

    let bus = Bus::connect().await;
    let mut client = app::start(&world.a);
    until_lease(&world.a.binding(), true, &client).await;
    let window = bus.window_of(client.child.id(), || client.log()).await;
    display::focus(client.child.id(), || client.log()).await;
    let log = || client.log();
    let press = |name: &'static str| move |e: &Element| e.name == name && e.actions == ["click"];
    for (what, name) in [
        ("the app's options", text(UiText::AppOptions)),
        ("the app's Peers entry", text(UiText::PeersTitle)),
        ("Add an address", text(UiText::AddAddress)),
    ] {
        let control = window.until(what, press(name), log).await;
        window.activate(&control, "click").await;
    }
    let paste = window
        .until("Paste an image", press(text(UiText::PasteCodeImage)), log)
        .await;

    // Text only: no image, said under the scan controls; nothing added.
    let words = clipboard::Owner::hold(
        "text/plain",
        clipboard::Holding::Whole(lines.join("\n").into_bytes()),
    );
    window.activate(&paste, "click").await;
    window
        .until(
            "the no-image line",
            |e: &Element| e.name == text(UiText::ClipboardNoImage),
            log,
        )
        .await;
    drop(words);
    assert_eq!(addresses(&world.a).await, before, "text pastes no add");

    // The code's image: added at once, as one add.
    let _image = clipboard::Owner::hold(
        "image/png",
        clipboard::Holding::Whole(code_png(&lines.join("\n"))),
    );
    window.activate(&paste, "click").await;
    let deadline = Instant::now() + PATIENCE;
    let after = loop {
        let now = addresses(&world.a).await;
        if now.len() >= before.len() + lines.len() {
            break now;
        }
        assert!(
            Instant::now() < deadline,
            "the pasted code was never added: {now:?}\n{}",
            client.log()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    let mut want = before.clone();
    want.extend(lines.iter().map(|line| BootstrapAddress {
        address: line.clone(),
        source: AddressSource::Added,
        dialled: true,
    }));
    assert_eq!(after, want, "its addresses and nothing else");
    assert_eq!(allowed(&world.a).await, trusted, "the trust set unchanged");
    assert!(client.terminate().success(), "{}", client.log());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hostile_clipboard_owner_is_given_up_at_the_deadline() {
    // The client's deadline is five seconds from the press. The margin is
    // the window's render and the screen reader's read of the line, on a
    // loaded host; an owner not given up runs to the byte bound, minutes
    // at a byte a round trip, so the margin cannot hide that.
    const IN_TIME: Duration = Duration::from_secs(25);
    // ui-slint's code::MAX_IMAGE_BYTES, which no dependency of this suite
    // exports. Were it lowered by 4 bytes or more, or raised by a chunk
    // (128 KiB) or more, the count below fails and says so; a smaller
    // change passes, since the client's reads are in 4-byte units.
    const BOUND: usize = 32 * 1024 * 1024;
    let _focus = display::exclusive().await;
    let world = two_daemons().await;
    let bus = Bus::connect().await;
    let mut client = app::start(&world.a);
    until_lease(&world.a.binding(), true, &client).await;
    let window = bus.window_of(client.child.id(), || client.log()).await;
    display::focus(client.child.id(), || client.log()).await;
    let log = || client.log();
    let press = |name: &'static str| move |e: &Element| e.name == name && e.actions == ["click"];
    for (what, name) in [
        ("the app's options", text(UiText::AppOptions)),
        ("the app's Peers entry", text(UiText::PeersTitle)),
        ("Add an address", text(UiText::AddAddress)),
    ] {
        let control = window.until(what, press(name), log).await;
        window.activate(&control, "click").await;
    }
    let paste = window
        .until("Paste an image", press(text(UiText::PasteCodeImage)), log)
        .await;

    for (holding, what, line) in [
        (
            clipboard::Holding::Silent,
            "an owner that never answers",
            text(UiText::ClipboardUnreadable),
        ),
        (
            clipboard::Holding::Endless,
            "an owner that keeps sending",
            text(UiText::ImageUnreadable),
        ),
    ] {
        let owner = clipboard::Owner::hold("image/png", holding);
        let start = Instant::now();
        paste_until_said(&window, &paste, what, line, log).await;
        let took = start.elapsed();
        assert!(took < IN_TIME, "{what}: given up after {took:?}");
        drop(owner);
    }

    // Text only between the two lines alike, so each press is seen to
    // end with its own.
    let words = clipboard::Owner::hold(
        "text/plain",
        clipboard::Holding::Whole(b"not an image".to_vec()),
    );
    paste_until_said(
        &window,
        &paste,
        "text only",
        text(UiText::ClipboardNoImage),
        log,
    )
    .await;
    drop(words);

    // An image past the byte bound, sent incrementally as the protocol
    // asks: refused, and the client stops taking chunks at the bound.
    // What it says cannot show the bound (an image past it is refused by
    // the decoder too); what it took from the owner can.
    let big = clipboard::Owner::hold(
        "image/png",
        clipboard::Holding::Chunked { total: 2 * BOUND },
    );
    paste_until_said(
        &window,
        &paste,
        "an image past the bound",
        text(UiText::ImageUnreadable),
        log,
    )
    .await;
    // Up to the bound and not a chunk past it: the bound is a whole number
    // of the owner's chunks, so a client held to it deletes exactly those
    // and refuses the next unread, while one that reads the next chunk
    // whole deletes it too. The count is read once the owner has seen the
    // client's window destroyed, after every deletion it made; the
    // destruction itself adds nothing (measured: the count was the same
    // two seconds later).
    let took = big.taken_when_gone().await;
    assert_eq!(took, BOUND, "took {took} bytes of {}", 2 * BOUND);
    drop(big);

    // The page is still usable: the next paste is read.
    let lines = [format!(
        "/ip4/10.0.0.9/tcp/4001/p2p/{}",
        world.b_peer.as_str()
    )];
    let before = addresses(&world.a).await;
    let _image = clipboard::Owner::hold(
        "image/png",
        clipboard::Holding::Whole(code_png(&lines.join("\n"))),
    );
    window.activate(&paste, "click").await;
    let deadline = Instant::now() + PATIENCE;
    while addresses(&world.a).await.len() <= before.len() {
        assert!(
            Instant::now() < deadline,
            "the paste after the hostile owners was never added\n{}",
            client.log()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(client.terminate().success(), "{}", client.log());
}

/// Press Paste an image and wait for its scan to end saying `line`. A
/// press while a scan is open changes nothing, so the next press waits
/// for this; and `line` must not be showing already, or a line a scan
/// before left would pass for this one's.
async fn paste_until_said(
    window: &crate::a11y::Window<'_>,
    paste: &Element,
    what: &str,
    line: &str,
    log: impl Fn() -> String + Copy,
) {
    let tree = window.read().await;
    assert!(
        !tree.iter().any(|e| e.name == line),
        "{what}: its line shows before the press\n{}",
        crate::a11y::describe(&tree)
    );
    window.activate(paste, "click").await;
    window.until(what, |e: &Element| e.name == line, log).await;
}

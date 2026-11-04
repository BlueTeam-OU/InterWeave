// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the windowed start and a conversation's paging cost on a backlog
//! (plan section 29's open item; SPIKE-011's entry, A 2026-10-31): time,
//! and the heap's peak above where each step began, counted by this
//! binary's allocator, on an on-disk store of one conversation's unread
//! rows of 512 bytes each.
//!
//! `tests/window.rs` counts rows; this counts bytes and milliseconds.
//! What it does NOT prove: the phone's figures (the device run's), a
//! cipher store's (SPIKE-011 measured the cipher against the plaintext),
//! SQLite's page cache, which it allocates through this allocator and so
//! is counted, or the view's drawing.

#![allow(clippy::expect_used, clippy::panic, clippy::print_stderr)]
#![allow(
    unsafe_code,
    reason = "a counting wrapper around the system allocator, for this test binary alone"
)]

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use interweave_human_app_core::{
    Command, FacadeSide, ModelSide, Opener, Surface, Update, WindowCommand,
};
use interweave_human_store::{
    AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_human_ui_model::{
    ConversationKey, Edge, UiModel, ViewEvent, WindowEvent, WindowInput,
};
use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, MediaType, TransportIdentity};

/// Bytes the heap holds now, and the most it has held since [`Peak::from_here`].
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

struct Counting;

// SAFETY: every call is forwarded to `System` with the caller's own
// arguments, so `System`'s contract is the caller's; the counters are
// atomics and allocate nothing.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's layout, as `GlobalAlloc::alloc` requires.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            grew(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as `alloc`.
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            grew(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator, so from `System`, with
        // `layout`.
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: as `dealloc`, and `new_size` is the caller's.
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
            grew(new_size);
        }
        p
    }
}

fn grew(size: usize) {
    let now = CURRENT.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(now, Ordering::Relaxed);
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// One step's cost: its time and the heap's peak above where it began.
struct Peak {
    base: usize,
    began: Instant,
}

impl Peak {
    fn from_here() -> Self {
        let base = CURRENT.load(Ordering::Relaxed);
        PEAK.store(base, Ordering::Relaxed);
        Self {
            base,
            began: Instant::now(),
        }
    }

    /// The step's milliseconds and peak bytes.
    fn end(self) -> (f64, usize) {
        (
            self.began.elapsed().as_secs_f64() * 1000.0,
            PEAK.load(Ordering::Relaxed).saturating_sub(self.base),
        )
    }
}

/// The counters are the process's: one test at a time measures, whatever
/// `--test-threads` says. Held across the test's awaits on purpose: each
/// test runs on a runtime of its own, so only another test waits on it.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

const WALL: u64 = 1_786_600_000_000;
const MEDIA: &str = "application/vnd.interweave-human-chat+json;v=2";
/// A row's payload, as SPIKE-011's harness sized its rows.
const ROW_BYTES: usize = 512;

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn node() -> FakeConfig {
    FakeConfig {
        peer: peer(),
        endpoints: vec![FakeEndpoint::open(human(), false)],
        default_endpoint: Some(human()),
        queue_bound: 16,
    }
}

fn facade(node: &FakeNode, store: HumanStore) -> FacadeSide<FakeNode, FakeNode> {
    let client = TransportClient::new(
        node.clone(),
        node.clone(),
        store,
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: vec![human()],
            channels: vec![],
            max_payload_bytes: 49_152,
        },
        Box::new(|| WALL),
        0,
    )
    .expect("facade");
    FacadeSide::new(client, || WALL)
}

/// Message `n` from `from`: a text envelope of [`ROW_BYTES`], classified.
fn message(n: u32, from: &TransportIdentity) -> NewInbound {
    let id = format!("{n:032x}");
    let head = format!(r#"{{"v":2,"kind":"text","app_message_id":"{id}","text":"message {n} "#);
    let fill = ROW_BYTES - head.len() - 2;
    NewInbound {
        app_message_id: AppMessageId::parse(id).expect("an id"),
        origin: InboundOrigin {
            peer: from.clone(),
            endpoint: Some(human()),
            channel: None,
        },
        media_type: Some(MediaType::parse(MEDIA).expect("media")),
        payload: format!(r#"{head}{}"}}"#, "x".repeat(fill)).into_bytes(),
        received_at: u64::from(n),
        kind: InboundKind::Text,
    }
}

#[derive(Default)]
struct Blind;

impl Surface for Blind {
    fn render(&mut self, _model: &UiModel) {}
    fn take_events(&mut self, _model: &UiModel) -> Vec<ViewEvent> {
        Vec::new()
    }
    fn take_window(&mut self) -> Vec<WindowEvent> {
        Vec::new()
    }
}

struct NoOpener;

impl Opener for NoOpener {
    fn open(&mut self, _destination: &str) {}
    fn ask_network_access(&mut self) {}
    fn authenticate(&mut self) {}
}

/// What a backlog cost: each step's milliseconds and peak heap bytes.
#[derive(Debug)]
struct Cost {
    rows: u32,
    start: (f64, usize),
    first_page: (f64, usize),
    walk: (f64, usize),
    pages: usize,
    held: usize,
    /// Whether the walk ended holding the conversation's oldest row: a
    /// walk can end at the model's own bounds first.
    reached_oldest: bool,
}

/// The model side's turn, every command carried out; how many of them
/// asked for a page.
async fn turn(
    side: &mut FacadeSide<FakeNode, FakeNode>,
    model: &mut ModelSide<Blind, NoOpener>,
) -> usize {
    let mut pages = 0;
    for command in model.turn() {
        if matches!(command, Command::Window(WindowCommand::FetchPage(_))) {
            pages += 1;
        }
        for update in side.execute(command, 1).await {
            model.apply(update);
        }
    }
    pages
}

/// `rows` unread rows of one conversation, then: a start over them, the
/// conversation opened (its first page), and its top edge reached until
/// the model asks no more pages -- at the conversation's oldest row, or
/// at its own bounds first.
async fn cost(rows: u32) -> Cost {
    let dir = common::private_tempdir();
    let path = dir.path().join("state").join("human.sqlite3");
    let from = peer();
    {
        let mut store = HumanStore::open(&path, StoreOptions::default()).expect("store");
        for n in 1..=rows {
            store
                .commit_unread_inbound(&message(n, &from))
                .expect("committed");
        }
    }
    let (a, _b) = FakeNetwork::pair(node(), node());
    // The kinds are this build's: the first start's one-off classification
    // walk is out of what is measured. A first start pays it once.
    drop(
        facade(
            &a,
            HumanStore::open(&path, StoreOptions::default()).expect("store"),
        )
        .start()
        .expect("started"),
    );

    let mut side = facade(
        &a,
        HumanStore::open(&path, StoreOptions::default()).expect("store"),
    );
    let mut model = ModelSide::new(Blind, NoOpener);
    let step = Peak::from_here();
    let started = side.start().expect("started");
    model.apply(Update::Started(Box::new(started)));
    let start = step.end();

    let key = ConversationKey::Direct {
        peer: from,
        endpoint: Some(human()),
    };
    let step = Peak::from_here();
    model.window(WindowInput::Opened(key.clone()));
    let mut pages = turn(&mut side, &mut model).await;
    let first_page = step.end();
    assert_eq!(pages, 1, "opening asks one page");

    let step = Peak::from_here();
    // Every page is 64 rows at most: a walk asking more than twice what
    // the backlog needs does not end, and fails rather than hanging.
    let bound = usize::try_from(rows).expect("small") / 32 + 8;
    loop {
        model.window(WindowInput::Reached {
            conversation: key.clone(),
            edge: Edge::Top,
        });
        let asked = turn(&mut side, &mut model).await;
        model.window(WindowInput::Left {
            conversation: key.clone(),
            edge: Edge::Top,
        });
        if asked == 0 {
            break;
        }
        pages += asked;
        assert!(pages <= bound, "the walk does not end: {pages} pages");
    }
    let walk = step.end();
    Cost {
        rows,
        start,
        first_page,
        walk,
        pages,
        held: model.model().messages(&key).len(),
        reached_oldest: model
            .model()
            .messages(&key)
            .iter()
            .any(|m| m.source.starts_with("message 1 ")),
    }
}

fn say(cost: &Cost) {
    let mib = |b: usize| f64::from(u32::try_from(b / 1024).unwrap_or(u32::MAX)) / 1024.0;
    eprintln!(
        "window cost at {} rows: start {:.1} ms / {:.2} MiB peak; first page {:.1} ms / {:.2} MiB; \
         walk until no page is asked {:.1} ms / {:.2} MiB over {} pages, {} rows held at the end, the oldest {}",
        cost.rows,
        cost.start.0,
        mib(cost.start.1),
        cost.first_page.0,
        mib(cost.first_page.1),
        cost.walk.0,
        mib(cost.walk.1),
        cost.pages,
        cost.held,
        if cost.reached_oldest {
            "among them"
        } else {
            "not reached"
        },
    );
}

/// What the larger backlog's start and first page may hold above the
/// smaller's: about one page's measured peak. A cost the same at both
/// sizes cancels between them, so what this bounds is growth with the
/// backlog, and reading a conversation whole is several times it.
const SLACK: usize = 256 * 1024;

/// The start and a conversation's first page cost what the window holds
/// and not what the store does: at ten times the backlog their peak heap
/// stays within a fixed slack of the smaller one's. The walk's figures are
/// printed (`--nocapture`) and bound nothing. Mutation: the start reading
/// every row of a conversation (`summaries` paging to the end) makes the
/// start's peak grow with the backlog.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn the_start_and_the_first_page_do_not_grow_with_the_backlog() {
    let _one = one_at_a_time();
    let small = cost(200).await;
    let large = cost(2_000).await;
    say(&small);
    say(&large);
    assert!(
        large.start.1 <= small.start.1 + SLACK,
        "the start's peak grew with the backlog: {small:?} against {large:?}"
    );
    assert!(
        large.first_page.1 <= small.first_page.1 + SLACK,
        "the first page's peak grew with the backlog: {small:?} against {large:?}"
    );
}

/// The figure for SPIKE-011's entry: a backlog of 20 000 rows. Run with
/// `cargo test --release -p interweave-human-app-core --test window_cost
/// -- --ignored --nocapture`; the figures are the printed line.
///
/// Measured 2026-11-04 on the desktop host (Core Ultra 7 155H, ext4),
/// release, one run: the start 21.0 ms and 0.02 MiB of peak heap, the
/// first page 5.1 ms and 0.23 MiB, and the walk 20.2 ms and 3.15 MiB over
/// 16 pages, ending at 1024 rows held with the oldest not reached -- the
/// model's held-row ceiling (`HELD_ROW_CAP`) and not the backlog bounds
/// what an unread conversation pages in. At 2000 rows, in the debug build
/// the other test runs in, the start's peak was the same 0.02 MiB.
#[tokio::test]
#[ignore = "a 20 000-row store takes over a minute to seed: the figure for SPIKE-011, run by hand"]
#[allow(clippy::await_holding_lock)]
async fn the_window_s_cost_on_a_backlog_of_twenty_thousand_rows() {
    let _one = one_at_a_time();
    let cost = cost(20_000).await;
    say(&cost);
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the windowed start and a conversation's paging cost on a backlog
//! (plan section 29's open item; SPIKE-011's entry, A 2026-10-31): time,
//! and two heaps' peaks above where each step began -- Rust's, counted by
//! this binary's allocator, and SQLite's own, which allocates with libc
//! and not through it, as SQLite counts it -- on an on-disk store of one
//! conversation's unread rows of 512 bytes each.
//!
//! `tests/window.rs` counts rows; this counts bytes and milliseconds.
//! What it does NOT prove: the phone's figures (the device run's), a
//! cipher store's (SPIKE-011 measured the cipher against the plaintext),
//! OpenSSL's heap, which neither counter sees, or the view's drawing.

#![allow(clippy::expect_used, clippy::panic, clippy::print_stderr)]
#![allow(
    unsafe_code,
    reason = "a counting wrapper around the system allocator, and SQLite's memory counters, for this test binary alone"
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

/// Bytes the Rust heap holds now, and the most it has held since
/// [`Peak::from_here`].
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

/// SQLite's own heap now (`sqlite3_memory_used`), and its peak since the
/// last reset (`sqlite3_memory_highwater`), which `reset` starts again
/// from now. The bundled build keeps SQLite's default memory statistics on.
fn sqlite_heap(reset: bool) -> (usize, usize) {
    // SAFETY: both are SQLite's thread-safe statistics calls, which take
    // and return plain integers.
    let (used, peak) = unsafe {
        (
            rusqlite::ffi::sqlite3_memory_used(),
            rusqlite::ffi::sqlite3_memory_highwater(i32::from(reset)),
        )
    };
    let bytes = |n: i64| usize::try_from(n).unwrap_or(0);
    (bytes(used), bytes(peak))
}

/// One step's cost: its milliseconds, and each heap's peak bytes above
/// where it began.
#[derive(Debug, Clone, Copy)]
struct Step {
    ms: f64,
    rust: usize,
    sqlite: usize,
}

/// A step being measured.
struct Peak {
    rust: usize,
    sqlite: usize,
    began: Instant,
}

impl Peak {
    fn from_here() -> Self {
        let rust = CURRENT.load(Ordering::Relaxed);
        PEAK.store(rust, Ordering::Relaxed);
        let (sqlite, _) = sqlite_heap(true);
        Self {
            rust,
            sqlite,
            began: Instant::now(),
        }
    }

    fn end(self) -> Step {
        Step {
            ms: self.began.elapsed().as_secs_f64() * 1000.0,
            rust: PEAK.load(Ordering::Relaxed).saturating_sub(self.rust),
            sqlite: sqlite_heap(false).1.saturating_sub(self.sqlite),
        }
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

/// What a backlog cost, step by step.
#[derive(Debug)]
struct Cost {
    rows: u32,
    start: Step,
    first_page: Step,
    walk: Step,
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
    let step = |s: &Step| {
        let mib = |b: usize| f64::from(u32::try_from(b / 1024).unwrap_or(u32::MAX)) / 1024.0;
        format!(
            "{:.1} ms / {:.2} MiB Rust + {:.2} MiB SQLite peak",
            s.ms,
            mib(s.rust),
            mib(s.sqlite)
        )
    };
    eprintln!(
        "window cost at {} rows: start {}; first page {}; walk until no page is asked {} \
         over {} pages, {} rows held at the end, the oldest {}",
        cost.rows,
        step(&cost.start),
        step(&cost.first_page),
        step(&cost.walk),
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
/// smaller's: about one page's measured Rust peak. A cost the same at both
/// sizes cancels between them, so what this bounds is growth with the
/// backlog, and reading a conversation whole is several times it.
const SLACK: usize = 256 * 1024;

/// The start and a conversation's first page cost what the window holds
/// in Rust's heap, and not what the store does: at ten times the backlog
/// their Rust peak, and the first page's SQLite peak, stay within a fixed
/// slack of the smaller one's. The start's SQLite peak does grow -- its
/// counts read every index page, into SQLite's page cache, whose ceiling
/// (`cache_size`, SQLite's default here) bounds it -- so it is printed and
/// bounds nothing, as are the walk's figures (`--nocapture`). Mutation: the
/// start reading every row of a conversation (`summaries` paging to the
/// end) makes the start's Rust peak grow with the backlog.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn the_start_and_the_first_page_do_not_grow_with_the_backlog() {
    let _one = one_at_a_time();
    let small = cost(200).await;
    let large = cost(2_000).await;
    say(&small);
    say(&large);
    for (what, small, large) in [
        ("the start", small.start, large.start),
        ("the first page", small.first_page, large.first_page),
    ] {
        assert!(
            large.rust <= small.rust + SLACK,
            "{what}'s Rust peak grew with the backlog: {small:?} against {large:?}"
        );
    }
    assert!(
        large.first_page.sqlite <= small.first_page.sqlite + SLACK,
        "the first page's SQLite peak grew with the backlog: {small:?} against {large:?}"
    );
}

/// The figure for SPIKE-011's entry: a backlog of 20 000 rows. Run with
/// `cargo test --release -p interweave-human-app-core --test window_cost
/// -- --ignored --nocapture`; the figures are the printed line.
///
/// Measured 2026-11-04 on the desktop host (Core Ultra 7 155H, ext4),
/// release, one run, peaks as Rust's heap + SQLite's: the start 31.9 ms
/// and 0.02 + 3.53 MiB; the first page 2.4 ms and 0.23 + 0.02 MiB; the
/// walk 34.8 ms and 3.14 + 0.02 MiB over 16 pages, ending at 1024 rows
/// held with the oldest not reached -- the model's held-row ceiling
/// (`HELD_ROW_CAP`) and not the backlog bounds what an unread
/// conversation pages in. The start's SQLite peak at 200, 2000 and 20 000
/// rows, debug, debug and release: 0.10, 0.42 and 3.53 MiB, its counts'
/// index pages in the page cache; past 20 000 not measured.
#[tokio::test]
#[ignore = "a 20 000-row store takes over a minute to seed: the figure for SPIKE-011, run by hand"]
#[allow(clippy::await_holding_lock)]
async fn the_window_s_cost_on_a_backlog_of_twenty_thousand_rows() {
    let _one = one_at_a_time();
    let cost = cost(20_000).await;
    say(&cost);
}

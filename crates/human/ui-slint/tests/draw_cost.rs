// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What a long conversation costs to draw, in a real window: rows made by
//! a draw cost that draw their own layout, not one of every row already
//! held (j77). The desktop's window and software renderer, drawn by
//! `take_snapshot` as a frame is: the list laid out, what is in view
//! drawn. The testing backend lays nothing out unless asked, and asking
//! by element visits every row.
//!
//! The figures are compared within one test, never against a constant:
//! two pages drawn above the same held rows, or a page against a frame
//! drawn with nothing changed, so a slower machine or a loaded one moves
//! both. Each is the drawing thread's own CPU time where the platform
//! says it (Linux), so other work on the host moves it less. One test in
//! this binary, since a process holds one window's event loop. What this
//! cannot prove: the cost on a phone, or with a screen reader reading the
//! tree.

#![cfg(feature = "desktop")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::Origin;
use interweave_human_core::RowId;
use interweave_human_ui_model::preview::cut;
use interweave_human_ui_model::{
    ConversationKey, Edge, ListedInbound, NewestRow, PageEnd, PageRows, RuntimeHost, StoredSummary,
    Table, UiModel, WindowInput,
};
use interweave_human_ui_slint::View;
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::ComponentHandle as _;

fn human() -> EndpointId {
    EndpointId::parse("human").expect("valid")
}

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

fn direct(peer: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(human()),
    }
}

fn window() -> View {
    let view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 600.0));
    view.window().show().expect("shown");
    view
}

/// The CPU time this thread has run, where the platform says it.
fn thread_cpu() -> Option<Duration> {
    let stat = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    Some(Duration::from_nanos(
        stat.split_whitespace().next()?.parse().ok()?,
    ))
}

/// How long the next frame takes to lay out and draw: CPU time where the
/// platform says it, else the time it took.
fn draw(view: &View) -> Duration {
    let (cpu, at) = (thread_cpu(), Instant::now());
    let _ = view.window().window().take_snapshot().expect("a snapshot");
    match (cpu, thread_cpu()) {
        (Some(before), Some(after)) => after.saturating_sub(before),
        _ => at.elapsed(),
    }
}

/// The middle of `costs`.
fn median(mut costs: Vec<Duration>) -> Duration {
    costs.sort();
    costs[costs.len() / 2]
}

/// The window's reports applied, and the next page above asked.
fn to_the_top(view: &mut View, model: &mut UiModel, from: &TransportIdentity) {
    slint::platform::update_timers_and_animations();
    let _ = draw(view);
    for event in view.take_window_events() {
        model.window_event(event);
    }
    model.window_input(WindowInput::Left {
        conversation: direct(from),
        edge: Edge::Top,
    });
    model.window_input(WindowInput::Reached {
        conversation: direct(from),
        edge: Edge::Top,
    });
}

/// The page asked, answered with rows `rows` of `from`.
fn page(model: &mut UiModel, from: &TransportIdentity, rows: std::ops::RangeInclusive<i64>) {
    let mut asks = model.take_page_asks();
    assert_eq!(asks.len(), 1, "one page asked: {asks:?}");
    let unread = rows
        .map(|n| ListedInbound {
            row: RowId::from_stored(n),
            origin: Origin::Direct {
                peer: from.clone(),
                endpoint: human(),
            },
            envelope: HumanChatV2 {
                v: 2,
                kind: MessageKind::Text,
                app_message_id: format!("{:032x}", n + 1_000),
                text: format!("message {n}"),
                reply_to: None,
                sent_at_ms: None,
                from_endpoint: None,
            },
            received_at: u64::try_from(n).expect("positive"),
        })
        .collect();
    model.page_arrived(PageRows {
        ask: asks.remove(0),
        unread,
        kept: Vec::new(),
        proposals: Vec::new(),
        end: PageEnd::More,
    });
}

/// A page of 32 arriving above some 500 held rows, against a page of one
/// above the same: the first draw after each, twice. Each row once read
/// its place as it was made, and each such read laid out every row held,
/// so a page cost its own rows times the held ones and 32 rows cost about
/// 32 times one; made and then placed together, a page costs the draw of
/// the rows held plus its own (64 rows above 256 held: 2 s in a release
/// window before, 70 ms placed, whatever was held). Comparing pages over
/// the same held rows leaves out what every draw spends on them.
///
/// Then a page of one above some 600 held rows, against a frame drawn
/// with nothing changed over the same rows. A row added above marks every
/// held row's size changed, and each held row's bubble, laid out from the
/// size the list gave it, measured all its text again (j307, thread CPU
/// time on the desktop's software renderer): one row cost some seven
/// frames there in an optimised build and some twenty in a debug one,
/// sized from its own content some three and some seven. The bound is
/// the build's: a debug build's bindings cost more beside its drawing,
/// and there the two lie closer, so a debug run caught the second
/// mutation in two of three runs where an optimised one caught it in
/// every run.
/// Mutations: a row's `in-view` and `item-top` read as it is made; the
/// row's content sized by the row (`row-content` without its `width` and
/// `height`).
#[test]
fn a_page_drawn_above_costs_its_own_rows_not_the_held_ones() {
    let mut view = window();
    let from = peer();
    let mut model = UiModel::new();
    model.started(
        Vec::new(),
        vec![StoredSummary {
            conversation: direct(&from),
            unread: 800,
            newest: Some(NewestRow {
                table: Table::Unread,
                row: RowId::from_stored(800),
                received_at: 800,
                preview: cut("newest"),
                card: false,
            }),
        }],
        Some(RowId::from_stored(i64::MAX)),
    );
    view.select(direct(&from));
    let _ = view.take_events(&model);
    view.render(&model);
    to_the_top(&mut view, &mut model, &from);
    let mut top = 800;
    let mut drawn = |size: i64| {
        page(&mut model, &from, (top - size + 1)..=top);
        top -= size;
        view.render(&model);
        let cost = draw(&view);
        to_the_top(&mut view, &mut model, &from);
        cost
    };
    for _ in 0..16 {
        let _ = drawn(32);
    }
    let costs = [drawn(32), drawn(1), drawn(32), drawn(1)];
    let (page, one) = ((costs[0] + costs[2]) / 2, (costs[1] + costs[3]) / 2);
    eprintln!("COST {costs:?}");
    assert!(
        page < one * 4,
        "32 rows cost about what one does above the same held rows: {page:?} and {one:?} ({costs:?})"
    );
    let ones: Vec<Duration> = (0..5).map(|_| drawn(1)).collect();
    let frames: Vec<Duration> = (0..5).map(|_| draw(&view)).collect();
    let (one, frame) = (median(ones.clone()), median(frames.clone()));
    eprintln!("HELD {ones:?} FRAME {frames:?}");
    let frames_bound = if cfg!(debug_assertions) { 12 } else { 5 };
    assert!(
        one < frame * frames_bound,
        "a row added above some 600 held rows costs a few frames, not one of each held row: \
         {one:?} against a frame's {frame:?} ({ones:?}, {frames:?})"
    );
}

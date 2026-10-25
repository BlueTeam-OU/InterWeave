// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the window shows, as lines for a driver that steps the app by
//! key events and cannot see the screen: android-e2e's Scan path (plan
//! section 29 row 2). Slint draws no accessibility tree on Android and
//! Show is a secured window, so without this the host could neither
//! tell where a TAB landed nor when an add or a Trust was answered.
//!
//! A line names only kinds: the page, the focused control's focus key
//! (a fixed name, or one with the integer handles the view gives rows),
//! Show's state, the add's result and each trust block's state. Never a
//! person-facing text, an address, a `PeerId` or a name -- a focus id
//! that is not a key's shape is written as `?`
//! (`a_trace_carries_no_address_peer_or_name`). Compiled under the
//! `ui-trace` feature alone, which only the debug APK turns on.

use std::rc::Rc;

use interweave_human_ui_model::{
    AddResult, BlockProblem, BlockState, Peers, ShowState, TrustSettings,
};
use interweave_transport_api::TransportIdentity;

/// Every line starts with it, so a driver reads the trace out of a log
/// that carries the runtime's lines too.
pub(crate) const PREFIX: &str = "ui-trace";

/// What one render showed, in kinds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Shown {
    pub(crate) page: &'static str,
    pub(crate) menu: bool,
    pub(crate) show: Option<&'static str>,
    pub(crate) adding: bool,
    pub(crate) add: Option<(&'static str, usize)>,
    /// Each block's handle, state and problem, in the page's order.
    pub(crate) blocks: Vec<Block>,
}

/// One trust block: its handle, state and problem.
pub(crate) type Block = (i32, &'static str, Option<&'static str>);

/// What a render showed: `page` and `menu` as the window has them, the
/// rest read from the model it drew; `handle` is the view's own handle
/// for a block's peer.
pub(crate) fn shown(
    page: &'static str,
    menu: bool,
    peers: &Peers,
    trust: &TrustSettings,
    mut handle: impl FnMut(&TransportIdentity) -> i32,
) -> Shown {
    let show = peers.show().map(|s| match s {
        ShowState::Reading => "reading",
        ShowState::Shown { .. } => "shown",
        ShowState::NoAddress => "no-address",
        ShowState::Unread(_) => "unread",
    });
    let add = peers.add_result().map(|r| match r {
        AddResult::Added { lines, .. } => ("added", *lines),
        AddResult::Unchanged { lines } => ("unchanged", *lines),
        AddResult::Checked { lines } => ("checked", *lines),
    });
    let blocks = peers
        .blocks(trust)
        .iter()
        .map(|b| (handle(&b.peer), state(b.state), b.problem.map(problem)))
        .collect();
    Shown {
        page,
        menu,
        show,
        adding: peers.add_waiting(),
        add,
        blocks,
    }
}

/// Where the lines go, and what was last said shown.
pub(crate) struct Tracer {
    sink: Rc<dyn Fn(&str)>,
    before: Shown,
}

impl Tracer {
    pub(crate) fn new(sink: Rc<dyn Fn(&str)>) -> Self {
        Self {
            sink,
            before: Shown::default(),
        }
    }

    /// Say how `now` differs from what was last said.
    pub(crate) fn render(&mut self, now: Shown) {
        for line in changes(&self.before, &now) {
            (self.sink)(&line);
        }
        self.before = now;
    }
}

const fn state(s: BlockState) -> &'static str {
    match s {
        BlockState::Undecided => "undecided",
        BlockState::Trusting => "trusting",
        BlockState::Checking => "checking",
        BlockState::Trusted => "trusted",
        BlockState::Later => "later",
        BlockState::LaterUnknown => "later-unknown",
    }
}

const fn problem(p: BlockProblem) -> &'static str {
    match p {
        BlockProblem::Trust(_) => "trust",
        BlockProblem::NotMade => "not-made",
        BlockProblem::Unknown => "unknown",
        BlockProblem::Superseded => "superseded",
    }
}

/// The lines saying how `now` differs from `before`: one per part that
/// changed, so a driver waits for the line it expects.
pub(crate) fn changes(before: &Shown, now: &Shown) -> Vec<String> {
    let mut lines = Vec::new();
    if now.page != before.page {
        lines.push(format!("{PREFIX} page={}", now.page));
    }
    if now.menu != before.menu {
        lines.push(format!(
            "{PREFIX} menu={}",
            if now.menu { "open" } else { "closed" }
        ));
    }
    if now.show != before.show {
        lines.push(format!("{PREFIX} show={}", now.show.unwrap_or("closed")));
    }
    if now.adding != before.adding && now.adding {
        lines.push(format!("{PREFIX} add=waiting"));
    }
    if now.add != before.add {
        lines.push(match now.add {
            Some((kind, n)) => format!("{PREFIX} add={kind} lines={n}"),
            None => format!("{PREFIX} add=none"),
        });
    }
    for block in &now.blocks {
        if !before.blocks.contains(block) {
            let (handle, state, problem) = block;
            lines.push(match problem {
                Some(p) => format!("{PREFIX} block:{handle}={state} problem={p}"),
                None => format!("{PREFIX} block:{handle}={state}"),
            });
        }
    }
    lines
}

/// The line for focus moving to `id`, the window's `focus-id`.
pub(crate) fn focus_line(id: &str) -> String {
    if id.is_empty() {
        format!("{PREFIX} focus=none")
    } else if is_key(id) {
        format!("{PREFIX} focus={id}")
    } else {
        format!("{PREFIX} focus=?")
    }
}

/// A focus key's shape: a lowercase name, then any `:`-separated integer
/// handles. Anything else might be content, and is never written.
fn is_key(id: &str) -> bool {
    let mut parts = id.split(':');
    let name_ok = parts.next().is_some_and(|n| {
        n.len() <= 32
            && n.starts_with(|c: char| c.is_ascii_lowercase())
            && n.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    });
    name_ok
        && parts.all(|h| {
            let digits = h.strip_prefix('-').unwrap_or(h);
            !digits.is_empty() && digits.len() <= 10 && digits.chars().all(|c| c.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_focus_key_is_written_and_anything_else_is_not() {
        assert_eq!(focus_line("add-address"), "ui-trace focus=add-address");
        assert_eq!(focus_line("block-trust:3"), "ui-trace focus=block-trust:3");
        assert_eq!(focus_line("action:-1:2"), "ui-trace focus=action:-1:2");
        assert_eq!(focus_line(""), "ui-trace focus=none");
        for content in [
            "/ip4/192.168.100.6/tcp/4001/p2p/12D3KooWExample",
            "12D3KooWExample",
            "Alice",
            "row:12D3KooW",
            "block:",
            "Add-address",
            "a b",
        ] {
            assert_eq!(focus_line(content), "ui-trace focus=?", "{content}");
        }
    }

    #[test]
    fn only_what_changed_is_written() {
        let before = Shown {
            page: "peers",
            ..Shown::default()
        };
        let now = Shown {
            page: "peers-add-trust",
            add: Some(("added", 2)),
            blocks: vec![(1, "undecided", None), (2, "trusted", None)],
            ..Shown::default()
        };
        assert_eq!(
            changes(&before, &now),
            [
                "ui-trace page=peers-add-trust",
                "ui-trace add=added lines=2",
                "ui-trace block:1=undecided",
                "ui-trace block:2=trusted",
            ]
        );
        assert!(changes(&now, &now).is_empty());
        let later = Shown {
            blocks: vec![(1, "undecided", Some("not-made")), (2, "trusted", None)],
            ..now.clone()
        };
        assert_eq!(
            changes(&now, &later),
            ["ui-trace block:1=undecided problem=not-made"]
        );
    }
}

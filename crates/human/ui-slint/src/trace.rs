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
//! where the start stands on onboarding's Connect, Show's state, the add's result and each trust block's state, the
//! connectivity and the session notice the window shows, the identity
//! screen over it and whether its page carries ADR-0043's warning, and each credential prompt the root raised and its
//! answer. Never a
//! person-facing text, an address, a `PeerId` or a name -- a focus id
//! that is not a key's shape is written as `?`
//! (`a_trace_carries_no_address_peer_or_name`). Compiled under the
//! `ui-trace` feature alone, which only the debug APK turns on.

use std::cell::Cell;
use std::rc::Rc;

use interweave_human_client_api::Connectivity;
use interweave_human_ui_model::{
    AddResult, BlockProblem, BlockState, ConnectPhase, Peers, ScreenId, SessionNotice, ShowState,
    TrustSettings,
};
use interweave_transport_api::TransportIdentity;

/// Every line starts with it, so a driver reads the trace out of a log
/// that carries the runtime's lines too.
pub(crate) const PREFIX: &str = "ui-trace";

/// What one render showed, in kinds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Shown {
    pub(crate) page: &'static str,
    /// Where the start stands on Connect: a driver waits for it to leave
    /// `owed` before it judges the page the app settled on, since Connect
    /// opens over Chats only once both reads answer.
    pub(crate) connect: &'static str,
    pub(crate) show: Option<&'static str>,
    pub(crate) adding: bool,
    pub(crate) add: Option<(&'static str, usize)>,
    /// Each block's handle, state and problem, in the page's order.
    pub(crate) blocks: Vec<Block>,
    /// The connectivity the window shows: `offline` while the key waits
    /// for the person, as the model has it.
    pub(crate) connectivity: &'static str,
    /// The session notice above the chats, if any.
    pub(crate) notice: Option<&'static str>,
}

/// One trust block: its handle, state and problem.
pub(crate) type Block = (i32, &'static str, Option<&'static str>);

/// What a render showed: `page` as the window has it, the rest read
/// from the model it drew; `handle` is the view's own handle
/// for a block's peer.
pub(crate) fn shown(
    page: &'static str,
    connect: ConnectPhase,
    connectivity: Connectivity,
    notice: Option<SessionNotice>,
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
    let connect = match connect {
        ConnectPhase::NotOwed => "not-owed",
        ConnectPhase::Owed => "owed",
        ConnectPhase::Open => "open",
        ConnectPhase::Settled => "settled",
        ConnectPhase::Deferred => "deferred",
    };
    let connectivity = match connectivity {
        Connectivity::OnlineDirect => "online-direct",
        Connectivity::OnlineRelay => "online-relay",
        Connectivity::OnlinePartial => "online-partial",
        Connectivity::Offline => "offline",
        Connectivity::Unknown => "unknown",
    };
    let notice = notice.map(|n| match n {
        SessionNotice::NoDaemon => "no-daemon",
        SessionNotice::Reconnecting => "reconnecting",
        SessionNotice::StorageDegraded => "storage-degraded",
        SessionNotice::Refused(_) => "refused",
        SessionNotice::NetworkDenied => "network-denied",
        SessionNotice::AuthenticationNeeded => "authentication-needed",
    });
    Shown {
        page,
        connect,
        show,
        adding: peers.add_waiting(),
        add,
        blocks,
        connectivity,
        notice,
    }
}

/// The identity screen a kind names: the design's screen id, lowercase
/// (design/2026-10-19-android-identity.md), `None` once the flow has
/// handed over.
const fn identity(screen: ScreenId) -> Option<&'static str> {
    Some(match screen {
        ScreenId::W => "w",
        ScreenId::C1 => "c1",
        ScreenId::C2 => "c2",
        ScreenId::C3 => "c3",
        ScreenId::C4 => "c4",
        ScreenId::R1 => "r1",
        ScreenId::R2 => "r2",
        ScreenId::R3 => "r3",
        ScreenId::R4 => "r4",
        ScreenId::K1 => "k1",
        ScreenId::S1 => "s1",
        ScreenId::S2 => "s2",
        ScreenId::Done => return None,
    })
}

/// A credential prompt the root asked the platform for, or its answer.
/// The prompt a cold launch raises before anything binds is the
/// Activity's own and has no asked line; its answer has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prompt {
    /// Show my phrase's prompt, before the words.
    Phrase,
    /// The prompt that unlocks the app: the notice's Unlock, or a key
    /// operation the presence window held back.
    Unlock,
    /// The person authenticated.
    Granted,
    /// Cancelled, refused, or left (Recents, Back): the platform's
    /// error, whichever.
    Cancelled,
}

/// The line for `prompt`.
#[must_use]
pub fn prompt_line(prompt: Prompt) -> String {
    let kind = match prompt {
        Prompt::Phrase => "phrase",
        Prompt::Unlock => "unlock",
        Prompt::Granted => "granted",
        Prompt::Cancelled => "cancelled",
    };
    format!("{PREFIX} prompt={kind}")
}

/// Where the lines go, and what was last said shown.
pub(crate) struct Tracer {
    sink: Rc<dyn Fn(&str)>,
    /// Focus lines said so far, so a statement after a prompt's answer
    /// is made only when no move said it ([`Self::focus_settled`]).
    focus_said: Rc<Cell<u64>>,
    before: Shown,
    /// The identity screen last said shown.
    identity: Option<&'static str>,
    /// Whether the identity page last said shown carries ADR-0043's
    /// warning.
    warns: bool,
}

impl Tracer {
    pub(crate) fn new(sink: Rc<dyn Fn(&str)>) -> Self {
        Self {
            sink,
            focus_said: Rc::new(Cell::new(0)),
            before: Shown::default(),
            identity: None,
            warns: false,
        }
    }

    /// What says focus moved to the focus key it is handed.
    pub(crate) fn focus_sayer(&self) -> impl Fn(&str) + 'static {
        let (sink, said) = (Rc::clone(&self.sink), Rc::clone(&self.focus_said));
        move |id| {
            said.set(said.get() + 1);
            sink(&focus_line(id));
        }
    }

    /// Say where focus stands once `settle` has passed, read by `focus`,
    /// unless a move was said meanwhile: a prompt answered with focus
    /// left where it was -- R's Try again cancelled (A15) -- moves
    /// nothing, and a driver waiting on a focus line after the answer
    /// would otherwise read none. `settle` covers the focus requests the
    /// answer makes, which land a turn or two later.
    pub(crate) fn focus_settled(
        &self,
        settle: std::time::Duration,
        focus: impl Fn() -> Option<String> + 'static,
    ) {
        let (sink, said) = (Rc::clone(&self.sink), Rc::clone(&self.focus_said));
        let at = said.get();
        slint::Timer::single_shot(settle, move || {
            if said.get() == at
                && let Some(id) = focus()
            {
                said.set(at + 1);
                sink(&focus_line(&id));
            }
        });
    }

    /// Say the identity screen `screen` is over the window, or, `None`,
    /// that the flow's page has left it, and then whether the page `warns`
    /// that the phrase may be live elsewhere -- each when it changed. A
    /// page that has left warns of nothing.
    pub(crate) fn identity(&mut self, screen: Option<ScreenId>, warns: bool) {
        let now = screen.and_then(identity);
        if let Some(line) = identity_change(self.identity, now) {
            (self.sink)(&line);
        }
        self.identity = now;
        let warns = warns && now.is_some();
        if warns != self.warns {
            let kind = if warns { "active-elsewhere" } else { "none" };
            (self.sink)(&format!("{PREFIX} warns={kind}"));
        }
        self.warns = warns;
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
        BlockProblem::SupersededUnread => "superseded-unread",
    }
}

/// The lines saying how `now` differs from `before`: one per part that
/// changed, so a driver waits for the line it expects.
pub(crate) fn changes(before: &Shown, now: &Shown) -> Vec<String> {
    let mut lines = Vec::new();
    if now.page != before.page {
        lines.push(format!("{PREFIX} page={}", now.page));
    }
    if now.connect != before.connect {
        lines.push(format!("{PREFIX} connect={}", now.connect));
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
    if now.connectivity != before.connectivity {
        lines.push(format!("{PREFIX} connectivity={}", now.connectivity));
    }
    if now.notice != before.notice {
        lines.push(format!("{PREFIX} notice={}", now.notice.unwrap_or("none")));
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

/// The line for the identity screen moving from `before` to `now`.
fn identity_change(before: Option<&'static str>, now: Option<&'static str>) -> Option<String> {
    (before != now).then(|| format!("{PREFIX} identity={}", now.unwrap_or("closed")))
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
            connect: "settled",
            add: Some(("added", 2)),
            blocks: vec![(1, "undecided", None), (2, "trusted", None)],
            ..Shown::default()
        };
        assert_eq!(
            changes(&before, &now),
            [
                "ui-trace page=peers-add-trust",
                "ui-trace connect=settled",
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

    /// The key waiting for the person: offline with its notice, then
    /// both gone once it is unlocked.
    #[test]
    fn the_locked_state_is_said_and_its_end() {
        let peers = Peers::default();
        let trust = TrustSettings::default();
        let at = |connectivity, notice| {
            shown(
                "conversations",
                ConnectPhase::NotOwed,
                connectivity,
                notice,
                &peers,
                &trust,
                |_| 0,
            )
        };
        let locked = at(
            Connectivity::Offline,
            Some(SessionNotice::AuthenticationNeeded),
        );
        assert_eq!(
            changes(&Shown::default(), &locked),
            [
                "ui-trace page=conversations",
                "ui-trace connect=not-owed",
                "ui-trace connectivity=offline",
                "ui-trace notice=authentication-needed",
            ]
        );
        let unlocked = at(Connectivity::Unknown, None);
        assert_eq!(
            changes(&locked, &unlocked),
            ["ui-trace connectivity=unknown", "ui-trace notice=none"]
        );
        let online = at(Connectivity::OnlinePartial, None);
        assert_eq!(
            changes(&unlocked, &online),
            ["ui-trace connectivity=online-partial"]
        );
    }

    /// Show my phrase from Settings: S1, its words, the flow left; a
    /// screen said once however often it is drawn.
    #[test]
    fn the_identity_screen_is_said_when_it_changes() {
        let said = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let to = Rc::clone(&said);
        let mut tracer = Tracer::new(Rc::new(move |l: &str| to.borrow_mut().push(l.to_owned())));
        tracer.identity(None, false);
        for screen in [ScreenId::S1, ScreenId::S1, ScreenId::S2, ScreenId::Done] {
            tracer.identity(Some(screen), false);
        }
        tracer.identity(None, false);
        tracer.identity(Some(ScreenId::K1), true);
        tracer.identity(Some(ScreenId::K1), true);
        tracer.identity(Some(ScreenId::R2), false);
        tracer.identity(Some(ScreenId::R1), true);
        // Done with a warning flag still set warns of nothing.
        tracer.identity(Some(ScreenId::Done), true);
        assert_eq!(
            *said.borrow(),
            [
                "ui-trace identity=s1",
                "ui-trace identity=s2",
                "ui-trace identity=closed",
                "ui-trace identity=k1",
                "ui-trace warns=active-elsewhere",
                "ui-trace identity=r2",
                "ui-trace warns=none",
                "ui-trace identity=r1",
                "ui-trace warns=active-elsewhere",
                "ui-trace identity=closed",
                "ui-trace warns=none",
            ]
        );
    }

    #[test]
    fn each_prompt_has_its_line() {
        assert_eq!(prompt_line(Prompt::Phrase), "ui-trace prompt=phrase");
        assert_eq!(prompt_line(Prompt::Unlock), "ui-trace prompt=unlock");
        assert_eq!(prompt_line(Prompt::Granted), "ui-trace prompt=granted");
        assert_eq!(prompt_line(Prompt::Cancelled), "ui-trace prompt=cancelled");
    }
}

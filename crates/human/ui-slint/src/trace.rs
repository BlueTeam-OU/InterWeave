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
//! where the start stands on onboarding's Connect, Show's state, the add's result, the add page's note and each trust block's state, the
//! connectivity and the session notice the window shows, the identity
//! screen over it and whether its page carries ADR-0043's warning, and each credential prompt the root raised and its
//! answer. Never a
//! person-facing text, an address, a `PeerId` or a name -- a focus id
//! that is not a key's shape is written as `?`
//! (`a_trace_carries_no_address_peer_or_name`). Compiled under the
//! `ui-trace` feature alone, which only the debug APK turns on.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use interweave_human_client_api::Connectivity;
use interweave_human_ui_model::{
    AddNote, AddResult, Barred, BlockProblem, BlockState, ConnectPhase, Peers, Refusal, ScreenId,
    SessionNotice, ShowState, TrustSettings, UiText,
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
    /// The add page's note, by kind, and the count the model bumps at
    /// every note set: a refusal is an add's answer as much as a result
    /// is, and without it a driver waiting on the add saw only patience
    /// run out. The count makes the same note set again a line again.
    pub(crate) note: Option<(&'static str, u64)>,
    /// Each block's handle, state and problem, in the page's order.
    pub(crate) blocks: Vec<Block>,
    /// The connectivity the window shows: `offline` while the key waits
    /// for the person, as the model has it.
    pub(crate) connectivity: &'static str,
    /// The notice above the chats, if any: a session notice, else the
    /// discard's shown once, else the unlock note's P-up notice.
    pub(crate) notice: Option<&'static str>,
}

/// One trust block: its handle, state and problem.
pub(crate) type Block = (i32, &'static str, Option<&'static str>);

/// A session notice's name in the trace.
pub(crate) const fn session_notice(n: SessionNotice) -> &'static str {
    match n {
        SessionNotice::NoDaemon => "no-daemon",
        SessionNotice::Reconnecting => "reconnecting",
        SessionNotice::StorageDegraded => "storage-degraded",
        SessionNotice::Refused(_) => "refused",
        SessionNotice::NetworkDenied => "network-denied",
        SessionNotice::AuthenticationNeeded => "authentication-needed",
    }
}

/// A notice shown once (`View::show_once`), by name: the discard's two,
/// and `once` for any other, which none is today.
pub(crate) const fn once_notice(text: UiText) -> &'static str {
    match text {
        UiText::StoreDiscarded => "store-discarded",
        UiText::StoreRestored => "store-restored",
        _ => "once",
    }
}

/// What a render showed: `page` as the window has it, the rest read
/// from the model it drew; `handle` is the view's own handle
/// for a block's peer.
pub(crate) fn shown(
    page: &'static str,
    connect: ConnectPhase,
    connectivity: Connectivity,
    notice: Option<&'static str>,
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
    let note = match peers.add_note_said() {
        (Some(n), said) => Some((note(n), said)),
        (None, _) => None,
    };
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
    Shown {
        page,
        connect,
        show,
        adding: peers.add_waiting(),
        add,
        note,
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

/// A moment of a held C4's or R4's start (add-a-peer section 5.1), said
/// so a run times the hold from the trace (section 8 item 12): the 1 s
/// from the press, the bound from the seal's success on C4, a launch's
/// 10 s from the service's start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// Continue pressed on C4 or R4: the screen holds.
    Pressed,
    /// C4's seal succeeded: the identity is committed.
    Sealed,
    /// `ConnectStarting` shows under the held Continue.
    Starting,
    /// The network service runs: a start succeeded.
    Running,
}

/// The line for `start`.
#[must_use]
pub fn start_line(start: Start) -> String {
    let kind = match start {
        Start::Pressed => "pressed",
        Start::Sealed => "sealed",
        Start::Starting => "starting",
        Start::Running => "running",
    };
    format!("{PREFIX} start={kind}")
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

    /// An identity screen is over the window, as last said: what is drawn
    /// under it is not shown.
    pub(crate) const fn covered(&self) -> bool {
        self.identity.is_some()
    }

    /// The page last said shown.
    pub(crate) const fn page_said(&self) -> &'static str {
        self.before.page
    }

    /// The identity screen went, uncovering `page`: said as a render of
    /// it would say it. True when the page changed, which owes a
    /// statement of focus.
    pub(crate) fn uncover(&mut self, page: &'static str) -> bool {
        let now = Shown {
            page,
            ..self.before.clone()
        };
        self.render(now)
    }

    /// Where the lines go, for one said from outside a render: the
    /// reader's times, said once a frame is drawn.
    pub(crate) fn sink(&self) -> Rc<dyn Fn(&str)> {
        Rc::clone(&self.sink)
    }

    /// Say how `now` differs from what was last said: true when the
    /// page changed, which owes a statement of focus.
    pub(crate) fn render(&mut self, now: Shown) -> bool {
        for line in changes(&self.before, &now) {
            (self.sink)(&line);
        }
        let paged = now.page != self.before.page;
        self.before = now;
        paged
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

const fn note(n: &AddNote) -> &'static str {
    match n {
        AddNote::AddressNeeded => "address-needed",
        AddNote::ScanAfterField => "scan-after-field",
        AddNote::ImageNoCode => "image-no-code",
        AddNote::NotAddress => "not-address",
        AddNote::ImageUnreadable => "image-unreadable",
        AddNote::ClipboardNoImage => "clipboard-no-image",
        AddNote::ClipboardUnreadable => "clipboard-unreadable",
        AddNote::CameraDenied => "camera-denied",
        AddNote::CameraDeniedSettings => "camera-denied-settings",
        AddNote::Refused { why, .. } => match why {
            Refusal::Grammar => "refused:grammar",
            Refusal::SpecialUse => "refused:special-use",
            Refusal::DnsFromScan => "refused:dns-from-scan",
            Refusal::TooMany => "refused:too-many",
            Refusal::Limit => "refused:limit",
            Refusal::ListOff => "refused:list-off",
            Refusal::OwnAddress => "refused:own-address",
        },
        AddNote::TooManyLines { .. } => "too-many-lines",
        AddNote::Failed => "failed",
        AddNote::Checking => "checking",
        AddNote::Unknown { .. } => "unknown",
        AddNote::Superseded { .. } => "superseded",
        AddNote::SeveralIncompatible { .. } => "several-incompatible",
        AddNote::Barred(Barred::NotPermitted) => "barred:not-permitted",
        AddNote::Barred(Barred::Incompatible) => "barred:incompatible",
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
    if now.note != before.note {
        lines.push(format!(
            "{PREFIX} note={}",
            now.note.map_or("none", |(kind, _)| kind)
        ));
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

/// The line for the reader's first frame drawn, `took` after the press
/// that opened it reached the view (design/2026-11-04-long-message.md
/// A5: within 0.5 s on the A40).
pub(crate) fn reader_drawn_line(took: Duration) -> String {
    format!("{PREFIX} reader=drawn ms={}", took.as_millis())
}

/// The line for the reader's rows handed to the window, `took` after the
/// press, where the renderer tells no frame (the desktop's software
/// renderer sets no rendering notifier): the frame that draws them comes
/// after it, so a driver reading it as the first screen reads it early,
/// and a Back it calls too late is one the frame may not have preceded.
pub(crate) fn reader_rows_line(took: Duration) -> String {
    format!("{} frame=untold", reader_drawn_line(took))
}

/// The line for End in the reader showing the source or the formatted
/// form: `took` from the key to the frame that showed the list at its
/// end, `frames` drawn in between counting that one, or, `at_end` false,
/// the frame at which the wait gave up (A5: within 1 s).
pub(crate) fn reader_end_line(took: Duration, frames: u32, at_end: bool, source: bool) -> String {
    let at_end = if at_end { "yes" } else { "no" };
    let form = if source { "source" } else { "formatted" };
    format!(
        "{PREFIX} reader=end ms={} frames={frames} at-end={at_end} form={form}",
        took.as_millis()
    )
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
    fn the_readers_times_are_numbers_and_kinds() {
        assert_eq!(
            reader_drawn_line(Duration::from_micros(412_900)),
            "ui-trace reader=drawn ms=412"
        );
        assert_eq!(
            reader_rows_line(Duration::from_micros(3_900)),
            "ui-trace reader=drawn ms=3 frame=untold"
        );
        assert_eq!(
            reader_end_line(Duration::from_millis(1_250), 4, true, false),
            "ui-trace reader=end ms=1250 frames=4 at-end=yes form=formatted"
        );
        assert_eq!(
            reader_end_line(Duration::from_secs(3), 180, false, true),
            "ui-trace reader=end ms=3000 frames=180 at-end=no form=source"
        );
    }

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
            Some(session_notice(SessionNotice::AuthenticationNeeded)),
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

    #[test]
    fn each_moment_of_a_held_start_has_its_line() {
        assert_eq!(start_line(Start::Pressed), "ui-trace start=pressed");
        assert_eq!(start_line(Start::Sealed), "ui-trace start=sealed");
        assert_eq!(start_line(Start::Starting), "ui-trace start=starting");
        assert_eq!(start_line(Start::Running), "ui-trace start=running");
    }
}

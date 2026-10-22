// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Peers by address, drawn (`design/2026-10-22-add-a-peer.md`): Peers'
//! list, the add page's two steps, its snackbar and Chats' no-reach
//! notice, from `ui-model`'s [`Peers`]. Every word is a label's; every
//! press goes back through the queue as a [`PeersInput`], judged by the
//! model when it is taken.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use interweave_human_client_api::{AddressOrigin, TrustProblem};
use interweave_human_ui_model::{
    ADDRESS_BOUND, AddNote, AddResult, Barred, BlockProblem, BlockState, CODE_LINES, HostText,
    LineSelection, ListProblem, Peers, PeersFocus, PeersInput, PeersSnack, Refusal, RowState,
    RuntimeHost, TYPED_LINES, TrustLine, TrustSettings, UiModel, UiText, fill, placeholder_en,
    short_peer,
};
use interweave_transport_api::TransportIdentity;
use slint::{ModelRc, SharedString, VecModel};

use crate::generated::{AppWindow, PeerAddressRow, PeerTrustBlock};
use crate::{Input, Page, Shared, View, enqueue, update_by_key};

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

/// What a refusal says, its bound filled from the records.
pub(crate) fn refusal_text(why: Refusal) -> String {
    let (template, max) = match why {
        Refusal::Grammar => (UiText::RefusedGrammar, 0),
        Refusal::SpecialUse => (UiText::RefusedSpecialUse, 0),
        Refusal::DnsFromScan => (UiText::RefusedDnsFromScan, 0),
        Refusal::TooMany => (UiText::RefusedTooMany, CODE_LINES),
        Refusal::Limit => (UiText::RefusedLimit, ADDRESS_BOUND),
        Refusal::ListOff => (UiText::RefusedListOff, 0),
    };
    fill(text(template), &[("max", &max.to_string())])
}

/// The note under Add an address: at the bound, or the list off.
fn add_address_note(why: Refusal) -> String {
    match why {
        Refusal::ListOff => text(UiText::PeersListOff).to_owned(),
        _ => fill(
            text(UiText::PeersLimit),
            &[("max", &ADDRESS_BOUND.to_string())],
        ),
    }
}

fn barred_text(host: RuntimeHost, b: Barred) -> &'static str {
    placeholder_en::host_text(
        host,
        match b {
            Barred::NotPermitted => HostText::PeersNotPermitted,
            Barred::Incompatible => HostText::PeersIncompatible,
        },
    )
}

/// The host and port an address names, as the row shows them: the first
/// host component's value and the first port after it, an IPv6 host in
/// brackets. An address of another shape is shown whole.
pub(crate) fn host_port(address: &str) -> String {
    let parts: Vec<&str> = address.split('/').collect();
    let mut host = None;
    let mut port = None;
    let mut i = 1;
    while i + 1 < parts.len() {
        match parts[i] {
            "ip4" | "dns" | "dns4" | "dns6" | "dnsaddr" if host.is_none() => {
                host = Some(parts[i + 1].to_owned());
            }
            "ip6" if host.is_none() => host = Some(format!("[{}]", parts[i + 1])),
            "tcp" | "udp" if host.is_some() && port.is_none() => port = Some(parts[i + 1]),
            _ => {}
        }
        i += 1;
    }
    match (host, port) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        _ => address.to_owned(),
    }
}

/// The character offsets of `selection` in the field's `text`, for the
/// field to select: the line itself, or from a line to the end.
fn selection_offsets(field: &str, selection: &LineSelection) -> Option<(i32, i32)> {
    let mut lines = Vec::new();
    let mut at = 0;
    for raw in field.split_inclusive(['\n', '\r']) {
        let body = raw.trim_end_matches(['\n', '\r']);
        let trimmed = body.trim_matches([' ', '\t']);
        if !trimmed.is_empty() {
            let start = at + (body.len() - body.trim_start_matches([' ', '\t']).len());
            lines.push((trimmed, start, start + trimmed.len()));
        }
        at += raw.len();
    }
    let (from, to) = match selection {
        LineSelection::Line(line) => {
            let (_, s, e) = lines.iter().find(|(l, _, _)| l == line)?;
            (*s, *e)
        }
        LineSelection::From(index) => (lines.get(*index)?.1, field.len()),
    };
    Some((i32::try_from(from).ok()?, i32::try_from(to).ok()?))
}

/// Peers' callbacks: each becomes an input, taken as the others are.
pub(crate) fn wire(window: &AppWindow, shared: &Rc<RefCell<Shared>>) {
    let on = |s: &Rc<RefCell<Shared>>, input: PeersInput| {
        let s = Rc::clone(s);
        move || enqueue(&s, Input::Peers(input.clone()))
    };
    let s = Rc::clone(shared);
    window.on_open_peers(move || enqueue(&s, Input::PeersPage(true)));
    let s = Rc::clone(shared);
    window.on_close_peers(move || enqueue(&s, Input::PeersPage(false)));
    window.on_peers_read_again(on(shared, PeersInput::ReadAgain));
    window.on_peers_trust_read_again(on(shared, PeersInput::TrustReadAgain));
    window.on_peers_open_add(on(shared, PeersInput::OpenAdd));
    window.on_peers_add_closed(on(shared, PeersInput::Close));
    window.on_peers_add(on(shared, PeersInput::Add));
    window.on_peers_scan(on(shared, PeersInput::ScanPressed));
    window.on_peers_done(on(shared, PeersInput::Close));
    let s = Rc::clone(shared);
    window.on_address_edited(move |typed| {
        enqueue(
            &s,
            Input::Peers(PeersInput::FieldChanged(typed.to_string())),
        );
    });
    let peer_of = |s: &Rc<RefCell<Shared>>, handle: i32| s.borrow().blocks.get(&handle).cloned();
    let s = Rc::clone(shared);
    window.on_block_trust(move |handle| {
        if let Some(peer) = peer_of(&s, handle) {
            enqueue(&s, Input::Peers(PeersInput::Trust(peer)));
        }
    });
    let s = Rc::clone(shared);
    window.on_block_not_now(move |handle| {
        if let Some(peer) = peer_of(&s, handle) {
            enqueue(&s, Input::Peers(PeersInput::NotNow(peer)));
        }
    });
    let s = Rc::clone(shared);
    window.on_row_trust(move |handle| {
        let address = s.borrow().peer_rows.get(&handle).map(|r| r.0.clone());
        if let Some(address) = address {
            enqueue(&s, Input::Peers(PeersInput::TrustRow(address)));
        }
    });
    let s = Rc::clone(shared);
    window.on_row_remove(move |handle| {
        let address = s.borrow().peer_rows.get(&handle).map(|r| r.0.clone());
        if let Some(address) = address {
            enqueue(&s, Input::Peers(PeersInput::Remove(address)));
        }
    });
    // The snackbar's action is Undo or Try again, by what it says; the
    // model resolves which when the press is taken.
    let s = Rc::clone(shared);
    window.on_peers_snack_pressed(move |id| {
        enqueue(&s, Input::PeersSnackAction(u64::try_from(id).unwrap_or(0)));
    });
    let s = Rc::clone(shared);
    window.on_peers_snack_closed(move |id| {
        enqueue(&s, Input::PeersSnackClosed(u64::try_from(id).unwrap_or(0)));
    });
}

/// The models the window's Peers lists are drawn from.
pub(crate) struct PeerLists {
    rows: Rc<VecModel<PeerAddressRow>>,
    row_keys: Vec<String>,
    row_handles: crate::Handles<String>,
    blocks: Rc<VecModel<PeerTrustBlock>>,
    block_keys: Vec<TransportIdentity>,
    block_handles: crate::Handles<TransportIdentity>,
    /// The focus request last carried out, by the model's count.
    focus_seen: u64,
    /// The snackbar last drawn, by the model's count, and the one timed.
    snack_seen: u64,
    snack_timed: u64,
}

impl PeerLists {
    pub(crate) fn new(window: &AppWindow, host: RuntimeHost) -> Self {
        let rows = Rc::new(VecModel::<PeerAddressRow>::default());
        let blocks = Rc::new(VecModel::<PeerTrustBlock>::default());
        window.set_peer_rows(ModelRc::from(Rc::clone(&rows)));
        window.set_trust_blocks(ModelRc::from(Rc::clone(&blocks)));
        let set = |t| SharedString::from(text(t));
        window.set_peers_title(set(UiText::PeersTitle));
        window.set_try_again_label(set(UiText::TryAgain));
        window.set_add_address_label(set(UiText::AddAddress));
        window.set_peers_no_trust(set(UiText::PeersNoTrust));
        window.set_copy_address_label(set(UiText::CopyAddress));
        window.set_remove_address_label(set(UiText::RemoveAddress));
        window.set_not_removable_label(set(UiText::PeerConfigNotRemovable));
        window.set_trust_this_label(set(UiText::TrustThisPeer));
        window.set_add_title(set(UiText::AddAddress));
        window.set_address_field_label(set(UiText::AddressField));
        window.set_address_help(set(UiText::AddressHelp));
        window.set_scan_code_label(set(UiText::ScanCode));
        window.set_open_image_label(set(UiText::OpenCodeImage));
        window.set_paste_image_label(set(UiText::PasteCodeImage));
        window.set_add_action_label(set(UiText::AddAction));
        window.set_trust_step_body(set(UiText::TrustStepBody));
        window.set_trust_step_both(set(UiText::TrustStepBoth));
        window.set_not_now_label(set(UiText::NotNow));
        window.set_copy_peer_id_label(set(UiText::CopyPeerId));
        window.set_done_label(set(UiText::IdDone));
        window.set_no_reach_text(set(UiText::NoReachNotice));
        let _ = host;
        Self {
            rows,
            row_keys: Vec::new(),
            row_handles: crate::Handles::new(),
            blocks,
            block_keys: Vec::new(),
            block_handles: crate::Handles::new(),
            focus_seen: 0,
            snack_seen: 0,
            snack_timed: 0,
        }
    }
}

impl View {
    /// Peers and its add page, and Chats' no-reach notice: what changed
    /// is returned once, for the render's one announcement.
    pub(crate) fn render_peers(&mut self, model: &UiModel) -> Option<String> {
        let on_page = self.page == Page::Peers;
        self.window.set_peers_page(on_page);
        self.window.set_no_reach_shown(model.no_reach());
        let (peers, trust) = (model.peers(), model.trust_settings());
        self.render_peer_list(peers, trust);
        self.render_add_page(peers, trust);
        let said = self.render_peers_snack(peers, on_page);
        // Focus, once per request, on Peers alone.
        let (focus, focuses) = peers.focus();
        if on_page && focuses != self.peer_lists.focus_seen {
            self.peer_lists.focus_seen = focuses;
            if let Some(focus) = focus {
                let target = self.focus_target(focus, peers.field());
                // Peers' focus mostly lands on a control the same render
                // makes: a step-two block, the snackbar's action, the
                // list's controls as the add page closes.
                self.focus_key_after_draw(target);
            }
        }
        said
    }

    /// Peers' list, or why it is not on show.
    fn render_peer_list(&mut self, peers: &Peers, trust: &TrustSettings) {
        let window = &self.window;
        let problem = peers.list_problem().map(|p| match p {
            ListProblem::Unavailable => (
                placeholder_en::host_text(self.host, HostText::PeersReadUnavailable),
                true,
            ),
            ListProblem::Failed => (text(UiText::PeersReadFailed), true),
            ListProblem::Barred(b) => (barred_text(self.host, b), false),
        });
        window.set_peers_problem(problem.map_or("", |p| p.0).into());
        window.set_peers_problem_retry(problem.is_some_and(|p| p.1));
        window.set_peers_list_read(peers.list().is_some());
        let trust_line = peers.trust_line(trust).map(|l| match l {
            TrustLine::Unread { retry } => (text(UiText::PeersTrustUnread), retry),
            TrustLine::NotPermitted => (
                placeholder_en::host_text(self.host, HostText::PeersTrustNotPermitted),
                false,
            ),
        });
        window.set_peers_trust_line(trust_line.map_or("", |l| l.0).into());
        window.set_peers_trust_retry(trust_line.is_some_and(|l| l.1));
        window.set_peers_add_note(
            peers
                .add_note()
                .map(add_address_note)
                .unwrap_or_default()
                .into(),
        );
        let rows = peers.rows(trust);
        window.set_peers_empty(
            if peers.list().is_some() && rows.is_empty() {
                text(UiText::PeersEmpty)
            } else {
                ""
            }
            .into(),
        );
        let keys: Vec<String> = rows.iter().map(|r| r.address.clone()).collect();
        self.peer_lists.row_handles.retain(&keys);
        let drawn: Vec<PeerAddressRow> = rows
            .iter()
            .map(|r| {
                let short = r
                    .peer
                    .as_ref()
                    .map(|p| short_peer(p.as_str()))
                    .unwrap_or_default();
                PeerAddressRow {
                    handle: self.peer_lists.row_handles.of(&r.address),
                    peer: short.clone().into(),
                    host: host_port(&r.address).into(),
                    address: r.address.clone().into(),
                    // Trust unknown: no state at all, neither trusted nor
                    // not, the trust line saying why (section 5.2).
                    state: match r.state {
                        RowState::Trusted => text(UiText::PeerStateTrusted),
                        RowState::NeedsTrust => text(UiText::PeerStateNeedsTrust),
                        RowState::ProviderOff => text(UiText::PeerStateProviderOff),
                        RowState::TrustUnknown => "",
                    }
                    .into(),
                    origin: text(match r.origin {
                        AddressOrigin::AddedHere => UiText::TrustAddedHere,
                        AddressOrigin::Configured => UiText::PeerFromConfig,
                    })
                    .into(),
                    needs_trust: matches!(r.state, RowState::NeedsTrust | RowState::TrustUnknown)
                        && r.peer.is_some(),
                    removable: r.origin == AddressOrigin::AddedHere,
                    menu_label: fill(text(UiText::PeerMenu), &[("peer", &short)]).into(),
                }
            })
            .collect();
        self.shared.borrow_mut().peer_rows = drawn
            .iter()
            .zip(&rows)
            .map(|(d, r)| (d.handle, (r.address.clone(), r.peer.clone())))
            .collect();
        update_by_key(
            &*self.peer_lists.rows,
            &mut self.peer_lists.row_keys,
            &keys,
            &drawn,
        );
    }

    /// The add page: step one's field, notes and result, and step two's
    /// blocks.
    fn render_add_page(&mut self, peers: &Peers, trust: &TrustSettings) {
        let window = &self.window;
        let host = self.host;
        window.set_peers_add_open(peers.add_open());
        window.set_add_step_one(peers.step_one_shown());
        let typing = self
            .shared
            .borrow()
            .inputs
            .iter()
            .any(|i| matches!(i, Input::Peers(PeersInput::FieldChanged(_))));
        if !typing && window.get_address_text().as_str() != peers.field() {
            window.set_address_text(peers.field().into());
        }
        let note = peers.add_page_note().map(|n| note_text(self.host, n));
        let (field, scan, under_add) = match (peers.add_page_note(), note) {
            (Some(AddNote::AddressNeeded | AddNote::TooManyLines { .. }), Some(n)) => {
                (n, String::new(), String::new())
            }
            (Some(AddNote::Refused { .. }), Some(n)) if peers.field().is_empty() => {
                (String::new(), n, String::new())
            }
            (Some(AddNote::Refused { .. }), Some(n)) => (n, String::new(), String::new()),
            (Some(AddNote::ScanAfterField), Some(n)) => (String::new(), n, String::new()),
            (Some(_), Some(n)) => (String::new(), String::new(), n),
            _ => (String::new(), String::new(), String::new()),
        };
        window.set_field_note(field.into());
        window.set_scan_note(scan.into());
        window.set_add_note(under_add.into());
        window.set_add_result(
            peers
                .add_result()
                .map(result_text)
                .unwrap_or_default()
                .into(),
        );
        let blocks = peers.blocks(trust);
        let block_keys: Vec<TransportIdentity> = blocks.iter().map(|b| b.peer.clone()).collect();
        self.peer_lists.block_handles.retain(&block_keys);
        let drawn: Vec<PeerTrustBlock> = blocks
            .iter()
            .map(|b| {
                let short = short_peer(b.peer.as_str());
                PeerTrustBlock {
                    handle: self.peer_lists.block_handles.of(&b.peer),
                    title: fill(text(UiText::TrustStepTitle), &[("peer", &short)]).into(),
                    peer: b.peer.as_str().into(),
                    undecided: b.state == BlockState::Undecided,
                    // Trust leaves the block for a problem no retry mends.
                    trust_offered: !matches!(
                        b.problem,
                        Some(BlockProblem::Trust(
                            TrustProblem::NotPermitted | TrustProblem::Incompatible
                        ))
                    ),
                    state: match b.state {
                        BlockState::Trusted => text(UiText::TrustStepDone).to_owned(),
                        BlockState::Later => text(UiText::TrustStepLater).to_owned(),
                        BlockState::LaterUnknown => text(UiText::TrustStepLaterUnknown).to_owned(),
                        BlockState::Checking => fill(
                            placeholder_en::host_text(host, HostText::TrustUnconfirmed),
                            &[("peer", b.peer.as_str())],
                        ),
                        BlockState::Undecided | BlockState::Trusting => String::new(),
                    }
                    .into(),
                    problem: b
                        .problem
                        .map(|p| block_problem(host, p, &short))
                        .unwrap_or_default()
                        .into(),
                }
            })
            .collect();
        self.shared.borrow_mut().blocks = drawn
            .iter()
            .zip(&block_keys)
            .map(|(d, p)| (d.handle, p.clone()))
            .collect();
        update_by_key(
            &*self.peer_lists.blocks,
            &mut self.peer_lists.block_keys,
            &block_keys,
            &drawn,
        );
        window.set_blocks_done_shown(blocks.len() > 1);
    }

    /// Peers' snackbar and its timeout, a later one replacing it: its text
    /// once, for the announcement, when it is new and Peers is on show.
    fn render_peers_snack(&mut self, peers: &Peers, on_page: bool) -> Option<String> {
        let window = &self.window;
        let (snack, snacks) = peers.snack();
        let closing = self
            .shared
            .borrow()
            .inputs
            .iter()
            .any(|i| matches!(i, Input::PeersSnackClosed(id) if *id == snacks));
        window.set_peers_snack_shown(on_page && snack.is_some() && !closing);
        window.set_peers_snack_id(i32::try_from(snacks).unwrap_or(i32::MAX));
        let said_snack = snack.map(snack_text);
        if let Some(s) = snack {
            window.set_peers_snack_text(snack_text(s).into());
            let action = match s {
                PeersSnack::Removed { undo: true, .. } => Some(UiText::Undo),
                PeersSnack::RemoveFailed { .. }
                | PeersSnack::RemoveUnknown { .. }
                | PeersSnack::UndoFailed { .. }
                | PeersSnack::UndoUnknown { .. } => Some(UiText::TryAgain),
                _ => None,
            };
            window.set_peers_snack_action_shown(action.is_some());
            window.set_peers_snack_action(action.map(text).unwrap_or_default().into());
            if self.peer_lists.snack_timed != snacks {
                self.peer_lists.snack_timed = snacks;
                let shared = Rc::clone(&self.shared);
                let ms = self.shared.borrow().touch.snackbar_ms;
                slint::Timer::single_shot(Duration::from_millis(ms), move || {
                    enqueue(&shared, Input::PeersSnackClosed(snacks));
                });
            }
        }

        let new_snack = snacks != self.peer_lists.snack_seen;
        self.peer_lists.snack_seen = snacks;
        if on_page && new_snack {
            return said_snack;
        }
        None
    }
}

fn note_text(host: RuntimeHost, note: &AddNote) -> String {
    match note {
        AddNote::AddressNeeded => text(UiText::AddressNeeded).to_owned(),
        AddNote::ScanAfterField => text(UiText::ScanAfterField).to_owned(),
        AddNote::Refused { line, why } => fill(
            text(UiText::AddRefused),
            &[("line", line), ("reason", &refusal_text(*why))],
        ),
        AddNote::TooManyLines { lines } => fill(
            text(UiText::RefusedTooManyLines),
            &[("n", &lines.to_string()), ("max", &TYPED_LINES.to_string())],
        ),
        AddNote::Failed => text(UiText::AddFailed).to_owned(),
        // Nothing is said until the list read back says it.
        AddNote::Checking => String::new(),
        AddNote::Unknown { lines: 1 } => text(UiText::AddUnknownOne).to_owned(),
        AddNote::Unknown { .. } => text(UiText::AddUnknownOther).to_owned(),
        AddNote::SeveralIncompatible { lines } => fill(
            text(UiText::AddSeveralIncompatible),
            &[("n", &lines.to_string())],
        ),
        AddNote::Barred(b) => barred_text(host, *b).to_owned(),
    }
}

fn block_problem(host: RuntimeHost, problem: BlockProblem, short: &str) -> String {
    match problem {
        BlockProblem::Trust(p) => placeholder_en::trust_problem(host, p).to_owned(),
        BlockProblem::NotMade => fill(text(UiText::TrustStepNotMade), &[("peer", short)]),
        BlockProblem::Unknown => fill(text(UiText::TrustStepUnknown), &[("peer", short)]),
    }
}

impl View {
    /// The window's focus key for a request.
    fn focus_target(&mut self, focus: &PeersFocus, field: &str) -> String {
        match focus {
            PeersFocus::Title => "peers-title".to_owned(),
            PeersFocus::Field { select } => {
                if let Some((from, to)) = select.as_ref().and_then(|s| selection_offsets(field, s))
                {
                    let window = &self.window;
                    window.set_address_select_from(from);
                    window.set_address_select_to(to);
                    window.set_address_select_seq(window.get_address_select_seq().wrapping_add(1));
                }
                "address-field".to_owned()
            }
            PeersFocus::Note => "add-page-note".to_owned(),
            PeersFocus::AddButton => "add-action".to_owned(),
            PeersFocus::Block(peer) => format!("block:{}", self.peer_lists.block_handles.of(peer)),
            PeersFocus::BlockProblem(peer) => {
                format!("block-problem:{}", self.peer_lists.block_handles.of(peer))
            }
            PeersFocus::Done => "peers-done".to_owned(),
            PeersFocus::Opener | PeersFocus::AddAddress => "add-address".to_owned(),
            PeersFocus::Row(address) => format!("row:{}", self.peer_lists.row_handles.of(address)),
            PeersFocus::RowMenu(address) => {
                format!("row-menu:{}", self.peer_lists.row_handles.of(address))
            }
            PeersFocus::AddAddressNote => "add-address-note".to_owned(),
            PeersFocus::ListProblem => "peers-problem".to_owned(),
            PeersFocus::TrustRetry => "trust-retry".to_owned(),
            PeersFocus::SnackAction => "peers-snack-action".to_owned(),
        }
    }
}

fn result_text(result: &AddResult) -> String {
    match result {
        AddResult::Added { lines, peers } => match peers.as_slice() {
            [peer] if *lines == 1 => fill(
                text(UiText::AddedLineOne),
                &[("peer", &short_peer(peer.as_str()))],
            ),
            [peer] => fill(
                text(UiText::AddedLineOther),
                &[
                    ("n", &lines.to_string()),
                    ("peer", &short_peer(peer.as_str())),
                ],
            ),
            _ => fill(
                text(UiText::AddedLineSeveral),
                &[
                    ("n", &lines.to_string()),
                    ("count", &peers.len().to_string()),
                ],
            ),
        },
        AddResult::Unchanged { lines: 1 } => text(UiText::AddedUnchangedOne).to_owned(),
        AddResult::Unchanged { .. } => text(UiText::AddedUnchangedOther).to_owned(),
        AddResult::Checked { lines: 1 } => text(UiText::AddedCheckedOne).to_owned(),
        AddResult::Checked { lines } => fill(
            text(UiText::AddedCheckedOther),
            &[("n", &lines.to_string())],
        ),
    }
}

fn snack_text(snack: &PeersSnack) -> String {
    match snack {
        PeersSnack::Removed { undo: false, .. } => text(UiText::ConfirmAddressRemovedFinal),
        PeersSnack::Removed { trusted: true, .. } => text(UiText::ConfirmAddressRemovedTrusted),
        PeersSnack::Removed { .. } => text(UiText::ConfirmAddressRemoved),
        PeersSnack::RemoveFailed { .. } => text(UiText::RemoveFailed),
        PeersSnack::RemoveUnknown { .. } => text(UiText::RemoveUnknown),
        PeersSnack::UndoRefused(Refusal::ListOff) => text(UiText::PeersListOff),
        PeersSnack::UndoRefused(_) => {
            return fill(
                text(UiText::PeersLimit),
                &[("max", &ADDRESS_BOUND.to_string())],
            );
        }
        PeersSnack::UndoFailed { .. } => text(UiText::UndoFailed),
        PeersSnack::UndoUnknown { .. } => text(UiText::UndoUnknown),
        PeersSnack::Added => text(UiText::ConfirmAddressAdded),
        PeersSnack::PeerTrusted => text(UiText::ConfirmPeerTrusted),
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_and_port_are_read_from_each_address_shape() {
        let p = "12D3KooWExample";
        assert_eq!(
            host_port(&format!("/ip4/192.168.1.20/tcp/4001/p2p/{p}")),
            "192.168.1.20:4001"
        );
        assert_eq!(
            host_port(&format!("/ip6/fe80::1/udp/4001/quic-v1/p2p/{p}")),
            "[fe80::1]:4001"
        );
        assert_eq!(
            host_port(&format!("/dns4/peer.example.net/tcp/1/p2p/{p}")),
            "peer.example.net:1"
        );
        assert_eq!(
            host_port(&format!(
                "/ip4/203.0.113.7/tcp/4001/p2p/R/p2p-circuit/p2p/{p}"
            )),
            "203.0.113.7:4001",
            "a circuit route shows its relay's"
        );
        assert_eq!(host_port("not an address"), "not an address");
    }

    #[test]
    fn a_refused_line_is_selected_where_the_field_holds_it() {
        let field = "  /a/one \r\n\n\t/b/two\n/c/three";
        assert_eq!(
            selection_offsets(field, &LineSelection::Line("/b/two".to_owned())),
            Some((13, 19))
        );
        assert_eq!(&field[13..19], "/b/two");
        assert_eq!(
            selection_offsets(field, &LineSelection::From(1)),
            Some((13, i32::try_from(field.len()).unwrap()))
        );
        assert_eq!(selection_offsets(field, &LineSelection::From(3)), None);
    }
}

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
    ADDRESS_BOUND, AddNote, AddResult, Barred, BlockHeld, BlockProblem, BlockState, CODE_LINES,
    HostText, LineSelection, ListProblem, Peers, PeersFocus, PeersInput, PeersSnack, Refusal,
    RowState, RuntimeHost, ScanControl, ShowProblem, ShowState, TYPED_LINES, TrustLine,
    TrustSettings, UiModel, UiText, fill, placeholder_en, short_peer,
};
use interweave_transport_api::TransportIdentity;
use slint::{Model as _, ModelRc, SharedString, VecModel};

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
        Refusal::OwnAddress => (UiText::RefusedOwnAddress, 0),
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
        LineSelection::End => (field.len(), field.len()),
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
    // A scan press names its control, which the view keeps to start the
    // matching reader once the model opens the scan, and which the model
    // keeps to send focus back to.
    for (control, set) in [
        (
            ScanControl::Camera,
            AppWindow::on_peers_scan as fn(&AppWindow, _),
        ),
        (ScanControl::OpenImage, AppWindow::on_peers_open_image),
        (ScanControl::PasteImage, AppWindow::on_peers_paste_image),
    ] {
        let s = Rc::clone(shared);
        set(
            window,
            Box::new(move || {
                s.borrow_mut().scan_control = Some(control);
                enqueue(&s, Input::Peers(PeersInput::ScanPressed(control)));
            }),
        );
    }
    window.on_peers_open_show(on(shared, PeersInput::OpenShow));
    window.on_peers_scanner_closed(on(shared, PeersInput::ScannerClosed));
    let s = Rc::clone(shared);
    window.on_peers_open_settings(move || {
        let open = s.borrow().open_settings.clone();
        if let Some(open) = open {
            open();
        }
    });
    window.on_peers_show_closed(on(shared, PeersInput::CloseShow));
    window.on_peers_show_again(on(shared, PeersInput::ShowTryAgain));
    // A reader's result, handed over on the window's thread.
    let s = Rc::clone(shared);
    window.on_peers_image_done(move || {
        let read = s
            .borrow()
            .scan_result
            .lock()
            .ok()
            .and_then(|mut r| r.take());
        let current = {
            let shared = s.borrow();
            shared.scan_reading.is_some().then_some(shared.scan_number)
        };
        // Only the scan open now takes a result; an earlier one's is
        // dropped.
        if let Some((number, input)) = read
            && Some(number) == current
        {
            enqueue(&s, Input::Peers(input));
        }
    });
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
    /// The key that request was sent as, and whether `PeerRowGoneAfterAct`
    /// was said for it: a read before its deferred move may change the
    /// target (`a_row_gone_before_its_focus_moves_is_said_and_focus_follows`).
    focus_sent: String,
    focus_gone_said: bool,
    /// The dropped row focus was in whose move is scheduled: said once,
    /// and no older request of the model's re-judged over it before it
    /// applies (`a_row_reached_without_an_act_a_later_read_drops_is_said`).
    gone_move: Option<String>,
    /// The block whose problem line focus left for its new state line,
    /// and that line's words: drawn without its live region while they
    /// stand, since focus reads it (design section 5.3).
    quiet_state: Option<(TransportIdentity, String)>,
    /// The block whose control, taken away by an unasked read, has its
    /// focus move scheduled: no older request of the model's is re-judged
    /// over it before it applies, as for `gone_move`.
    block_move: Option<TransportIdentity>,
    /// The row read back last acted on, by the model's count.
    read_back_seen: u64,
    /// The add page's note last drawn, by the model's count, and its text.
    note_seen: u64,
    note_text: String,
    /// The page's add was awaiting its answer when last drawn: its wait
    /// is announced once, as it begins.
    add_waiting: bool,
    /// The trust line's failed Try again last said, by the model's count.
    trust_line_seen: u64,
    /// The snackbar last drawn, by the model's count, and the one timed.
    snack_seen: u64,
    snack_timed: u64,
    /// The lines Show's code was last drawn for, and whether a code holds
    /// them: drawn again only when they change.
    show_drawn: Option<(Vec<String>, bool)>,
    /// The platform's secure flag for Show, where it has one.
    pub(crate) show_guard: Option<ShowGuard>,
    /// The platform's camera, asked to start (`true`) or stop, where it
    /// has one.
    pub(crate) camera: Option<Rc<dyn Fn(bool)>>,
    /// The last code the camera read is no address list (§5.5).
    pub(crate) viewfinder_not_address: bool,
}

/// Show's secure flag, asked of the platform ([`View::guard_show`]).
///
/// Every ask on is counted and every confirmation, which the platform
/// gives in the order asked: the addresses are drawn only once the last
/// ask on has been confirmed, so a confirmation of an earlier opening
/// secures nothing later. An ask off waits for a frame without Show and
/// is dropped if Show opened again meanwhile, so a closed opening's off
/// never follows a newer opening's on (#409 C3).
pub(crate) struct ShowGuard {
    ask: Rc<dyn Fn(bool)>,
    /// Shared with an off waiting for its frame.
    state: Rc<std::cell::Cell<GuardState>>,
}

#[derive(Clone, Copy, Default)]
struct GuardState {
    /// Asked on, and not asked off since.
    open: bool,
    /// Asks on so far.
    asked: u64,
    /// The platform's confirmations so far.
    confirmed: u64,
}

impl ShowGuard {
    pub(crate) fn new(ask: Rc<dyn Fn(bool)>) -> Self {
        Self {
            ask,
            state: Rc::default(),
        }
    }

    /// The platform confirmed an ask on; one beyond the asks is none's.
    pub(crate) fn secured(&self) {
        let mut state = self.state.get();
        state.confirmed = (state.confirmed + 1).min(state.asked);
        self.state.set(state);
    }

    /// Whether the window may draw Show's addresses: Show asked on, and
    /// its latest ask confirmed.
    fn drawable(&self) -> bool {
        let state = self.state.get();
        state.open && state.confirmed == state.asked
    }

    /// Show is open: the flag asked on, once per opening.
    fn opened(&self) {
        let mut state = self.state.get();
        if !state.open {
            state.open = true;
            state.asked += 1;
            self.state.set(state);
            (self.ask)(true);
        }
    }

    /// Show is closed: the off to run once a frame without it has been
    /// drawn, which does nothing if Show opened again by then; `None`
    /// when it was not open.
    fn closed(&self) -> Option<impl FnOnce() + 'static> {
        let mut state = self.state.get();
        if !state.open {
            return None;
        }
        state.open = false;
        self.state.set(state);
        let (ask, shared) = (Rc::clone(&self.ask), Rc::clone(&self.state));
        Some(move || {
            if !shared.get().open {
                ask(false);
            }
        })
    }
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
        window.set_add_waiting_label(set(UiText::AddWaiting));
        window.set_trust_step_body(set(UiText::TrustStepBody));
        window.set_trust_step_both(set(UiText::TrustStepBoth));
        window.set_not_now_label(set(UiText::NotNow));
        window.set_copy_peer_id_label(set(UiText::CopyPeerId));
        window.set_done_label(set(UiText::IdDone));
        window.set_no_reach_text(set(UiText::NoReachNotice));
        window.set_show_my_address_label(set(UiText::ShowMyAddress));
        window.set_show_warning(set(UiText::ShowWarning));
        window.set_copy_addresses_label(set(UiText::CopyAddresses));
        window.set_viewfinder_why(set(UiText::CameraWhy));
        window.set_viewfinder_hint(set(UiText::ScanHint));
        window.set_close_scanner_label(set(UiText::CloseScanner));
        window.set_open_settings_label(set(UiText::OpenAppSettings));
        let _ = host;
        Self {
            rows,
            row_keys: Vec::new(),
            row_handles: crate::Handles::new(),
            blocks,
            block_keys: Vec::new(),
            block_handles: crate::Handles::new(),
            focus_seen: 0,
            focus_sent: String::new(),
            focus_gone_said: false,
            gone_move: None,
            quiet_state: None,
            block_move: None,
            read_back_seen: 0,
            note_seen: 0,
            trust_line_seen: 0,
            note_text: String::new(),
            add_waiting: false,
            snack_seen: 0,
            snack_timed: 0,
            show_drawn: None,
            show_guard: None,
            camera: None,
            viewfinder_not_address: false,
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
        // The row focus is in, read before the list retires its handle.
        let held_row = self.focused_peer_row();
        let held_block = self.focused_block_control();
        // Its move applied, or replaced by another request.
        if self.focus_wanted.borrow().as_deref() != Some(self.peer_lists.focus_sent.as_str()) {
            self.peer_lists.gone_move = None;
            self.peer_lists.block_move = None;
        }
        self.render_peer_list(peers, trust, model);
        self.render_add_page(peers, trust);
        self.render_show(peers);
        let said = self.render_peers_snack(peers, on_page);
        // The add's wait, said once as it begins, claiming no outcome
        // (design 5.3; `an_add_on_its_way_reads_adding_and_is_said_once`).
        let waiting = peers.add_waiting();
        let begun = waiting && !self.peer_lists.add_waiting;
        self.peer_lists.add_waiting = waiting;
        let said = if begun && on_page {
            let wait = text(UiText::AddWaiting).to_owned();
            Some(said.map_or_else(
                || wait.clone(),
                |rest| {
                    fill(
                        text(UiText::AnnounceBoth),
                        &[("first", &wait), ("rest", &rest)],
                    )
                },
            ))
        } else {
            said
        };
        // A note under the field -- focus kept on the field, which no
        // longer carries it as its description -- is said once each time it
        // is set (design section 5.3); any other set again with the same
        // words -- an unknown add retried without a write -- changes nothing
        // a live region hears, so it is said here too (section 5.7).
        let (note, notes) = peers.add_note_said();
        let text = note.map(|n| note_text(self.host, n)).unwrap_or_default();
        let in_field = !self.window.get_field_note().is_empty();
        // A scan's own line is said once as it is set, focus staying on
        // the control it came from (§5.5).
        let scan_line = note.is_some_and(AddNote::is_scan_line);
        let again = notes != self.peer_lists.note_seen
            && !text.is_empty()
            && (in_field || scan_line || text == self.peer_lists.note_text);
        self.peer_lists.note_seen = notes;
        self.peer_lists.note_text.clone_from(&text);
        let mut said = said.or_else(|| (on_page && again).then_some(text));
        // The trust line's Try again failed: the line is said again, its
        // words perhaps unchanged, focus kept on Try again (design
        // section 8).
        let line_again = peers.trust_line_said() != self.peer_lists.trust_line_seen;
        self.peer_lists.trust_line_seen = peers.trust_line_said();
        let line = self.window.get_peers_trust_line().to_string();
        if on_page && line_again && !line.is_empty() {
            said = Some(said.map_or_else(
                || line.clone(),
                |rest| {
                    fill(
                        placeholder_en::text(UiText::AnnounceBoth),
                        &[("first", &rest), ("rest", &line)],
                    )
                },
            ));
        }
        // Focus, once per request, on Peers alone -- and judged again
        // while its deferred move is still pending: a read since may have
        // taken the row it names (design section 8). A field's request is
        // not, since resolving it sets the field's selection.
        let (focus, focuses) = peers.focus();
        let fresh = focuses != self.peer_lists.focus_seen;
        let pending = !fresh
            && !self.peer_lists.focus_sent.is_empty()
            && self.peer_lists.gone_move.is_none()
            && self.peer_lists.block_move.is_none()
            && self.focus_wanted.borrow().as_deref() == Some(self.peer_lists.focus_sent.as_str())
            && !matches!(focus, Some(PeersFocus::Field { .. }));
        if on_page && (fresh || pending) {
            self.peer_lists.focus_seen = focuses;
            if fresh {
                self.peer_lists.focus_gone_said = false;
                self.peer_lists.gone_move = None;
                self.peer_lists.block_move = None;
            }
            if let Some(focus) = focus {
                let target = self.focus_target(&focus, peers.field());
                if fresh || target != self.peer_lists.focus_sent {
                    // A row a read showed gone: why focus lands
                    // elsewhere, said first, once per request.
                    if peers.focus_row_gone() && !self.peer_lists.focus_gone_said {
                        self.peer_lists.focus_gone_said = true;
                        let gone = placeholder_en::text(UiText::PeerRowGoneAfterAct).to_owned();
                        said = Some(said.map_or_else(
                            || gone.clone(),
                            |rest| {
                                fill(
                                    placeholder_en::text(UiText::AnnounceBoth),
                                    &[("first", &gone), ("rest", &rest)],
                                )
                            },
                        ));
                    }
                    self.peer_lists.focus_sent.clone_from(&target);
                    // Peers' focus mostly lands on a control the same
                    // render makes: a step-two block, the snackbar's
                    // action, the list's controls as the add page closes.
                    self.focus_key_after_draw(target);
                }
            }
        }
        let handled = on_page && (fresh || pending);
        let free = on_page && !handled && !peers.add_open();
        // Focus on a block's Trust, Not now or problem line, which went as
        // a read the person did not ask for decided the block: where the
        // model says (design sections 5.3 and 8). Focus anywhere else
        // stays. Moved to the new line that took the problem line's
        // place, it is read there, so its live region says nothing more.
        if on_page
            && !handled
            && let Some((peer, held)) = held_block
            && let Some(target) = peers.focus_after_block_decided(trust, &peer, held)
        {
            let key = self.focus_target(&target, peers.field());
            // Judged against the move still waiting, not the last key
            // sent: an earlier move to the same place has applied and is
            // no reason to skip this one
            // (`a_move_to_the_title_an_earlier_act_sent_focus_to_is_made_again`).
            if self.focus_wanted.borrow().as_deref() != Some(key.as_str()) {
                if held == BlockHeld::Problem {
                    self.quieten_state_line(&peer);
                }
                self.peer_lists.focus_sent.clone_from(&key);
                self.peer_lists.block_move = Some(peer.clone());
                self.focus_key_after_draw(key);
            }
        }
        // A row's own Remove or Undo a read decided: focus on that row,
        // or a control of it, goes where the decision says, with neither
        // row-gone text; focus anywhere else stays (design section 8, the
        // read back after RemoveUnknown's snackbar leaves).
        let (back, backs) = peers.read_back();
        let back = (backs != self.peer_lists.read_back_seen)
            .then_some(back)
            .flatten();
        self.peer_lists.read_back_seen = backs;
        let back_row = back.map(|back| {
            if free && held_row.as_deref() == Some(back.row.as_str()) {
                self.window.set_peer_menu(-1);
                let key = self.focus_target(&back.focus, peers.field());
                self.peer_lists.focus_sent.clone_from(&key);
                self.peer_lists.gone_move = Some(back.row.clone());
                self.focus_key_after_draw(key);
            }
            back.row
        });
        // Focus still in a row, or a control in it, that a later read
        // dropped -- its request carried out, reached by the person's own
        // Tab or pointer, or placed by the app -- goes as for any row
        // gone, PeerRowGone said first (design section 8's second row-gone
        // row). Not that row's own read back, above. Focus elsewhere stays.
        if free
            && let Some(address) = held_row.clone()
            && back_row.as_deref() != Some(address.as_str())
            && let Some(target) = peers.focus_after_gone(&address)
        {
            let gone = placeholder_en::text(UiText::PeerRowGone).to_owned();
            said = Some(said.map_or_else(
                || gone.clone(),
                |rest| {
                    fill(
                        placeholder_en::text(UiText::AnnounceBoth),
                        &[("first", &gone), ("rest", &rest)],
                    )
                },
            ));
            // Its menu, if open, went with it.
            self.window.set_peer_menu(-1);
            let key = self.focus_target(&target, peers.field());
            self.peer_lists.focus_sent.clone_from(&key);
            self.peer_lists.gone_move = Some(address);
            self.focus_key_after_draw(key);
        } else if free
            && held_row.is_none()
            && let Some(row) = self.peer_lists.gone_move.clone()
            && let Some(target) = peers.focus_after_gone(&row)
        {
            // A move still scheduled whose target a later read dropped
            // too: aimed again from the row it left, said once already
            // (`a_scheduled_move_whose_target_a_read_drops_is_aimed_again`).
            let key = self.focus_target(&target, peers.field());
            if key != self.peer_lists.focus_sent {
                self.peer_lists.focus_sent.clone_from(&key);
                self.focus_key_after_draw(key);
            }
        }
        said
    }

    /// The address of the row focus is in -- on the row itself or a
    /// control in it, by the window's `focus-id` -- while its handle lives.
    fn focused_peer_row(&self) -> Option<String> {
        let id = self.window.get_focus_id();
        let handle = [
            "row:",
            "row-menu:",
            "row-trust:",
            "row-copy:",
            "row-remove:",
        ]
        .iter()
        .find_map(|prefix| id.strip_prefix(prefix))?
        .parse::<i32>()
        .ok()?;
        self.peer_lists.row_handles.key_of(handle).cloned()
    }

    /// Draw `peer`'s block's state line without its live region, as it
    /// now reads, until its words change.
    fn quieten_state_line(&mut self, peer: &TransportIdentity) {
        let blocks = &self.peer_lists.blocks;
        let Some(at) = self.peer_lists.block_keys.iter().position(|k| k == peer) else {
            return;
        };
        let Some(mut row) = blocks.row_data(at) else {
            return;
        };
        self.peer_lists.quiet_state = Some((peer.clone(), row.state.to_string()));
        row.state_live = false;
        blocks.set_row_data(at, row);
    }

    /// The peer of the step-two block whose Trust, Not now or problem
    /// line focus is on, and which, by the window's `focus-id`, while its
    /// handle lives. The id stays after the control goes, which is how a
    /// decided block is found.
    fn focused_block_control(&self) -> Option<(TransportIdentity, BlockHeld)> {
        let id = self.window.get_focus_id();
        let (handle, held) = [
            ("block-trust:", BlockHeld::Choice),
            ("block-later:", BlockHeld::Choice),
            ("block-problem:", BlockHeld::Problem),
        ]
        .iter()
        .find_map(|(prefix, held)| Some((id.strip_prefix(prefix)?, *held)))?;
        let handle = handle.parse::<i32>().ok()?;
        Some((self.peer_lists.block_handles.key_of(handle).cloned()?, held))
    }

    /// Peers' list, or why it is not on show.
    fn render_peer_list(&mut self, peers: &Peers, trust: &TrustSettings, model: &UiModel) {
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
        // A row's menu is open only while its row is drawn: a read that
        // dropped the row, wherever focus is, takes its menu with it, or
        // Back would close a menu no one sees
        // (`a_row_menu_whose_row_a_read_drops_goes_with_it`).
        if window.get_peer_menu() >= 0
            && self
                .peer_lists
                .row_handles
                .key_of(window.get_peer_menu())
                .is_none()
        {
            window.set_peer_menu(-1);
        }
        let drawn: Vec<PeerAddressRow> = rows
            .iter()
            .map(|r| {
                let short = r
                    .peer
                    .as_ref()
                    .map(|p| short_peer(p.as_str()))
                    .unwrap_or_default();
                let names: Vec<String> = r
                    .peer
                    .as_ref()
                    .map(|p| model.contact_names_on(p))
                    .unwrap_or_default();
                PeerAddressRow {
                    handle: self.peer_lists.row_handles.of(&r.address),
                    peer: short.clone().into(),
                    names: ModelRc::new(VecModel::from(
                        names
                            .into_iter()
                            .map(SharedString::from)
                            .collect::<Vec<_>>(),
                    )),
                    host: host_port(&r.address).into(),
                    address: r.address.clone().into(),
                    // Trust unknown: no state at all, neither trusted nor
                    // not, the trust line saying why (section 5.2).
                    state: match r.state {
                        RowState::Trusted => text(UiText::PeerStateTrusted),
                        RowState::NeedsTrust => text(UiText::PeerStateNeedsTrust),
                        RowState::ProviderOff => text(UiText::PeerStateProviderOff),
                        RowState::TrustUnknown => "",
                        RowState::ThisDevice => text(UiText::PeerStateThisDevice),
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
    /// Show my address (§5.4): the code and its lines, why there is none,
    /// or why they could not be read.
    fn render_show(&mut self, peers: &Peers) {
        let show = peers.show();
        // The flag before any address can be drawn; off once a frame
        // without Show has been, never before.
        let mut held = false;
        let mut clear = None;
        if let Some(guard) = self.peer_lists.show_guard.as_ref() {
            if show.is_some() {
                guard.opened();
                held = !guard.drawable();
            } else {
                clear = guard.closed();
            }
        }
        if let Some(off) = clear {
            let _ = self.after_next_frame(off);
        }
        let window = &self.window;
        window.set_peers_show_open(show.is_some());
        let host = self.host;
        // No address: what, and what to do; no reason, which the read
        // does not report (§5.4).
        let none = matches!(show, Some(ShowState::NoAddress));
        let (problem, retry) = match show {
            Some(ShowState::Unread(ShowProblem::Failed)) => (text(UiText::ShowReadFailed), true),
            Some(ShowState::Unread(ShowProblem::Incompatible)) => (
                placeholder_en::host_text(host, HostText::ShowIncompatible),
                false,
            ),
            Some(ShowState::Unread(ShowProblem::NotPermitted)) => (
                placeholder_en::host_text(host, HostText::ShowNotPermitted),
                false,
            ),
            _ => ("", false),
        };
        let (none, what) = if none {
            (text(UiText::ShowNoAddress), text(UiText::ShowNoAddressWhat))
        } else {
            ("", "")
        };
        let drawn = match show {
            Some(ShowState::Shown { peer, lines }) if !held => Some((peer, lines)),
            _ => None,
        };
        window.set_show_none(none.into());
        window.set_show_none_what(what.into());
        window.set_show_problem(problem.into());
        window.set_show_problem_retry(retry);
        let Some((peer, lines)) = drawn else {
            window.set_show_code_shown(false);
            window.set_show_lines(SharedString::new());
            window.set_show_peer(SharedString::new());
            self.peer_lists.show_drawn = None;
            return;
        };
        if self.peer_lists.show_drawn.as_ref().map(|(l, _)| l) != Some(lines) {
            // A code that does not fit is no code: no image and nothing
            // announced as one, while the lines still show (#409 r3 F2).
            let image = crate::code::drawn(lines);
            let fits = image.is_some();
            window.set_show_code(image.unwrap_or_default());
            self.peer_lists.show_drawn = Some((lines.clone(), fits));
        }
        let fits = self.peer_lists.show_drawn.as_ref().is_some_and(|(_, f)| *f);
        window.set_show_code_shown(true);
        window.set_show_code_drawn(fits);
        window.set_show_too_many(text(UiText::ShowTooManyForCode).into());
        let label = if lines.len() == 1 {
            text(UiText::QrLabelOne).to_owned()
        } else {
            fill(
                text(UiText::QrLabelOther),
                &[("n", &lines.len().to_string())],
            )
        };
        window.set_show_code_label(label.into());
        window.set_show_lines(lines.join("\n").into());
        window.set_show_peer(peer.as_str().into());
    }

    /// The scan controls this platform offers, and the reader a scan the
    /// model opened needs, started once: the desktop's image dialog or its
    /// clipboard. The camera's viewfinder is the Android app's.
    fn render_scan(&mut self, peers: &Peers) {
        let mut shared = self.shared.borrow_mut();
        let camera = peers.scan_open() && shared.scan_control == Some(ScanControl::Camera);
        let opening = camera && !self.window.get_viewfinder_open();
        self.window.set_viewfinder_open(camera);
        // Focus to the viewfinder's Close as it opens: its one stop.
        if opening {
            self.focus_key_after_draw("close-scanner".to_owned());
        }
        if !camera {
            self.peer_lists.viewfinder_not_address = false;
            // The last frame goes with the viewfinder: no picture is kept
            // (CameraWhy; #418 F3).
            self.window.set_viewfinder_has_frame(false);
            self.window.set_viewfinder_frame(slint::Image::default());
        }
        self.window.set_viewfinder_note(
            if self.peer_lists.viewfinder_not_address {
                text(UiText::ScanNotAddress)
            } else {
                ""
            }
            .into(),
        );
        if !peers.scan_open() {
            let ended = shared.scan_reading.take();
            if ended.is_some() {
                shared.scan_number = shared.scan_number.wrapping_add(1);
            }
            drop(shared);
            // The camera stops with its scan, however the scan ended.
            if ended == Some(ScanControl::Camera)
                && let Some(ask) = self.peer_lists.camera.as_ref()
            {
                ask(false);
            }
            return;
        }
        if shared.scan_reading.is_some() {
            return;
        }
        let Some(control) = shared.scan_control else {
            return;
        };
        shared.scan_reading = Some(control);
        if control == ScanControl::Camera {
            drop(shared);
            // The platform's camera, asked to start: its frames and the
            // codes read from them come back through the View
            // (`View::camera_frame`, `View::camera_read`). With none, the
            // scan ends as a cancelled one.
            match self.peer_lists.camera.as_ref() {
                Some(ask) => ask(true),
                None => enqueue(&self.shared, Input::Peers(PeersInput::ScannerClosed)),
            }
            return;
        }
        #[cfg(feature = "desktop")]
        {
            let slot = std::sync::Arc::clone(&shared.scan_result);
            let number = shared.scan_number;
            drop(shared);
            let window = slint::ComponentHandle::as_weak(&self.window);
            let done = move |input: PeersInput| {
                if let Ok(mut held) = slot.lock() {
                    *held = Some((number, input));
                }
                let _ = window.upgrade_in_event_loop(|w| w.invoke_peers_image_done());
            };
            crate::scan_images::read(control, text(UiText::CodeImageFilter).to_owned(), done);
        }
        // No reader in this build: the scan ends as a cancelled one.
        #[cfg(not(feature = "desktop"))]
        {
            drop(shared);
            enqueue(&self.shared, Input::Peers(PeersInput::ScannerClosed));
        }
    }

    fn render_add_page(&mut self, peers: &Peers, trust: &TrustSettings) {
        self.render_scan(peers);
        let window = &self.window;
        let host = self.host;
        window.set_peers_add_open(peers.add_open());
        window.set_add_waiting(peers.add_waiting());
        // The page that covers the list closes a row's menu too, whatever
        // opened the page, or Back would close a menu no one sees and
        // leave the page open (design section 5.3;
        // `a_row_menu_open_under_the_add_page_leaves_back_to_the_page`).
        if peers.add_open() {
            window.set_peer_menu(-1);
        }
        window.set_add_step_one(peers.step_one_shown());
        window.set_peers_add_busy(peers.busy());
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
            (Some(n), Some(t)) if n.is_scan_line() => (String::new(), t, String::new()),
            (Some(_), Some(n)) => (String::new(), String::new(), n),
            _ => (String::new(), String::new(), String::new()),
        };
        window.set_field_note(field.into());
        window.set_scan_note(scan.into());
        window.set_open_settings_shown(matches!(
            peers.add_page_note(),
            Some(AddNote::CameraDeniedSettings)
        ));
        window.set_add_note(under_add.into());
        // A barred add sends focus to its line, which then says it: live
        // as well, it would be heard twice.
        window.set_add_note_live(!matches!(peers.add_page_note(), Some(AddNote::Barred(_))));
        let (head, line, tail) = match peers.add_page_note() {
            Some(AddNote::Refused { line, why }) => (
                text(UiText::AddRefusedLead).to_owned(),
                if why.names_line() {
                    line.clone()
                } else {
                    String::new()
                },
                refusal_text(*why),
            ),
            _ => Default::default(),
        };
        window.set_refused_head(head.into());
        window.set_refused_line(line.into());
        window.set_refused_tail(tail.into());
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
                    later: matches!(b.state, BlockState::Later | BlockState::LaterUnknown),
                    buttons: matches!(
                        b.state,
                        BlockState::Undecided | BlockState::Later | BlockState::LaterUnknown
                    ),
                    waiting: b.state == BlockState::Trusting,
                    // Trust leaves the block for a problem no retry mends.
                    trust_offered: !matches!(
                        b.problem,
                        Some(BlockProblem::Trust(
                            TrustProblem::NotPermitted | TrustProblem::Incompatible
                        ))
                    ),
                    state: match b.state {
                        // A block whose Trust was pressed keeps its last
                        // line, its problem, not a Not now line (5.3).
                        BlockState::Later | BlockState::LaterUnknown if b.problem.is_some() => {
                            String::new()
                        }
                        BlockState::Trusted => text(UiText::TrustStepDone).to_owned(),
                        BlockState::Later => text(UiText::TrustStepLater).to_owned(),
                        BlockState::LaterUnknown => text(UiText::TrustStepLaterUnknown).to_owned(),
                        BlockState::Checking => fill(
                            placeholder_en::host_text(host, HostText::TrustUnconfirmed),
                            &[("peer", b.peer.as_str())],
                        ),
                        BlockState::Trusting => {
                            fill(text(UiText::TrustStepWaiting), &[("peer", &short)])
                        }
                        BlockState::Undecided => String::new(),
                    }
                    .into(),
                    // While its Trust is awaited the waiting line stands in
                    // place of any earlier line (design section 5.3).
                    problem: b
                        .problem
                        .filter(|_| b.state != BlockState::Trusting)
                        .map(|p| block_problem(host, p, &short))
                        .unwrap_or_default()
                        .into(),
                    state_live: true,
                }
            })
            .collect();
        // The line focus moved to stays quiet while its words stand; once
        // they change, or the block goes, it is a live region again.
        let quiet = self.peer_lists.quiet_state.take().filter(|(peer, said)| {
            blocks
                .iter()
                .zip(&drawn)
                .any(|(b, d)| &b.peer == peer && d.state.as_str() == said)
        });
        let drawn: Vec<PeerTrustBlock> = drawn
            .into_iter()
            .zip(&blocks)
            .map(|(d, b)| PeerTrustBlock {
                state_live: quiet.as_ref().is_none_or(|(peer, _)| peer != &b.peer),
                ..d
            })
            .collect();
        self.peer_lists.quiet_state = quiet;
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
        window.set_blocks_done_shown(peers.done_shown(trust));
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
        let shown = on_page && snack.is_some() && !closing;
        window.set_peers_snack_shown(shown);
        // The pointer's report goes with the snackbar it was on; a new
        // one sets it again as the pointer moves.
        if !shown {
            window.set_peers_snack_pointer_on(false);
        }
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
                // A new snackbar's action holds no focus until it says so:
                // its own report follows the id, on a new element or one
                // kept under focus.
                window.set_peers_snack_action_focused(false);
                let shared = Rc::clone(&self.shared);
                let ms = self.shared.borrow().touch.snackbar_ms;
                if superseded_snack(s) {
                    let up = Duration::from_millis(ms.max(NO_ACTION_MIN_MS));
                    close_when_free(slint::ComponentHandle::as_weak(window), shared, snacks, up);
                } else {
                    slint::Timer::single_shot(Duration::from_millis(ms), move || {
                        enqueue(&shared, Input::PeersSnackClosed(snacks));
                    });
                }
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

/// The least time up of a snackbar with no action, three sentences long
/// with nothing to keep it on screen (add-a-peer §5.7): the platform's
/// longer timeout still wins.
pub(crate) const NO_ACTION_MIN_MS: u64 = 10_000;

/// How soon a snackbar held by the pointer is looked at again.
const HELD_RECHECK: Duration = Duration::from_secs(1);

/// The two snackbars a superseded refusal says, which have no action.
const fn superseded_snack(snack: &PeersSnack) -> bool {
    matches!(
        snack,
        PeersSnack::RemoveSuperseded { .. } | PeersSnack::UndoSuperseded { .. }
    )
}

/// Close snackbar `id` once `after` has passed and the pointer is not on
/// it, looking again each [`HELD_RECHECK`] while it is (§5.7). It has no
/// control, so keyboard focus cannot be on it.
fn close_when_free(
    window: slint::Weak<AppWindow>,
    shared: Rc<RefCell<Shared>>,
    id: u64,
    after: Duration,
) {
    slint::Timer::single_shot(after, move || {
        let held = window.upgrade().is_some_and(|w| {
            w.get_peers_snack_shown()
                && i64::from(w.get_peers_snack_id()) == i64::try_from(id).unwrap_or(-1)
                && w.get_peers_snack_pointer_on()
        });
        if held {
            close_when_free(window, shared, id, HELD_RECHECK);
        } else {
            enqueue(&shared, Input::PeersSnackClosed(id));
        }
    });
}

fn note_text(host: RuntimeHost, note: &AddNote) -> String {
    match note {
        AddNote::AddressNeeded => text(UiText::AddressNeeded).to_owned(),
        AddNote::ScanAfterField => text(UiText::ScanAfterField).to_owned(),
        AddNote::ImageNoCode => text(UiText::ImageNoCode).to_owned(),
        AddNote::NotAddress => text(UiText::ScanNotAddress).to_owned(),
        AddNote::ImageUnreadable => text(UiText::ImageUnreadable).to_owned(),
        AddNote::ClipboardNoImage => text(UiText::ClipboardNoImage).to_owned(),
        AddNote::ClipboardUnreadable => text(UiText::ClipboardUnreadable).to_owned(),
        AddNote::CameraDenied => text(UiText::CameraDenied).to_owned(),
        AddNote::CameraDeniedSettings => text(UiText::CameraDeniedSettings).to_owned(),
        AddNote::Refused { line, why } if why.names_line() => fill(
            text(UiText::AddRefused),
            &[("line", line), ("reason", &refusal_text(*why))],
        ),
        // A refusal of the whole add names no line: the lead and the
        // reason read together (§5.3).
        AddNote::Refused { why, .. } => {
            format!("{} {}", text(UiText::AddRefusedLead), refusal_text(*why))
        }
        AddNote::TooManyLines { lines } => fill(
            text(UiText::RefusedTooManyLines),
            &[("n", &lines.to_string()), ("max", &TYPED_LINES.to_string())],
        ),
        AddNote::Failed => text(UiText::AddFailed).to_owned(),
        // Nothing is said until the list read back says it.
        AddNote::Checking => String::new(),
        AddNote::Unknown { lines: 1 } => text(UiText::AddUnknownOne).to_owned(),
        AddNote::Unknown { .. } => text(UiText::AddUnknownOther).to_owned(),
        AddNote::Superseded { lines: 1 } => text(UiText::AddSupersededOne).to_owned(),
        AddNote::Superseded { .. } => text(UiText::AddSupersededOther).to_owned(),
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
        BlockProblem::Superseded => fill(text(UiText::TrustStepSuperseded), &[("peer", short)]),
        BlockProblem::SupersededUnread => {
            fill(text(UiText::TrustStepSupersededUnread), &[("peer", short)])
        }
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
            PeersFocus::Scan(ScanControl::Camera) => "scan-code".to_owned(),
            PeersFocus::Scan(ScanControl::OpenImage) => "open-image".to_owned(),
            PeersFocus::Scan(ScanControl::PasteImage) => "paste-image".to_owned(),
            PeersFocus::Block(peer) => format!("block:{}", self.peer_lists.block_handles.of(peer)),
            PeersFocus::BlockProblem(peer) => {
                format!("block-problem:{}", self.peer_lists.block_handles.of(peer))
            }
            PeersFocus::BlockState(peer) => {
                format!("block-state:{}", self.peer_lists.block_handles.of(peer))
            }
            PeersFocus::Done => "peers-done".to_owned(),
            PeersFocus::Opener | PeersFocus::AddAddress => "add-address".to_owned(),
            PeersFocus::Row(address) => format!("row:{}", self.peer_lists.row_handles.of(address)),
            PeersFocus::RowMenu(address) => {
                format!("row-menu:{}", self.peer_lists.row_handles.of(address))
            }
            PeersFocus::AddAddressNote => "add-address-note".to_owned(),
            PeersFocus::ShowWarning => "show-warning".to_owned(),
            PeersFocus::ShowProblem => "show-problem".to_owned(),
            PeersFocus::ShowOpener => "show-my-address".to_owned(),
            PeersFocus::ListProblem => "peers-problem".to_owned(),
            PeersFocus::TrustRetry => "trust-retry".to_owned(),
            PeersFocus::TrustLine => "trust-line".to_owned(),
            PeersFocus::AddNote => "add-note".to_owned(),
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
        // Each refusal by its own words (design section 5.2): the
        // device's bound and the list off as Add an address says them,
        // any refusal of the address itself as the add page does.
        PeersSnack::UndoRefused(Refusal::ListOff) => text(UiText::PeersListOff),
        PeersSnack::UndoRefused(Refusal::Limit) => {
            return fill(
                text(UiText::PeersLimit),
                &[("max", &ADDRESS_BOUND.to_string())],
            );
        }
        PeersSnack::UndoRefused(
            why @ (Refusal::Grammar
            | Refusal::SpecialUse
            | Refusal::DnsFromScan
            | Refusal::TooMany
            | Refusal::OwnAddress),
        ) => return refusal_text(*why),
        PeersSnack::UndoFailed { .. } => text(UiText::UndoFailed),
        PeersSnack::UndoUnknown { .. } => text(UiText::UndoUnknown),
        PeersSnack::RemoveSuperseded { .. } => text(UiText::RemoveSuperseded),
        PeersSnack::UndoSuperseded { .. } => text(UiText::UndoSuperseded),
        PeersSnack::Added => text(UiText::ConfirmAddressAdded),
        // Step one's own line, as the page closes on it (section 5.3).
        PeersSnack::AddedUnchanged { lines } => {
            return result_text(&AddResult::Unchanged { lines: *lines });
        }
        PeersSnack::AddedChecked { lines } => {
            return result_text(&AddResult::Checked { lines: *lines });
        }
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

#[cfg(test)]
mod show_guard_tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::ShowGuard;

    fn guard() -> (ShowGuard, Rc<RefCell<Vec<bool>>>) {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&asked);
        (
            ShowGuard::new(Rc::new(move |on| log.borrow_mut().push(on))),
            asked,
        )
    }

    /// A closed opening's off, run after Show opened again, asks
    /// nothing: the newer opening's on is never followed by it (#409 C3).
    #[test]
    fn a_closed_openings_off_never_follows_a_newer_on() {
        let (g, asked) = guard();
        g.opened();
        g.secured();
        let off = g.closed().expect("it was open");
        g.opened();
        off();
        assert_eq!(*asked.borrow(), vec![true, true], "no off after the new on");
        // Closed for good, its off runs.
        let off = g.closed().expect("open again");
        off();
        assert_eq!(*asked.borrow(), vec![true, true, false]);
    }

    /// Only the latest ask's confirmation lets the addresses be drawn: an
    /// earlier opening's, arriving late, secures nothing (#409 C3).
    #[test]
    fn only_the_latest_asks_confirmation_draws() {
        let (g, _) = guard();
        g.opened();
        let _ = g.closed();
        g.opened();
        assert!(!g.drawable());
        g.secured();
        assert!(!g.drawable(), "the first opening's confirmation");
        g.secured();
        assert!(g.drawable(), "the second's");
        g.secured();
        let _ = g.closed();
        g.opened();
        assert!(
            !g.drawable(),
            "a confirmation beyond the asks counts for none"
        );
    }
}

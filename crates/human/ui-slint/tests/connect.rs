// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Onboarding's Connect, drawn (`design/2026-10-22-add-a-peer.md` §5.1):
//! the window driven as a screen reader drives it, by each control's
//! accessible label and default action, the runtime's answers given as
//! the facade would hand them over. What the window shows, where focus is
//! asked to go and what is said are checked; no platform's accessibility
//! tool reads this tree here, and no phone runs it.

#![allow(clippy::expect_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use interweave_human_client_api::{
    AddressAnswer, AddressList, AddressOrigin, AddressOutcome, PeerAddress, TrustList, TrustRow,
};
use interweave_human_ui_model::{
    ConnectGate, Intent, RuntimeHost, TrustChange, UiModel, UiText, placeholder_en,
};
use interweave_human_ui_slint::{View, ViewEvent};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use slint::ComponentHandle as _;

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer")
}

fn at(p: &TransportIdentity) -> String {
    format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", p.as_str())
}

fn added(p: &TransportIdentity) -> PeerAddress {
    PeerAddress {
        address: at(p),
        peer: Some(p.clone()),
        origin: AddressOrigin::AddedHere,
        dialled: true,
    }
}

fn trust_list(me: &TransportIdentity, trusted: &[TransportIdentity]) -> TrustList {
    TrustList {
        local_peer: Some(me.clone()),
        allowed: trusted.iter().cloned().map(TrustRow::added_here).collect(),
    }
}

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

fn view() -> View {
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    view
}

/// The root's loop: take, apply, take again, until empty.
fn intents(view: &mut View, model: &mut UiModel) -> Vec<Intent> {
    let mut out = Vec::new();
    loop {
        let events = view.take_events(model);
        if events.is_empty() {
            view.render(model);
            settle();
            return out;
        }
        for event in events {
            match event {
                ViewEvent::Intent(intent) => out.push(intent),
                ViewEvent::DraftChanged { key, draft } => model.draft_changed(key, draft),
                ViewEvent::Trust(input) => out.extend(model.trust_settings_mut().input(input)),
                ViewEvent::Peers(input) => out.extend(model.peers_input(input)),
                ViewEvent::Connect(input) => out.extend(model.connect_input(input)),
                ViewEvent::Undo(id) => out.extend(model.undo(id)),
                ViewEvent::SnackbarClosed(id) => model.snackbar_closed(id),
                ViewEvent::IdentifierCopied(key) => model.identifier_copied(key),
            }
        }
        view.render(model);
    }
}

/// A focus moves on a timer's turn and is confirmed a turn later.
fn settle() {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
}

fn announcement(e: &ElementHandle) -> bool {
    e.id()
        .is_some_and(|id| id.ends_with("announce-a") || id.ends_with("announce-b"))
}

fn labelled(view: &View, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label)
        .filter(|e| !announcement(e))
        .collect()
}

fn press(view: &View, label: &str) {
    let found = labelled(view, label);
    assert_eq!(found.len(), 1, "one control labelled {label:?}");
    found[0].invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
}

fn said(view: &View) -> String {
    let w = view.window();
    format!("{} {}", w.get_announcement_a(), w.get_announcement_b())
}

fn focus(view: &View) -> String {
    view.window().get_focus_target().to_string()
}

/// A new identity's first start whose two reads found both lists empty:
/// Connect open and drawn.
fn connect_open(view: &mut View, me: &TransportIdentity) -> UiModel {
    let mut model = UiModel::new();
    model.connect_owed();
    assert_eq!(model.read_addresses(), Some(Intent::ReadAddresses));
    assert_eq!(model.trust_settings_mut().opened(), Some(Intent::ReadTrust));
    model.trust_settings_mut().read(Ok(trust_list(me, &[])));
    model.addresses_read(Ok(AddressList::default()));
    assert_eq!(model.connect_turn(), Some(ConnectGate::Open));
    view.render(&model);
    settle();
    model
}

/// Add an address from Connect, typed, answered added with `them` in the
/// list after: step two drawn in Peers' place.
fn add_from_connect(view: &mut View, model: &mut UiModel, them: &TransportIdentity) {
    press(view, text(UiText::AddAddress));
    assert!(intents(view, model).is_empty());
    assert!(
        view.window().get_peers_page(),
        "the add page in Peers' place"
    );
    assert!(view.window().get_peers_add_open());
    view.window().invoke_address_edited(at(them).into());
    press(view, text(UiText::AddAction));
    assert_eq!(
        intents(view, model),
        vec![Intent::AddAddresses(vec![at(them)])]
    );
    model.addresses_added(
        &[at(them)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(them.clone()),
            },
            list: Ok(AddressList {
                addresses: vec![added(them)],
            }),
        }),
    );
    view.render(model);
    settle();
}

/// §5.1 and §8: Connect opens over the panes with focus on its heading,
/// its two actions and Not now and no line; Not now leaves for Chats, its
/// title focused.
#[test]
fn connect_opens_on_its_heading_and_not_now_leaves_for_chats() {
    let mut view = view();
    let me = peer();
    let mut model = connect_open(&mut view, &me);
    assert!(view.window().get_connect_open());
    assert!(!view.window().get_peers_page());
    assert_eq!(focus(&view), "connect-title");
    assert_eq!(labelled(&view, text(UiText::ConnectTitle)).len(), 1);
    assert_eq!(labelled(&view, text(UiText::ConnectBody)).len(), 1);
    assert_eq!(labelled(&view, text(UiText::AddAddress)).len(), 1);
    assert_eq!(labelled(&view, text(UiText::ShowMyAddress)).len(), 1);
    assert!(labelled(&view, text(UiText::IdContinue)).is_empty());
    assert_eq!(view.window().get_connect_line().as_str(), "");

    press(&view, text(UiText::NotNow));
    assert!(intents(&mut view, &mut model).is_empty());
    assert!(model.connect().is_none());
    assert!(!view.window().get_connect_open());
    assert!(
        view.window().get_list_shown(),
        "Chats, on a narrow window too"
    );
    assert_eq!(focus(&view), "heading", "Chats' title (§8)");
    assert_eq!(model.connect_turn(), Some(ConnectGate::Settled));
    assert!(model.no_reach(), "Chats then shows the no-reach notice");
}

/// The platform's Back is Not now on Connect, and on a page Connect
/// opened it is that page's own Back: Connect comes back, focus on the
/// control that opened the page.
#[test]
fn back_closes_the_page_connect_opened_then_leaves_connect() {
    let mut view = view();
    let me = peer();
    let mut model = connect_open(&mut view, &me);
    press(&view, text(UiText::AddAddress));
    assert!(intents(&mut view, &mut model).is_empty());
    assert!(view.window().get_peers_add_open());

    assert!(view.window().invoke_handle_back());
    assert!(intents(&mut view, &mut model).is_empty());
    assert!(!model.peers().add_open());
    assert!(model.connect().is_some(), "Connect, not Chats");
    assert!(!view.window().get_peers_page());
    assert_eq!(focus(&view), "connect-add");
    assert_eq!(
        view.window().get_connect_line().as_str(),
        "",
        "nothing added"
    );

    assert!(view.window().invoke_handle_back());
    assert!(intents(&mut view, &mut model).is_empty());
    assert!(model.connect().is_none());
    assert_eq!(focus(&view), "heading");
}

/// An add left untrusted: Connect's line says so, is said once, and
/// offers Trust; Trust reopens step two and, once made, the line says
/// trusted and Continue -- the primary in Not now's place -- takes focus
/// and leaves for Chats.
#[test]
fn an_untrusted_add_offers_trust_then_continue() {
    let mut view = view();
    let (me, them) = (peer(), peer());
    let mut model = connect_open(&mut view, &me);
    add_from_connect(&mut view, &mut model, &them);
    // Step two's Not now, the add page's own.
    press(&view, text(UiText::NotNow));
    assert!(intents(&mut view, &mut model).is_empty());
    assert!(!view.window().get_peers_page());
    let needs = text(UiText::ConnectDoneNeedsTrust);
    assert_eq!(view.window().get_connect_line().as_str(), needs);
    assert!(view.window().get_connect_line_trust());
    assert!(said(&view).contains(needs), "said: {}", said(&view));
    assert_eq!(focus(&view), "connect-add");
    assert_eq!(labelled(&view, text(UiText::IdContinue)).len(), 1);
    assert!(
        labelled(&view, text(UiText::NotNow)).is_empty(),
        "Continue in its place"
    );

    press(&view, text(UiText::TrustThisPeer));
    assert!(intents(&mut view, &mut model).is_empty());
    assert!(view.window().get_peers_add_open(), "step two again");
    press(&view, text(UiText::TrustThisPeer));
    let change = TrustChange {
        peer: them.clone(),
        allowed: true,
    };
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::SetTrust(change.clone())]
    );
    model
        .trust_settings_mut()
        .set(change, Ok(trust_list(&me, std::slice::from_ref(&them))));
    assert_eq!(model.peers_turn(), None);
    view.render(&model);
    settle();
    assert!(!view.window().get_peers_page());
    let trusted = text(UiText::ConnectDoneTrusted);
    assert_eq!(view.window().get_connect_line().as_str(), trusted);
    assert!(!view.window().get_connect_line_trust());
    assert!(labelled(&view, text(UiText::TrustThisPeer)).is_empty());
    assert_eq!(focus(&view), "connect-continue", "Trust gone: Continue");

    press(&view, text(UiText::IdContinue));
    assert!(intents(&mut view, &mut model).is_empty());
    assert!(model.connect().is_none());
    assert_eq!(focus(&view), "heading");
}

/// A return to Connect from a page that did something §5.1 counts says
/// the line, the same words again included, with no snackbar; a return
/// that did nothing counted says nothing, a line still showing included.
#[test]
fn a_return_says_the_line_when_its_page_did_something_counted() {
    let mut view = view();
    let (me, them) = (peer(), peer());
    let mut model = connect_open(&mut view, &me);
    let clear = |view: &View| {
        view.window().set_announcement_a("".into());
        view.window().set_announcement_b("".into());
    };
    press(&view, text(UiText::ShowMyAddress));
    let _ = intents(&mut view, &mut model);
    clear(&view);
    assert!(view.window().invoke_handle_back(), "Show's Back");
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        view.window().get_connect_line().as_str(),
        "",
        "nothing drawn"
    );
    assert_eq!(said(&view).trim(), "", "no line, nothing said");

    add_from_connect(&mut view, &mut model, &them);
    press(&view, text(UiText::NotNow));
    let _ = intents(&mut view, &mut model);
    let needs = text(UiText::ConnectDoneNeedsTrust);
    assert!(said(&view).contains(needs), "said: {}", said(&view));
    assert!(!view.window().get_peers_snack_shown(), "no snackbar");
    clear(&view);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    assert!(view.window().invoke_handle_back(), "the add page's Back");
    let _ = intents(&mut view, &mut model);
    assert_eq!(view.window().get_connect_line().as_str(), needs, "still");
    assert_eq!(said(&view).trim(), "", "nothing added: nothing said");
    clear(&view);
    press(&view, text(UiText::TrustThisPeer));
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_peers_add_open());
    assert_eq!(said(&view).trim(), "", "nothing of Connect's while away");
    press(&view, text(UiText::NotNow));
    let _ = intents(&mut view, &mut model);
    assert_eq!(view.window().get_connect_line().as_str(), needs, "the same");
    assert!(said(&view).contains(needs), "said again: {}", said(&view));
    assert!(!view.window().get_peers_snack_shown(), "no snackbar");
}

/// A Trust lost as step two closed leaves the line's Trust, with focus
/// on it once step two was reopened from there; its answer read late
/// takes Trust away and focus goes to Continue, never to nothing.
#[test]
fn a_late_trust_answer_takes_trust_away_and_focus_goes_to_continue() {
    let mut view = view();
    let (me, them) = (peer(), peer());
    let mut model = connect_open(&mut view, &me);
    add_from_connect(&mut view, &mut model, &them);
    press(&view, text(UiText::NotNow));
    let _ = intents(&mut view, &mut model);
    press(&view, text(UiText::TrustThisPeer));
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_peers_add_open(), "step two again");
    press(&view, text(UiText::TrustThisPeer));
    let change = TrustChange {
        peer: them.clone(),
        allowed: true,
    };
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::SetTrust(change.clone())]
    );
    assert!(view.window().invoke_handle_back(), "Back while awaited");
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        view.window().get_connect_line().as_str(),
        text(UiText::ConnectDoneTrustUnknown)
    );
    assert_eq!(view.window().get_focus_id().as_str(), "connect-trust");

    model
        .trust_settings_mut()
        .set(change, Ok(trust_list(&me, std::slice::from_ref(&them))));
    assert_eq!(model.connect_turn(), None);
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        view.window().get_connect_line().as_str(),
        text(UiText::ConnectDoneTrusted)
    );
    assert!(!view.window().get_connect_line_trust());
    assert_eq!(focus(&view), "connect-continue");
    assert_eq!(view.window().get_focus_id().as_str(), "connect-continue");
}

/// Every element of `view` with a label and a size lies inside `width`.
fn overflowing(view: &View, width: f32) -> Vec<String> {
    i_slint_backend_testing::ElementQuery::from_root(view.window())
        .find_all()
        .into_iter()
        .filter(|e| e.accessible_label().is_some_and(|l| !l.is_empty()) && e.size().width > 0.0)
        .filter(|e| !announcement(e))
        .filter(|e| e.absolute_position().x + e.size().width > width + 0.5)
        .map(|e| format!("{:?}", e.accessible_label()))
        .collect()
}

/// A phone's narrowest window, 320 px: Connect with its longest line and
/// Trust beside it keeps every element inside the window.
#[test]
fn connect_fits_a_320_px_window() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(320.0, 640.0));
    slint::platform::update_timers_and_animations();
    let (me, them) = (peer(), peer());
    let mut model = connect_open(&mut view, &me);
    assert_eq!(overflowing(&view, 320.0), Vec::<String>::new(), "opened");
    add_from_connect(&mut view, &mut model, &them);
    press(&view, text(UiText::NotNow));
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        view.window().get_connect_line().as_str(),
        text(UiText::ConnectDoneNeedsTrust)
    );
    assert_eq!(overflowing(&view, 320.0), Vec::<String>::new(), "a line");
}

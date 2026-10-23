// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Peers by address, drawn (`design/2026-10-22-add-a-peer.md`): the
//! window driven as a screen reader drives it, by each control's
//! accessible label and default action, its answers given as the facade
//! would hand them over. What the window shows and asks is checked; the
//! runtime's rules are not, and no platform's accessibility tool reads
//! this tree here.

#![allow(clippy::expect_used, clippy::panic)]

use i_slint_backend_testing::ElementHandle;
use interweave_human_client_api::{
    AddressAnswer, AddressList, AddressOrigin, AddressOutcome, AddressRefused, PeerAddress,
    TrustList, TrustRow,
};
use interweave_human_ui_model::{
    Intent, RuntimeHost, TrustChange, UiModel, UiText, fill, placeholder_en, short_peer,
};
use interweave_human_ui_slint::{View, ViewEvent};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;
use slint::{ComponentHandle as _, Model as _};

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

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

fn view() -> View {
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Daemon).expect("a window");
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
            return out;
        }
        for event in events {
            match event {
                ViewEvent::Intent(intent) => out.push(intent),
                ViewEvent::DraftChanged { key, draft } => model.draft_changed(key, draft),
                ViewEvent::Trust(input) => out.extend(model.trust_settings_mut().input(input)),
                ViewEvent::Peers(input) => out.extend(model.peers_input(input)),
                ViewEvent::Undo(id) => out.extend(model.undo(id)),
                ViewEvent::SnackbarClosed(id) => model.snackbar_closed(id),
                ViewEvent::IdentifierCopied(key) => model.identifier_copied(key),
            }
        }
        view.render(model);
    }
}

fn labelled(view: &View, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label).collect()
}

fn press(view: &View, label: &str) {
    let found = labelled(view, label);
    assert!(!found.is_empty(), "a control labelled {label:?}");
    found[0].invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
}

/// Peers from the app's options, both lists read in.
fn open_peers(
    view: &mut View,
    model: &mut UiModel,
    me: &TransportIdentity,
    trusted: &[TransportIdentity],
    rows: &[PeerAddress],
) {
    press(view, text(UiText::AppOptions));
    press(view, text(UiText::PeersTitle));
    assert_eq!(
        intents(view, model),
        vec![Intent::ReadTrust, Intent::ReadAddresses]
    );
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me.clone()),
        allowed: trusted.iter().cloned().map(TrustRow::added_here).collect(),
    }));
    model.addresses_read(Ok(AddressList {
        addresses: rows.to_vec(),
    }));
    view.render(model);
}

/// Each row says its peer in short, the host and port, what this device
/// does with the address, and where it comes from; a row needing trust
/// offers Trust.
#[test]
fn each_row_says_its_peer_host_state_and_origin() {
    let mut view = view();
    let mut model = UiModel::new();
    let (me, t, u) = (peer(), peer(), peer());
    let configured = PeerAddress {
        origin: AddressOrigin::Configured,
        ..added(&t)
    };
    open_peers(
        &mut view,
        &mut model,
        &me,
        std::slice::from_ref(&t),
        &[configured, added(&u)],
    );
    assert!(view.window().get_peers_page());
    let rows = view.window().get_peer_rows();
    let drawn: Vec<(String, String, String, String, bool, bool)> = (0..rows.row_count())
        .map(|i| {
            let r = rows.row_data(i).expect("a row");
            (
                r.peer.to_string(),
                r.host.to_string(),
                r.state.to_string(),
                r.origin.to_string(),
                r.needs_trust,
                r.removable,
            )
        })
        .collect();
    assert_eq!(
        drawn,
        vec![
            (
                short_peer(u.as_str()),
                "192.168.1.20:4001".to_owned(),
                text(UiText::PeerStateNeedsTrust).to_owned(),
                text(UiText::TrustAddedHere).to_owned(),
                true,
                true,
            ),
            (
                short_peer(t.as_str()),
                "192.168.1.20:4001".to_owned(),
                text(UiText::PeerStateTrusted).to_owned(),
                text(UiText::PeerFromConfig).to_owned(),
                false,
                false,
            ),
        ]
    );
    let menu = fill(text(UiText::PeerMenu), &[("peer", &short_peer(u.as_str()))]);
    assert_eq!(
        labelled(&view, &menu).len(),
        1,
        "each row's menu names its peer"
    );
}

/// A typed address goes out whole from the add page; its untrusted peer
/// gets a block naming the full `PeerId`, whose Trust is the confirmed
/// change.
#[test]
fn a_typed_address_is_added_and_its_peer_trusted_from_the_page() {
    let mut view = view();
    let mut model = UiModel::new();
    let (me, them) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    assert_eq!(
        view.window().get_peers_empty().as_str(),
        text(UiText::PeersEmpty)
    );
    press(&view, text(UiText::AddAddress));
    assert!(intents(&mut view, &mut model).is_empty());
    assert!(view.window().get_peers_add_open());
    view.window()
        .invoke_address_edited(format!("  {}\n", at(&them)).into());
    press(&view, text(UiText::AddAction));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::AddAddresses(vec![at(&them)])]
    );
    model.addresses_added(
        &[at(&them)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(them.clone()),
            },
            list: Ok(AddressList {
                addresses: vec![added(&them)],
            }),
        }),
    );
    view.render(&model);
    let short = short_peer(them.as_str());
    assert_eq!(
        view.window().get_add_result().as_str(),
        fill(text(UiText::AddedLineOne), &[("peer", &short)])
    );
    let blocks = view.window().get_trust_blocks();
    let block = blocks.row_data(0).expect("a block");
    assert_eq!(block.peer.as_str(), them.as_str(), "the full PeerId");
    assert_eq!(
        block.title.as_str(),
        fill(text(UiText::TrustStepTitle), &[("peer", &short)])
    );
    assert!(
        labelled(&view, them.as_str())
            .iter()
            .any(|e| e.accessible_label().as_deref() == Some(them.as_str()))
    );
    let trust = labelled(&view, text(UiText::TrustThisPeer));
    assert_eq!(trust.len(), 1, "one block, one Trust");
    assert_eq!(
        trust[0].accessible_description().as_deref(),
        Some(them.as_str())
    );
    trust[0].invoke_accessible_default_action();
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::SetTrust(TrustChange {
            peer: them,
            allowed: true
        })]
    );
}

/// A refusal is said under the field, naming the line and its reason;
/// nothing else is shown as done.
#[test]
fn a_refused_address_is_named_under_the_field() {
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    let line = "/ip4/127.0.0.1/tcp/1/p2p/x".to_owned();
    view.window().invoke_address_edited(line.clone().into());
    press(&view, text(UiText::AddAction));
    let _ = intents(&mut view, &mut model);
    model.addresses_added(
        std::slice::from_ref(&line),
        Ok(AddressAnswer {
            outcome: AddressOutcome::Refused(AddressRefused::SpecialUse),
            list: Ok(AddressList::default()),
        }),
    );
    view.render(&model);
    assert_eq!(
        view.window().get_field_note().as_str(),
        fill(
            text(UiText::AddRefused),
            &[("line", &line), ("reason", text(UiText::RefusedSpecialUse))]
        )
    );
    assert_eq!(view.window().get_add_result().as_str(), "");
    assert_eq!(view.window().get_address_text().as_str(), line, "kept");
}

/// Chats says it can reach no one while both lists are read and empty,
/// and its action opens Peers; an address read in hides it.
#[test]
fn the_no_reach_notice_opens_peers_and_goes_with_an_address() {
    let mut view = view();
    let mut model = UiModel::new();
    let (me, them) = (peer(), peer());
    let _ = model.trust_settings_mut().opened();
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me),
        allowed: Vec::new(),
    }));
    let _ = model.read_addresses();
    model.addresses_read(Ok(AddressList::default()));
    view.render(&model);
    assert!(view.window().get_no_reach_shown());
    assert_eq!(
        labelled(&view, text(UiText::NoReachNotice)).len(),
        1,
        "read by a screen reader"
    );
    press(&view, text(UiText::PeersTitle));
    let asked = intents(&mut view, &mut model);
    assert!(asked.contains(&Intent::ReadAddresses), "{asked:?}");
    assert!(view.window().get_peers_page());

    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&them)],
    }));
    view.render(&model);
    assert!(!view.window().get_no_reach_shown());
}

/// Remove from a row's menu goes out at once; its snackbar offers Undo,
/// which adds the address again.
#[test]
fn remove_from_a_rows_menu_offers_undo() {
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a)]);
    let menu = fill(text(UiText::PeerMenu), &[("peer", &short_peer(a.as_str()))]);
    press(&view, &menu);
    press(&view, text(UiText::RemoveAddress));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::RemoveAddress(at(&a))]
    );
    model.address_removed(
        &at(&a),
        Ok(AddressAnswer {
            outcome: AddressOutcome::Removed,
            list: Ok(AddressList::default()),
        }),
    );
    view.render(&model);
    assert!(view.window().get_peers_snack_shown());
    assert_eq!(
        view.window().get_peers_snack_text().as_str(),
        text(UiText::ConfirmAddressRemoved)
    );
    press(&view, text(UiText::Undo));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::AddAddresses(vec![at(&a)])]
    );
}

/// Every labelled element of `view` that is laid out lies inside the
/// window's width.
fn overflowing(view: &View, width: f32) -> Vec<String> {
    i_slint_backend_testing::ElementQuery::from_root(view.window())
        .find_all()
        .into_iter()
        .filter(|e| e.accessible_label().is_some_and(|l| !l.is_empty()) && e.size().width > 0.0)
        .filter(|e| e.absolute_position().x + e.size().width > width + 0.5)
        .map(|e| {
            format!(
                "{:?} at {} + {}",
                e.accessible_label(),
                e.absolute_position().x,
                e.size().width
            )
        })
        .collect()
}

/// A phone's narrowest window, 320 px: Peers with a configuration row and
/// an added one, then the add page with a refusal and a step-two block,
/// keep every element inside the window.
#[test]
fn peers_and_the_add_page_fit_a_320_px_window() {
    let mut view = view();
    let mut model = UiModel::new();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(320.0, 800.0));
    slint::platform::update_timers_and_animations();
    let (me, c, a) = (peer(), peer(), peer());
    let configured = PeerAddress {
        origin: AddressOrigin::Configured,
        address: format!(
            "/ip6/2001:db8:85a3:8d3:1319:8a2e:370:7348/udp/4001/quic-v1/p2p/{}",
            c.as_str()
        ),
        ..added(&c)
    };
    open_peers(&mut view, &mut model, &me, &[], &[configured, added(&a)]);
    let menu = fill(text(UiText::PeerMenu), &[("peer", &short_peer(c.as_str()))]);
    press(&view, &menu);
    view.render(&model);
    assert_eq!(overflowing(&view, 320.0), Vec::<String>::new(), "Peers");

    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    let long = format!(
        "/ip4/203.0.113.7/tcp/4001/p2p/{}/p2p-circuit/p2p/{}",
        peer().as_str(),
        a.as_str()
    );
    view.window().invoke_address_edited(long.clone().into());
    press(&view, text(UiText::AddAction));
    let _ = intents(&mut view, &mut model);
    model.addresses_added(
        std::slice::from_ref(&long),
        Ok(AddressAnswer {
            outcome: AddressOutcome::Refused(AddressRefused::Grammar),
            list: Ok(AddressList::default()),
        }),
    );
    view.render(&model);
    assert_eq!(overflowing(&view, 320.0), Vec::<String>::new(), "a refusal");

    let them = peer();
    view.window().invoke_address_edited(at(&them).into());
    press(&view, text(UiText::AddAction));
    let _ = intents(&mut view, &mut model);
    model.addresses_added(
        &[at(&them)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(them.clone()),
            },
            list: Ok(AddressList {
                addresses: vec![added(&them)],
            }),
        }),
    );
    view.render(&model);
    assert_eq!(view.window().get_trust_blocks().row_count(), 1);
    assert_eq!(overflowing(&view, 320.0), Vec::<String>::new(), "step two");
}

/// A row's Trust opens the trust step for its peer alone: no field and
/// no Add above the block, so nothing drawn is a control a press would
/// not reach. Add an address draws step one.
#[test]
fn a_rows_trust_draws_the_trust_step_without_step_one() {
    let mut view = view();
    let mut model = UiModel::new();
    let (me, u) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&u)]);
    let row = view.window().get_peer_rows().row_data(0).expect("a row");
    assert!(row.needs_trust);
    view.window().invoke_row_trust(row.handle);
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_peers_add_open());
    assert_eq!(view.window().get_trust_blocks().row_count(), 1);
    assert!(!view.window().get_add_step_one());
    assert!(
        labelled(&view, text(UiText::AddAction)).is_empty(),
        "no Add above the trust step"
    );

    // A second window: the testing platform is set up once per thread.
    let mut view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window().show().expect("shown");
    let mut model = UiModel::new();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_add_step_one());
    assert!(!labelled(&view, text(UiText::AddAction)).is_empty());
}

/// The trust list unread: every row is drawn with no trust state, the
/// trust-unread line stands where the line on what trust means would,
/// and its Try again reads trust again (section 5.2's trust unread).
#[test]
fn an_unread_trust_list_draws_its_line_and_no_row_state() {
    let mut view = view();
    let mut model = UiModel::new();
    let (me, t) = (peer(), peer());
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::PeersTitle));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::ReadTrust, Intent::ReadAddresses]
    );
    model
        .trust_settings_mut()
        .read(Err(interweave_human_client_api::TrustProblem::Unavailable));
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&t)],
    }));
    view.render(&model);

    let row = view.window().get_peer_rows().row_data(0).expect("a row");
    assert_eq!(row.state, "", "no trust state claimed");
    assert!(row.needs_trust, "Trust stays offered");
    assert_eq!(
        view.window().get_peers_trust_line(),
        text(UiText::PeersTrustUnread)
    );
    assert!(!labelled(&view, text(UiText::PeersTrustUnread)).is_empty());
    assert!(
        labelled(&view, text(UiText::PeersNoTrust)).is_empty(),
        "the trust line stands in its place"
    );
    for state in [UiText::PeerStateTrusted, UiText::PeerStateNeedsTrust] {
        assert!(labelled(&view, text(state)).is_empty());
    }

    press(&view, text(UiText::TryAgain));
    assert_eq!(intents(&mut view, &mut model), vec![Intent::ReadTrust]);
    model
        .trust_settings_mut()
        .read(Err(interweave_human_client_api::TrustProblem::Unavailable));
    let _ = model.peers_turn();
    view.render(&model);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "trust-retry",
        "failed again: focus stays on Try again"
    );
    press(&view, text(UiText::TryAgain));
    let _ = intents(&mut view, &mut model);
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me),
        allowed: vec![TrustRow::added_here(t)],
    }));
    let _ = model.peers_turn();
    view.render(&model);
    assert_eq!(view.window().get_peers_trust_line(), "");
    let row = view.window().get_peer_rows().row_data(0).expect("a row");
    assert_eq!(row.state, text(UiText::PeerStateTrusted));
}

/// A removal whose outcome is unknown puts keyboard focus on the
/// snackbar's Try again (design section 8), not where the row's menu was.
#[test]
fn an_unknown_removal_focuses_the_snackbars_try_again() {
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a)]);
    let _ = model.peers_input(interweave_human_ui_model::PeersInput::Remove(at(&a)));
    model.address_removed(
        &at(&a),
        Err(
            interweave_human_client_api::AddressChangeFailure::Unconfirmed(
                interweave_human_client_api::AddressProblem::Unavailable,
            ),
        ),
    );
    assert_eq!(model.peers_turn(), Some(Intent::ReadAddresses));
    model.addresses_read(Err(
        interweave_human_client_api::AddressProblem::Unavailable,
    ));
    view.render(&model);
    // A focus moves on a timer's turn and is reported a turn later.
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    assert!(view.window().get_peers_snack_shown());
    assert!(
        view.window().get_peers_snack_action_shown(),
        "{:?}",
        view.window().get_peers_snack_text()
    );
    assert_eq!(
        view.window().get_focus_target().as_str(),
        "peers-snack-action"
    );
    // A render after it, as the root's turn makes: the id stands.
    view.render(&model);
    assert_eq!(view.window().get_focus_id().as_str(), "peers-snack-action");
}

/// Escape on the add page is its Back (design section 5.3), from
/// wherever focus is on it: on step one it discards the field and focus
/// goes to Add an address; on step two it is Done, and the page closes
/// with its snackbar.
#[test]
fn escape_on_the_add_page_is_its_back() {
    use slint::platform::{Key, WindowEvent};
    let escape = |view: &View| {
        let text: slint::SharedString = Key::Escape.into();
        let window = view.window().window();
        window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        window.dispatch_event(WindowEvent::KeyReleased { text });
    };
    let settle = |view: &mut View, model: &mut UiModel| {
        let _ = intents(view, model);
        for _ in 0..2 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
        }
        view.render(model);
    };
    let mut view = view();
    let mut model = UiModel::new();
    let (me, u) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);

    // Step one, focus where opening the page put it.
    press(&view, text(UiText::AddAddress));
    settle(&mut view, &mut model);
    assert!(view.window().get_peers_add_open());
    assert!(!view.window().get_focus_id().is_empty());
    escape(&view);
    settle(&mut view, &mut model);
    assert!(!view.window().get_peers_add_open(), "step one discarded");
    assert_eq!(view.window().get_focus_id().as_str(), "add-address");

    // Step two, focus in a block: Done.
    press(&view, text(UiText::AddAddress));
    settle(&mut view, &mut model);
    let _ = model.peers_input(interweave_human_ui_model::PeersInput::FieldChanged(at(&u)));
    let _ = model.peers_input(interweave_human_ui_model::PeersInput::Add);
    model.addresses_added(
        &[at(&u)],
        Ok(interweave_human_client_api::AddressAnswer {
            outcome: interweave_human_client_api::AddressOutcome::Added {
                peer: Some(u.clone()),
            },
            list: Ok(AddressList {
                addresses: vec![added(&u)],
            }),
        }),
    );
    settle(&mut view, &mut model);
    assert_eq!(view.window().get_trust_blocks().row_count(), 1);
    escape(&view);
    settle(&mut view, &mut model);
    assert!(!view.window().get_peers_add_open(), "step two: Done");
    assert!(view.window().get_peers_snack_shown());
}

/// What step two draws and where focus lands (design sections 5.3 and
/// 8): a block decided by Not now keeps its Trust but not its Not now; a
/// Trust awaited gives way to its waiting line, which takes focus.
#[test]
fn step_two_draws_waiting_and_later_blocks_and_focus_lands() {
    use interweave_human_ui_model::PeersInput;
    let settle = |view: &mut View, model: &mut UiModel| {
        view.render(model);
        for _ in 0..2 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
        }
        view.render(model);
    };
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a, b) = (peer(), peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    let two = format!("{}\n{}", at(&a), at(&b));
    let _ = model.peers_input(PeersInput::Scanned(two));
    model.addresses_added(
        &[at(&a), at(&b)],
        Ok(interweave_human_client_api::AddressAnswer {
            outcome: interweave_human_client_api::AddressOutcome::Added { peer: None },
            list: Ok(AddressList {
                addresses: vec![added(&a), added(&b)],
            }),
        }),
    );
    settle(&mut view, &mut model);
    let _ = model.peers_input(PeersInput::NotNow(a.clone()));
    settle(&mut view, &mut model);
    assert_eq!(
        labelled(&view, text(UiText::TrustThisPeer)).len(),
        2,
        "A, decided by Not now, keeps its Trust"
    );
    assert_eq!(
        labelled(&view, text(UiText::NotNow)).len(),
        1,
        "only B, undecided, has Not now"
    );
    let _ = model.peers_input(PeersInput::Trust(b.clone()));
    settle(&mut view, &mut model);
    let blocks = view.window().get_trust_blocks();
    let (first, second) = (
        blocks.row_data(0).expect("a"),
        blocks.row_data(1).expect("b"),
    );
    assert!(second.waiting, "B's Trust is awaited");
    assert!(
        !second.buttons,
        "B's Trust and Not now gave way to its line"
    );
    assert_eq!(
        second.state.as_str(),
        fill(
            text(UiText::TrustStepWaiting),
            &[("peer", &short_peer(b.as_str()))]
        )
    );
    assert!(
        first.later && first.buttons && first.trust_offered,
        "A keeps its Trust"
    );
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("block-state:{}", second.handle),
        "B's Trust pressed last: focus on its waiting line"
    );
}

/// An add refused as not permitted puts keyboard focus on the line under
/// Add, which says why (design section 8).
#[test]
fn a_barred_add_focuses_the_line_under_add() {
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    let _ = model.peers_input(PeersInput::FieldChanged(at(&a)));
    let _ = model.peers_input(PeersInput::Add);
    model.addresses_added(
        &[at(&a)],
        Err(interweave_human_client_api::AddressChangeFailure::NotMade(
            interweave_human_client_api::AddressProblem::NotPermitted,
        )),
    );
    view.render(&model);
    for _ in 0..2 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    }
    view.render(&model);
    assert_eq!(view.window().get_focus_id().as_str(), "add-note");
}

/// A row whose `PeerId` has contacts draws each name on its own line
/// above the short `PeerId`, each its own screen-reader item (design
/// section 5.2).
#[test]
fn a_rows_contact_names_are_drawn_one_per_line() {
    use interweave_human_ui_model::{ContactView, RouteView};
    let mut view = view();
    let mut model = UiModel::new();
    let (me, p) = (peer(), peer());
    let contact = |id: &str, name: &str, endpoint: &str| ContactView {
        contact_id: id.to_owned(),
        name: name.to_owned(),
        name_from_card: true,
        photo_from_card: None,
        routes: vec![RouteView {
            peer: p.clone(),
            endpoint: interweave_transport_api::EndpointId::parse(endpoint).expect("valid"),
            fingerprint: None,
        }],
    };
    model.contacts_listed(vec![
        contact("1", "Smith, John", "home"),
        contact("2", "Ann", "work"),
    ]);
    open_peers(&mut view, &mut model, &me, &[], &[added(&p)]);
    let row = view.window().get_peer_rows().row_data(0).expect("a row");
    let names: Vec<String> = (0..row.names.row_count())
        .map(|i| row.names.row_data(i).expect("a name").to_string())
        .collect();
    assert_eq!(names, vec!["Ann".to_owned(), "Smith, John".to_owned()]);
    // Each name its own screen-reader item, never one label joining them;
    // the row itself is named by its short PeerId.
    assert_eq!(labelled(&view, "Ann").len(), 1);
    assert_eq!(labelled(&view, "Smith, John").len(), 1);
    assert!(!labelled(&view, &short_peer(p.as_str())).is_empty());
}

/// An add left unknown and retried without a write keeps the same note,
/// which a live region hears no change in: the view says it again as
/// the render's announcement (design sections 5.7 and 8).
#[test]
fn an_unknown_add_retried_without_a_write_is_said_again() {
    use interweave_human_client_api::{AddressChangeFailure, AddressProblem};
    use interweave_human_ui_model::PeersInput;
    let slots = |view: &View| {
        (
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        )
    };
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    let _ = model.peers_input(PeersInput::FieldChanged(at(&a)));
    let _ = model.peers_input(PeersInput::Add);
    model.addresses_added(
        &[at(&a)],
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    let _ = model.peers_turn();
    model.addresses_read(Err(AddressProblem::Unavailable));
    view.render(&model);
    let before = slots(&view);
    let _ = model.peers_input(PeersInput::Add);
    model.addresses_added(
        &[at(&a)],
        Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
    );
    view.render(&model);
    let after = slots(&view);
    assert_ne!(after, before, "said again");
    let note = view.window().get_add_note().to_string();
    assert!(!note.is_empty());
    assert!(after.0 == note || after.1 == note, "{after:?} / {note}");
}

/// On Peers a trust step's outcome is its block's to say: a Trust whose
/// answer was lost and not read back, pressed again and failing before
/// it was sent, leaves the block unknown, focus on its line -- and the
/// trust settings' "nothing was changed", which is not known, is neither
/// announced nor left for the Trust page (design sections 5.7 and 8).
#[test]
fn a_pending_trust_retried_without_a_write_says_no_nothing_changed() {
    use interweave_human_client_api::{TrustProblem, TrustSetFailure};
    use interweave_human_ui_model::{BlockProblem, PeersInput};
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    let _ = model.peers_input(PeersInput::FieldChanged(at(&a)));
    let _ = model.peers_input(PeersInput::Add);
    model.addresses_added(
        &[at(&a)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(a.clone()),
            },
            list: Ok(AddressList {
                addresses: vec![added(&a)],
            }),
        }),
    );
    let _ = model.peers_turn();
    view.render(&model);
    let change = TrustChange {
        peer: a.clone(),
        allowed: true,
    };
    let _ = model.peers_input(PeersInput::Trust(a.clone()));
    model.trust_settings_mut().set(
        change.clone(),
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    let _ = model.peers_turn();
    view.render(&model);
    assert_eq!(
        model.trust_settings_mut().take_reread(),
        Some(Intent::ReadTrust)
    );
    model
        .trust_settings_mut()
        .read(Err(TrustProblem::Unavailable));
    let _ = model.peers_turn();
    view.render(&model);
    // The step's re-read failing is its block's to say: never the
    // settings' "open the trust settings again".
    let again = text(UiText::TrustNotReadAgain);
    for slot in [
        view.window().get_announcement_a().to_string(),
        view.window().get_announcement_b().to_string(),
    ] {
        assert!(!slot.contains(again), "{slot:?}");
    }
    assert_eq!(
        intents(&mut view, &mut model),
        Vec::<Intent>::new(),
        "settled"
    );
    let _ = model.peers_input(PeersInput::Trust(a.clone()));
    model.trust_settings_mut().set(
        change,
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    let _ = model.peers_turn();
    view.render(&model);
    for _ in 0..2 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    }
    view.render(&model);
    let block = view.window().get_trust_blocks().row_data(0).expect("a");
    assert_eq!(
        model
            .peers()
            .blocks(model.trust_settings())
            .first()
            .and_then(|b| b.problem),
        Some(BlockProblem::Unknown)
    );
    let not_written = placeholder_en::trust_problem(RuntimeHost::Daemon, TrustProblem::Unavailable);
    for slot in [
        view.window().get_announcement_a().to_string(),
        view.window().get_announcement_b().to_string(),
    ] {
        assert!(!slot.contains(not_written), "{slot:?}");
    }
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("block-problem:{}", block.handle)
    );
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::TrustHeading));
    let _ = intents(&mut view, &mut model);
    assert_eq!(view.window().get_trust_outcome().as_str(), "");
}

/// Focus asked for a row a read showed gone -- a row's trust step closing
/// after another administrator removed its address -- goes to the row
/// after its place, and the screen reader hears why first; a row still
/// there takes focus with nothing said (design section 8).
#[test]
fn a_row_gone_when_focus_returns_is_said_and_focus_goes_after_it() {
    row_focus_returns(true);
}

/// The control: the same row still listed takes focus, nothing said.
#[test]
fn a_row_still_there_when_focus_returns_takes_it_unsaid() {
    row_focus_returns(false);
}

fn row_focus_returns(removed: bool) {
    use interweave_human_ui_model::PeersInput;
    let settle = |view: &mut View, model: &mut UiModel| {
        view.render(model);
        for _ in 0..2 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
        }
        view.render(model);
    };
    let gone = text(UiText::PeerRowGone);
    let slots = |view: &View| {
        [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ]
    };
    {
        let mut view = view();
        let mut model = UiModel::new();
        let (me, a, b, c) = (peer(), peer(), peer(), peer());
        // Added rows show newest first: C, B, A.
        open_peers(
            &mut view,
            &mut model,
            &me,
            &[],
            &[added(&a), added(&b), added(&c)],
        );
        let _ = model.peers_input(PeersInput::TrustRow(at(&b)));
        settle(&mut view, &mut model);
        if removed {
            model.addresses_read(Ok(AddressList {
                addresses: vec![added(&a), added(&c)],
            }));
        }
        let _ = model.peers_input(PeersInput::Close);
        settle(&mut view, &mut model);
        let rows = view.window().get_peer_rows();
        let handle = (0..rows.row_count())
            .filter_map(|i| rows.row_data(i))
            .find(|r| r.address.as_str() == at(if removed { &a } else { &b }))
            .expect("the row")
            .handle;
        assert_eq!(
            view.window().get_focus_id().as_str(),
            format!("row:{handle}"),
            "removed: {removed}"
        );
        let said = slots(&view).iter().any(|s| s.starts_with(gone));
        assert_eq!(said, removed, "{:?}", slots(&view));
    }
}

/// A trust outcome is said by where its change was made, not by the page
/// on show when its answer lands: a change confirmed in the trust
/// settings is announced and shown there even if Peers is open when it
/// is answered; a trust step's is its block's, so landing on the Trust
/// page says nothing and leaves nothing shown there.
#[test]
fn a_trust_outcome_is_said_where_its_change_was_made() {
    use interweave_human_client_api::{TrustProblem, TrustSetFailure};
    use interweave_human_ui_model::{PeersInput, TrustInput};
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a, b) = (peer(), peer(), peer());
    // No row on show while the Trust page is opened by its menu entry:
    // a row's Trust is labelled "Trust" too.
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let slots = |view: &View| {
        [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ]
    };
    let to_trust = |view: &mut View, model: &mut UiModel| {
        press(view, text(UiText::AppOptions));
        press(view, text(UiText::TrustHeading));
        let _ = intents(view, model);
    };
    let to_peers = |view: &mut View, model: &mut UiModel| {
        press(view, text(UiText::AppOptions));
        press(view, text(UiText::PeersTitle));
        let _ = intents(view, model);
    };

    // A trust read that fails while Peers is on show is its trust line's
    // to say, never the settings' "nothing was changed".
    assert_eq!(model.trust_settings_mut().opened(), Some(Intent::ReadTrust));
    model
        .trust_settings_mut()
        .read(Err(TrustProblem::Unavailable));
    view.render(&model);
    let unreachable = placeholder_en::trust_problem(RuntimeHost::Daemon, TrustProblem::Unavailable);
    assert!(
        !slots(&view).iter().any(|s| s.contains(unreachable)),
        "{:?}",
        slots(&view)
    );
    assert!(
        view.window().get_peers_trust_line() != "",
        "the line says it"
    );

    // Confirmed in the settings, answered while Peers is open.
    to_trust(&mut view, &mut model);
    let settings = model.trust_settings_mut();
    let _ = settings.input(TrustInput::EntryChanged(a.as_str().to_owned()));
    let _ = settings.input(TrustInput::ProposeAllow);
    let change = TrustChange {
        peer: a.clone(),
        allowed: true,
    };
    assert_eq!(
        settings.input(TrustInput::Confirm(change.clone())),
        Some(Intent::SetTrust(change.clone()))
    );
    to_peers(&mut view, &mut model);
    model
        .trust_settings_mut()
        .set(change, Err(TrustSetFailure::NotMade(TrustProblem::Refused)));
    view.render(&model);
    let refused = placeholder_en::trust_problem(RuntimeHost::Daemon, TrustProblem::Refused);
    assert!(
        slots(&view).iter().any(|s| s.contains(refused)),
        "announced: {:?}",
        slots(&view)
    );
    to_trust(&mut view, &mut model);
    assert_eq!(view.window().get_trust_outcome().as_str(), refused, "shown");

    // A trust step's Trust, answered once the Trust page is open.
    to_peers(&mut view, &mut model);
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&b)],
    }));
    view.render(&model);
    let _ = model.peers_input(PeersInput::TrustRow(at(&b)));
    let step = TrustChange {
        peer: b.clone(),
        allowed: true,
    };
    assert_eq!(
        model.peers_input(PeersInput::Trust(b.clone())),
        vec![Intent::SetTrust(step.clone())]
    );
    view.render(&model);
    to_trust(&mut view, &mut model);
    let before = slots(&view);
    model.trust_settings_mut().set(
        step,
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    view.render(&model);
    let not_written = placeholder_en::trust_problem(RuntimeHost::Daemon, TrustProblem::Unavailable);
    assert!(
        !slots(&view).iter().any(|s| s.contains(not_written)),
        "{before:?} -> {:?}",
        slots(&view)
    );
    assert_eq!(
        view.window().get_trust_outcome().as_str(),
        "",
        "nothing left"
    );
}

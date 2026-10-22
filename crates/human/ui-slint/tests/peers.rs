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

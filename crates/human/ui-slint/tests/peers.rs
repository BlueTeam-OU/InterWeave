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
    HostText, Intent, RuntimeHost, TrustChange, UiModel, UiText, fill, placeholder_en, short_peer,
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

/// The window's announcement slots: they carry what is said, not an item
/// a person moves to.
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
        .filter(|e| !announcement(e))
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
    // The refused line alone, never cut: it wraps over more lines than
    // the words above it take.
    let height = |id: &str| {
        let found: Vec<_> =
            ElementHandle::find_by_element_id(view.window(), &format!("RefusedLines::{id}"))
                .collect();
        assert_eq!(found.len(), 1, "{id}");
        found[0].size().height
    };
    assert!(
        height("line") > 1.5 * height("head"),
        "{} / {}",
        height("line"),
        height("head")
    );

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
    let _ = model.peers_input(interweave_human_ui_model::PeersInput::ScanPressed(
        interweave_human_ui_model::ScanControl::Camera,
    ));
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
        name_at: None,
        photo_from_card: None,
        photo_at: None,
        photo: None,
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

/// A trust step refused because another admin connection's later change
/// stands says so on its block, once the list read again does not list
/// the peer.
#[test]
fn a_superseded_trust_says_a_later_change_was_kept() {
    use interweave_human_client_api::{TrustList, TrustSetFailure};
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
    model
        .trust_settings_mut()
        .set(change, Err(TrustSetFailure::Superseded));
    let _ = model.peers_turn();
    assert_eq!(
        model.trust_settings_mut().take_reread(),
        Some(Intent::ReadTrust)
    );
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me.clone()),
        allowed: Vec::new(),
    }));
    let _ = model.peers_turn();
    view.render(&model);
    let block = view.window().get_trust_blocks().row_data(0).expect("a");
    assert_eq!(
        block.problem.as_str(),
        fill(
            text(UiText::TrustStepSuperseded),
            &[("peer", &short_peer(a.as_str()))]
        )
    );
}

/// In the trust settings, a change refused because another admin
/// connection's later change stands says so, with the peer in full.
#[test]
fn a_superseded_change_in_the_settings_says_a_later_change_was_kept() {
    use interweave_human_client_api::{TrustList, TrustSetFailure};
    use interweave_human_ui_model::TrustInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::TrustHeading));
    let _ = intents(&mut view, &mut model);
    let settings = model.trust_settings_mut();
    // The read the page's opening sent, answered.
    settings.read(Ok(TrustList {
        local_peer: Some(me),
        allowed: Vec::new(),
    }));
    let _ = settings.input(TrustInput::EntryChanged(a.as_str().to_owned()));
    let _ = settings.input(TrustInput::ProposeAllow);
    let again = TrustChange {
        peer: a.clone(),
        allowed: true,
    };
    assert_eq!(
        settings.input(TrustInput::Confirm(again.clone())),
        Some(Intent::SetTrust(again.clone()))
    );
    settings.set(again, Err(TrustSetFailure::Superseded));
    view.render(&model);
    assert_eq!(
        view.window().get_trust_outcome().as_str(),
        text(UiText::TrustSuperseded).replace("{peer}", a.as_str())
    );
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
    let gone = text(UiText::PeerRowGoneAfterAct);
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

    // A trust step's answer made but not read back, its re-read on its way
    // as the Trust page opens and failing there: a read, said on that
    // page, which draws the list from before the change.
    to_peers(&mut view, &mut model);
    // The read the Trust page's opening sent, answered.
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me.clone()),
        allowed: Vec::new(),
    }));
    let c = peer();
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&b), added(&c)],
    }));
    view.render(&model);
    // B's block takes its answer at the turn's end, then is let go.
    let _ = model.peers_turn();
    let _ = model.peers_input(PeersInput::NotNow(b.clone()));
    assert!(!model.peers().add_open());
    let _ = model.peers_input(PeersInput::TrustRow(at(&c)));
    let trusted = TrustChange {
        peer: c.clone(),
        allowed: true,
    };
    assert_eq!(
        model.peers_input(PeersInput::Trust(c.clone())),
        vec![Intent::SetTrust(trusted.clone())]
    );
    model.trust_settings_mut().set(
        trusted,
        Err(TrustSetFailure::MadeNotReadBack(TrustProblem::Unavailable)),
    );
    let _ = model.peers_turn();
    assert_eq!(
        model.trust_settings_mut().take_reread(),
        Some(Intent::ReadTrust)
    );
    to_trust(&mut view, &mut model);
    model
        .trust_settings_mut()
        .read(Err(TrustProblem::Unavailable));
    view.render(&model);
    let stale = text(UiText::TrustNotReadAgain);
    assert!(
        slots(&view).iter().any(|s| s.contains(stale)),
        "announced: {:?}",
        slots(&view)
    );
    assert_eq!(view.window().get_trust_outcome().as_str(), stale, "shown");
}

/// A refused add's words are drawn in three lines, as ux-designer-01
/// laid them out (REPLY 01a16704): what was not added, the refused line
/// alone in the code font, then why -- typed under the field, scanned
/// under the scan controls -- while each note's label is still the
/// whole sentence. Any other note is drawn as one sentence.
#[test]
fn a_refused_line_is_drawn_alone_between_its_words() {
    use interweave_human_ui_model::{PeersInput, Refusal};
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    let own = at(&me);
    let _ = model.peers_input(PeersInput::FieldChanged(own.clone()));
    let _ = model.peers_input(PeersInput::Add);
    view.render(&model);
    let w = view.window();
    let sentence = fill(
        text(UiText::AddRefused),
        &[("line", &own), ("reason", text(UiText::RefusedOwnAddress))],
    );
    assert_eq!(w.get_field_note().as_str(), sentence, "the label");
    assert_eq!(w.get_refused_line().as_str(), own, "the line alone");
    assert_eq!(w.get_refused_head().as_str(), "Not added:");
    assert_eq!(
        w.get_refused_tail().as_str(),
        text(UiText::RefusedOwnAddress),
        "no full stop before the reason"
    );
    assert_eq!(
        labelled(&view, &sentence).len(),
        1,
        "one item reads the sentence"
    );
    assert!(labelled(&view, &own).is_empty(), "the line is drawn only");

    // Scanned: under the scan controls, the same three lines.
    let _ = model.peers_input(PeersInput::FieldChanged(String::new()));
    let code = format!("{}\n{own}", at(&peer()));
    let _ = model.peers_input(interweave_human_ui_model::PeersInput::ScanPressed(
        interweave_human_ui_model::ScanControl::Camera,
    ));
    let _ = model.scanned(&code);
    view.render(&model);
    let w = view.window();
    assert_eq!(w.get_scan_note().as_str(), sentence);
    assert_eq!(w.get_refused_line().as_str(), own);
    assert!(matches!(
        model.peers().add_page_note(),
        Some(interweave_human_ui_model::AddNote::Refused {
            why: Refusal::OwnAddress,
            ..
        })
    ));

    // Any other note: one sentence, no lines.
    let _ = model.peers_input(PeersInput::Add);
    view.render(&model);
    assert_eq!(view.window().get_refused_line().as_str(), "");
}

/// A refusal of the whole add -- the list off, a code of too many lines --
/// names no line (design section 5.3): it draws the lead
/// and the reason only, its label the two read together, and a typed one
/// leaves the caret at the field's end with nothing selected.
#[test]
fn a_refusal_of_the_whole_add_names_no_line() {
    use interweave_human_ui_model::{CODE_LINES, PeersInput, Refusal};
    let settle = |view: &mut View, model: &mut UiModel| {
        view.render(model);
        for _ in 0..2 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
        }
        view.render(model);
    };
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    let line = at(&a);
    let _ = model.peers_input(PeersInput::FieldChanged(line.clone()));
    let _ = model.peers_input(PeersInput::Add);
    model.addresses_added(
        std::slice::from_ref(&line),
        Ok(AddressAnswer {
            outcome: AddressOutcome::Refused(AddressRefused::ListOff),
            list: Ok(AddressList::default()),
        }),
    );
    settle(&mut view, &mut model);
    let w = view.window();
    let reason = text(UiText::RefusedListOff).to_owned();
    let label = format!("{} {reason}", text(UiText::AddRefusedLead));
    assert_eq!(w.get_field_note().as_str(), label, "lead and reason");
    assert_eq!(w.get_refused_head().as_str(), text(UiText::AddRefusedLead));
    assert_eq!(w.get_refused_line().as_str(), "", "no line named");
    assert_eq!(w.get_refused_tail().as_str(), reason);
    assert_eq!(labelled(&view, &label).len(), 1, "one item reads it");
    let end = i32::try_from(line.len()).expect("fits");
    assert_eq!(
        (w.get_address_select_from(), w.get_address_select_to()),
        (end, end),
        "the caret at the end, nothing selected"
    );
    assert_eq!(w.get_focus_id().as_str(), "address-field");

    // A code of too many lines: the same, under the scan controls.
    let _ = model.peers_input(PeersInput::FieldChanged(String::new()));
    let long: Vec<String> = (0..=CODE_LINES).map(|_| at(&peer())).collect();
    let _ = model.peers_input(interweave_human_ui_model::PeersInput::ScanPressed(
        interweave_human_ui_model::ScanControl::Camera,
    ));
    let _ = model.scanned(&long.join("\n"));
    view.render(&model);
    let w = view.window();
    assert!(matches!(
        model.peers().add_page_note(),
        Some(interweave_human_ui_model::AddNote::Refused {
            why: Refusal::TooMany,
            ..
        })
    ));
    assert_eq!(w.get_refused_line().as_str(), "", "no line named");
    assert!(
        w.get_scan_note()
            .as_str()
            .starts_with(text(UiText::AddRefusedLead)),
        "{}",
        w.get_scan_note()
    );
    assert!(!w.get_scan_note().contains(long[CODE_LINES].as_str()));
}

/// A read that lands after a focus request was drawn but before its
/// deferred move -- a row's trust step closed, then another
/// administrator's removal read in -- is judged again: focus goes to the
/// row after the gone row's place, else the one before, and the screen
/// reader hears why first, once for the request (design section 8).
#[test]
fn a_row_gone_before_its_focus_moves_is_said_and_focus_follows() {
    use interweave_human_ui_model::PeersInput;
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
    view.render(&model);
    let _ = model.peers_input(PeersInput::Close);
    // The return to B's row is drawn; its move waits for the next turn.
    view.render(&model);
    let gone = text(UiText::PeerRowGoneAfterAct);
    let slots = |view: &View| {
        [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ]
    };
    assert!(
        !slots(&view).iter().any(|s| s.contains(gone)),
        "B still listed"
    );
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&a), added(&c)],
    }));
    view.render(&model);
    assert!(
        slots(&view).iter().any(|s| s.starts_with(gone)),
        "{:?}",
        slots(&view)
    );
    // A second read before the move takes A too: focus follows to the
    // row before B's place, and nothing more is said for this request.
    let said = slots(&view);
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&c)],
    }));
    view.render(&model);
    assert_eq!(slots(&view), said, "said once");
    for _ in 0..3 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    }
    view.render(&model);
    let rows = view.window().get_peer_rows();
    let handle = (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .find(|r| r.address.as_str() == at(&c))
        .expect("C's row")
        .handle;
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("row:{handle}"),
        "the row before B's place, A gone too"
    );
}

/// The view holds the field while an add is on its way, as the model
/// does (design section 5.3).
#[test]
fn the_field_is_read_only_while_its_add_is_on_its_way() {
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    view.render(&model);
    assert!(!view.window().get_peers_add_busy(), "free before the add");
    let _ = model.peers_input(PeersInput::FieldChanged(at(&a)));
    let _ = model.peers_input(PeersInput::Add);
    view.render(&model);
    assert!(view.window().get_peers_add_busy(), "held while on its way");
    model.addresses_added(
        &[at(&a)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Refused(AddressRefused::Grammar),
            list: Ok(AddressList::default()),
        }),
    );
    view.render(&model);
    assert!(!view.window().get_peers_add_busy(), "free once answered");
}

/// The trust line's Try again failing with the same words says the line
/// again, focus kept on Try again, each time (design section 8): the
/// settings' own outcome is not said on Peers, so nothing else would.
#[test]
fn a_trust_retry_that_fails_again_says_the_line_again() {
    use interweave_human_client_api::TrustProblem;
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
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a)]);
    let _ = model.trust_settings_mut().opened();
    model
        .trust_settings_mut()
        .read(Err(TrustProblem::Unavailable));
    settle(&mut view, &mut model);
    let line = text(UiText::PeersTrustUnread);
    assert_eq!(view.window().get_peers_trust_line().as_str(), line);
    for round in 0..2 {
        let before = [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ];
        assert_eq!(
            model.peers_input(PeersInput::TrustReadAgain),
            vec![Intent::ReadTrust]
        );
        model
            .trust_settings_mut()
            .read(Err(TrustProblem::Unavailable));
        let _ = model.peers_turn();
        settle(&mut view, &mut model);
        let after = [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ];
        assert_ne!(after, before, "round {round}: said again");
        assert!(after.iter().any(|s| s.contains(line)), "{after:?}");
        assert_eq!(view.window().get_focus_id().as_str(), "trust-retry");
    }
}

/// A note under the field is said once each time it is set, the first
/// time included, focus staying on the field or not (design section 5.3):
/// the field no longer carries it as its description, so a focus move
/// does not say it twice. The barred line under Add, which focus is sent
/// to, is not live, for the same reason.
#[test]
fn a_note_under_the_field_is_said_once_each_time_it_is_set() {
    use interweave_human_client_api::AddressProblem;
    use interweave_human_ui_model::PeersInput;
    let slots = |view: &View| {
        [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ]
    };
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    view.render(&model);
    // Add with the field empty.
    let _ = model.peers_input(PeersInput::Add);
    view.render(&model);
    let needed = text(UiText::AddressNeeded);
    assert!(
        slots(&view).iter().any(|s| s == needed),
        "{:?}",
        slots(&view)
    );
    // This device's own address, twice: said the first time, and again.
    let own = at(&me);
    let _ = model.peers_input(PeersInput::FieldChanged(own.clone()));
    for round in 0..2 {
        let before = slots(&view);
        let _ = model.peers_input(PeersInput::Add);
        view.render(&model);
        let note = view.window().get_field_note().to_string();
        assert!(note.contains(&own), "{note}");
        assert_ne!(slots(&view), before, "round {round}: said");
        assert!(slots(&view).contains(&note), "{:?}", slots(&view));
    }
    let field = ElementHandle::find_by_accessible_label(view.window(), text(UiText::AddressField))
        .next()
        .expect("the field");
    assert_eq!(field.accessible_description(), None, "not carried");

    // Not permitted: the line under Add takes focus, and is not live.
    let _ = model.peers_input(PeersInput::FieldChanged(at(&peer())));
    let _ = model.peers_input(PeersInput::Add);
    let line = model.peers().field().to_owned();
    model.addresses_added(
        &[line],
        Err(interweave_human_client_api::AddressChangeFailure::NotMade(
            AddressProblem::NotPermitted,
        )),
    );
    view.render(&model);
    assert!(!view.window().get_add_note_live());
}

/// Focus on a row, or a control of it, however it got there -- here
/// landed back after a row's trust step closed, or on a failed removal's
/// menu -- and a later read drops that row: focus goes to the row after
/// its place, `PeerRowGone` said first (design section 8's second
/// row-gone row). Focus the person moved elsewhere stays, nothing said.
#[test]
fn a_focused_row_a_later_read_drops_is_said_and_focus_follows() {
    row_held_then_dropped(Held::TrustStep);
}

/// The menu of a row whose removal failed, focused, then dropped.
#[test]
fn a_focused_row_menu_a_later_read_drops_is_said_and_focus_follows() {
    row_held_then_dropped(Held::RemoveFailed);
}

/// A row the person reached by Tab or pointer, with no act on it.
#[test]
fn a_row_reached_without_an_act_a_later_read_drops_is_said() {
    row_held_then_dropped(Held::Reached);
}

/// A control of such a row -- its Trust -- reached the same way.
#[test]
fn a_row_control_reached_without_an_act_a_later_read_drops_is_said() {
    row_held_then_dropped(Held::ReachedTrust);
}

/// The control: the person moved focus elsewhere before the read.
#[test]
fn a_dropped_row_focus_moved_from_is_left_alone() {
    row_held_then_dropped(Held::MovedAway);
}

#[derive(Clone, Copy, PartialEq)]
enum Held {
    TrustStep,
    RemoveFailed,
    Reached,
    ReachedTrust,
    MovedAway,
}

fn row_held_then_dropped(held: Held) {
    use interweave_human_client_api::{AddressChangeFailure, AddressProblem};
    use interweave_human_ui_model::PeersInput;
    let settle = |view: &mut View, model: &mut UiModel| {
        view.render(model);
        for _ in 0..3 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
        }
        view.render(model);
    };
    let slots = |view: &View| {
        [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ]
    };
    let handle_of = |view: &View, address: &str| {
        let rows = view.window().get_peer_rows();
        (0..rows.row_count())
            .filter_map(|i| rows.row_data(i))
            .find(|r| r.address.as_str() == address)
            .expect("the row")
            .handle
    };
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
    let b_handle = handle_of(&view, &at(&b));
    match held {
        Held::TrustStep | Held::MovedAway => {
            let _ = model.peers_input(PeersInput::TrustRow(at(&b)));
            settle(&mut view, &mut model);
            let _ = model.peers_input(PeersInput::NotNow(b.clone()));
            settle(&mut view, &mut model);
            assert_eq!(
                view.window().get_focus_id().as_str(),
                format!("row:{b_handle}"),
                "landed on B"
            );
        }
        Held::RemoveFailed => {
            let _ = model.peers_input(PeersInput::Remove(at(&b)));
            model.address_removed(
                &at(&b),
                Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
            );
            settle(&mut view, &mut model);
            assert_eq!(
                view.window().get_focus_id().as_str(),
                format!("row-menu:{b_handle}"),
                "landed on B's menu"
            );
        }
        // Focus moved to the row or its Trust by the view itself, as a
        // Tab or a pointer moves it, with no request of the model's: the
        // control's own focused handler names it.
        Held::Reached | Held::ReachedTrust => {
            settle(&mut view, &mut model);
            let key = if held == Held::Reached {
                "row"
            } else {
                "row-trust"
            };
            let key = format!("{key}:{b_handle}");
            view.window().set_focus_target(key.clone().into());
            view.window()
                .set_focus_seq(view.window().get_focus_seq().wrapping_add(1));
            settle(&mut view, &mut model);
            assert_eq!(view.window().get_focus_id().as_str(), key, "on B");
        }
    }
    if held == Held::MovedAway {
        view.window().set_focus_id("add-address".into());
    }
    let before = slots(&view);
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&a), added(&c)],
    }));
    view.render(&model);
    if held == Held::MovedAway {
        assert_eq!(slots(&view), before, "nothing said");
        settle(&mut view, &mut model);
        assert_eq!(view.window().get_focus_id().as_str(), "add-address");
        return;
    }
    let gone = text(UiText::PeerRowGone);
    assert!(
        slots(&view).iter().any(|s| s.starts_with(gone)),
        "{:?}",
        slots(&view)
    );
    settle(&mut view, &mut model);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("row:{}", handle_of(&view, &at(&a))),
        "the row after B's place"
    );
}

/// A removal left unknown, its snackbar leaving with focus on its Try
/// again: focus waits on that row's menu button while the list is read
/// again, and the read decides it by where focus is when it answers
/// (design section 8) -- still on the row, as Remove done without it
/// (the row after it) or `RemoveFailed` with it (its menu button);
/// moved elsewhere, it stays. Neither row-gone text is said.
#[test]
fn a_removal_read_back_without_its_row_moves_focus_from_it_unsaid() {
    removal_read_back(false, false);
}

/// The same read back listing the row: `RemoveFailed`, focus on the
/// row's menu button.
#[test]
fn a_removal_read_back_with_its_row_keeps_focus_on_its_menu() {
    removal_read_back(true, false);
}

/// The control: focus moved elsewhere before the read answered stays.
#[test]
fn a_removal_read_back_leaves_focus_moved_elsewhere() {
    removal_read_back(false, true);
}

fn removal_read_back(kept: bool, moved: bool) {
    use interweave_human_client_api::{AddressChangeFailure, AddressProblem};
    use interweave_human_ui_model::PeersInput;
    let case = format!("kept {kept}, moved {moved}");
    let settle = |view: &mut View, model: &mut UiModel| {
        view.render(model);
        for _ in 0..3 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
        }
        view.render(model);
    };
    let handle_of = |view: &View, address: &str| {
        let rows = view.window().get_peer_rows();
        (0..rows.row_count())
            .filter_map(|i| rows.row_data(i))
            .find(|r| r.address.as_str() == address)
            .expect("the row")
            .handle
    };
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
    let b_handle = handle_of(&view, &at(&b));
    let _ = model.peers_input(PeersInput::Remove(at(&b)));
    model.address_removed(
        &at(&b),
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    let _ = model.peers_turn();
    model.addresses_read(Err(AddressProblem::Unavailable));
    settle(&mut view, &mut model);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "peers-snack-action",
        "{case}: on Try again"
    );
    let id = view.window().get_peers_snack_id();
    view.window().invoke_peers_snack_closed(id);
    let _ = intents(&mut view, &mut model);
    settle(&mut view, &mut model);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("row-menu:{b_handle}"),
        "{case}: B's menu button while it reads"
    );
    if moved {
        view.window().set_focus_target("add-address".into());
        view.window()
            .set_focus_seq(view.window().get_focus_seq().wrapping_add(1));
        settle(&mut view, &mut model);
        assert_eq!(view.window().get_focus_id().as_str(), "add-address");
    }
    let rows = if kept {
        vec![added(&a), added(&b), added(&c)]
    } else {
        vec![added(&a), added(&c)]
    };
    let said = [text(UiText::PeerRowGone), text(UiText::PeerRowGoneAfterAct)];
    assert_eq!(model.peers_turn(), Some(Intent::ReadAddresses), "{case}");
    model.addresses_read(Ok(AddressList { addresses: rows }));
    view.render(&model);
    for slot in [
        view.window().get_announcement_a().to_string(),
        view.window().get_announcement_b().to_string(),
    ] {
        assert!(
            !said.iter().any(|gone| slot.contains(gone)),
            "{case}: {slot}"
        );
    }
    settle(&mut view, &mut model);
    let wanted = match (kept, moved) {
        (_, true) => "add-address".to_owned(),
        (true, false) => format!("row-menu:{b_handle}"),
        (false, false) => format!("row:{}", handle_of(&view, &at(&a))),
    };
    assert_eq!(view.window().get_focus_id().as_str(), wanted, "{case}");
}

fn settle_view(view: &mut View, model: &mut UiModel) {
    view.render(model);
    for _ in 0..3 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    }
    view.render(model);
}

fn row_handle(view: &View, address: &str) -> i32 {
    let rows = view.window().get_peer_rows();
    (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .find(|r| r.address.as_str() == address)
        .expect("the row")
        .handle
}

/// A removal left unknown, focus moved off its Try again before its
/// snackbar goes, `focus-id` left naming the action (no control having
/// rewritten it): the action's own focus says it was not there, so
/// focus is sent to no row menu (design section 8: focus anywhere else
/// stays).
#[test]
fn a_stale_focus_id_alone_sends_no_focus_to_the_row_menu() {
    use interweave_human_client_api::{AddressChangeFailure, AddressProblem};
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a, b) = (peer(), peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a), added(&b)]);
    let _ = model.peers_input(PeersInput::Remove(at(&b)));
    model.address_removed(
        &at(&b),
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    let _ = model.peers_turn();
    model.addresses_read(Err(AddressProblem::Unavailable));
    settle_view(&mut view, &mut model);
    assert_eq!(view.window().get_focus_id().as_str(), "peers-snack-action");
    view.window().set_focus_target("add-address".into());
    view.window()
        .set_focus_seq(view.window().get_focus_seq().wrapping_add(1));
    settle_view(&mut view, &mut model);
    assert_eq!(view.window().get_focus_id().as_str(), "add-address");
    // The stale record: what a control with no focus handler would leave.
    view.window().set_focus_id("peers-snack-action".into());
    let id = view.window().get_peers_snack_id();
    view.window().invoke_peers_snack_closed(id);
    let _ = intents(&mut view, &mut model);
    assert_ne!(
        model.peers().focus().0,
        Some(interweave_human_ui_model::PeersFocus::RowMenu(at(&b))),
        "no row menu asked"
    );
}

/// Focus on a row a read drops is sent to the row after its place; a
/// second read dropping that row too before the move applies sends it
/// on from the first row's place, said once (design section 8).
#[test]
fn a_scheduled_move_whose_target_a_read_drops_is_aimed_again() {
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
    settle_view(&mut view, &mut model);
    let (b_handle, c_handle) = (row_handle(&view, &at(&b)), row_handle(&view, &at(&c)));
    let key = format!("row:{b_handle}");
    view.window().set_focus_target(key.clone().into());
    view.window()
        .set_focus_seq(view.window().get_focus_seq().wrapping_add(1));
    settle_view(&mut view, &mut model);
    assert_eq!(view.window().get_focus_id().as_str(), key, "on B");
    let slots = |view: &View| {
        [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ]
    };
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&a), added(&c)],
    }));
    view.render(&model);
    let said = slots(&view);
    assert!(
        said.iter()
            .any(|s| s.starts_with(text(UiText::PeerRowGone))),
        "{said:?}"
    );
    // Before the move to A applies, A goes too.
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&c)],
    }));
    view.render(&model);
    assert_eq!(slots(&view), said, "said once");
    settle_view(&mut view, &mut model);
    assert_eq!(
        view.window().get_focus_id().as_str(),
        format!("row:{c_handle}"),
        "from B's place: none after, C before"
    );
}

/// An Undo refused by name shows that refusal's own words, and no Try
/// again (design section 5.2).
#[test]
fn an_undo_refused_by_name_is_drawn_in_its_own_words_without_an_action() {
    use interweave_human_client_api::{AddressAnswer, AddressOutcome, AddressRefused};
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a), added(&me)]);
    let _ = model.peers_input(PeersInput::Remove(at(&me)));
    model.address_removed(
        &at(&me),
        Ok(AddressAnswer {
            outcome: AddressOutcome::Removed,
            list: Ok(AddressList {
                addresses: vec![added(&a)],
            }),
        }),
    );
    let _ = model.peers_input(PeersInput::Undo);
    model.addresses_added(
        &[at(&me)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Refused(AddressRefused::OwnPeer),
            list: Ok(AddressList {
                addresses: vec![added(&a)],
            }),
        }),
    );
    view.render(&model);
    assert_eq!(
        view.window().get_peers_snack_text().as_str(),
        text(UiText::RefusedOwnAddress)
    );
    assert!(!view.window().get_peers_snack_action_shown());
}

/// A new snackbar's action holds no focus until it takes it: a record
/// left true by an earlier snackbar that went from under focus does not
/// carry over to one whose action is never focused -- here a removal's,
/// which sends focus to the next row.
#[test]
fn a_new_snackbar_starts_with_its_action_unfocused() {
    use interweave_human_client_api::{AddressAnswer, AddressOutcome};
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a, b) = (peer(), peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a), added(&b)]);
    view.window().set_peers_snack_action_focused(true);
    let _ = model.peers_input(PeersInput::Remove(at(&b)));
    model.address_removed(
        &at(&b),
        Ok(AddressAnswer {
            outcome: AddressOutcome::Removed,
            list: Ok(AddressList {
                addresses: vec![added(&a)],
            }),
        }),
    );
    settle_view(&mut view, &mut model);
    assert!(view.window().get_peers_snack_shown());
    assert!(!view.window().get_peers_snack_action_focused());
}

/// A removal left unknown, its Try again pressed from the snackbar and
/// writing nothing: the new snackbar comes up on the same focused action
/// (design section 8: a Try again that writes nothing stays on Try
/// again). When it goes, focus that was on that Try again still waits on
/// the row's menu button while the list is read again.
#[test]
fn a_try_again_that_writes_nothing_keeps_its_action_focused_for_the_read_back() {
    use interweave_human_client_api::{AddressChangeFailure, AddressProblem};
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a, b) = (peer(), peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a), added(&b)]);
    let _ = model.peers_input(PeersInput::Remove(at(&b)));
    model.address_removed(
        &at(&b),
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    let _ = model.peers_turn();
    model.addresses_read(Err(AddressProblem::Unavailable));
    settle_view(&mut view, &mut model);
    assert_eq!(view.window().get_focus_id().as_str(), "peers-snack-action");
    let first = view.window().get_peers_snack_id();
    view.window().invoke_peers_snack_pressed(first);
    let asked = intents(&mut view, &mut model);
    assert_eq!(asked, vec![Intent::RemoveAddress(at(&b))], "Try again");
    model.address_removed(
        &at(&b),
        Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
    );
    settle_view(&mut view, &mut model);
    let second = view.window().get_peers_snack_id();
    assert_ne!(second, first, "a new snackbar");
    assert_eq!(view.window().get_focus_id().as_str(), "peers-snack-action");
    view.window().invoke_peers_snack_closed(second);
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        model.peers().focus().0,
        Some(interweave_human_ui_model::PeersFocus::RowMenu(at(&b))),
        "on B's menu button while it reads"
    );
}

/// A snackbar whose action holds focus goes from under it: a render
/// between its closing and the take destroys the action, and its record
/// still says focus was there, so focus waits on the row's menu button
/// (design section 8).
#[test]
fn a_snackbar_gone_before_its_close_is_taken_still_reports_its_focus() {
    use interweave_human_client_api::{AddressChangeFailure, AddressProblem};
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a, b) = (peer(), peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a), added(&b)]);
    let _ = model.peers_input(PeersInput::Remove(at(&b)));
    model.address_removed(
        &at(&b),
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    let _ = model.peers_turn();
    model.addresses_read(Err(AddressProblem::Unavailable));
    settle_view(&mut view, &mut model);
    assert_eq!(view.window().get_focus_id().as_str(), "peers-snack-action");
    view.window()
        .invoke_peers_snack_closed(view.window().get_peers_snack_id());
    settle_view(&mut view, &mut model);
    assert!(
        !view.window().get_peers_snack_shown(),
        "gone before the take"
    );
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        model.peers().focus().0,
        Some(interweave_human_ui_model::PeersFocus::RowMenu(at(&b)))
    );
}

/// A row's menu open when the add page or a row's trust step covers the
/// list: the menu goes with the list, so one Back closes the page (design
/// section 5.3), not a menu that is no longer drawn, and focus goes where
/// closing the page sends it (section 8): Add an address, or the row.
#[test]
fn a_row_menu_open_under_the_add_page_leaves_back_to_the_page() {
    row_menu_under_a_page(Under::AddPage, Close::Back);
}

/// The same with a row's trust step opened over the open menu.
#[test]
fn a_row_menu_open_under_a_trust_step_leaves_back_to_the_page() {
    row_menu_under_a_page(Under::TrustStep, Close::Back);
}

/// The page closed by its own back arrow instead: the menu does not come
/// back with the rows to take focus from where closing sends it.
#[test]
fn a_row_menu_covered_by_a_page_does_not_come_back_when_it_closes() {
    row_menu_under_a_page(Under::AddPage, Close::Arrow);
}

#[derive(Clone, Copy, PartialEq)]
enum Under {
    AddPage,
    TrustStep,
}

#[derive(Clone, Copy, PartialEq)]
enum Close {
    Back,
    Arrow,
}

fn row_menu_under_a_page(under: Under, close: Close) {
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a, b) = (peer(), peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a), added(&b)]);
    settle_view(&mut view, &mut model);
    let (a_handle, b_handle) = (row_handle(&view, &at(&a)), row_handle(&view, &at(&b)));
    view.window().set_peer_menu(a_handle);
    if under == Under::TrustStep {
        let _ = model.peers_input(PeersInput::TrustRow(at(&b)));
    } else {
        press(&view, text(UiText::AddAddress));
        let _ = intents(&mut view, &mut model);
    }
    settle_view(&mut view, &mut model);
    assert!(view.window().get_peers_add_open(), "the page is open");
    assert_eq!(
        view.window().get_peer_menu(),
        -1,
        "the menu went with the list"
    );
    match close {
        Close::Back => assert!(view.back(), "Back taken"),
        Close::Arrow => {
            let _ = model.peers_input(PeersInput::Close);
        }
    }
    let _ = intents(&mut view, &mut model);
    settle_view(&mut view, &mut model);
    settle_view(&mut view, &mut model);
    assert!(
        !view.window().get_peers_add_open(),
        "one Back closes the page"
    );
    assert_eq!(view.window().get_peer_menu(), -1, "no menu back");
    let wanted = match under {
        Under::AddPage => "add-address".to_owned(),
        Under::TrustStep => format!("row:{b_handle}"),
    };
    assert_eq!(view.window().get_focus_id().as_str(), wanted);
}

/// A row's menu open while focus is on another row, and a read drops the
/// menu's row: the menu goes with it, so Back is not spent closing a menu
/// no one sees, and focus stays where the person put it.
#[test]
fn a_row_menu_whose_row_a_read_drops_goes_with_it() {
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a, b) = (peer(), peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a), added(&b)]);
    settle_view(&mut view, &mut model);
    let (a_handle, b_handle) = (row_handle(&view, &at(&a)), row_handle(&view, &at(&b)));
    view.window().set_peer_menu(a_handle);
    // The open menu's first item takes focus; the person then moves on.
    settle_view(&mut view, &mut model);
    let key = format!("row:{b_handle}");
    view.window().set_focus_target(key.clone().into());
    view.window()
        .set_focus_seq(view.window().get_focus_seq().wrapping_add(1));
    settle_view(&mut view, &mut model);
    assert_eq!(view.window().get_focus_id().as_str(), key, "on B");
    model.addresses_read(Ok(AddressList {
        addresses: vec![added(&b)],
    }));
    settle_view(&mut view, &mut model);
    assert_eq!(view.window().get_peer_menu(), -1, "A's menu went with A");
    assert_eq!(
        view.window().get_focus_id().as_str(),
        key,
        "focus stays on B"
    );
}

/// Peers at 320 px with 200% text -- the platform's default font size at
/// 28 px, as Android's font scale sets it -- keeps every element inside
/// the window: a row's short `PeerId` wraps rather than set the row's
/// width, so its Trust and menu stay on screen (design sections 6 and 8,
/// human-client-ui section 11).
#[test]
fn a_peers_row_fits_a_narrow_window_with_large_text() {
    let mut view = view();
    let mut model = UiModel::new();
    i_slint_core::window::WindowInner::from_pub(view.window().window())
        .context()
        .set_platform_default_font_size(Some(i_slint_core::lengths::LogicalLength::new(28.0)));
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(320.0, 1600.0));
    slint::platform::update_timers_and_animations();
    // A wide short PeerId -- the tail with the most wide letters of many
    // drawn -- so a row that would not wrap it does not fit by luck.
    let wide = |p: &TransportIdentity| {
        short_peer(p.as_str())
            .chars()
            .filter(|c| matches!(c, 'W' | 'M' | 'm' | 'w'))
            .count()
    };
    let a = (0..500).map(|_| peer()).max_by_key(wide).expect("a peer");
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[added(&a)]);
    view.render(&model);
    assert_eq!(overflowing(&view, 320.0), Vec::<String>::new());
    // Inside the Peers column too, not only the window: its help line
    // spans the column, and a control past its end is cut off by the
    // pane's clip before it reaches the window's edge.
    let edge = |label: &str| {
        let e = ElementHandle::find_by_accessible_label(view.window(), label)
            .next()
            .unwrap_or_else(|| panic!("{label}"));
        e.absolute_position().x + e.size().width
    };
    let column = edge(text(UiText::PeersNoTrust));
    let menu = fill(text(UiText::PeerMenu), &[("peer", &short_peer(a.as_str()))]);
    for control in [text(UiText::TrustThisPeer), menu.as_str()] {
        assert!(
            edge(control) <= column + 0.5,
            "{control} ends at {} past the column at {column}",
            edge(control)
        );
    }
}

/// An add on its way (design 5.3): Add keeps focus and reads `AddWaiting`
/// beside a progress mark, said once as the wait begins and not again on
/// later draws; the answer gives Add back its own label.
#[test]
fn an_add_on_its_way_reads_adding_and_is_said_once() {
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, them) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    let _ = model.peers_input(PeersInput::FieldChanged(at(&them)));
    settle_view(&mut view, &mut model);
    view.window().set_focus_target("add-action".into());
    view.window()
        .set_focus_seq(view.window().get_focus_seq().wrapping_add(1));
    settle_view(&mut view, &mut model);
    assert_eq!(view.window().get_focus_id().as_str(), "add-action");
    let slots = |view: &View| {
        [
            view.window().get_announcement_a().to_string(),
            view.window().get_announcement_b().to_string(),
        ]
    };
    let _ = model.peers_input(PeersInput::Add);
    view.render(&model);
    let waiting = text(UiText::AddWaiting);
    assert!(view.window().get_add_waiting());
    assert!(
        ElementHandle::find_by_accessible_label(view.window(), waiting)
            .next()
            .is_some(),
        "Add reads {waiting}"
    );
    let mark = |view: &View| {
        ElementHandle::find_by_element_id(view.window(), "AppWindow::add-wait-mark")
            .next()
            .is_some()
    };
    assert!(mark(&view), "the progress mark beside it");
    let said = slots(&view);
    assert!(said.iter().any(|s| s.starts_with(waiting)), "{said:?}");
    settle_view(&mut view, &mut model);
    assert_eq!(slots(&view), said, "said once");
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "add-action",
        "focus kept"
    );
    model.addresses_added(
        &[at(&them)],
        Ok(interweave_human_client_api::AddressAnswer {
            outcome: interweave_human_client_api::AddressOutcome::Refused(
                interweave_human_client_api::AddressRefused::Grammar,
            ),
            list: Ok(AddressList { addresses: vec![] }),
        }),
    );
    view.render(&model);
    assert!(!view.window().get_add_waiting());
    assert!(!mark(&view), "the mark gone with the wait");
    assert!(
        ElementHandle::find_by_accessible_label(view.window(), text(UiText::AddAction))
            .next()
            .is_some(),
        "Add's own label back"
    );
}

/// The desktop's image controls are drawn once offered and not before;
/// an image with no code says so under them with focus back on the
/// control used (§5.5, §8). With no image reader compiled -- this crate
/// without its `desktop` feature -- a press ends the scan at once as a
/// cancelled one, focus back on the control; with it, a press would open
/// the platform's dialog, so the scan is opened through the model.
#[test]
fn the_image_controls_scan_and_an_image_without_a_code_is_said_under_them() {
    use interweave_human_ui_model::{PeersInput, ScanControl};
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    assert!(
        labelled(&view, text(UiText::OpenCodeImage)).is_empty(),
        "not offered"
    );

    view.offer_scans(false, true, true);
    view.render(&model);
    assert!(
        labelled(&view, text(UiText::ScanCode)).is_empty(),
        "no camera here"
    );
    assert_eq!(labelled(&view, text(UiText::PasteCodeImage)).len(), 1);
    for (control, label, key) in [
        (ScanControl::OpenImage, UiText::OpenCodeImage, "open-image"),
        (
            ScanControl::PasteImage,
            UiText::PasteCodeImage,
            "paste-image",
        ),
    ] {
        let _ = model.peers_input(PeersInput::ScanPressed(control));
        view.render(&model);
        assert!(model.peers().scan_open());
        let _ = model.peers_input(PeersInput::ImageNoCode);
        let _ = intents(&mut view, &mut model);
        assert_eq!(
            view.window().get_scan_note().as_str(),
            text(UiText::ImageNoCode)
        );
        slint::platform::update_timers_and_animations();
        assert_eq!(view.window().get_focus_target().as_str(), key);
        if !cfg!(feature = "desktop") {
            press(&view, text(label));
            // The press, then the reader's at-once ending, each a turn.
            let _ = intents(&mut view, &mut model);
            let _ = intents(&mut view, &mut model);
            assert!(!model.peers().scan_open(), "no reader: ended at once");
        }
        // Focus moves on a timer's turn, after the render that asks it.
        slint::platform::update_timers_and_animations();
        assert_eq!(view.window().get_focus_target().as_str(), key);
    }
}

/// Show my address on Peers reads this device's addresses only when
/// pressed; the page puts its warning first, focused, then a code that
/// decodes to exactly the lines listed under it, Copy all addresses and
/// the `PeerId` alone; Back closes it, focus back on its control (§5.4,
/// §6, §8).
#[test]
fn show_draws_a_code_that_reads_back_as_its_lines() {
    use interweave_human_client_api::ShowAddresses;
    use interweave_human_ui_model::PeersInput;
    use interweave_human_ui_slint::code::{CodeRead, read_rgba};
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    assert!(!view.window().get_peers_show_open());
    press(&view, text(UiText::ShowMyAddress));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::ReadOwnAddresses]
    );
    slint::platform::update_timers_and_animations();
    assert!(view.window().get_peers_show_open());
    assert_eq!(view.window().get_focus_target().as_str(), "show-warning");
    assert_eq!(labelled(&view, text(UiText::ShowWarning)).len(), 1);
    assert!(
        !view.window().get_show_code_shown(),
        "nothing before the read"
    );

    let lines = vec![
        format!("/ip4/203.0.113.9/tcp/4001/p2p/{}", me.as_str()),
        format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", me.as_str()),
    ];
    model.own_addresses_read(Ok(ShowAddresses {
        peer: me.clone(),
        addresses: lines.clone(),
        truncated: false,
    }));
    view.render(&model);
    let w = view.window();
    assert!(w.get_show_code_shown());
    assert!(w.get_show_code_drawn());
    let label = text(UiText::QrLabelOther).replace("{n}", "2");
    assert_eq!(
        labelled(&view, &label).len(),
        1,
        "the code is one labelled item"
    );
    assert_eq!(w.get_show_lines().as_str(), lines.join("\n"));
    assert_eq!(w.get_show_peer().as_str(), me.as_str());
    assert_eq!(labelled(&view, text(UiText::CopyAddresses)).len(), 1);
    // What is drawn is what is listed: the code, scaled as a screen
    // shows it, reads back as the lines.
    let code = w.get_show_code().to_rgba8().expect("pixels");
    let (cw, ch) = (code.width() as usize, code.height() as usize);
    let mut big = Vec::with_capacity(cw * ch * 64);
    for y in 0..ch * 4 {
        for x in 0..cw * 4 {
            let p = code.as_slice()[(y / 4) * cw + x / 4];
            big.extend_from_slice(&[p.r, p.g, p.b, p.a]);
        }
    }
    assert_eq!(
        read_rgba(cw * 4, ch * 4, &big),
        CodeRead::Text(lines.join("\n"))
    );

    let _ = model.peers_input(PeersInput::CloseShow);
    view.render(&model);
    slint::platform::update_timers_and_animations();
    assert!(!view.window().get_peers_show_open());
    assert_eq!(view.window().get_focus_target().as_str(), "show-my-address");
}

/// Sixteen lines past what any code holds (version 40-L, 2953 bytes) draw
/// no image and announce no code, while the lines and the `PeerId` still
/// show (#409 r3 F2).
#[test]
fn show_announces_no_code_for_lines_no_code_holds() {
    use interweave_human_client_api::ShowAddresses;
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::ShowMyAddress));
    let _ = intents(&mut view, &mut model);
    // Circuit routes through IPv6 relays in full form, as a device with
    // several relays lists them: about 190 bytes each.
    let lines: Vec<String> = (0..16)
        .map(|i| {
            format!(
                "/ip6/2001:0db8:0000:0000:0000:0000:0000:{i:04x}/udp/4001/quic-v1/p2p/{relay}/p2p-circuit/p2p/{me}",
                relay = me.as_str(),
                me = me.as_str(),
            )
        })
        .collect();
    assert!(lines.join("\n").len() > 2953, "past version 40-L");
    model.own_addresses_read(Ok(ShowAddresses {
        peer: me.clone(),
        addresses: lines.clone(),
        truncated: false,
    }));
    view.render(&model);
    let w = view.window();
    assert!(w.get_show_code_shown());
    assert!(!w.get_show_code_drawn(), "no code is drawn");
    let label = text(UiText::QrLabelOther).replace("{n}", "16");
    assert!(labelled(&view, &label).is_empty(), "no code is announced");
    assert_eq!(w.get_show_lines().as_str(), lines.join("\n"));
    assert_eq!(w.get_show_peer().as_str(), me.as_str());
}

/// No address is said with what to do and no reason, never as a failure; a
/// failed read is never "no address", with Try again only where a retry
/// may mend it, focus on its line (§5.4, §8).
#[test]
fn show_says_no_address_and_a_failed_read_apart() {
    use interweave_human_client_api::{AddressProblem, ShowAddresses};
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenShow);
    model.own_addresses_read(Ok(ShowAddresses {
        peer: me.clone(),
        addresses: Vec::new(),
        truncated: false,
    }));
    view.render(&model);
    let w = view.window();
    assert!(!w.get_show_code_shown());
    assert_eq!(w.get_show_none().as_str(), text(UiText::ShowNoAddress));
    for reason in [UiText::ShowNoAddressOffline, UiText::ShowNoAddressLocalOnly] {
        assert!(
            labelled(&view, text(reason)).is_empty(),
            "no reason is claimed"
        );
    }
    assert_eq!(
        w.get_show_none_what().as_str(),
        text(UiText::ShowNoAddressWhat)
    );
    assert_eq!(w.get_show_problem().as_str(), "");
    let _ = model.peers_input(PeersInput::CloseShow);

    for (problem, words, retry) in [
        (
            AddressProblem::Unavailable,
            text(UiText::ShowReadFailed),
            true,
        ),
        (
            AddressProblem::Incompatible,
            placeholder_en::host_text(RuntimeHost::Daemon, HostText::ShowIncompatible),
            false,
        ),
        (
            AddressProblem::NotPermitted,
            placeholder_en::host_text(RuntimeHost::Daemon, HostText::ShowNotPermitted),
            false,
        ),
    ] {
        let _ = model.peers_input(PeersInput::OpenShow);
        model.own_addresses_read(Err(problem));
        view.render(&model);
        slint::platform::update_timers_and_animations();
        let w = view.window();
        assert_eq!(w.get_show_problem().as_str(), words);
        assert_eq!(w.get_show_none().as_str(), "", "never no address");
        assert_eq!(w.get_show_problem_retry(), retry, "{problem:?}");
        assert_eq!(w.get_focus_target().as_str(), "show-problem");
        let _ = model.peers_input(PeersInput::CloseShow);
    }
}

/// Where the platform secures the window, Show asks the flag on as it
/// opens and draws no address until the platform says it is on; leaving
/// the screen closes Show (§5.4). The testing backend cannot say when a
/// frame was drawn, so the flag is never asked off here: kept on rather
/// than cleared early.
#[test]
fn show_draws_no_address_until_the_window_is_secured() {
    use interweave_human_client_api::ShowAddresses;
    use interweave_human_ui_model::PeersInput;
    use std::cell::RefCell;
    use std::rc::Rc;
    let mut view = view();
    let asked = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&asked);
    view.guard_show(move |on| log.borrow_mut().push(on));
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    assert!(asked.borrow().is_empty(), "nothing asked before Show");
    // A confirmation no ask is waiting on secures nothing to come.
    view.show_secured();
    let _ = model.peers_input(PeersInput::OpenShow);
    model.own_addresses_read(Ok(ShowAddresses {
        peer: me.clone(),
        addresses: vec![at(&me)],
        truncated: false,
    }));
    view.render(&model);
    assert_eq!(*asked.borrow(), vec![true]);
    let w = view.window();
    assert!(!w.get_show_code_shown(), "held until secured");
    assert_eq!(w.get_show_lines().as_str(), "", "no line either");
    assert_eq!(labelled(&view, text(UiText::ShowWarning)).len(), 1);

    view.show_secured();
    view.render(&model);
    assert!(view.window().get_show_code_shown());
    assert_eq!(*asked.borrow(), vec![true], "asked once");

    view.leave_show();
    let _ = intents(&mut view, &mut model);
    assert_eq!(model.peers().show(), None, "leaving the screen closes Show");
    assert_eq!(
        *asked.borrow(),
        vec![true],
        "never off before a frame is known drawn"
    );
}

/// A scan's own line is drawn under the scan controls and said once as
/// it is set, focus staying on the control it came from (§5.5).
#[test]
fn a_scan_line_is_drawn_under_the_controls_and_said_once() {
    use interweave_human_ui_model::{PeersInput, ScanControl};
    let slots = |view: &View| {
        format!(
            "{}{}",
            view.window().get_announcement_a(),
            view.window().get_announcement_b()
        )
    };
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    let _ = model.peers_input(PeersInput::OpenAdd);
    view.offer_scans(false, true, true);
    view.render(&model);
    for (given, words) in [
        (PeersInput::ImageUnreadable, text(UiText::ImageUnreadable)),
        (PeersInput::ClipboardNoImage, text(UiText::ClipboardNoImage)),
        (
            PeersInput::ClipboardUnreadable,
            text(UiText::ClipboardUnreadable),
        ),
    ] {
        let _ = model.peers_input(PeersInput::ScanPressed(ScanControl::PasteImage));
        view.render(&model);
        let _ = model.peers_input(given);
        view.render(&model);
        assert_eq!(view.window().get_scan_note().as_str(), words);
        assert!(slots(&view).contains(words), "said as it is set");
        let before = slots(&view);
        view.render(&model);
        assert_eq!(slots(&view), before, "said once");
    }
}

/// Leaving Peers closes Show, never left open behind another page: to
/// the trust settings, by a conversation picked, or closing Peers
/// (§5.4; #409 F1).
#[test]
fn leaving_peers_closes_show() {
    use interweave_human_client_api::ShowAddresses;
    use interweave_human_ui_model::PeersInput;
    let me = peer();
    let open_show = |view: &mut View, model: &mut UiModel| {
        let _ = model.peers_input(PeersInput::OpenShow);
        model.own_addresses_read(Ok(ShowAddresses {
            peer: me.clone(),
            addresses: vec![at(&me)],
            truncated: false,
        }));
        view.render(model);
        assert!(model.peers().show().is_some());
    };
    let mut view = view();
    let mut model = UiModel::new();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    open_show(&mut view, &mut model);
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::TrustHeading));
    let _ = intents(&mut view, &mut model);
    assert!(model.peers().show().is_none(), "to the trust settings");

    // Back to Peers, Show again, then Peers closed.
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::PeersTitle));
    let _ = intents(&mut view, &mut model);
    open_show(&mut view, &mut model);
    view.window().invoke_close_peers();
    let _ = intents(&mut view, &mut model);
    assert!(model.peers().show().is_none(), "Peers closed");
}

/// Each image source is offered on its own: the desktop offers the file
/// and withholds the clipboard's image until it can be read within the
/// file's bounds (#409 F1).
#[test]
fn the_clipboard_image_is_offered_apart_from_the_file() {
    let mut view = view();
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    view.offer_scans(false, true, false);
    view.render(&model);
    assert_eq!(labelled(&view, text(UiText::OpenCodeImage)).len(), 1);
    assert!(
        labelled(&view, text(UiText::PasteCodeImage)).is_empty(),
        "withheld"
    );
    view.offer_scans(false, true, true);
    view.render(&model);
    assert_eq!(labelled(&view, text(UiText::PasteCodeImage)).len(), 1);
}

/// The facade's answer to a change refused `superseded`, its list read
/// back holding `rows`.
#[allow(clippy::unnecessary_wraps)]
fn superseded(
    rows: &[PeerAddress],
) -> Result<AddressAnswer, interweave_human_client_api::AddressChangeFailure> {
    Ok(AddressAnswer {
        outcome: AddressOutcome::Refused(AddressRefused::Superseded),
        list: Ok(AddressList {
            addresses: rows.to_vec(),
        }),
    })
}

/// An add refused `superseded`, its read back lacking the line, says
/// `AddSuperseded` under Add -- one line and several by their own texts
/// -- announced, and never the not-written `AddFailed` (design §5.7).
#[test]
fn a_superseded_add_of_one_line_says_so_under_add() {
    superseded_add(false);
}

/// The same for several lines, by its own text.
#[test]
fn a_superseded_add_of_several_lines_says_so_under_add() {
    superseded_add(true);
}

fn superseded_add(several: bool) {
    use interweave_human_ui_model::PeersInput;
    {
        let mut view = view();
        let mut model = UiModel::new();
        let (me, a, b) = (peer(), peer(), peer());
        open_peers(&mut view, &mut model, &me, &[], &[]);
        let lines = if several {
            vec![at(&a), at(&b)]
        } else {
            vec![at(&a)]
        };
        let _ = model.peers_input(PeersInput::OpenAdd);
        let _ = model.peers_input(PeersInput::FieldChanged(lines.join("\n")));
        let _ = model.peers_input(PeersInput::Add);
        model.addresses_added(&lines, superseded(&[]));
        view.render(&model);
        let expected = if several {
            UiText::AddSupersededOther
        } else {
            UiText::AddSupersededOne
        };
        assert_eq!(view.window().get_add_note().as_str(), text(expected));
        assert!(view.window().get_add_note_live(), "announced");
    }
}

/// A removal refused `superseded` and read back with its row shows
/// `RemoveSuperseded` with no action; it is still up after a snackbar
/// with an action would have gone, closes at 10 seconds, and stays while
/// the pointer is on it (design §5.7).
#[test]
fn a_superseded_removal_snackbar_has_no_action_and_stays_ten_seconds() {
    superseded_removal_snackbar(false);
}

/// The same snackbar, the pointer on it at 10 seconds: it stays, and
/// goes once the pointer leaves.
#[test]
fn a_superseded_removal_snackbar_stays_while_the_pointer_is_on_it() {
    superseded_removal_snackbar(true);
}

/// With the platform's timeout longer than 10 seconds, the longer one
/// holds (Android's accessibility timeout, design §5.7).
#[test]
fn a_superseded_removal_snackbar_takes_the_platforms_longer_timeout() {
    let step = |view: &mut View, model: &mut UiModel, ms: u64| {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(ms));
        let _ = intents(view, model);
    };
    let mut view = view();
    view.set_touch_settings(interweave_human_ui_slint::TouchSettings {
        snackbar_ms: 15_000,
        ..interweave_human_ui_slint::TouchSettings::default()
    });
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a)]);
    let _ = model.peers_input(interweave_human_ui_model::PeersInput::Remove(at(&a)));
    model.address_removed(&at(&a), superseded(&[added(&a)]));
    view.render(&model);
    step(&mut view, &mut model, 10_100);
    assert!(view.window().get_peers_snack_shown(), "up past 10 s");
    step(&mut view, &mut model, 5_000);
    assert!(!view.window().get_peers_snack_shown(), "gone at 15 s");
}

fn superseded_removal_snackbar(pointer: bool) {
    use slint::platform::WindowEvent;
    let step = |view: &mut View, model: &mut UiModel, ms: u64| {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(ms));
        let _ = intents(view, model);
    };
    {
        let mut view = view();
        let mut model = UiModel::new();
        let (me, a) = (peer(), peer());
        open_peers(&mut view, &mut model, &me, &[], &[added(&a)]);
        let _ = model.peers_input(interweave_human_ui_model::PeersInput::Remove(at(&a)));
        model.address_removed(&at(&a), superseded(&[added(&a)]));
        view.render(&model);
        assert!(view.window().get_peers_snack_shown());
        assert_eq!(
            view.window().get_peers_snack_text().as_str(),
            text(UiText::RemoveSuperseded)
        );
        assert!(!view.window().get_peers_snack_action_shown(), "no action");
        if pointer {
            let line = labelled(&view, text(UiText::RemoveSuperseded));
            let at = line[0].absolute_position();
            view.window()
                .window()
                .dispatch_event(WindowEvent::PointerMoved {
                    position: slint::LogicalPosition::new(at.x + 2.0, at.y + 2.0),
                });
            step(&mut view, &mut model, 10);
            assert!(
                view.window().get_peers_snack_pointer_on(),
                "the pointer seen"
            );
        }
        step(&mut view, &mut model, 6_100);
        assert!(view.window().get_peers_snack_shown(), "up past 6 s");
        step(&mut view, &mut model, 4_000);
        assert_eq!(
            view.window().get_peers_snack_shown(),
            pointer,
            "at 10 s: gone, unless the pointer is on it"
        );
        if pointer {
            view.window()
                .window()
                .dispatch_event(WindowEvent::PointerExited);
            step(&mut view, &mut model, 10);
            step(&mut view, &mut model, 1_100);
            assert!(!view.window().get_peers_snack_shown(), "gone once left");
        }
    }
}

/// An Undo refused `superseded` and read back without its row shows
/// `UndoSuperseded`, with no action.
#[test]
fn a_superseded_undo_says_it_was_not_added_back() {
    use interweave_human_ui_model::PeersInput;
    let mut view = view();
    let mut model = UiModel::new();
    let (me, a) = (peer(), peer());
    open_peers(&mut view, &mut model, &me, &[], &[added(&a)]);
    let _ = model.peers_input(PeersInput::Remove(at(&a)));
    model.address_removed(
        &at(&a),
        Ok(AddressAnswer {
            outcome: AddressOutcome::Removed,
            list: Ok(AddressList::default()),
        }),
    );
    let _ = model.peers_input(PeersInput::Undo);
    model.addresses_added(&[at(&a)], superseded(&[]));
    view.render(&model);
    assert_eq!(
        view.window().get_peers_snack_text().as_str(),
        text(UiText::UndoSuperseded)
    );
    assert!(!view.window().get_peers_snack_action_shown(), "no action");
}

/// A superseded Trust whose trust list cannot be read again says
/// `TrustStepSupersededUnread` on its block, never `TrustStepUnknown`.
#[test]
fn a_superseded_trust_left_unread_says_so_on_its_block() {
    use interweave_human_client_api::{TrustProblem, TrustSetFailure};
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
    let change = TrustChange {
        peer: a.clone(),
        allowed: true,
    };
    let _ = model.peers_input(PeersInput::Trust(a.clone()));
    model
        .trust_settings_mut()
        .set(change, Err(TrustSetFailure::Superseded));
    let _ = model.peers_turn();
    let _ = model.trust_settings_mut().take_reread();
    model
        .trust_settings_mut()
        .read(Err(TrustProblem::Unavailable));
    let _ = model.peers_turn();
    view.render(&model);
    let block = view.window().get_trust_blocks().row_data(0).expect("a");
    assert_eq!(
        block.problem.as_str(),
        fill(
            text(UiText::TrustStepSupersededUnread),
            &[("peer", &short_peer(a.as_str()))]
        )
    );
}

/// Scan a code opens the viewfinder and asks the platform's camera:
/// what the camera is for until the first frame, then the frames and
/// the hint; a code that is no address list says so and stays; an
/// address list is the scan's add, the viewfinder closing and the camera
/// stopping; Close stops it too, focus back on Scan a code (§5.5, §8).
#[test]
fn the_viewfinder_reads_a_code_and_stops_the_camera() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let mut view = view();
    let asked = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&asked);
    view.camera_hook(move |on| log.borrow_mut().push(on));
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    view.offer_scans(true, false, false);
    view.render(&model);
    // Closed with its ✕: the camera stops, focus on Scan a code.
    press(&view, text(UiText::ScanCode));
    let _ = intents(&mut view, &mut model);
    assert_eq!(*asked.borrow(), vec![true]);
    assert!(view.window().get_viewfinder_open());
    assert_eq!(labelled(&view, text(UiText::CloseScanner)).len(), 1);
    press(&view, text(UiText::CloseScanner));
    let _ = intents(&mut view, &mut model);
    slint::platform::update_timers_and_animations();
    assert!(!view.window().get_viewfinder_open());
    assert_eq!(*asked.borrow(), vec![true, false]);
    assert_eq!(view.window().get_focus_target().as_str(), "scan-code");

    press(&view, text(UiText::ScanCode));
    let _ = intents(&mut view, &mut model);
    assert_eq!(*asked.borrow(), vec![true, false, true]);
    let w = view.window();
    assert!(w.get_viewfinder_open());
    assert!(!w.get_viewfinder_has_frame());
    assert_eq!(
        labelled(&view, text(UiText::CameraWhy)).len(),
        1,
        "before a frame"
    );

    view.camera_frame(4, 4, &[128; 16]);
    view.render(&model);
    assert!(view.window().get_viewfinder_has_frame());
    assert_eq!(labelled(&view, text(UiText::ScanHint)).len(), 1);

    view.camera_read("https://example.org");
    view.render(&model);
    assert!(view.window().get_viewfinder_open(), "stays open");
    assert_eq!(
        view.window().get_viewfinder_note().as_str(),
        text(UiText::ScanNotAddress)
    );

    let code = at(&peer());
    view.camera_read(&code);
    let sent = intents(&mut view, &mut model);
    assert_eq!(sent, vec![Intent::AddAddresses(vec![code])]);
    assert!(!view.window().get_viewfinder_open());
    assert_eq!(
        *asked.borrow(),
        vec![true, false, true, false],
        "the camera stopped"
    );
}

/// The camera refused for good: its line under the scan controls with
/// Open settings, which opens the platform's settings; back from them,
/// focus on Scan a code. Refused once: its line, no Open settings
/// (§5.5, §8).
#[test]
fn a_camera_refused_for_good_offers_open_settings() {
    use std::cell::Cell;
    use std::rc::Rc;
    let mut view = view();
    view.camera_hook(|_| {});
    let opened = Rc::new(Cell::new(0));
    let count = Rc::clone(&opened);
    view.on_open_settings(move || count.set(count.get() + 1));
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    view.offer_scans(true, false, false);
    view.render(&model);
    for (for_good, words) in [
        (false, text(UiText::CameraDenied)),
        (true, text(UiText::CameraDeniedSettings)),
    ] {
        press(&view, text(UiText::ScanCode));
        let _ = intents(&mut view, &mut model);
        view.camera_denied(for_good);
        let _ = intents(&mut view, &mut model);
        assert!(!view.window().get_viewfinder_open());
        assert_eq!(view.window().get_scan_note().as_str(), words);
        assert_eq!(
            labelled(&view, text(UiText::OpenAppSettings)).len(),
            usize::from(for_good)
        );
    }
    press(&view, text(UiText::OpenAppSettings));
    assert_eq!(opened.get(), 1);
    view.settings_returned();
    let _ = intents(&mut view, &mut model);
    slint::platform::update_timers_and_animations();
    assert_eq!(view.window().get_focus_target().as_str(), "scan-code");
}

/// The camera stopping under an open scan -- it failed, the system took
/// it, or the app left the screen -- closes the viewfinder as its ✕ does;
/// with no scan open it does nothing (§5.5).
#[test]
fn the_camera_stopping_closes_the_viewfinder() {
    let mut view = view();
    view.camera_hook(|_| {});
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    view.offer_scans(true, false, false);
    view.render(&model);
    view.camera_closed();
    assert_eq!(view.queued_inputs(), 0, "no scan open: nothing queued");
    press(&view, text(UiText::ScanCode));
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_viewfinder_open());
    view.camera_closed();
    let _ = intents(&mut view, &mut model);
    assert!(!view.window().get_viewfinder_open());
    assert!(!model.peers().scan_open());
}

/// The viewfinder is modal: Tab and Backtab keep focus on its Close,
/// never the add page under it; and when it closes, its last frame goes
/// with it, no picture kept (#418 F3, F5).
#[test]
fn the_viewfinder_keeps_focus_and_keeps_no_frame() {
    use slint::platform::{Key, WindowEvent};
    let key = |view: &View, k: Key| {
        let text: slint::SharedString = k.into();
        let window = view.window().window();
        window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        window.dispatch_event(WindowEvent::KeyReleased { text });
        slint::platform::update_timers_and_animations();
    };
    let mut view = view();
    view.camera_hook(|_| {});
    let mut model = UiModel::new();
    let me = peer();
    open_peers(&mut view, &mut model, &me, &[], &[]);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    view.offer_scans(true, false, false);
    view.render(&model);
    press(&view, text(UiText::ScanCode));
    let _ = intents(&mut view, &mut model);
    // The window's draw makes the viewfinder's controls before focus
    // moves; headless, a walk of the tree stands for it. Focus moves a
    // turn after the render, and once more the turn after.
    assert_eq!(labelled(&view, text(UiText::CloseScanner)).len(), 1);
    slint::platform::update_timers_and_animations();
    slint::platform::update_timers_and_animations();
    assert_eq!(view.window().get_focus_target().as_str(), "close-scanner");
    assert_eq!(view.window().get_focus_id().as_str(), "close-scanner");
    for k in [Key::Tab, Key::Backtab, Key::Tab] {
        key(&view, k);
        assert_eq!(
            view.window().get_focus_id().as_str(),
            "close-scanner",
            "{k:?}"
        );
    }
    view.camera_frame(4, 4, &[128; 16]);
    view.render(&model);
    assert!(view.window().get_viewfinder_frame().size().width > 0);
    press(&view, text(UiText::CloseScanner));
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        view.window().get_viewfinder_frame().size().width,
        0,
        "the last frame went with the viewfinder"
    );
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The debug build's trace (`src/trace.rs`), from the window a person
//! steps through: Peers, Show holding this device's addresses and
//! `PeerId`, an add of another peer's address, its trust block and its
//! Trust answered, with a contact's name on the row. What the trace
//! says is checked against everything those pages held: kinds only.

#![allow(clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use interweave_human_client_api::{
    AddressAnswer, AddressList, AddressOrigin, AddressOutcome, PeerAddress, ShowAddresses,
    TrustList, TrustRow,
};
use interweave_human_ui_model::{
    ContactView, Intent, RouteView, RuntimeHost, TrustChange, UiModel, UiText, placeholder_en,
    short_peer,
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

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

fn press(view: &View, label: &str) {
    // Not the announcement slots, which carry what is said.
    let found: Vec<ElementHandle> = ElementHandle::find_by_accessible_label(view.window(), label)
        .filter(|e| {
            !e.id()
                .is_some_and(|id| id.ends_with("announce-a") || id.ends_with("announce-b"))
        })
        .collect();
    assert!(!found.is_empty(), "a control labelled {label:?}");
    found[0].invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
}

/// The root's loop: take, apply, take again, until empty.
fn intents(view: &mut View, model: &mut UiModel) -> Vec<Intent> {
    let mut out = Vec::new();
    loop {
        let events = view.take_events(model);
        if events.is_empty() {
            view.render(model);
            slint::platform::update_timers_and_animations();
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

#[test]
fn a_trace_carries_no_address_peer_or_name() {
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let mut model = UiModel::new();
    let (me, them, known) = (peer(), peer(), peer());
    let my_lines = vec![
        format!("/ip4/203.0.113.9/tcp/4001/p2p/{}", me.as_str()),
        format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", me.as_str()),
    ];
    let theirs = format!("/ip4/192.168.1.30/tcp/4001/p2p/{}", them.as_str());
    let known_at = format!("/ip4/192.168.1.40/tcp/4001/p2p/{}", known.as_str());
    model.contacts_listed(vec![ContactView {
        contact_id: "1".to_owned(),
        name: "Smith, John".to_owned(),
        name_from_card: true,
        name_at: None,
        photo_from_card: None,
        photo_at: None,
        photo: None,
        routes: vec![RouteView {
            peer: known.clone(),
            endpoint: interweave_transport_api::EndpointId::parse("home").expect("valid"),
            fingerprint: None,
        }],
    }]);
    view.render(&model);

    // Peers, with a known peer's row.
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::PeersTitle));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::ReadTrust, Intent::ReadAddresses]
    );
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me.clone()),
        allowed: vec![TrustRow::added_here(known.clone())],
    }));
    model.addresses_read(Ok(AddressList {
        addresses: vec![PeerAddress {
            address: known_at.clone(),
            peer: Some(known.clone()),
            origin: AddressOrigin::AddedHere,
            dialled: true,
        }],
    }));
    view.render(&model);

    // Show, holding this device's addresses and PeerId; then back.
    press(&view, text(UiText::ShowMyAddress));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::ReadOwnAddresses]
    );
    model.own_addresses_read(Ok(ShowAddresses {
        peer: me.clone(),
        addresses: my_lines.clone(),
        truncated: false,
    }));
    view.render(&model);
    slint::platform::update_timers_and_animations();
    view.leave_show();
    let _ = intents(&mut view, &mut model);

    // An add of the other peer's address, its block, and Trust answered.
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    view.window().invoke_address_edited(theirs.clone().into());
    press(&view, text(UiText::AddAction));
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::AddAddresses(vec![theirs.clone()])]
    );
    model.addresses_added(
        std::slice::from_ref(&theirs),
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(them.clone()),
            },
            list: Ok(AddressList {
                addresses: vec![PeerAddress {
                    address: theirs.clone(),
                    peer: Some(them.clone()),
                    origin: AddressOrigin::AddedHere,
                    dialled: true,
                }],
            }),
        }),
    );
    view.render(&model);
    press(&view, text(UiText::TrustThisPeer));
    let change = TrustChange {
        peer: them.clone(),
        allowed: true,
    };
    assert_eq!(
        intents(&mut view, &mut model),
        vec![Intent::SetTrust(change.clone())]
    );
    model.trust_settings_mut().set(
        change,
        Ok(TrustList {
            local_peer: Some(me.clone()),
            allowed: vec![
                TrustRow::added_here(known.clone()),
                TrustRow::added_here(them.clone()),
            ],
        }),
    );
    view.render(&model);
    slint::platform::update_timers_and_animations();

    let said = said.borrow();
    let mut held: Vec<String> = vec![
        "Smith, John".to_owned(),
        "Smith".to_owned(),
        "John".to_owned(),
    ];
    for p in [&me, &them, &known] {
        held.push(p.as_str().to_owned());
        held.push(short_peer(p.as_str()));
    }
    held.extend(my_lines);
    held.extend([theirs, known_at]);
    held.extend(["203.0.113.9", "192.168.1.", "4001"].map(str::to_owned));
    for line in said.iter() {
        assert!(line.starts_with("ui-trace "), "{line}");
        for h in &held {
            assert!(!line.contains(h.as_str()), "{line:?} carries {h:?}");
        }
    }
    // And the trace was live through every page: the control for the
    // absence above.
    for expected in [
        "ui-trace page=peers",
        "ui-trace show=shown",
        "ui-trace show=closed",
        "ui-trace page=peers-add",
        "ui-trace add=added lines=1",
        "ui-trace page=peers-add-trust",
    ] {
        assert!(
            said.iter().any(|l| l == expected),
            "{expected:?} in {said:#?}"
        );
    }
    assert!(
        said.iter()
            .any(|l| l.starts_with("ui-trace block:") && l.ends_with("=trusted")),
        "the block's Trust answered, in {said:#?}"
    );
    assert!(
        said.iter().any(|l| l.starts_with("ui-trace focus=")),
        "focus reported, in {said:#?}"
    );
}

/// An owed start says `connect=owed` on Chats until both reads answer,
/// then `connect=open` with Connect's page: the line a driver waits for
/// before it judges where the app settled. A start that owes nothing
/// says `connect=not-owed` from its first render.
#[test]
fn the_trace_says_where_the_start_stands_on_connect() {
    i_slint_backend_testing::init_no_event_loop();
    let trace = |view: &mut View| {
        let said = Rc::new(RefCell::new(Vec::<String>::new()));
        let to = Rc::clone(&said);
        view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
        said
    };
    let me = peer();

    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = trace(&mut view);
    let mut model = UiModel::new();
    model.connect_owed();
    view.render(&model);
    assert!(said.borrow().contains(&"ui-trace connect=owed".to_owned()));
    assert!(
        said.borrow()
            .contains(&"ui-trace page=conversations".to_owned())
    );
    let first = said.borrow().len();
    assert_eq!(model.read_addresses(), Some(Intent::ReadAddresses));
    assert_eq!(model.trust_settings_mut().opened(), Some(Intent::ReadTrust));
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me.clone()),
        allowed: Vec::new(),
    }));
    model.addresses_read(Ok(AddressList::default()));
    assert!(model.connect_turn().is_some());
    view.render(&model);
    let after = said.borrow()[first..].to_vec();
    assert!(
        after.contains(&"ui-trace connect=open".to_owned()),
        "{after:?}"
    );
    assert!(
        after.contains(&"ui-trace page=connect".to_owned()),
        "{after:?}"
    );

    let mut quiet = View::new(RuntimeHost::Embedded).expect("a window");
    quiet.window().show().expect("shown");
    let said = trace(&mut quiet);
    quiet.render(&UiModel::new());
    assert!(
        said.borrow()
            .contains(&"ui-trace connect=not-owed".to_owned())
    );
}

fn tab(view: &View) {
    use slint::platform::{Key, WindowEvent};
    let key: slint::SharedString = Key::Tab.into();
    let window = view.window().window();
    window.dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text: key });
}

/// Show my phrase as a driver steps it: Settings' Show recovery phrase
/// reached by TAB says its focus key, the screens over the window say
/// theirs as the root names them, and the key waiting for the person is
/// said as offline with its notice until it is unlocked.
#[test]
fn the_trace_says_the_identity_flow_and_the_locked_state() {
    use interweave_human_ui_model::ScreenId;
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let mut model = UiModel::new();
    model.authentication_needed(true);
    view.render(&model);
    for expected in [
        "ui-trace connectivity=offline",
        "ui-trace notice=authentication-needed",
    ] {
        assert!(
            said.borrow().iter().any(|l| l == expected),
            "{expected:?} in {:#?}",
            said.borrow()
        );
    }
    let first = said.borrow().len();
    model.authentication_needed(false);
    view.render(&model);
    // A model whose session never opened is still re-opening: the
    // notice the unlock leaves is the session's own.
    assert_eq!(
        said.borrow()[first..],
        [
            "ui-trace connectivity=unknown",
            "ui-trace notice=reconnecting"
        ]
    );

    view.offer_show_phrase(|| {});
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::TrustHeading));
    let _ = intents(&mut view, &mut model);
    // Bounded: Settings holds a few dozen stops at most.
    let reached = (0..64).any(|_| {
        tab(&view);
        said.borrow()
            .last()
            .is_some_and(|l| l == "ui-trace focus=show-phrase")
    });
    assert!(reached, "show-phrase by TAB in {:#?}", said.borrow());

    let first = said.borrow().len();
    for screen in [ScreenId::S1, ScreenId::S1, ScreenId::S2, ScreenId::Done] {
        view.trace_identity(Some(screen));
    }
    assert_eq!(
        said.borrow()[first..],
        [
            "ui-trace identity=s1",
            "ui-trace identity=s2",
            "ui-trace identity=closed",
        ]
    );
}

/// S1 from Settings, as a driver steps it: TAB reaches the primary
/// action, which says its focus key, and Enter on it is the primary --
/// the press that raises the phrase prompt. The keyboard cannot leave
/// the page for the window behind it on the way.
#[test]
fn the_identity_primary_is_reached_by_tab_and_pressed_by_enter() {
    use interweave_human_ui_model::{IdentityFlow, Start};
    use interweave_human_ui_slint::{IdentityInput, identity_page};
    use slint::platform::{Key, WindowEvent};
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let model = UiModel::new();
    view.render(&model);
    let inputs = Rc::new(RefCell::new(Vec::new()));
    let into = Rc::clone(&inputs);
    view.on_identity(move |input| into.borrow_mut().push(input));
    let flow = IdentityFlow::new(&["abandon"], Start::ShowPhrase);
    view.show_identity(Some(identity_page(&flow)));
    slint::platform::update_timers_and_animations();
    let first = said.borrow().len();
    let reached = (0..16).any(|_| {
        tab(&view);
        said.borrow()
            .last()
            .is_some_and(|l| l == "ui-trace focus=identity-primary")
    });
    assert!(reached, "identity-primary by TAB in {:#?}", said.borrow());
    // Past the page's last stop, TAB comes round to its first again.
    for _ in 0..4 {
        tab(&view);
    }
    let lines = said.borrow()[first..].to_vec();
    for line in &lines {
        assert!(
            line.starts_with("ui-trace focus=identity-"),
            "focus left the page: {lines:#?}"
        );
    }
    while said.borrow().last().map(String::as_str) != Some("ui-trace focus=identity-primary") {
        tab(&view);
    }
    let key: slint::SharedString = Key::Return.into();
    let window = view.window().window();
    window.dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text: key });
    assert_eq!(*inputs.borrow(), [IdentityInput::Primary]);
}

/// Welcome's two actions: TAB steps through them in reading order and
/// round again, Shift+TAB back, and neither reaches the window behind.
#[test]
fn the_identity_page_keeps_tab_on_its_own_stops() {
    use interweave_human_ui_model::{IdentityFlow, Start};
    use interweave_human_ui_slint::identity_page;
    use slint::platform::{Key, WindowEvent};
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    view.render(&UiModel::new());
    let flow = IdentityFlow::new(&["abandon"], Start::FirstRun { void_paper: false });
    view.show_identity(Some(identity_page(&flow)));
    slint::platform::update_timers_and_animations();
    let first = said.borrow().len();
    for _ in 0..3 {
        tab(&view);
    }
    let key: slint::SharedString = Key::Backtab.into();
    let window = view.window().window();
    window.dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text: key });
    assert_eq!(
        said.borrow()[first..],
        [
            "ui-trace focus=identity-primary",
            "ui-trace focus=identity-secondary",
            "ui-trace focus=identity-primary",
            "ui-trace focus=identity-secondary",
        ]
    );
}

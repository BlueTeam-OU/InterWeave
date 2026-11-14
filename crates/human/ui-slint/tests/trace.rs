// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The debug build's trace (`src/trace.rs`), from the window a person
//! steps through: Peers, Show holding this device's addresses and
//! `PeerId`, an add of another peer's address, its trust block and its
//! Trust answered, with a contact's name on the row. What the trace
//! says is checked against everything those pages held: kinds only.

#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use interweave_human_client_api::{
    AddressAnswer, AddressList, AddressOrigin, AddressOutcome, AddressRefused, PeerAddress,
    ShowAddresses, TrustList, TrustRow,
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
                ViewEvent::MessageCopied(key) => model.message_copied(key),
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
        view.trace_identity(Some(screen), None);
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

/// R1 and K1 with a restore are said to warn, from the pages the flow
/// hands the window, and R2 and K1 with nothing to restore are not: the
/// line TM-48's device case reads instead of the screen
/// (`tests/android-e2e/tests/identity_lines.rs`).
#[test]
fn the_trace_says_which_identity_page_carries_the_warning() {
    use interweave_human_ui_model::{IdentityFlow, RecoveryReason, ScreenId, Start};
    use interweave_human_ui_slint::{IdentityInput, apply_identity, identity_page};
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let mut say = |flow: &IdentityFlow| {
        let page = identity_page(flow);
        view.trace_identity(Some(flow.screen()), Some(&page));
    };

    let mut first_run = IdentityFlow::new(&["abandon"], Start::FirstRun { void_paper: false });
    say(&first_run);
    let _ = apply_identity(&mut first_run, IdentityInput::Secondary);
    assert_eq!(first_run.screen(), ScreenId::R1);
    say(&first_run);
    let _ = apply_identity(&mut first_run, IdentityInput::Primary);
    assert_eq!(first_run.screen(), ScreenId::R2);
    say(&first_run);
    let recovery = |expected| {
        IdentityFlow::new(
            &["abandon"],
            Start::Recovery {
                reason: RecoveryReason::Key,
                expected,
                new_keys: false,
            },
        )
    };
    say(&recovery(Some(peer())));
    say(&recovery(None));
    assert_eq!(
        *said.borrow(),
        [
            "ui-trace identity=w",
            "ui-trace identity=r1",
            "ui-trace warns=active-elsewhere",
            "ui-trace identity=r2",
            "ui-trace warns=none",
            "ui-trace identity=k1",
            "ui-trace warns=active-elsewhere",
            "ui-trace warns=none",
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

/// A prompt answered says where focus stands once it settled: R's Try
/// again cancelled leaves focus where it was, and the trace states it
/// (j98, A15); granted, R goes with its button and the move to Chats'
/// heading is said once, with no statement after it (A14).
#[test]
fn a_prompt_answered_says_where_focus_stands() {
    use interweave_human_ui_model::{StoreRecovery, StoreScreen};
    use interweave_human_ui_slint::store_page;
    let settle = || {
        for _ in 0..3 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(100));
            slint::platform::update_timers_and_animations();
        }
    };
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let model = UiModel::new();
    view.render(&model);
    view.show_identity(Some(store_page(&StoreRecovery::new(
        StoreScreen::Recovery,
        1,
    ))));
    slint::platform::update_timers_and_animations();
    let reached = (0..16).any(|_| {
        tab(&view);
        let _ =
            ElementHandle::find_by_accessible_label(view.window(), text(UiText::TryAgain)).count();
        said.borrow()
            .last()
            .is_some_and(|l| l == "ui-trace focus=identity-primary")
    });
    assert!(reached, "Try again by TAB in {:#?}", said.borrow());

    let first = said.borrow().len();
    settle();
    assert_eq!(said.borrow().len(), first, "control: nothing unasked");
    view.trace_focus_settled();
    settle();
    assert_eq!(
        said.borrow()[first..],
        ["ui-trace focus=identity-primary"],
        "cancelled: focus stays, and is said"
    );

    let first = said.borrow().len();
    view.trace_focus_settled();
    view.show_identity(None);
    view.render(&model);
    settle();
    assert_eq!(
        said.borrow()[first..],
        ["ui-trace focus=conversations-heading"],
        "granted: the move is said, once"
    );
}

/// A screen that comes without a focus move still has focus stated: a
/// conversation opened from Chats, and R rising over a start's refusal
/// (j98, 3b item 4), each once it settled. Mutation: without the
/// statements neither says a focus line.
#[test]
fn a_screen_change_says_where_focus_stands() {
    use interweave_human_ui_model::{ConversationKey, StoreRecovery, StoreScreen};
    use interweave_human_ui_slint::store_page;
    let settle = || {
        for _ in 0..3 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(100));
            slint::platform::update_timers_and_animations();
        }
    };
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let mut model = UiModel::new();
    view.render(&model);
    settle();

    let first = said.borrow().len();
    view.select(ConversationKey::Direct {
        peer: peer(),
        endpoint: None,
    });
    let _ = intents(&mut view, &mut model);
    settle();
    let lines = said.borrow()[first..].to_vec();
    assert_eq!(
        lines.first().map(String::as_str),
        Some("ui-trace page=conversation")
    );
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("ui-trace focus="))
            .count(),
        1,
        "the conversation's focus, once, in {lines:#?}"
    );

    let first = said.borrow().len();
    view.show_identity(Some(store_page(&StoreRecovery::new(
        StoreScreen::Recovery,
        1,
    ))));
    settle();
    let lines = said.borrow()[first..].to_vec();
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("ui-trace focus="))
            .count(),
        1,
        "R's focus, once, in {lines:#?}"
    );
}

/// R raised by a refused start: the trace names its heading's own key,
/// which is what identity note section 10 item 15 reads, and nothing
/// else once it settled.
#[test]
fn a_raised_r_says_its_heading() {
    use interweave_human_ui_model::{StoreRecovery, StoreScreen};
    use interweave_human_ui_slint::store_page;
    let settle = || {
        for _ in 0..3 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(100));
            slint::platform::update_timers_and_animations();
        }
    };
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let model = UiModel::new();
    view.render(&model);
    settle();

    let first = said.borrow().len();
    let r = store_page(&StoreRecovery::new(StoreScreen::Recovery, 1));
    let title = r.title.to_string();
    view.show_identity(Some(r));
    view.raise_identity_heading();
    // A headless window makes the heading, inside an `if`, only when the
    // tree is walked; a real one draws first.
    let _ = ElementHandle::find_by_accessible_label(view.window(), &title).count();
    settle();
    let focus: Vec<String> = said.borrow()[first..]
        .iter()
        .filter(|l| l.starts_with("ui-trace focus="))
        .cloned()
        .collect();
    assert!(
        !focus.is_empty() && focus.iter().all(|l| l == "ui-trace focus=r-heading"),
        "R's heading, and only it, in {:#?}",
        said.borrow()[first..].to_vec()
    );
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
    // Ctrl+Tab is no Tab of the page's (`FocusMode.navigates`), and draws
    // no ring; Tab draws it, the page having taken focus by a call.
    common::ctrl_tab(view.window().window());
    assert_eq!(said.borrow().len(), first, "Ctrl+Tab moves no focus");
    assert!(
        common::shown_ring(view.window()).is_none(),
        "control: no ring yet"
    );
    for _ in 0..3 {
        tab(&view);
    }
    assert!(
        common::shown_ring(view.window()).is_some(),
        "Tab draws the ring"
    );
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

fn tap(view: &View, x: f32, y: f32) {
    use slint::platform::{PointerEventButton, WindowEvent};
    let position = slint::LogicalPosition::new(x, y);
    let window = view.window().window();
    window.dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    window.dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}

/// One word per letter but x, so the keypad has a disabled key; "act"
/// is a whole word that starts two more, so its chip leads a row of
/// three (design/2026-10-19-android-identity.md section 4).
const WORDS: &[&str] = &[
    "act", "action", "actor", "bean", "cabin", "dawn", "eagle", "fabric", "galaxy", "habit",
    "icon", "jacket", "kale", "label", "machine", "napkin", "oak", "paddle", "quarter", "rabbit",
    "saddle", "table", "ugly", "vacant", "wagon", "yard", "zebra", "zone",
];

/// The identity page as the root runs it: each input applied to the
/// flow and the page shown again, then two timer turns, as the app's
/// loop gives them.
struct Identity {
    view: View,
    flow: interweave_human_ui_model::IdentityFlow,
    said: Rc<RefCell<Vec<String>>>,
    queued: Rc<RefCell<Vec<interweave_human_ui_slint::IdentityInput>>>,
    applied: Vec<interweave_human_ui_slint::IdentityInput>,
}

impl Identity {
    fn new(flow: interweave_human_ui_model::IdentityFlow) -> Self {
        i_slint_backend_testing::init_no_event_loop();
        let mut view = View::new(RuntimeHost::Embedded).expect("a window");
        view.window().show().expect("shown");
        let said = Rc::new(RefCell::new(Vec::<String>::new()));
        let to = Rc::clone(&said);
        view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
        view.render(&UiModel::new());
        let queued = Rc::new(RefCell::new(Vec::new()));
        let into = Rc::clone(&queued);
        view.on_identity(move |input| into.borrow_mut().push(input));
        let mut page = Self {
            view,
            flow,
            said,
            queued,
            applied: Vec::new(),
        };
        page.settle();
        page
    }

    /// R2, the restore's word entry, with nothing entered.
    fn at_r2() -> Self {
        use interweave_human_ui_model::{IdentityFlow, ScreenId, Start};
        use interweave_human_ui_slint::{IdentityInput, apply_identity};
        let mut flow = IdentityFlow::new(WORDS, Start::FirstRun { void_paper: false });
        apply_identity(&mut flow, IdentityInput::Secondary);
        apply_identity(&mut flow, IdentityInput::Primary);
        assert_eq!(flow.screen(), ScreenId::R2, "control: at R2");
        Self::new(flow)
    }

    /// C3, checking positions 8 and 20 (galaxy, saddle) of WORDS' first
    /// 24, with nothing entered.
    fn at_c3() -> Self {
        use interweave_human_ui_model::{IdentityFlow, ScreenId, Start};
        use interweave_human_ui_slint::{IdentityInput, apply_identity};
        use interweave_transport_api::TransportIdentity;
        let mut flow = IdentityFlow::new(WORDS, Start::FirstRun { void_paper: false });
        apply_identity(&mut flow, IdentityInput::Primary);
        let words: [&'static str; 24] = WORDS[..24].try_into().expect("24 words");
        let peer = TransportIdentity::parse("12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e")
            .expect("a PeerId");
        flow.created(words, peer, [8, 20]);
        apply_identity(&mut flow, IdentityInput::Primary);
        apply_identity(&mut flow, IdentityInput::Primary);
        assert_eq!(flow.screen(), ScreenId::C3, "control: at C3");
        Self::new(flow)
    }

    fn settle(&mut self) {
        let queued: Vec<_> = self.queued.borrow_mut().drain(..).collect();
        for input in queued {
            self.applied.push(input.clone());
            interweave_human_ui_slint::apply_identity(&mut self.flow, input);
        }
        self.view
            .show_identity(Some(interweave_human_ui_slint::identity_page(&self.flow)));
        // A headless window makes an element inside an `if` only when
        // the tree is walked; a real one draws first.
        let _ = ElementHandle::find_by_accessible_label(self.view.window(), "q").count();
        for _ in 0..2 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
            slint::platform::update_timers_and_animations();
        }
    }

    fn key(&mut self, key: impl Into<slint::SharedString>) {
        use slint::platform::WindowEvent;
        let text = key.into();
        let window = self.view.window().window();
        window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        window.dispatch_event(WindowEvent::KeyReleased { text });
        self.settle();
    }

    /// Keys dispatched in one batch, before the inputs they queued are
    /// applied: as an event loop that drains every pending input event
    /// before its timers run hands them over.
    fn burst(&mut self, keys: &[slint::SharedString]) {
        use slint::platform::WindowEvent;
        let window = self.view.window().window();
        for text in keys {
            window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
            window.dispatch_event(WindowEvent::KeyReleased { text: text.clone() });
        }
        self.settle();
    }

    fn tab(&mut self) {
        self.key(slint::platform::Key::Tab);
    }

    fn backtab(&mut self) {
        self.key(slint::platform::Key::Backtab);
    }

    /// Tab with Shift held, as a hardware keyboard may send Shift+Tab.
    fn shift_tab(&mut self) {
        use slint::platform::{Key, WindowEvent};
        let (shift, tab): (slint::SharedString, slint::SharedString) =
            (Key::Shift.into(), Key::Tab.into());
        let window = self.view.window().window();
        window.dispatch_event(WindowEvent::KeyPressed {
            text: shift.clone(),
        });
        window.dispatch_event(WindowEvent::KeyPressed { text: tab.clone() });
        window.dispatch_event(WindowEvent::KeyReleased { text: tab });
        window.dispatch_event(WindowEvent::KeyReleased { text: shift });
        self.settle();
    }

    fn enter(&mut self) {
        self.key(slint::platform::Key::Return);
    }

    fn last(&self) -> String {
        self.said.borrow().last().cloned().unwrap_or_default()
    }

    /// The word the restore holds at `position`.
    fn word(&self, position: usize) -> Option<&'static str> {
        self.flow.entry().and_then(|entry| entry.word(position))
    }

    /// The letters typed at the position being entered.
    fn typed(&self) -> String {
        self.flow
            .picker()
            .map(|picker| picker.typed().to_owned())
            .unwrap_or_default()
    }

    /// What the page's controls did since the last call.
    fn take(&mut self) -> Vec<interweave_human_ui_slint::IdentityInput> {
        std::mem::take(&mut self.applied)
    }
}

/// R2 by keyboard alone (design section 8): TAB reaches every grid
/// position, then every key of the keypad in its reading order, a
/// disabled one too, then the actions, and comes round; Enter presses
/// the focused key and focus stays on it while the page is rebuilt.
#[test]
fn r2_reaches_every_position_and_key_by_tab_in_reading_order() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    // R2 opens on the first key; back round to the primary, the last.
    for _ in 0..26 {
        page.backtab();
    }
    assert_eq!(page.last(), "ui-trace focus=identity-primary", "control");
    // The app bar's arrow heads the order (section 8).
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-back");
    for i in 0..24 {
        page.tab();
        assert_eq!(page.last(), format!("ui-trace focus=identity-slot:{i}"));
    }
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Slot(23)]);
    // A position chosen hands focus to the keypad's first key, typing
    // (section 8); the key's focus line names the kind alone, never its
    // letter. Off it and back, to navigate.
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.tab();
    page.backtab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('q')], "the first key is q");
    // The page rebuilt; focus stayed on q, so the next stop is w.
    page.key(slint::platform::Key::Backspace);
    page.take();
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')]);
    page.key(slint::platform::Key::Backspace);
    page.take();
    // On to x, which no word continues with: a stop, pressing nothing.
    for _ in 0..19 {
        page.tab();
    }
    page.enter();
    assert_eq!(page.take(), [], "x is disabled and keeps its stop");
    for _ in 0..5 {
        page.tab();
    }
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('m')], "m is the last key");
    page.key(slint::platform::Key::Backspace);
    page.take();
    let first = page.said.borrow().len();
    loop {
        page.tab();
        if page.last() == "ui-trace focus=identity-slot:0" {
            break;
        }
        assert!(page.said.borrow().len() - first < 8, "came round");
    }
    let lines = page.said.borrow()[first..].to_vec();
    assert_eq!(
        lines[..4],
        [
            "ui-trace focus=identity-delete",
            "ui-trace focus=identity-clear",
            "ui-trace focus=identity-primary",
            "ui-trace focus=identity-back",
        ]
    );
    for line in &lines {
        assert!(line.starts_with("ui-trace focus=identity-"), "{lines:#?}");
    }
}

/// A chip filled a position: focus goes to the keypad's first enabled
/// key (section 8). Enter takes the whole-word chip when the letters
/// typed are a word, beside the words it starts.
#[test]
fn a_chip_hands_focus_to_the_keypads_first_enabled_key() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    for letter in ["z", "e"] {
        page.key(letter);
    }
    assert_eq!(
        page.take(),
        [IdentityInput::Key('z'), IdentityInput::Key('e')]
    );
    // R2 opened on the first key; the one chip left is the stop before.
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-chip");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Chip("zebra".to_owned())]);
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    // A chip taken by Enter returns typing too (section 8): Enter then
    // presses no key, and with no letters takes nothing.
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter], "no key pressed");
    assert_eq!(page.typed(), "");
    // Control: focus is on q, the first enabled, so TAB and Enter press w.
    page.tab();
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Key('w')],
        "q, the first enabled"
    );

    // Typing again: Enter is the suggestion rule's, not the focused
    // key's, and with three words and no whole one it takes nothing.
    page.key(slint::platform::Key::Backspace);
    page.key("a");
    page.enter();
    assert_eq!(
        page.take(),
        [
            IdentityInput::Delete,
            IdentityInput::Key('a'),
            IdentityInput::Enter
        ]
    );
    assert_eq!(page.typed(), "a", "no chip taken, no key pressed");
    for letter in ["c", "t"] {
        page.key(letter);
    }
    page.take();
    let chips = interweave_human_ui_slint::identity_page(&page.flow).chips;
    assert_eq!(
        slint::Model::row_count(&chips),
        3,
        "control: act, action, actor"
    );
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter]);
    assert_eq!(page.word(1), Some("act"));
    // Still typing after the rule's Enter: Space and Enter add nothing to
    // the key focus went to.
    page.key(" ");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter], "Space sends nothing");
    assert_eq!(page.typed(), "", "typing: Space and Enter press no key");
    // TAB is navigating again: Enter presses the focused key, w after q.
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')]);
}

impl Identity {
    /// R2 with positions 1 to 23 filled from WORDS but `empty` (an
    /// index), and position 24 current.
    fn at_r2_but_24(empty: Option<usize>) -> Self {
        use interweave_human_ui_model::{IdentityFlow, ScreenId, Start};
        use interweave_human_ui_slint::{IdentityInput, apply_identity};
        let mut flow = IdentityFlow::new(WORDS, Start::FirstRun { void_paper: false });
        apply_identity(&mut flow, IdentityInput::Secondary);
        apply_identity(&mut flow, IdentityInput::Primary);
        assert_eq!(flow.screen(), ScreenId::R2, "control: at R2");
        for (at, word) in WORDS[..23].iter().enumerate() {
            if Some(at) == empty {
                continue;
            }
            apply_identity(&mut flow, IdentityInput::Slot(at));
            take_word(&mut flow, word);
        }
        apply_identity(&mut flow, IdentityInput::Slot(23));
        let entry = flow.entry().expect("R2's entry");
        assert_eq!(entry.current(), 23, "control: 24 current");
        assert_eq!(
            (0..23).filter(|&at| entry.word(at).is_none()).count(),
            usize::from(empty.is_some()),
            "control: only `empty` left empty"
        );
        Self::new(flow)
    }
}

/// The exception to the chip's rule (section 8): a chip that leaves no
/// R2 position empty sets navigating with focus on Check my words, so
/// the next Enter runs the check rather than doing nothing on a key.
#[test]
fn a_chip_that_fills_the_last_empty_position_hands_focus_to_check() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2_but_24(None);
    page.take();
    page.key("v");
    assert_eq!(page.take(), [IdentityInput::Key('v')]);
    // vacant is the one chip; the stop before the first key.
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-chip", "control");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Chip("vacant".to_owned())]);
    assert_eq!(page.word(23), Some("vacant"));
    assert_eq!(page.last(), "ui-trace focus=identity-primary");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Primary], "navigating");
    // R2 waits on the check, which the root runs off the page.
    assert!(page.flow.waiting(), "the check started");
}

/// The same chip with position 3 still empty keeps the chip's rule:
/// position 3 becomes current, focus on the keypad's first enabled key,
/// typing, so the next Enter does nothing.
#[test]
fn a_chip_that_leaves_a_position_empty_hands_focus_to_the_keypad() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2_but_24(Some(2));
    page.take();
    page.key("v");
    assert_eq!(page.take(), [IdentityInput::Key('v')]);
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-chip", "control");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Chip("vacant".to_owned())]);
    assert_eq!(
        page.flow.entry().expect("R2").current(),
        2,
        "position 3 current"
    );
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter], "typing");
}

/// Enter judged on the letters typed, not on the page shown: letters and
/// Enter that reach the window before the page is rebuilt for them take
/// the whole-word chip all the same (j261).
#[test]
fn enter_takes_the_whole_word_typed_before_the_page_caught_up() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    // Off the first key, so the chip's focus rule has somewhere to move.
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-key", "control: w");
    page.burst(&letters_then("act", &[slint::platform::Key::Return]));
    assert_eq!(
        page.take(),
        [
            IdentityInput::Key('a'),
            IdentityInput::Key('c'),
            IdentityInput::Key('t'),
            IdentityInput::Enter,
        ]
    );
    assert_eq!(page.word(0), Some("act"), "the whole word was taken");
    assert_eq!(page.typed(), "");
    // The chip's focus rule followed the page that showed it taken: on
    // q, the first enabled key, so TAB and Enter press w, not e.
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')]);
}

/// `word`'s letters, then `keys`, as one burst gives them.
fn letters_then(word: &str, keys: &[slint::platform::Key]) -> Vec<slint::SharedString> {
    word.chars()
        .map(|letter| slint::SharedString::from(letter.to_string()))
        .chain(keys.iter().map(|key| (*key).into()))
        .collect()
}

/// An Enter queued behind the Escape that asks the discard dialog takes
/// nothing: in the order the keys came, the dialog was up (#634 F1).
#[test]
fn enter_queued_behind_the_discard_dialog_takes_nothing() {
    use interweave_human_ui_slint::IdentityInput;
    use slint::platform::Key;
    let mut page = Identity::at_r2();
    page.burst(&letters_then("act", &[Key::Escape, Key::Return]));
    assert_eq!(
        page.take(),
        [
            IdentityInput::Key('a'),
            IdentityInput::Key('c'),
            IdentityInput::Key('t'),
            IdentityInput::Back,
            IdentityInput::Enter,
        ],
        "control: the stale page let the Enter through"
    );
    assert!(page.flow.discard_asked());
    assert_eq!(page.word(0), None, "no word taken behind the dialog");
    assert_eq!(page.typed(), "act");
}

/// Type `word`'s letters on `flow` until it is a chip, then take it.
fn take_word(flow: &mut interweave_human_ui_model::IdentityFlow, word: &str) {
    use interweave_human_ui_slint::{IdentityInput, apply_identity};
    for letter in word.chars() {
        if flow.picker().and_then(|p| p.chip(word)).is_some() {
            break;
        }
        apply_identity(flow, IdentityInput::Key(letter));
    }
    apply_identity(flow, IdentityInput::Chip(word.to_owned()));
}

impl Identity {
    /// TAB until `control` has focus, at most once round the page.
    fn focus_on(&mut self, control: &str) {
        let line = format!("ui-trace focus=identity-{control}");
        for _ in 0..80 {
            if self.last() == line {
                return;
            }
            self.tab();
        }
        panic!("{control} never took focus");
    }
}

/// Check my words, pressed by a Return queued behind the Escape that
/// asks the discard dialog, checks nothing: in the order the keys came,
/// the dialog was up (#634 review, pre-existing 1).
#[test]
fn check_queued_behind_the_discard_dialog_checks_nothing() {
    use interweave_human_ui_model::{Effect, IdentityFlow, ScreenId, Start};
    use interweave_human_ui_slint::{IdentityInput, apply_identity};
    use slint::platform::Key;
    let mut flow = IdentityFlow::new(WORDS, Start::FirstRun { void_paper: false });
    apply_identity(&mut flow, IdentityInput::Secondary);
    apply_identity(&mut flow, IdentityInput::Primary);
    for word in &WORDS[..24] {
        take_word(&mut flow, word);
    }
    assert!(
        flow.entry()
            .is_some_and(interweave_human_ui_model::PhraseEntry::complete),
        "control: full"
    );
    let mut page = Identity::new(flow);
    page.focus_on("primary");
    page.burst(&letters_then("", &[Key::Escape, Key::Return]));
    assert_eq!(
        page.take(),
        [IdentityInput::Back, IdentityInput::Primary],
        "control: the stale page let the Return through"
    );
    assert!(page.flow.discard_asked(), "the dialog stays up");
    assert_eq!(page.flow.screen(), ScreenId::R2);
    page.flow.discard(false);
    assert_eq!(
        page.flow.primary(),
        [Effect::Check(WORDS[..24].join(" "))],
        "no check was started behind the dialog"
    );
}

/// C3's Try again and Show words again, pressed by a Return queued
/// behind the Escape that asks the discard dialog, do nothing (#634
/// review, pre-existing 1).
#[test]
fn c3s_try_again_queued_behind_the_discard_dialog_does_nothing() {
    c3s_action_queued_behind_the_dialog(
        "secondary",
        &interweave_human_ui_slint::IdentityInput::Secondary,
    );
}

/// As above, for Show words again.
#[test]
fn c3s_show_words_again_queued_behind_the_discard_dialog_does_nothing() {
    c3s_action_queued_behind_the_dialog(
        "tertiary",
        &interweave_human_ui_slint::IdentityInput::Tertiary,
    );
}

fn c3s_action_queued_behind_the_dialog(
    control: &str,
    input: &interweave_human_ui_slint::IdentityInput,
) {
    use interweave_human_ui_model::ScreenId;
    use interweave_human_ui_slint::IdentityInput;
    use slint::platform::Key;
    let mut page = Identity::at_c3();
    take_word(&mut page.flow, "galaxy");
    take_word(&mut page.flow, "habit");
    assert_eq!(page.flow.check(), Some((20, Some(20))), "control: wrong");
    page.settle();
    page.take();
    page.focus_on(control);
    page.burst(&letters_then("", &[Key::Escape, Key::Return]));
    assert_eq!(
        page.take(),
        [IdentityInput::Back, input.clone()],
        "control: the stale page let the Return through"
    );
    assert!(page.flow.discard_asked(), "the dialog stays up");
    assert_eq!(page.flow.screen(), ScreenId::C3);
    assert_eq!(page.flow.check(), Some((20, Some(20))), "nothing acted");
}

impl Identity {
    /// A finger's tap on the middle of the element whose accessible
    /// label is `label` (a button's, or a text's when `text`), queued as
    /// a pointer press and release with no turn after it, so a burst of
    /// keys can follow in the same batch.
    fn tap_queued(&mut self, label: &str, text: bool) {
        use i_slint_backend_testing::AccessibleRole;
        use slint::platform::{PointerEventButton, WindowEvent};
        let role = if text {
            AccessibleRole::Text
        } else {
            AccessibleRole::Button
        };
        let found = ElementHandle::find_by_accessible_label(self.view.window(), label)
            .find(|e| e.accessible_role() == Some(role))
            .unwrap_or_else(|| panic!("no {role:?} labelled {label:?}"));
        let (at, size) = (found.absolute_position(), found.size());
        let position =
            slint::LogicalPosition::new(at.x + size.width / 2.0, at.y + size.height / 2.0);
        let window = self.view.window().window();
        window.dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        window.dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
    }

    /// A screen reader's activate on the button labelled `label`.
    fn activate(&mut self, label: &str) {
        use i_slint_backend_testing::AccessibleRole;
        ElementHandle::find_by_accessible_label(self.view.window(), label)
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .unwrap_or_else(|| panic!("no button labelled {label:?}"))
            .invoke_accessible_default_action();
        self.settle();
    }

    /// Taps on the buttons labelled `labels`, each applied before the
    /// next, as a finger gives them.
    fn tap(&mut self, labels: &[&str]) {
        for label in labels {
            self.tap_queued(label, false);
            self.settle();
        }
    }

    /// The position line R2 or C3 shows over the letters, as said.
    fn position_text(&self) -> String {
        interweave_human_ui_slint::identity_page(&self.flow)
            .position
            .to_string()
    }

    /// How many focus lines the trace has said.
    fn said_count(&self) -> usize {
        self.said.borrow().len()
    }
}

/// A tap on an enabled key is typing, and moves no focus (design
/// section 8, acceptance 15): on a newly opened R2 a tap on a leaves
/// focus on q, which "a" disables, and Enter and Space then send no
/// press of it -- Enter is the suggestion rule's, and with three words
/// and none whole it takes nothing. Replaces #644's test, written when a
/// tap set navigating.
#[test]
fn a_tapped_key_is_typing_and_moves_no_focus() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    assert_eq!(page.last(), "ui-trace focus=identity-key", "control: q");
    let lines = page.said_count();
    page.tap(&["a"]);
    assert_eq!(page.take(), [IdentityInput::Key('a')]);
    assert_eq!(page.said_count(), lines, "the tap moved no focus");
    page.key(" ");
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Enter],
        "typing: Space sends nothing, Enter is the rule's"
    );
    assert_eq!(page.typed(), "a", "no chip taken, no key pressed");
    assert_eq!(page.word(0), None);
    // Control: focus is still q, so TAB and Enter press w; a tap that
    // had taken focus to a would lead to s.
    page.key(slint::platform::Key::Backspace);
    page.take();
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')], "focus was on q");
}

/// Taps on a, c and t, then Enter, take the whole-word chip "act"
/// (acceptance 15's "car"), whether the Enter comes after the last tap
/// was applied or in one burst with it, before the page caught up.
#[test]
fn taps_then_enter_take_the_whole_word() {
    taps_then_enter(false);
}

/// As above, the last tap and the Enter in one burst.
#[test]
fn taps_then_enter_in_one_burst_take_the_whole_word() {
    taps_then_enter(true);
}

fn taps_then_enter(burst: bool) {
    use interweave_human_ui_slint::IdentityInput;
    use slint::platform::Key;
    let mut page = Identity::at_r2();
    page.tap(&["a", "c"]);
    page.tap_queued("t", false);
    if !burst {
        page.settle();
    }
    page.burst(&letters_then("", &[Key::Return]));
    assert_eq!(
        page.take(),
        [
            IdentityInput::Key('a'),
            IdentityInput::Key('c'),
            IdentityInput::Key('t'),
            IdentityInput::Enter,
        ]
    );
    assert_eq!(page.word(0), Some("act"), "Enter took the whole word");
}

/// After a Tab to a key, a tap on another is typing all the same: Enter
/// does not press the focused key.
#[test]
fn a_tap_after_tab_is_typing() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-key", "control: w");
    page.tap(&["a"]);
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('a'), IdentityInput::Enter]);
    assert_eq!(page.typed(), "a", "w was not pressed");
}

/// A tap on the delete key is typing: "acti", a tap on delete, Enter
/// takes "act". A tap on the clear key is typing too, and Enter then
/// takes nothing. Pressed by Enter instead, each is navigating, so a
/// second Enter presses it again.
#[test]
fn tapped_delete_and_clear_are_typing_and_pressed_ones_navigate() {
    use interweave_human_ui_model::UiText;
    use interweave_human_ui_slint::IdentityInput;
    let (delete, clear) = (text(UiText::IdKeyDelete), text(UiText::IdKeyClear));
    let mut page = Identity::at_r2();
    page.tap(&["a", "c", "t", "i", delete]);
    page.enter();
    assert_eq!(page.take().last(), Some(&IdentityInput::Enter));
    assert_eq!(page.word(0), Some("act"), "Enter took the whole word");

    page.tap(&["a", clear]);
    assert_eq!(page.typed(), "", "control: cleared");
    page.enter();
    assert_eq!(
        page.take(),
        [
            IdentityInput::Key('a'),
            IdentityInput::Clear,
            IdentityInput::Enter
        ]
    );
    assert_eq!(page.word(1), None, "nothing taken");

    // Control: delete pressed by Enter navigates, so the next Enter
    // presses it again.
    page.tap(&["a", "c"]);
    page.focus_on("delete");
    page.take();
    page.enter();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Delete, IdentityInput::Delete]);
    assert_eq!(page.typed(), "");
}

/// A tap on text that does nothing -- the position line, the letters --
/// leaves the mode as it was: typing after taps on a, c and t, so Enter
/// takes "act"; navigating after a Tab to w, so Enter presses w.
#[test]
fn a_tap_on_text_leaves_the_mode_as_it_was() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    page.tap(&["a", "c", "t"]);
    let position = page.position_text();
    let lines = page.said_count();
    page.tap_queued(&position, true);
    page.settle();
    assert_eq!(page.said_count(), lines, "the tap moved no focus");
    page.enter();
    assert_eq!(page.take().last(), Some(&IdentityInput::Enter));
    assert_eq!(page.word(0), Some("act"), "still typing");

    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-key", "control: w");
    let position = page.position_text();
    page.tap_queued(&position, true);
    page.settle();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')], "still navigating");
}

/// A tap on a disabled key or the disabled primary acts on nothing, so
/// it leaves the mode as it was and moves no focus: navigating on w, a
/// tap on each, then Enter presses w.
#[test]
fn a_tap_on_a_disabled_key_leaves_the_mode_as_it_was() {
    use interweave_human_ui_model::UiText;
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-key", "control: w");
    // A fresh R2 enables every key but x, and the primary waits for 24
    // words: both taps land on a disabled control.
    page.tap(&["x", text(UiText::IdCheckWords)]);
    page.enter();
    // Navigating, focus still on w: a tap that took focus would press
    // x or the primary (nothing), one that set typing would send Enter.
    assert_eq!(page.take(), [IdentityInput::Key('w')]);
}

/// A screen reader's activate is a press, not a tap (ux-designer,
/// REPLY 01a1d262): after a tap on a key, delete or clear set typing,
/// activating the same control sets navigating, so Enter presses the
/// focused key rather than sending the suggestion rule's Enter.
#[test]
fn a_screen_reader_activate_after_a_tap_navigates() {
    use interweave_human_ui_model::UiText;
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    // Each control is tapped before it is activated: `by-tap` is the
    // control's own, so only a tap on the same one could leave it set. A
    // key is a new element on each page, so its tap and activate come in
    // one batch, before the page is rebuilt.
    let (delete, clear) = (text(UiText::IdKeyDelete), text(UiText::IdKeyClear));
    for (taps, control, typed) in [
        (&["r", "a"][..], "b", "rabb"),
        (&["a", "c", delete], delete, ""),
        (&["a", clear, "a"], clear, ""),
    ] {
        page.tap(taps);
        if control == "b" {
            page.tap_queued("b", false);
        }
        page.take();
        page.activate(control);
        assert_eq!(page.typed(), typed, "control: {control} acted");
        page.take();
        page.enter();
        assert_ne!(page.take(), [IdentityInput::Enter], "{control}: typing");
        page.tap(&[text(UiText::IdKeyClear)]);
        page.take();
    }
}

/// Identity acceptance 15, the screen-reader check as a host test
/// (design section 8): on a newly opened R2, an activate on the disabled
/// x does nothing and leaves typing, so Enter is the suggestion rule's;
/// then letters typed with focus on a key, and that key activated, and
/// the next Enter presses it again. The design's "ap", p, "app" needs a
/// word list with "ap" words; WORDS has rabbit, so "ra", b, "rabb".
#[test]
fn a_screen_reader_activate_then_enter_presses_the_key_again() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    page.activate("x");
    assert_eq!(page.typed(), "", "control: x is disabled");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter], "still typing");
    // From q along the reading order (q to p, a to l, z x c v) to b.
    for _ in 0..23 {
        page.tab();
    }
    page.key("r");
    page.key("a");
    page.take();
    page.activate("b");
    assert_eq!(page.typed(), "rab");
    page.take();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('b')], "navigating, on b");
    assert_eq!(page.typed(), "rabb");
}

/// A chip tapped fills the position and returns typing, focus on the
/// first enabled key: Enter then takes nothing.
#[test]
fn a_tapped_chip_returns_typing_on_the_first_enabled_key() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    page.tab();
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-key", "control: e");
    page.key("z");
    page.tap(&["zebra"]);
    assert_eq!(page.word(0), Some("zebra"));
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.take();
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Enter],
        "typing: no key pressed"
    );
    assert_eq!(page.typed(), "");
    // Control: the focus is q, the first enabled key, so TAB and Enter
    // press w; on e, where it was, they would press r.
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')]);
}

/// A grid position chosen, by a tap or by Enter, puts focus on the
/// keypad's first enabled key, typing: Enter then takes the whole-word
/// chip, so the position keeps its word. When the word put back leaves
/// no letter enabled, focus goes to the delete key.
#[test]
fn a_chosen_position_returns_typing_on_the_keypad() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    take_word(&mut page.flow, "act");
    take_word(&mut page.flow, "zebra");
    page.settle();
    page.take();
    page.tap(&["1 act"]);
    assert_eq!(page.take(), [IdentityInput::Slot(0)]);
    assert_eq!(page.typed(), "act", "control: the word put back");
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter]);
    assert_eq!(page.word(0), Some("act"), "position 1 keeps its word");

    // By Enter on the focused position, the word no letter continues.
    page.focus_on("slot:1");
    page.take();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Slot(1)]);
    assert_eq!(page.typed(), "zebra", "control: the word put back");
    assert_eq!(page.last(), "ui-trace focus=identity-delete");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter], "typing: not delete");
    assert_eq!(page.word(1), Some("zebra"), "position 2 keeps its word");
}

/// A focused chip the letters take off the row hands focus to the
/// keypad's first enabled key; one that stays keeps it, wherever the row
/// puts it.
#[test]
fn a_focused_chip_the_letters_remove_hands_focus_to_the_keypad() {
    let mut page = Identity::at_r2();
    page.key("a");
    page.key("c");
    // act, action, actor: back from q to the first chip, act.
    for _ in 0..3 {
        page.backtab();
    }
    assert_eq!(page.last(), "ui-trace focus=identity-chip", "control");
    page.tap(&["t", "i"]);
    assert_eq!(page.typed(), "acti", "control: only action is left");
    assert_eq!(page.last(), "ui-trace focus=identity-key", "act left");
    page.take();
    // Control: focus is on o, the one enabled key, so Tab and Enter
    // reach the key after it, p, which is disabled: nothing.
    page.tab();
    page.enter();
    assert_eq!(page.take(), [], "p is disabled");
}

/// A focused chip the letters keep holds focus and follows its word
/// wherever the row puts it: action, second of act, action and actor,
/// is first and alone after "i", and second again after delete. One
/// `Identity` per test: a second one panics (`init_no_event_loop`).
#[test]
fn a_focused_chip_the_letters_keep_holds_focus() {
    use interweave_human_ui_model::UiText;
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    for letter in ["a", "c", "t"] {
        page.key(letter);
    }
    // From q: actor, then action.
    page.backtab();
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-chip", "control");
    page.take();
    page.tap(&["i"]);
    assert_eq!(page.typed(), "acti", "control: action alone is left");
    // The trace says a focus line once while it holds, so a chip kept is
    // the last line still.
    assert_eq!(page.last(), "ui-trace focus=identity-chip", "on a chip");
    page.tap(&[text(UiText::IdKeyDelete)]);
    assert_eq!(page.typed(), "act", "control: act, action, actor again");
    assert_eq!(page.last(), "ui-trace focus=identity-chip", "on a chip");
    // Focus followed action back to second, so Tab reaches actor; held
    // at the first chip's index it would reach action.
    page.take();
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Chip("actor".to_owned())]);
}

/// app.slint's `chip-index` looks at four chips, the most a row holds.
#[test]
fn the_chip_row_holds_at_most_four() {
    assert_eq!(interweave_human_ui_model::MAX_CHIPS, 4);
}

/// C3 by keyboard: letters and Enter in one burst take the word and move
/// to the second position, focus on the first enabled key (#634 F3).
#[test]
fn c3s_enter_takes_the_word_typed() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_c3();
    page.tab();
    page.burst(&letters_then("gal", &[slint::platform::Key::Return]));
    assert_eq!(page.take().last(), Some(&IdentityInput::Enter));
    assert_eq!(
        page.flow.check(),
        Some((20, None)),
        "galaxy, the right word"
    );
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')], "focus was on q");
}

/// The platform's text size: 14 px is 100 %, as Android's font scale
/// sets it.
fn text_size(page: &mut Identity, px: f32) {
    i_slint_core::window::WindowInner::from_pub(page.view.window().window())
        .context()
        .set_platform_default_font_size(Some(i_slint_core::lengths::LogicalLength::new(px)));
    page.settle();
    page.settle();
}

/// Over 130 % font R2's grid is one button, "Show all my words", that
/// opens the words as a full list (section 4.4, section 8): the list
/// opens on the current position's row, Tab goes back arrow then rows,
/// no letter reaches the hidden picker, Back closes it onto the button
/// with nothing asked, a row closes it onto that position with focus on
/// the first enabled key, and the text falling to 100 % closes it onto
/// the grid's current position. At exactly 130 % the grid stays.
#[test]
fn r2s_grid_is_a_full_list_behind_a_button_over_130_percent() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    text_size(&mut page, 28.0);
    for letter in ["a", "c", "t"] {
        page.key(letter);
    }
    page.enter();
    assert_eq!(page.take().last(), Some(&IdentityInput::Enter));
    assert_eq!(page.word(0), Some("act"));
    assert_eq!(
        page.flow
            .entry()
            .map(interweave_human_ui_model::PhraseEntry::current),
        Some(1),
        "control"
    );

    // Focus is on the first key after the chip: the stop before it is
    // the button, the grid's one stop, and before it, at the banner-less
    // page's start, the app bar's arrow.
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-review");
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-back");
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-review");
    page.tab();
    assert_eq!(
        page.last(),
        "ui-trace focus=identity-key",
        "the button is the grid's one stop"
    );
    page.backtab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Review(true)]);
    assert!(page.flow.reviewing());
    assert_eq!(
        page.last(),
        "ui-trace focus=identity-slot:1",
        "the current row"
    );
    page.backtab();
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-back");
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-slot:0");
    let first = page.said.borrow().len();
    for _ in 0..24 {
        page.tab();
    }
    assert_eq!(
        page.said.borrow()[first..].last().map(String::as_str),
        Some("ui-trace focus=identity-back"),
        "rows 1 to 24, then round to the arrow: nothing of R2 behind"
    );
    page.key("b");
    page.key(slint::platform::Key::Backspace);
    assert_eq!(page.take(), [], "no letter edits a word unseen");
    assert_eq!(
        ElementHandle::find_by_accessible_label(page.view.window(), "q").count(),
        0,
        "R2's keypad is hidden under the list"
    );

    page.key(slint::platform::Key::Escape);
    assert_eq!(page.take(), [IdentityInput::Back]);
    assert!(!page.flow.reviewing() && !page.flow.discard_asked());
    assert_eq!(
        page.last(),
        "ui-trace focus=identity-review",
        "what opened it"
    );

    // The arrow is Back too.
    page.enter();
    page.backtab();
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-back");
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Review(true), IdentityInput::Back]
    );
    assert!(!page.flow.reviewing() && !page.flow.discard_asked());

    // A row: the list closes and R2 edits that position.
    page.enter();
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-slot:0");
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Review(true), IdentityInput::Slot(0)]
    );
    assert!(!page.flow.reviewing());
    assert_eq!(
        page.flow
            .picker()
            .map(interweave_human_ui_model::WordPicker::typed),
        Some("act")
    );
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    // In typing mode: Enter is the suggestion rule's, taking "act".
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter]);
    assert_eq!(page.word(0), Some("act"));

    // The text back to 100 % closes the list onto the grid.
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-review");
    page.enter();
    page.take();
    assert!(page.flow.reviewing(), "control: open");
    text_size(&mut page, 14.0);
    assert_eq!(page.take(), [IdentityInput::Review(false)]);
    assert!(!page.flow.reviewing());
    assert_eq!(
        page.last(),
        "ui-trace focus=identity-slot:1",
        "the grid's current"
    );

    // Exactly 130 %: still the grid.
    text_size(&mut page, 18.2);
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-slot:2");
}

/// The full list opens with the current position in view (section 4.4):
/// position 21 at 200 % on a 360 x 640 window, far below the first rows.
#[test]
fn r2s_full_list_opens_with_the_current_row_in_view() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    page.view
        .window()
        .window()
        .set_size(slint::LogicalSize::new(360.0, 640.0));
    text_size(&mut page, 28.0);
    interweave_human_ui_slint::apply_identity(&mut page.flow, IdentityInput::Slot(20));
    page.settle();
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-review", "control");
    page.enter();
    assert_eq!(page.last(), "ui-trace focus=identity-slot:20");
    // A row the Flickable clips away is not in the tree at all.
    let inside = |label: &str| {
        ElementHandle::find_by_accessible_label(page.view.window(), label)
            .filter(|e| {
                e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button)
            })
            .any(|e| {
                let (y, h) = (e.absolute_position().y, e.size().height);
                y >= 0.0 && y + h <= 640.0
            })
    };
    assert!(inside("21"), "the current row shows");
    assert!(!inside("1"), "control: the list scrolled to reach it");
}

/// C3's Back with a word entered asks the same dialog as R2's (section
/// 7): Continue entering stays on C3 with the letters, Delete words goes
/// back to C2, and with nothing entered Back goes straight back.
#[test]
fn c3_back_with_words_entered_asks_the_discard_dialog() {
    use interweave_human_ui_model::ScreenId;
    use interweave_human_ui_slint::IdentityInput;
    use slint::platform::Key;
    let mut page = Identity::at_c3();
    page.key("g");
    page.key(Key::Escape);
    assert_eq!(page.take(), [IdentityInput::Key('g'), IdentityInput::Back]);
    assert!(page.flow.discard_asked(), "letters typed: the dialog shows");
    assert_eq!(page.last(), "ui-trace focus=identity-dialog-keep");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Dialog(false)]);
    assert_eq!(page.flow.screen(), ScreenId::C3);
    assert_eq!(
        page.flow.picker().map(|p| p.typed().to_owned()),
        Some("g".to_owned()),
        "Continue entering keeps the letters"
    );
    page.key(Key::Escape);
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-dialog-delete");
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Back, IdentityInput::Dialog(true)]
    );
    assert_eq!(page.flow.screen(), ScreenId::C2, "Delete words: back to C2");
    page.flow.primary();
    page.settle();
    assert_eq!(page.flow.screen(), ScreenId::C3);
    page.key(Key::Escape);
    assert_eq!(page.take(), [IdentityInput::Back]);
    assert!(!page.flow.discard_asked());
    assert_eq!(
        page.flow.screen(),
        ScreenId::C2,
        "nothing entered: straight back"
    );
}

/// A banner that appears takes focus (section 8): Welcome's refused
/// lock, from its primary.
#[test]
fn a_banner_that_appears_takes_focus() {
    use interweave_human_ui_model::{IdentityFlow, Start, StepFailed};
    let mut page = Identity::new(IdentityFlow::new(
        WORDS,
        Start::FirstRun { void_paper: false },
    ));
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-primary");
    page.flow.held(Err(StepFailed::Busy));
    page.settle();
    assert_eq!(page.last(), "ui-trace focus=identity-banner");
    // Shown again, it stays where the person moved focus.
    page.tab();
    let at = page.last();
    assert_ne!(at, "ui-trace focus=identity-banner", "control: off it");
    page.settle();
    assert_eq!(page.last(), at);
    // No tap moves focus (section 8), the banner's text included.
    let notice = interweave_human_ui_slint::identity_page(&page.flow).notice;
    page.tap_queued(&notice, true);
    page.settle();
    assert_eq!(page.last(), at, "the tap left focus where it was");
}

/// The discard dialog (section 7): it opens with focus on Continue
/// entering, keeps TAB and every letter to itself, and closing gives
/// focus back to what had it; Back on it continues entering.
#[test]
fn the_discard_dialog_takes_focus_holds_it_and_gives_it_back() {
    use interweave_human_ui_slint::IdentityInput;
    use slint::platform::Key;
    let mut page = Identity::at_r2();
    page.key("o");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('o'), IdentityInput::Enter]);
    assert_eq!(page.word(0), Some("oak"));
    // Focus on w, the second key.
    page.tab();
    page.key(Key::Escape);
    assert!(page.flow.discard_asked(), "control: the dialog shows");
    assert_eq!(page.last(), "ui-trace focus=identity-dialog-keep");
    // Ctrl+Tab moves no focus in the dialog (`FocusMode.navigates`).
    let first = page.said.borrow().len();
    common::ctrl_tab(page.view.window().window());
    page.settle();
    assert_eq!(page.said.borrow().len(), first, "Ctrl+Tab moves no focus");
    page.tab();
    page.tab();
    page.backtab();
    assert_eq!(
        page.said.borrow()[first..],
        [
            "ui-trace focus=identity-dialog-delete",
            "ui-trace focus=identity-dialog-keep",
            "ui-trace focus=identity-dialog-delete",
        ]
    );
    page.key("b");
    page.key(Key::Backspace);
    assert_eq!(page.take(), [IdentityInput::Back], "only Escape's Back");
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Dialog(false)]);
    assert!(!page.flow.discard_asked());
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')], "back on w");

    // A tap outside the dialog is Continue entering too.
    page.key(Key::Backspace);
    page.key(Key::Escape);
    page.take();
    assert!(page.flow.discard_asked(), "control: the dialog shows");
    tap(&page.view, 2.0, 2.0);
    page.settle();
    assert_eq!(page.take(), [IdentityInput::Dialog(false)]);

    page.key(Key::Escape);
    page.key(Key::Escape);
    assert!(!page.flow.discard_asked(), "Back on the dialog continues");
    assert_eq!(
        page.flow.entry().and_then(|e| e.word(0)),
        Some("oak"),
        "and keeps the words"
    );
}

/// Shift+TAB steps back through R2's stops in the reverse of TAB's
/// order, across the segments: the last key, the actions, round to the
/// app bar's arrow and into the grid.
#[test]
fn the_identity_page_steps_back_in_reverse_order() {
    let mut page = Identity::at_r2();
    tab_to(&mut page, "ui-trace focus=identity-delete");
    page.backtab();
    let mut forward = vec![page.last()];
    for _ in 0..6 {
        page.tab();
        forward.push(page.last());
    }
    let mut back = vec![page.last()];
    for _ in 0..6 {
        page.backtab();
        back.push(page.last());
    }
    back.reverse();
    assert_eq!(back, forward);
    assert!(
        forward[0] == "ui-trace focus=identity-key"
            && forward[4] == "ui-trace focus=identity-back"
            && forward[6] == "ui-trace focus=identity-slot:1",
        "control: the walk crosses the segments: {forward:#?}"
    );
    // Shift+Tab can come as Tab with Shift held, as Slint's own
    // navigation takes it: back, from the grid's first row to the arrow.
    tab_to(&mut page, "ui-trace focus=identity-slot:0");
    page.shift_tab();
    assert_eq!(
        page.last(),
        "ui-trace focus=identity-back",
        "Tab with Shift steps back"
    );
    // Ctrl+Tab is no Tab of the page's (`FocusMode.navigates`).
    let before = page.said.borrow().len();
    common::ctrl_tab(page.view.window().window());
    page.settle();
    assert_eq!(page.said.borrow().len(), before, "Ctrl+Tab moves no focus");
}

/// Over 130 % font, a full-list row whose word no letter continues
/// puts focus on the delete key, the keypad having no enabled key
/// (section 8).
#[test]
fn a_row_whose_word_no_letter_continues_focuses_delete() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_r2();
    text_size(&mut page, 28.0);
    page.key("o");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('o'), IdentityInput::Enter]);
    assert_eq!(page.word(0), Some("oak"));
    tab_to(&mut page, "ui-trace focus=identity-review");
    page.enter();
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-slot:0", "control");
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Review(true), IdentityInput::Slot(0)]
    );
    assert_eq!(
        page.flow
            .picker()
            .map(interweave_human_ui_model::WordPicker::typed),
        Some("oak"),
        "control: no word continues oak"
    );
    assert_eq!(page.last(), "ui-trace focus=identity-delete");
    // Typing: Enter is the suggestion rule's, taking the whole word, not
    // the focused delete.
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter]);
    assert_eq!(page.word(0), Some("oak"));
}

/// Back from R2 that K1 opened puts focus on K1's "Enter my phrase"
/// (section 7).
#[test]
fn back_from_r2_to_k1_focuses_its_primary() {
    use interweave_human_ui_model::{IdentityFlow, RecoveryReason, ScreenId, Start};
    use interweave_transport_api::TransportIdentity;
    use slint::platform::Key;
    let peer = TransportIdentity::parse("12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e")
        .expect("a PeerId");
    let flow = IdentityFlow::new(
        WORDS,
        Start::Recovery {
            reason: RecoveryReason::Key,
            expected: Some(peer),
            new_keys: false,
        },
    );
    let mut page = Identity::new(flow);
    assert_eq!(page.flow.screen(), ScreenId::K1, "control");
    tab_to(&mut page, "ui-trace focus=identity-primary");
    page.enter();
    assert_eq!(page.flow.screen(), ScreenId::R2, "control: opened");
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.key(Key::Escape);
    assert_eq!(page.flow.screen(), ScreenId::K1);
    assert_eq!(page.last(), "ui-trace focus=identity-primary");
}

/// The dialog reached with no Tab before it: it opens on Continue
/// entering with no ring, Ctrl+Tab moves no focus and draws none, and
/// Tab moves to delete and draws it (#547's `FocusMode.navigates`).
#[test]
fn the_discard_dialog_draws_its_ring_on_tab_and_not_on_ctrl_tab() {
    use slint::platform::Key;
    let mut page = Identity::at_r2();
    page.key("o");
    page.enter();
    page.key(Key::Escape);
    assert!(page.flow.discard_asked(), "control: the dialog shows");
    assert_eq!(page.last(), "ui-trace focus=identity-dialog-keep");
    assert!(
        common::shown_ring(page.view.window()).is_none(),
        "control: no ring yet"
    );
    let before = page.said.borrow().len();
    common::ctrl_tab(page.view.window().window());
    page.settle();
    assert_eq!(page.said.borrow().len(), before, "Ctrl+Tab moves no focus");
    assert!(
        common::shown_ring(page.view.window()).is_none(),
        "Ctrl+Tab draws no ring"
    );
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-dialog-delete");
    assert!(
        common::shown_ring(page.view.window()).is_some(),
        "Tab draws the ring"
    );
}

/// TAB until focus is on `line`, a bounded number of times.
fn tab_to(page: &mut Identity, line: &str) {
    for _ in 0..64 {
        if page.last() == line {
            return;
        }
        page.tab();
    }
    panic!("TAB never reached {line}");
}

/// No tap moves focus (section 8): a tap on K1's title, a focus stop,
/// leaves focus where it was.
#[test]
fn the_heading_takes_no_tap() {
    use interweave_human_ui_model::{IdentityFlow, RecoveryReason, Start};
    use interweave_transport_api::TransportIdentity;
    let peer = TransportIdentity::parse("12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e")
        .expect("a PeerId");
    let mut page = Identity::new(IdentityFlow::new(
        WORDS,
        Start::Recovery {
            reason: RecoveryReason::Key,
            expected: Some(peer),
            new_keys: false,
        },
    ));
    let shown = interweave_human_ui_slint::identity_page(&page.flow);
    assert!(
        !shown.heading_key.is_empty(),
        "control: K1's title is a stop"
    );
    // TAB reaches the title, then the primary.
    tab_to(&mut page, "ui-trace focus=k1-heading");
    tab_to(&mut page, "ui-trace focus=identity-primary");
    let at = page.last();
    page.tap_queued(&shown.title, true);
    page.settle();
    assert_eq!(page.last(), at, "the tap left focus where it was");
}

/// R1, the restore's first screen, with nothing entered.
fn at_r1() -> Identity {
    use interweave_human_ui_model::{IdentityFlow, ScreenId, Start};
    use interweave_human_ui_slint::{IdentityInput, apply_identity};
    let mut flow = IdentityFlow::new(WORDS, Start::FirstRun { void_paper: false });
    apply_identity(&mut flow, IdentityInput::Secondary);
    assert_eq!(flow.screen(), ScreenId::R1, "control: at R1");
    Identity::new(flow)
}

/// R2 and C3 open with focus on the keypad's first enabled key, in
/// typing mode (section 8, acceptance 15): Enter takes nothing with no
/// suggestion, after letters with no single suggestion it still takes
/// nothing, Space never presses the key, and Enter after a whole word
/// takes its chip. Opened by R1's primary, pressed by Enter, it is
/// typing all the same.
#[test]
fn r2_and_c3_open_on_the_first_enabled_key_typing() {
    use interweave_human_ui_model::ScreenId;
    use interweave_human_ui_slint::IdentityInput;
    let mut page = at_r1();
    // The app bar's arrow, then the primary.
    page.tab();
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-primary", "control");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Primary]);
    assert_eq!(page.flow.screen(), ScreenId::R2, "control: opened");
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.enter();
    page.key(" ");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Enter, IdentityInput::Enter]);
    assert_eq!(page.typed(), "", "typing: nothing to take, q not pressed");
    for letter in ["a", "c"] {
        page.key(letter);
    }
    page.enter();
    page.key(" ");
    page.enter();
    assert_eq!(
        page.take(),
        [
            IdentityInput::Key('a'),
            IdentityInput::Key('c'),
            IdentityInput::Enter,
            IdentityInput::Enter
        ],
    );
    assert_eq!(
        page.typed(),
        "ac",
        "three words fit ac: Enter and Space take nothing"
    );
    page.key("t");
    page.enter();
    page.key(" ");
    assert_eq!(
        page.take(),
        [IdentityInput::Key('t'), IdentityInput::Enter],
        "Space sends nothing"
    );
    assert_eq!(page.word(0), Some("act"), "the whole word's chip");
    assert_eq!(page.typed(), "", "then Space nothing");
}

/// C3 opens on the first enabled key, typing, as R2 does (section 8).
#[test]
fn c3_opens_on_the_first_enabled_key_typing() {
    use interweave_human_ui_slint::IdentityInput;
    let mut page = Identity::at_c3();
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.enter();
    page.key(" ");
    assert_eq!(page.take(), [IdentityInput::Enter]);
    assert_eq!(page.typed(), "", "C3 opens typing too: q not pressed");
}

/// Back from R2 puts focus on R1's "Enter my words", which opened it
/// (section 7), with nothing entered and by Delete words.
#[test]
fn back_from_r2_focuses_what_opened_it() {
    use interweave_human_ui_model::ScreenId;
    use interweave_human_ui_slint::IdentityInput;
    use slint::platform::Key;
    let mut page = at_r1();
    // The app bar's arrow, then the primary.
    page.tab();
    page.tab();
    page.enter();
    assert_eq!(page.flow.screen(), ScreenId::R2, "control");
    page.key(Key::Escape);
    assert_eq!(page.flow.screen(), ScreenId::R1);
    assert_eq!(page.last(), "ui-trace focus=identity-primary");
    page.take();
    page.enter();
    page.key("o");
    page.enter();
    assert_eq!(
        page.take(),
        [
            IdentityInput::Primary,
            IdentityInput::Key('o'),
            IdentityInput::Enter
        ],
        "control: R2 again, a word entered"
    );
    assert_eq!(page.word(0), Some("oak"));
    page.key(Key::Escape);
    assert!(page.flow.discard_asked(), "control: a word entered");
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-dialog-delete");
    page.enter();
    assert_eq!(page.flow.screen(), ScreenId::R1);
    assert_eq!(page.last(), "ui-trace focus=identity-primary");
    assert_eq!(
        page.take().last(),
        Some(&IdentityInput::Dialog(true)),
        "control: Delete words"
    );
}

/// Back from C3 puts focus on C2's primary, which opened it (section 7),
/// with nothing entered and by Delete words.
#[test]
fn back_from_c3_focuses_what_opened_it() {
    use interweave_human_ui_model::ScreenId;
    use slint::platform::Key;
    let mut page = Identity::at_c3();
    page.key(Key::Escape);
    assert_eq!(page.flow.screen(), ScreenId::C2);
    assert_eq!(page.last(), "ui-trace focus=identity-primary");
    page.enter();
    assert_eq!(page.flow.screen(), ScreenId::C3, "control: it opened C3");
    page.key("g");
    page.key(Key::Escape);
    page.tab();
    page.enter();
    assert_eq!(page.flow.screen(), ScreenId::C2);
    assert_eq!(page.last(), "ui-trace focus=identity-primary");
}

/// Closing the discard dialog gives back the mode of the moment Back was
/// pressed (section 8): typing, with focus on q (enabled), Back, a Tab
/// between the dialog's actions and Back again, and Enter still presses
/// no key.
#[test]
fn closing_the_dialog_restores_the_mode_of_backs_moment() {
    use interweave_human_ui_slint::IdentityInput;
    use slint::platform::Key;
    let mut page = Identity::at_r2();
    page.key("o");
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Key('o'), IdentityInput::Enter],
        "control: typing, a word entered, focus on q"
    );
    assert_eq!(page.word(0), Some("oak"));
    page.key(Key::Escape);
    assert!(page.flow.discard_asked(), "control: the dialog shows");
    page.tab();
    page.tab();
    page.key(Key::Escape);
    assert!(!page.flow.discard_asked(), "control: closed");
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.enter();
    assert_eq!(
        page.take(),
        [
            IdentityInput::Back,
            IdentityInput::Back,
            IdentityInput::Enter
        ],
    );
    assert_eq!(page.typed(), "", "typing again: Enter presses no key");
}

/// Arrows move focus to the nearest stop their way (section 8), a
/// disabled key included, and Enter after an arrow presses that key:
/// Down from q is a, Down again z, Right x (no word has it), Right c;
/// across and down the grid's positions too.
#[test]
fn arrows_move_focus_to_the_nearest_stop_their_way() {
    use interweave_human_ui_slint::IdentityInput;
    use slint::platform::Key;
    let mut page = Identity::at_r2();
    page.key(Key::DownArrow);
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('a')], "below q");
    page.key(Key::Backspace);
    page.key(Key::DownArrow);
    page.key(Key::RightArrow);
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Delete], "x: disabled, a stop");
    page.key(Key::RightArrow);
    page.key(" ");
    assert_eq!(page.take(), [IdentityInput::Key('c')], "Space presses c");
    page.key(Key::Backspace);
    page.take();
    tab_to(&mut page, "ui-trace focus=identity-slot:0");
    page.key(Key::RightArrow);
    assert_eq!(page.last(), "ui-trace focus=identity-slot:1");
    page.key(Key::DownArrow);
    assert_eq!(page.last(), "ui-trace focus=identity-slot:5");
    page.key(Key::LeftArrow);
    assert_eq!(page.last(), "ui-trace focus=identity-slot:4");
    page.key(Key::UpArrow);
    assert_eq!(page.last(), "ui-trace focus=identity-slot:0");
    // A Tab after an arrow goes on from where the arrow left focus.
    page.key(Key::RightArrow);
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-slot:2");
}

/// An arrow never lands on a grid position the page's Flickable has
/// scrolled out of view, though its place lies the arrow's way: Up from
/// q on a short window reaches a position in view.
#[test]
fn an_arrow_skips_a_position_scrolled_out_of_view() {
    use slint::platform::Key;
    let mut page = Identity::at_r2();
    page.view
        .window()
        .window()
        .set_size(slint::LogicalSize::new(360.0, 640.0));
    page.settle();
    // A position the Flickable clips is not in the tree.
    let shown = |page: &Identity| -> Vec<usize> {
        (0..24)
            .filter(|i| {
                ElementHandle::find_by_accessible_label(page.view.window(), &(i + 1).to_string())
                    .any(|e| {
                        e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button)
                    })
            })
            .collect()
    };
    let before = shown(&page);
    assert!(before.len() < 24, "control: some positions scrolled away");
    assert_eq!(page.last(), "ui-trace focus=identity-key", "control: on q");
    page.key(Key::UpArrow);
    let at: usize = page
        .last()
        .strip_prefix("ui-trace focus=identity-slot:")
        .and_then(|i| i.parse().ok())
        .expect("a position");
    assert!(before.contains(&at), "slot {at} was in view: {before:?}");
}

/// Section 3's app bar: every screen whose Back stays in the app shows
/// the back arrow at the start side, labelled `AddClose`, heading the
/// page's TAB order, and pressing it is Back; W, a first screen, shows
/// none and TAB never meets one. In RTL the arrow is at the end side.
/// Mutation: without the page's arrow (or with `back_arrow` false), C1
/// has no `identity-back` stop and this fails; drawn on W, the count
/// there fails.
#[test]
fn the_app_bar_arrow_is_back_on_every_screen_that_has_one() {
    use interweave_human_ui_model::{IdentityFlow, ScreenId, Start};
    use interweave_human_ui_slint::{IdentityInput, apply_identity};
    let arrows = |page: &Identity| {
        ElementHandle::find_by_accessible_label(page.view.window(), text(UiText::AddClose))
            .filter(|e| {
                e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button)
            })
            .map(|e| e.absolute_position().x)
            .collect::<Vec<_>>()
    };

    let width = 360.0;
    let sized = |page: &mut Identity| {
        page.view
            .window()
            .window()
            .set_size(slint::LogicalSize::new(width, 640.0));
        page.settle();
    };
    let mut page = Identity::new(IdentityFlow::new(
        WORDS,
        Start::FirstRun { void_paper: false },
    ));
    sized(&mut page);
    assert_eq!(
        arrows(&page),
        Vec::<f32>::new(),
        "W leaves the app: no arrow"
    );
    let first = page.said.borrow().len();
    for _ in 0..4 {
        page.tab();
    }
    assert!(
        page.said.borrow()[first..]
            .iter()
            .all(|l| l != "ui-trace focus=identity-back"),
        "{:#?}",
        page.said.borrow()
    );

    apply_identity(&mut page.flow, IdentityInput::Primary);
    page.settle();
    assert_eq!(page.flow.screen(), ScreenId::C1, "control");
    let at = arrows(&page);
    assert_eq!(at.len(), 1, "C1's arrow");
    assert!(at[0] < width / 2.0, "at the start side: {at:?} of {width}");
    // The first TAB from the page itself reaches the arrow, then the
    // primary; Enter on the arrow is Back.
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-back");
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-primary");
    page.backtab();
    assert_eq!(page.last(), "ui-trace focus=identity-back");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Back]);
    assert_eq!(page.flow.screen(), ScreenId::W);

    // One platform per test: S1 on the same window.
    page.flow = IdentityFlow::new(WORDS, Start::ShowPhrase);
    page.view.window().set_rtl(true);
    sized(&mut page);
    let at = arrows(&page);
    assert_eq!(at.len(), 1, "S1's arrow, in RTL");
    assert!(
        at[0] > width / 2.0,
        "at the end side in RTL: {at:?} of {width}"
    );
    page.tab();
    assert_eq!(page.last(), "ui-trace focus=identity-back");
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Back]);
}

/// The notice in Chats' slot is named whichever it is: the discard's
/// shown once, and the unlock note's P-up notice, each under the
/// session notices (j245 finding 2).
#[test]
fn the_discards_and_the_p_up_notice_are_named() {
    use interweave_human_ui_model::{KeyUnlock, UnlockFacts};
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    // Wide: the pane carries the notice, under its offline line.
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 800.0));
    slint::platform::update_timers_and_animations();
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let mut model = UiModel::new();
    model.client_event(interweave_human_client_api::ClientEvent::Session(
        interweave_human_client_api::SessionState::Ready { endpoint: None },
    ));
    model.unlock_facts(
        Some(UnlockFacts {
            in_force: KeyUnlock::BackgroundCompatible,
            configured: Some(KeyUnlock::UserPresence),
            pending: None,
            restart_needs_unlock: Some(false),
        }),
        1,
        false,
    );
    view.show_once(UiText::StoreDiscarded);
    view.render(&model);
    model.network_access(false);
    view.render(&model);
    model.network_access(true);
    view.render(&model);
    // Offline on this wide window: the pane's offline line hides both,
    // so the trace names neither though both are held (#610 review F3).
    model.client_event(interweave_human_client_api::ClientEvent::Connectivity(
        interweave_human_client_api::Connectivity::Offline,
    ));
    view.render(&model);
    model.client_event(interweave_human_client_api::ClientEvent::Connectivity(
        interweave_human_client_api::Connectivity::OnlineDirect,
    ));
    view.render(&model);
    press(&view, text(UiText::Close));
    view.render(&model);
    model.client_event(interweave_human_client_api::ClientEvent::Connectivity(
        interweave_human_client_api::Connectivity::Offline,
    ));
    view.render(&model);
    let said = said.borrow();
    let notices: Vec<&String> = said
        .iter()
        .filter(|l| l.starts_with("ui-trace notice="))
        .collect();
    assert_eq!(
        notices,
        [
            "ui-trace notice=store-discarded",
            "ui-trace notice=network-denied",
            "ui-trace notice=store-discarded",
            "ui-trace notice=none",
            "ui-trace notice=store-discarded",
            "ui-trace notice=unlock-pending",
            "ui-trace notice=none",
        ],
        "in {said:#?}"
    );
}

/// A typed add refused is an answer the trace says, as a result is: a
/// driver waiting on the add fails on it instead of on its patience
/// (A40, 2026-11-09: a `PeerId` typed by key events lost its capitals and
/// the refusal printed nothing). The same refusal again is said again,
/// after the `none` the next Add's send clears it with.
#[test]
fn a_refused_add_is_said_each_time() {
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let mut model = UiModel::new();
    let me = peer();
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::PeersTitle));
    let _ = intents(&mut view, &mut model);
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me),
        allowed: Vec::new(),
    }));
    model.addresses_read(Ok(AddressList::default()));
    view.render(&model);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    let lowered = format!("/ip4/192.168.1.30/tcp/4001/p2p/{}", peer().as_str()).to_lowercase();
    view.window().invoke_address_edited(lowered.clone().into());
    for _ in 0..2 {
        press(&view, text(UiText::AddAction));
        assert_eq!(
            intents(&mut view, &mut model),
            vec![Intent::AddAddresses(vec![lowered.clone()])]
        );
        model.addresses_added(
            std::slice::from_ref(&lowered),
            Ok(AddressAnswer {
                outcome: AddressOutcome::Refused(AddressRefused::Grammar),
                list: Ok(AddressList::default()),
            }),
        );
        view.render(&model);
    }

    let said = said.borrow();
    let answers: Vec<&String> = said
        .iter()
        .filter(|l| l.starts_with("ui-trace add=") || l.starts_with("ui-trace note="))
        .collect();
    assert_eq!(
        answers,
        [
            "ui-trace add=waiting",
            "ui-trace note=refused:grammar",
            "ui-trace add=waiting",
            "ui-trace note=none",
            "ui-trace note=refused:grammar",
        ],
        "in {said:#?}"
    );
    for line in said.iter() {
        assert!(!line.contains("192.168.1.30"), "{line:?}");
    }
}

/// The same note set again with nothing between is said again: Add
/// pressed twice on an empty field sets `address-needed` twice and never
/// `none` (trace.rs's note count, #608 F4).
#[test]
fn the_same_note_set_twice_is_said_twice() {
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let mut model = UiModel::new();
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::PeersTitle));
    let _ = intents(&mut view, &mut model);
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(peer()),
        allowed: Vec::new(),
    }));
    model.addresses_read(Ok(AddressList::default()));
    view.render(&model);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    view.render(&model);
    for _ in 0..2 {
        press(&view, text(UiText::AddAction));
        assert!(intents(&mut view, &mut model).is_empty(), "nothing to send");
        view.render(&model);
    }
    let said = said.borrow();
    let notes: Vec<&String> = said
        .iter()
        .filter(|l| l.starts_with("ui-trace note="))
        .collect();
    assert_eq!(
        notes,
        [
            "ui-trace note=address-needed",
            "ui-trace note=address-needed"
        ],
        "in {said:#?}"
    );
}

/// The scan's own sets are said again too: a scan control pressed twice
/// with the field filled says `scan-after-field` twice, and the same
/// refused code scanned twice says its refusal twice (#608 F8).
#[test]
fn a_scan_note_set_twice_is_said_twice() {
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window().show().expect("shown");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    view.camera_hook(|_| {});
    let mut model = UiModel::new();
    let me = peer();
    press(&view, text(UiText::AppOptions));
    press(&view, text(UiText::PeersTitle));
    let _ = intents(&mut view, &mut model);
    model.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me.clone()),
        allowed: Vec::new(),
    }));
    model.addresses_read(Ok(AddressList::default()));
    view.render(&model);
    press(&view, text(UiText::AddAddress));
    let _ = intents(&mut view, &mut model);
    view.offer_scans(true, false, false);
    view.render(&model);

    let typed = format!("/ip4/192.168.1.30/tcp/4001/p2p/{}", peer().as_str());
    view.window().invoke_address_edited(typed.into());
    let _ = intents(&mut view, &mut model);
    for _ in 0..2 {
        press(&view, text(UiText::ScanCode));
        assert!(intents(&mut view, &mut model).is_empty(), "nothing to send");
    }
    view.window().invoke_address_edited("".into());
    let _ = intents(&mut view, &mut model);
    let own = format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", me.as_str());
    for _ in 0..2 {
        press(&view, text(UiText::ScanCode));
        let _ = intents(&mut view, &mut model);
        assert!(view.window().get_viewfinder_open(), "the scanner opens");
        view.camera_read(&own);
        assert!(intents(&mut view, &mut model).is_empty(), "nothing to send");
    }

    let said = said.borrow();
    let notes: Vec<&String> = said
        .iter()
        .filter(|l| l.starts_with("ui-trace note="))
        .collect();
    assert_eq!(
        notes,
        [
            "ui-trace note=scan-after-field",
            "ui-trace note=scan-after-field",
            "ui-trace note=none",
            "ui-trace note=refused:own-address",
            "ui-trace note=refused:own-address",
        ],
        "in {said:#?}"
    );
}

/// A page drawn under an identity screen is said when the screen goes,
/// so the first `page=` after a held C4's Continue is the page shown --
/// Connect -- never Chats under the screen and then Connect (add-a-peer
/// §5.1, §8 item 12). What else changed is said as it changes.
/// Mutation: saying the covered page at its render fails here.
#[test]
fn a_page_under_an_identity_screen_is_said_when_the_screen_goes() {
    use interweave_human_ui_model::ScreenId;
    i_slint_backend_testing::init_no_event_loop();
    let mut view = View::new(RuntimeHost::Embedded).expect("a window");
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let to = Rc::clone(&said);
    view.on_trace(move |line| to.borrow_mut().push(line.to_owned()));
    let page = |said: &Rc<RefCell<Vec<String>>>| {
        said.borrow()
            .iter()
            .filter(|l| l.starts_with("ui-trace page="))
            .cloned()
            .collect::<Vec<_>>()
    };

    view.trace_identity(Some(ScreenId::C4), None);
    let mut model = UiModel::new();
    model.connect_owed();
    view.render(&model);
    assert_eq!(page(&said), Vec::<String>::new(), "Chats, covered: unsaid");
    assert!(
        said.borrow().iter().any(|l| l == "ui-trace connect=owed"),
        "the rest is said: {:#?}",
        said.borrow()
    );
    // Both start reads answered on two empty lists: Connect opens.
    let _ = model.read_addresses();
    let _ = model.trust_settings_mut().opened();
    model
        .trust_settings_mut()
        .read(Ok(interweave_human_client_api::TrustList {
            local_peer: Some(peer()),
            allowed: Vec::new(),
        }));
    model.addresses_read(Ok(interweave_human_client_api::AddressList {
        addresses: Vec::new(),
    }));
    assert_eq!(
        model.connect_turn(),
        Some(interweave_human_ui_model::ConnectGate::Open),
        "control"
    );
    view.render(&model);
    assert_eq!(
        page(&said),
        Vec::<String>::new(),
        "Connect, covered: unsaid"
    );
    view.trace_identity(Some(ScreenId::Done), None);
    let lines = said.borrow().clone();
    let closed = lines
        .iter()
        .position(|l| l == "ui-trace identity=closed")
        .expect("the screen went");
    assert_eq!(page(&said), ["ui-trace page=connect"], "{lines:#?}");
    assert!(
        lines[closed..].iter().any(|l| l == "ui-trace page=connect"),
        "after the screen went"
    );
}

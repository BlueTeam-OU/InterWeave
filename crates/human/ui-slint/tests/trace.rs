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

    fn tab(&mut self) {
        self.key(slint::platform::Key::Tab);
    }

    fn backtab(&mut self) {
        self.key(slint::platform::Key::Backtab);
    }

    fn enter(&mut self) {
        self.key(slint::platform::Key::Return);
    }

    fn last(&self) -> String {
        self.said.borrow().last().cloned().unwrap_or_default()
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
    page.tab();
    // The key's focus line names the kind alone, never its letter.
    assert_eq!(page.last(), "ui-trace focus=identity-key");
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
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Key('q')],
        "q, the first enabled"
    );

    // Typing again: Enter is the suggestion rule's, not the focused
    // key's, and with three words and no whole one it takes nothing.
    page.key(slint::platform::Key::Backspace);
    page.key("a");
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Delete, IdentityInput::Key('a')]
    );
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
    assert_eq!(page.take(), [IdentityInput::Chip("act".to_owned())]);
    // Still typing after the rule's Enter: Space and Enter add nothing to
    // the key focus went to.
    page.key(" ");
    page.enter();
    assert_eq!(page.take(), [], "typing: Space and Enter press no key");
    // TAB is navigating again: Enter presses the focused key, w after q.
    page.tab();
    page.enter();
    assert_eq!(page.take(), [IdentityInput::Key('w')]);
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
    assert_eq!(
        page.take().last(),
        Some(&IdentityInput::Chip("act".to_owned()))
    );
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
    assert_eq!(page.take(), [IdentityInput::Chip("act".to_owned())]);

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
    page.settle();
    assert_eq!(page.last(), at);
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
    assert_eq!(
        page.take(),
        [
            IdentityInput::Key('o'),
            IdentityInput::Chip("oak".to_owned())
        ]
    );
    // Focus on w, the second key.
    page.tab();
    page.key(Key::Escape);
    assert!(page.flow.discard_asked(), "control: the dialog shows");
    assert_eq!(page.last(), "ui-trace focus=identity-dialog-keep");
    let first = page.said.borrow().len();
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
    assert_eq!(
        page.take(),
        [
            IdentityInput::Key('o'),
            IdentityInput::Chip("oak".to_owned())
        ]
    );
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
    assert_eq!(page.take(), [IdentityInput::Chip("oak".to_owned())]);
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
    assert_eq!(page.take(), [], "typing: nothing to take, q not pressed");
    for letter in ["a", "c"] {
        page.key(letter);
    }
    page.enter();
    page.key(" ");
    page.enter();
    assert_eq!(
        page.take(),
        [IdentityInput::Key('a'), IdentityInput::Key('c')],
        "three words fit ac: Enter and Space take nothing"
    );
    page.key("t");
    page.enter();
    page.key(" ");
    assert_eq!(
        page.take(),
        [
            IdentityInput::Key('t'),
            IdentityInput::Chip("act".to_owned())
        ],
        "the whole word's chip, then Space nothing"
    );
}

/// C3 opens on the first enabled key, typing, as R2 does (section 8).
#[test]
fn c3_opens_on_the_first_enabled_key_typing() {
    let mut page = Identity::at_c3();
    assert_eq!(page.last(), "ui-trace focus=identity-key");
    page.enter();
    page.key(" ");
    assert_eq!(page.take(), [], "C3 opens typing too");
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
            IdentityInput::Chip("oak".to_owned())
        ],
        "control: R2 again, a word entered"
    );
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
        [
            IdentityInput::Key('o'),
            IdentityInput::Chip("oak".to_owned())
        ],
        "control: typing, a word entered, focus on q"
    );
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
        [IdentityInput::Back, IdentityInput::Back],
        "typing again: Enter presses no key"
    );
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

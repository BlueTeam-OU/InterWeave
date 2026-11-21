// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identity switch, the switcher, the switch's snackbar and the line
//! above Send (plan §30 step 1; contact-cards design §5.7.1 to §5.7.3 as
//! the identities note §4, §5, §5.2, §11 and §12 amend them), through
//! Slint's testing backend.
//!
//! Nothing feeds a second identity in production yet, so these hand the
//! model its identities directly and apply the view's identity intents as
//! a root would. What is proved is the view's half: what the window draws,
//! names, focuses and returns for a press. NOT proved here: the rendered
//! look (a snapshot under the display wrapper is the check), a screen
//! reader's reading, a phone's Back key reaching `handle-back`, and the
//! closing of the model's own snackbar by a switch.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::Duration;

use i_slint_backend_testing::{ElementHandle, ElementQuery};
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::{
    ConversationKey, HeldIdentity, IdentityIntent, IdentityKey, IdentityState, Intent, RuntimeHost,
    UiModel, cluster, isolated,
};
use interweave_human_ui_slint::{View, ViewEvent};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::ComponentHandle as _;
use slint::platform::{Key, WindowEvent};

fn key(tag: &str) -> IdentityKey {
    IdentityKey::new(format!("test-endpoint-{tag}"))
}

fn held(tags: &[&str]) -> Vec<HeldIdentity> {
    tags.iter()
        .enumerate()
        .map(|(i, tag)| HeldIdentity {
            key: key(tag),
            card_name: Some(format!("Name {tag}")),
            alias: format!("alias-{tag}"),
            state: IdentityState::Ready,
            is_first: i == 0,
            is_default: i == 0,
            unread: 0,
            not_sent: false,
        })
        .collect()
}

fn hold(model: &mut UiModel, identities: Vec<HeldIdentity>) {
    model
        .identities_mut()
        .held_changed(identities)
        .expect("a held list");
}

/// A model holding `tags`, the first in front, with one conversation.
fn model_with(tags: &[&str]) -> (UiModel, ConversationKey) {
    let mut model = UiModel::new();
    hold(&mut model, held(tags));
    model.identities_mut().start(None);
    let peer: TransportIdentity = ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer id");
    let human = EndpointId::parse("human").expect("an endpoint");
    model.received(Received {
        row: RowId::from_stored(1),
        origin: Origin::Direct {
            peer: peer.clone(),
            endpoint: human.clone(),
        },
        envelope: HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{:032x}", 1),
            text: "hello".to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: 1,
    });
    model.read(RowId::from_stored(1));
    let conversation = ConversationKey::Direct {
        peer,
        endpoint: Some(human),
    };
    (model, conversation)
}

thread_local! {
    static PLATFORM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A window `width` wide; the testing platform set once per thread, as a
/// test that makes two windows needs.
fn view(width: f32) -> View {
    if !PLATFORM.replace(true) {
        i_slint_backend_testing::init_no_event_loop();
    }
    let view = View::new(RuntimeHost::Embedded).expect("a window");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(width, 800.0));
    view.window().show().expect("shown");
    view
}

/// Every press since the last turn applied as a root would, then a render,
/// then the timers a focus request or a first focus waits on. Returns the
/// identity intents taken.
fn turn(view: &mut View, model: &mut UiModel) -> Vec<IdentityIntent> {
    let mut taken = Vec::new();
    for _ in 0..8 {
        let events = view.take_events(model);
        let intents = view.take_identity_intents();
        if events.is_empty() && intents.is_empty() {
            break;
        }
        for event in events {
            match event {
                ViewEvent::SnackbarClosed(id) => model.snackbar_closed(id),
                ViewEvent::DraftChanged { key, draft } => model.draft_changed(key, draft),
                ViewEvent::Intent(Intent::MarkRead(row)) => model.read(row),
                _ => {}
            }
        }
        for intent in intents {
            model.identities_mut().act(intent.clone(), false);
            taken.push(intent);
        }
    }
    view.render(model);
    for _ in 0..3 {
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(2));
        slint::platform::update_timers_and_animations();
    }
    taken
}

fn press(view: &View, key: Key) {
    let text: slint::SharedString = key.into();
    let window = view.window().window();
    window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text });
}

fn focus_id(view: &View) -> String {
    view.window().get_focus_id().to_string()
}

fn labelled(view: &View, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label).collect()
}

fn one(view: &View, label: &str) -> ElementHandle {
    let found = labelled(view, label);
    assert_eq!(found.len(), 1, "one element labelled {label:?}");
    found.into_iter().next().unwrap()
}

/// Every accessible label in the window.
fn labels(view: &View) -> Vec<String> {
    ElementQuery::from_root(view.window())
        .find_all()
        .iter()
        .filter_map(ElementHandle::accessible_label)
        .map(|l| l.to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn name(tag: &str) -> String {
    isolated(&format!("Name {tag}"))
}

fn switch_label(tag: &str) -> String {
    format!("Identity: {}. Switch identity", name(tag))
}

/// Tab until focus is on `id`, at most `tries` times; the stops passed.
fn tab_to(view: &View, id: &str, tries: usize) -> Vec<String> {
    let mut passed = Vec::new();
    for _ in 0..tries {
        press(view, Key::Tab);
        let on = focus_id(view);
        passed.push(on.clone());
        if on == id {
            return passed;
        }
    }
    panic!("Tab never reached {id}; passed {passed:?}");
}

#[test]
fn with_one_identity_nothing_of_the_switch_is_shown() {
    for tags in [&["a"][..], &[][..]] {
        let (mut model, conversation) = model_with(tags);
        let mut view = view(1000.0);
        view.select(conversation);
        turn(&mut view, &mut model);
        let all = labels(&view);
        assert!(!all.is_empty(), "the tree is read");
        assert!(
            all.iter().any(|l| l == "Send"),
            "the control: Send is drawn, named as with one identity"
        );
        for label in &all {
            assert!(
                !label.contains("Switch identity")
                    && !label.starts_with("As ")
                    && !label.starts_with("Send as"),
                "{tags:?}: {label:?} names an identity"
            );
        }
        assert!(!view.window().get_id_switch().shown);
        assert!(!view.window().get_id_composer().shown);
        // Nothing opens the switcher: the root asked for it, the model
        // answers no.
        model
            .identities_mut()
            .act(IdentityIntent::OpenSwitcher, false);
        turn(&mut view, &mut model);
        assert!(!view.window().get_id_switcher().open);
        assert!(labelled(&view, "Switch identity").is_empty());
        // And Back has nothing of it to close.
        assert!(view.take_identity_intents().is_empty());
    }
    // The control: two held draw the switch and the line.
    let (mut model, conversation) = model_with(&["a", "b"]);
    let mut view = view(1000.0);
    view.select(conversation);
    turn(&mut view, &mut model);
    assert_eq!(labelled(&view, &switch_label("a")).len(), 1);
    assert_eq!(labelled(&view, &format!("As {}", name("a"))).len(), 1);
}

#[test]
fn the_switch_names_the_identity_in_front_and_the_badges_words() {
    let mut identities = held(&["a", "b"]);
    identities[1].unread = 3;
    let (mut model, _) = model_with(&["a", "b"]);
    hold(&mut model, identities.clone());
    let mut view = view(1000.0);
    turn(&mut view, &mut model);
    let switch = one(
        &view,
        &format!(
            "{}. 3 unread messages in your other identity",
            switch_label("a")
        ),
    );
    assert!(switch.size().height >= 48.0, "at least 48 px high");
    assert_eq!(view.window().get_id_switch().count, "3");
    // Nothing unread there, a message not sent: a dot, no count, and the
    // not-sent sentence.
    identities[1].unread = 0;
    identities[1].not_sent = true;
    hold(&mut model, identities.clone());
    turn(&mut view, &mut model);
    one(
        &view,
        &format!(
            "{}. A message in another identity was not sent.",
            switch_label("a")
        ),
    );
    let drawn = view.window().get_id_switch();
    assert!(drawn.badge && drawn.count.is_empty());
    // Nothing there: no badge.
    identities[1].not_sent = false;
    hold(&mut model, identities);
    turn(&mut view, &mut model);
    one(&view, &switch_label("a"));
    assert!(!view.window().get_id_switch().badge);
}

/// identities.slint cannot call app.slint's `Drawn.spoken`, so the view
/// speaks its accessible texts itself: a character drawn as U+FFFD is
/// read in words, never silent (long-message note section 6).
#[test]
fn a_replaced_character_in_a_name_is_read_in_words() {
    let mut identities = held(&["a", "b"]);
    identities[0].card_name = Some("A\u{FFFD}".to_owned());
    let (mut model, conversation) = model_with(&["a", "b"]);
    hold(&mut model, identities);
    let mut view = view(1000.0);
    view.select(conversation);
    turn(&mut view, &mut model);
    let words = ", character not shown,";
    let said = |raw: &str| cluster::spoken(raw, words).into_owned();
    let switch = said(&format!(
        "Identity: {}. Switch identity",
        isolated("A\u{FFFD}")
    ));
    assert!(switch.contains(words), "the premise: {switch:?}");
    one(&view, &switch).invoke_accessible_default_action();
    one(&view, &said(&format!("As {}", isolated("A\u{FFFD}"))));
    turn(&mut view, &mut model);
    one(&view, &said("A\u{FFFD}"));
}

#[test]
fn activating_the_switch_opens_the_switcher_and_escape_closes_it_with_nothing_changed() {
    let (mut model, _) = model_with(&["a", "b"]);
    let mut view = view(1000.0);
    turn(&mut view, &mut model);
    one(&view, &switch_label("a")).invoke_accessible_default_action();
    assert_eq!(turn(&mut view, &mut model), [IdentityIntent::OpenSwitcher]);
    assert!(view.window().get_id_switcher().open);
    one(&view, "Switch identity");
    one(&view, "Name a");
    one(&view, "Name b");
    one(&view, "Your identities");
    assert_eq!(
        focus_id(&view),
        "identity-switcher-title",
        "its title takes focus as it opens (F1)"
    );
    assert_eq!(
        one(&view, "Name a").accessible_checked(),
        Some(true),
        "the identity in front is the chosen row"
    );
    assert_eq!(one(&view, "Name b").accessible_checked(), Some(false));

    press(&view, Key::Escape);
    assert_eq!(turn(&mut view, &mut model), [IdentityIntent::CloseSwitcher]);
    assert!(!view.window().get_id_switcher().open);
    assert!(labelled(&view, "Name b").is_empty(), "the switcher is gone");
    assert_eq!(
        model.identities().front(),
        Some(&key("a")),
        "nothing changed"
    );
    assert_eq!(model.identities().previous(), None);
    assert!(
        !view.window().get_id_snack_shown(),
        "no switch, no snackbar"
    );
    one(&view, &switch_label("a"));
    assert_eq!(
        focus_id(&view),
        "identity-switch",
        "back to the switch (F2)"
    );
}

#[test]
fn choosing_the_other_row_switches_names_it_and_offers_switch_back() {
    let (mut model, _) = model_with(&["a", "b"]);
    let mut view = view(1000.0);
    turn(&mut view, &mut model);
    one(&view, &switch_label("a")).invoke_accessible_default_action();
    turn(&mut view, &mut model);

    one(&view, "Name b").invoke_accessible_default_action();
    assert_eq!(
        turn(&mut view, &mut model),
        [IdentityIntent::Choose(key("b"))]
    );
    assert!(
        !view.window().get_id_switcher().open,
        "closed by the choice"
    );
    assert_eq!(model.identities().front(), Some(&key("b")));
    one(&view, &switch_label("b"));
    let said = format!("You are now using {}", name("b"));
    assert_eq!(view.window().get_id_snack_text(), said.as_str());
    assert!(view.window().get_id_snack_shown());
    // The snackbar's text, and the live region that announced it.
    assert!(!labelled(&view, &said).is_empty());
    one(&view, "Switch back");
    assert_eq!(
        focus_id(&view),
        "identity-switch",
        "focus on the switch, which names the new identity (F3)"
    );
    let announced = [
        view.window().get_announcement_a().to_string(),
        view.window().get_announcement_b().to_string(),
    ];
    assert!(announced.contains(&said), "announced: {announced:?}");

    one(&view, "Switch back").invoke_accessible_default_action();
    let taken = turn(&mut view, &mut model);
    assert!(
        matches!(taken.as_slice(), [IdentityIntent::SwitchBack(_)]),
        "{taken:?}"
    );
    assert_eq!(model.identities().front(), Some(&key("a")));
    one(&view, &switch_label("a"));
    assert_eq!(
        view.window().get_id_snack_text(),
        format!("You are now using {}", name("a")).as_str()
    );
    assert_eq!(focus_id(&view), "identity-switch");
}

#[test]
fn choosing_the_chosen_row_closes_the_switcher_with_nothing_changed() {
    let (mut model, _) = model_with(&["a", "b"]);
    let mut view = view(390.0);
    turn(&mut view, &mut model);
    one(&view, &switch_label("a")).invoke_accessible_default_action();
    turn(&mut view, &mut model);
    one(&view, "Name a").invoke_accessible_default_action();
    assert_eq!(
        turn(&mut view, &mut model),
        [IdentityIntent::Choose(key("a"))]
    );
    assert!(!view.window().get_id_switcher().open);
    assert_eq!(model.identities().previous(), None);
    assert!(!view.window().get_id_snack_shown());
    assert_eq!(focus_id(&view), "identity-switch");
}

#[test]
fn a_narrow_switcher_is_a_bottom_sheet_and_a_wide_one_hangs_under_the_bar() {
    for (width, sheet) in [(390.0_f32, true), (1000.0, false)] {
        let (mut model, _) = model_with(&["a", "b"]);
        let mut view = view(width);
        turn(&mut view, &mut model);
        one(&view, &switch_label("a")).invoke_accessible_default_action();
        turn(&mut view, &mut model);
        let last = one(&view, "Your identities");
        let title = one(&view, "Switch identity");
        let bottom = last.absolute_position().y + last.size().height;
        assert!(bottom <= 800.0, "{width}: Your identities in the window");
        assert!(
            title.absolute_position().y >= 56.0,
            "{width}: under the bar"
        );
        let row = one(&view, "Name b");
        assert!(row.size().height >= 56.0, "{width}: a 56 px row");
        if sheet {
            assert!(bottom > 700.0, "{width}: at the window's bottom ({bottom})");
            assert!(row.size().width > 300.0, "{width}: the window's width");
        } else {
            assert!(bottom < 600.0, "{width}: hangs under the bar ({bottom})");
            assert!(row.size().width <= 360.0, "{width}: a dialog's width");
        }
    }
}

#[test]
fn the_as_line_and_send_as_follow_the_identity_in_front() {
    let (mut model, conversation) = model_with(&["a", "b"]);
    let mut view = view(1000.0);
    view.select(conversation);
    turn(&mut view, &mut model);
    one(&view, &format!("As {}", name("a")));
    one(&view, &format!("Send as {}", name("a")));
    // A switch, as the switcher makes it.
    model
        .identities_mut()
        .act(IdentityIntent::OpenSwitcher, false);
    model
        .identities_mut()
        .act(IdentityIntent::Choose(key("b")), false);
    turn(&mut view, &mut model);
    one(&view, &format!("As {}", name("b")));
    one(&view, &format!("Send as {}", name("b")));
    assert!(labelled(&view, &format!("As {}", name("a"))).is_empty());
    // The line is text, never a control (cards §5.7.2).
    let line = one(&view, &format!("As {}", name("b")));
    assert_eq!(
        line.accessible_role(),
        Some(i_slint_backend_testing::AccessibleRole::Text)
    );
    // Above Send: it ends at or above the control's top.
    let send = one(&view, &format!("Send as {}", name("b")));
    assert!(
        line.absolute_position().y + line.size().height <= send.absolute_position().y,
        "the line stands above Send"
    );
}

#[test]
fn a_starting_identity_disables_send_with_its_reason() {
    let (mut model, conversation) = model_with(&["a", "b"]);
    let mut view = view(1000.0);
    view.select(conversation);
    turn(&mut view, &mut model);
    view.window().set_draft("hi".into());
    let send = one(&view, &format!("Send as {}", name("a")));
    assert_eq!(
        send.accessible_enabled(),
        Some(true),
        "the control: it sends"
    );
    // Focus on Send as its identity stops being able to: to the field
    // (identities §5.2, F21).
    tab_to(&view, "send", 80);
    let mut identities = held(&["a", "b"]);
    identities[0].state = IdentityState::Starting {
        after_try_again: false,
    };
    hold(&mut model, identities);
    turn(&mut view, &mut model);
    let reason = format!("{} is connecting and cannot send yet.", name("a"));
    one(&view, &reason);
    assert!(labelled(&view, &format!("As {}", name("a"))).is_empty());
    // Text in the field, so only the identity's state holds Send back.
    view.window().set_draft("hi".into());
    let send = one(&view, "Send");
    assert_eq!(send.accessible_enabled(), Some(false), "Send disabled");
    assert_eq!(focus_id(&view), "composer", "F21");
}

#[test]
fn could_not_start_offers_try_again_on_the_line_and_the_row() {
    let (mut model, conversation) = model_with(&["a", "b"]);
    let mut identities = held(&["a", "b"]);
    identities[0].state = IdentityState::NotHidden;
    hold(&mut model, identities);
    let mut view = view(1000.0);
    view.select(conversation);
    turn(&mut view, &mut model);
    let try_again = labelled(&view, "Try again");
    assert_eq!(try_again.len(), 1, "the line's");
    try_again[0].invoke_accessible_default_action();
    assert_eq!(
        turn(&mut view, &mut model),
        [IdentityIntent::TryAgain(key("a"))]
    );
    one(&view, &switch_label("a")).invoke_accessible_default_action();
    turn(&mut view, &mut model);
    let try_again = labelled(&view, "Try again");
    assert_eq!(try_again.len(), 2, "the line's and the row's");
}

#[test]
fn tab_reaches_the_switch_and_every_stop_of_the_switcher() {
    let mut identities = held(&["a", "b", "c"]);
    identities[2].state = IdentityState::NotHidden;
    let (mut model, _) = model_with(&["a", "b", "c"]);
    hold(&mut model, identities);
    let mut view = view(1000.0);
    turn(&mut view, &mut model);
    // Chats' bar: the title, the switch, the app's options.
    let passed = tab_to(&view, "identity-switch", 20);
    assert_eq!(
        passed.iter().rev().nth(1).map(String::as_str),
        Some("conversations-heading"),
        "after the title: {passed:?}"
    );
    press(&view, Key::Tab);
    assert_eq!(focus_id(&view), "app-options", "then the app's options");
    press(&view, Key::Backtab);
    assert_eq!(focus_id(&view), "identity-switch");
    // Enter opens it; Tab goes round its stops and never leaves it.
    press(&view, Key::Return);
    assert_eq!(turn(&mut view, &mut model), [IdentityIntent::OpenSwitcher]);
    assert_eq!(focus_id(&view), "identity-switcher-title");
    let mut stops = Vec::new();
    for _ in 0..6 {
        press(&view, Key::Tab);
        stops.push(focus_id(&view));
    }
    assert_eq!(
        stops,
        [
            "identity-row:0",
            "identity-row:1",
            "identity-row:2",
            "identity-try-again:2",
            "identity-your-identities",
            "identity-switcher-title",
        ]
    );
    press(&view, Key::Backtab);
    assert_eq!(
        focus_id(&view),
        "identity-your-identities",
        "round, backwards"
    );
    // Arrows move between the rows and stop at the ends.
    tab_to(&view, "identity-row:0", 4);
    press(&view, Key::DownArrow);
    turn(&mut view, &mut model);
    assert_eq!(focus_id(&view), "identity-row:1");
    press(&view, Key::DownArrow);
    turn(&mut view, &mut model);
    press(&view, Key::DownArrow);
    turn(&mut view, &mut model);
    assert_eq!(focus_id(&view), "identity-row:2", "the last row stays");
    press(&view, Key::UpArrow);
    turn(&mut view, &mut model);
    assert_eq!(focus_id(&view), "identity-row:1");
    // Enter on a row chooses it; Your identities by Enter.
    press(&view, Key::Return);
    assert_eq!(
        turn(&mut view, &mut model),
        [IdentityIntent::Choose(key("b"))]
    );
    press(&view, Key::Return);
    turn(&mut view, &mut model);
    tab_to(&view, "identity-your-identities", 8);
    press(&view, Key::Return);
    assert_eq!(
        turn(&mut view, &mut model),
        [IdentityIntent::OpenIdentities]
    );
    assert!(!view.window().get_id_switcher().open);
}

#[test]
fn back_closes_the_switcher_with_nothing_changed() {
    let (mut model, _) = model_with(&["a", "b"]);
    let mut view = view(390.0);
    turn(&mut view, &mut model);
    one(&view, &switch_label("a")).invoke_accessible_default_action();
    turn(&mut view, &mut model);
    assert!(view.window().invoke_handle_back(), "Back is taken");
    assert_eq!(turn(&mut view, &mut model), [IdentityIntent::CloseSwitcher]);
    assert!(!view.window().get_id_switcher().open);
    assert_eq!(model.identities().front(), Some(&key("a")));
}

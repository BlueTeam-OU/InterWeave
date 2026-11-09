// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A long message in the conversation and in the reader
//! (design/2026-11-04-long-message.md), through Slint's testing backend:
//! the cut row, its menu, the reader's opening and closing, where focus
//! goes, what reads a message, and what Copy message source gives.
//!
//! What this cannot prove: that the platform's clipboard receives the
//! copy (the testing backend keeps it and offers no way to read it back
//! but a paste, which replaces a long cluster as a draft must), that the
//! reader lays out only what is near the screen, its first-screen time
//! (A5), or what a screen reader says (A6's `TalkBack` half, A8, A12).

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementQuery};
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::{
    ConversationKey, Intent, RuntimeHost, UiModel, UiText, placeholder_en,
};
use interweave_human_ui_slint::{View, ViewEvent};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle as _, Model as _};

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer id")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("an endpoint")
}

fn direct(peer: &TransportIdentity) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: Some(human()),
    }
}

fn received(row: i64, from: &TransportIdentity, text: &str) -> Received {
    Received {
        row: RowId::from_stored(row),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: human(),
        },
        envelope: HumanChatV2 {
            v: 2,
            kind: MessageKind::Text,
            app_message_id: format!("{:032x}", u128::try_from(row).expect("positive") + 1_000),
            text: text.to_owned(),
            reply_to: None,
            sent_at_ms: None,
            from_endpoint: None,
        },
        received_at: u64::try_from(row).expect("positive"),
    }
}

fn view() -> View {
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Daemon).expect("a window");
    view.window().show().expect("shown");
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 4000.0));
    view
}

fn text(t: UiText) -> &'static str {
    placeholder_en::text(t)
}

/// The root's loop: what the model does itself is done, intents returned.
fn intents(view: &mut View, model: &mut UiModel) -> Vec<Intent> {
    let mut out = Vec::new();
    loop {
        let events = view.take_events(model);
        if events.is_empty() {
            break;
        }
        for event in events {
            match event {
                ViewEvent::Intent(Intent::MarkRead(row)) => {
                    model.read(row);
                    out.push(Intent::MarkRead(row));
                }
                ViewEvent::Intent(Intent::HideConversation(key)) => model.hide_conversation(&key),
                ViewEvent::Intent(intent) => out.push(intent),
                ViewEvent::DraftChanged { key, draft } => model.draft_changed(key, draft),
                ViewEvent::Undo(id) => out.extend(model.undo(id)),
                ViewEvent::SnackbarClosed(id) => model.snackbar_closed(id),
                ViewEvent::IdentifierCopied(key) => model.identifier_copied(key),
                ViewEvent::MessageCopied(key) => model.message_copied(key),
                ViewEvent::Trust(_) | ViewEvent::Peers(_) | ViewEvent::Connect(_) => {}
            }
        }
    }
    view.render(model);
    slint::platform::update_timers_and_animations();
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(0));
    out
}

fn open(view: &mut View, model: &mut UiModel, key: &ConversationKey) {
    view.select(key.clone());
    let _ = intents(view, model);
}

fn labelled(view: &View, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label).collect()
}

fn the(view: &View, label: &str) -> ElementHandle {
    let mut found = labelled(view, label);
    assert_eq!(found.len(), 1, "exactly one element labelled {label:?}");
    found.remove(0)
}

fn key(view: &View, key: Key) {
    let text: slint::SharedString = key.into();
    view.window()
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    view.window()
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}

/// The message rows as drawn.
fn rows(view: &View) -> Vec<interweave_human_ui_slint::MessageRow> {
    let rows = view.window().get_messages();
    (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .collect()
}

/// The labels of the open menu of the row whose label is `label`, in
/// order, opened as a screen reader does, by the row's default action.
fn menu(view: &View, label: &str) -> Vec<String> {
    let label = &spoken(label);
    let item = ElementQuery::from_root(view.window())
        .match_accessible_role(AccessibleRole::ListItem)
        .find_all()
        .into_iter()
        .find(|e| e.accessible_label().as_deref() == Some(label))
        .expect("the row's item");
    item.invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
    ElementQuery::from_root(view.window())
        .find_all()
        .into_iter()
        .filter(|e| e.accessible_description().as_deref() == Some(label))
        .filter_map(|e| e.accessible_label().map(|l| l.to_string()))
        .collect()
}

/// `label` as a screen reader is given it: a drawn mark as words.
fn spoken(label: &str) -> String {
    interweave_human_ui_model::cluster::spoken(label, text(UiText::ReplacedCharacter)).into_owned()
}

/// Choose `entry` from the open menu of the row labelled `label`.
fn choose(view: &View, label: &str, entry: &str) {
    let _ = menu(view, label);
    let label = &spoken(label);
    ElementQuery::from_root(view.window())
        .find_all()
        .into_iter()
        .find(|e| {
            e.accessible_label().as_deref() == Some(entry)
                && e.accessible_description().as_deref() == Some(label)
        })
        .expect("the menu's entry")
        .invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
}

/// The message row's item, as a screen reader reads it, formatted or as
/// source (the Chats list's rows are items too).
fn item_label(view: &View) -> String {
    let row = &rows(view)[0];
    let forms = [spoken(&row.label), spoken(&row.source_label)];
    ElementQuery::from_root(view.window())
        .match_accessible_role(AccessibleRole::ListItem)
        .find_all()
        .into_iter()
        .filter_map(|e| e.accessible_label())
        .find(|l| forms.iter().any(|f| l == f.as_str()))
        .expect("the message row's item")
        .to_string()
}

/// A body of `n` paragraphs, "Paragraph 1." to "Paragraph n.".
fn paragraphs(n: usize) -> String {
    (1..=n)
        .map(|i| format!("Paragraph {i}."))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The reader's rows' text, in order.
fn reader_text(view: &View) -> Vec<String> {
    let rows = view.window().get_reader_rows();
    (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .map(|r| {
            if r.line.text.is_empty() {
                r.text.to_string()
            } else {
                r.line.text.to_string()
            }
        })
        .collect()
}

/// Every U+FFFD in an accessible text the client sets is spoken as
/// `ReplacedCharacter`'s words, set apart by spaces: the bubble's line and
/// its label, the Chats row's description, and the reader's lines and
/// source -- a mark drawn for a character past the cluster bound and one
/// the body already carried alike (section 6, A12). Nothing in the tree
/// gives a screen reader U+FFFD itself, so what it says never depends on
/// its punctuation setting.
#[test]
fn a_replaced_character_is_spoken_as_words() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    let past = format!("a{}", "\u{301}".repeat(255));
    let body = format!(
        "x{past}y and \u{FFFD}z\n\n```\nq\u{FFFD}r\n```\n\n{}",
        paragraphs(60)
    );
    model.received(received(1, &alice, &body));
    open(&mut view, &mut model, &direct(&alice));
    let words = text(UiText::ReplacedCharacter);
    // Words that start with punctuation take no space before them
    // (section 6), as the en words do.
    let spoken = format!("x{words} y and {words} z");
    let said = |view: &View| -> Vec<String> {
        ElementQuery::from_root(view.window())
            .find_all()
            .iter()
            .flat_map(|e| [e.accessible_label(), e.accessible_description()])
            .flatten()
            .map(|s| s.to_string())
            .collect()
    };
    let spoken_never_the_mark = |view: &View, place: &str| {
        let said = said(view);
        assert!(
            said.iter().any(|s| s.contains(&spoken)),
            "{place}: the words, set apart"
        );
        assert!(
            !said.iter().any(|s| s.contains('\u{FFFD}')),
            "{place}: never the mark itself"
        );
    };

    let code = format!("q{words} r");
    assert_eq!(labelled(&view, &spoken).len(), 1, "the bubble's line");
    assert_eq!(labelled(&view, &code).len(), 1, "the bubble's code line");
    assert!(item_label(&view).contains(&spoken), "the bubble's label");
    spoken_never_the_mark(&view, "the conversation and the Chats line");

    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_reader_open());
    assert!(
        reader_text(&view).iter().any(|t| t.contains('\u{FFFD}')),
        "control: the reader draws the mark"
    );
    assert_eq!(labelled(&view, &code).len(), 1, "the reader's code line");
    spoken_never_the_mark(&view, "the reader");

    view.window().invoke_reader_switch();
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_reader_as_source());
    spoken_never_the_mark(&view, "the reader's source");

    key(&view, Key::Escape);
    let _ = intents(&mut view, &mut model);
    assert!(!view.window().get_reader_open());
    let label = item_label(&view);
    choose(&view, &label, text(UiText::ShowSource));
    let _ = intents(&mut view, &mut model);
    assert!(
        item_label(&view).contains(&spoken),
        "the bubble's label as source"
    );
    spoken_never_the_mark(&view, "the bubble's source");
}

/// A cut row shows its preview, the cut line with the share shown, and
/// Open the whole message; a whole row shows neither, and its label is
/// the whole body (A2, section 9).
#[test]
fn a_cut_row_shows_its_preview_its_line_and_the_way_to_the_whole() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, "Short and whole."));
    model.received(received(2, &alice, &paragraphs(60)));
    open(&mut view, &mut model, &direct(&alice));

    let drawn = rows(&view);
    assert_eq!(drawn[0].cut_line, "", "control: a short body is whole");
    let cut = &drawn[1];
    assert!(cut.lines.row_count() <= 32, "the preview's line figure");
    assert_eq!(
        cut.lines
            .row_data(cut.lines.row_count() - 1)
            .map(|l| l.text),
        Some("Paragraph 32.".into()),
        "ends at a block boundary"
    );
    let shown = cut.cut_line.to_string();
    assert!(
        shown.starts_with("The first ") && shown.ends_with("% of the message is shown."),
        "{shown}"
    );
    assert!(
        cut.label.contains(&shown),
        "the cut line is said with the bubble"
    );
    assert!(!cut.label.contains("Paragraph 33."), "never the whole body");
    assert_eq!(
        labelled(&view, text(UiText::OpenWholeMessage)).len(),
        1,
        "one Open the whole message: the cut row's"
    );
}

/// A cut row's menu: its actions, then Open the whole message, Copy
/// message source and Show source last; a whole row's lacks Open the
/// whole message (section 4.3, message-swipe section 7 revised).
#[test]
fn a_rows_menu_has_open_the_whole_message_only_when_cut_and_copy_always() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, "Short and whole."));
    model.received(received(2, &alice, &paragraphs(60)));
    open(&mut view, &mut model, &direct(&alice));
    let drawn = rows(&view);

    let actions = |row: &interweave_human_ui_slint::MessageRow| -> Vec<String> {
        (0..row.actions.row_count())
            .filter_map(|i| row.actions.row_data(i))
            .map(|a| a.text.to_string())
            .collect()
    };
    let mut whole = actions(&drawn[0]);
    whole.extend([text(UiText::CopyMessage), text(UiText::ShowSource)].map(str::to_owned));
    assert_eq!(menu(&view, &drawn[0].label), whole);
    key(&view, Key::Escape);
    slint::platform::update_timers_and_animations();

    let mut cut = actions(&drawn[1]);
    cut.extend(
        [
            text(UiText::OpenWholeMessage),
            text(UiText::CopyMessage),
            text(UiText::ShowSource),
        ]
        .map(str::to_owned),
    );
    assert_eq!(menu(&view, &drawn[1].label), cut);
}

/// Open the whole message opens the reader on the whole body, focus on
/// its scroll; Escape closes it, the conversation as it was and focus on
/// the row's Open the whole message (sections 5, 9: A6).
#[test]
fn the_reader_shows_the_whole_body_and_escape_returns_to_the_button() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, &paragraphs(60)));
    open(&mut view, &mut model, &direct(&alice));
    let handle = rows(&view)[0].handle;

    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_reader_open());
    let shown = reader_text(&view);
    assert!(shown.contains(&"Paragraph 1.".to_owned()));
    assert!(
        shown.contains(&"Paragraph 60.".to_owned()),
        "the whole body"
    );
    assert!(!view.window().get_reader_as_source());
    // Asked after the draw, over the loop's next turns.
    for _ in 0..3 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        slint::platform::update_timers_and_animations();
    }
    assert_eq!(view.window().get_focus_id(), "reader-scroll");

    key(&view, Key::Escape);
    let _ = intents(&mut view, &mut model);
    assert!(!view.window().get_reader_open());
    assert_eq!(
        view.window().get_focus_id(),
        format!("read:{handle}").as_str()
    );
}

/// The debug build's trace names the reader's page, and End moves its
/// list to its end, which the window says (`reader-at-end`): what the
/// device case's End time waits for (A5,
/// `tests/android-e2e/tests/long_message.rs`). The times themselves need
/// a renderer that says when it drew, which this backend is not.
#[test]
fn the_trace_names_the_reader_and_end_reaches_its_list_end() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(1000.0, 800.0));
    let lines = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let sink = std::rc::Rc::clone(&lines);
    view.on_trace(move |line| sink.borrow_mut().push(line.to_owned()));
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, &paragraphs(400)));
    open(&mut view, &mut model, &direct(&alice));
    let pages = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .filter(|l| l.starts_with("ui-trace page="))
            .cloned()
            .collect()
    };
    assert_eq!(
        pages(&lines.borrow()).last().map(String::as_str),
        Some("ui-trace page=conversation")
    );

    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        pages(&lines.borrow()).last().map(String::as_str),
        Some("ui-trace page=reader")
    );
    for _ in 0..3 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        slint::platform::update_timers_and_animations();
    }
    assert_eq!(view.window().get_focus_id(), "reader-scroll");
    assert!(
        !view.window().get_reader_at_end(),
        "400 paragraphs pass one screen"
    );

    // One End: the rows laid out near the end make the content taller
    // than first measured, and the list is held at its end through it.
    key(&view, Key::End);
    slint::platform::update_timers_and_animations();
    assert!(
        view.window().get_reader_at_end(),
        "End reaches the list's end"
    );
    key(&view, Key::Home);
    slint::platform::update_timers_and_animations();
    assert!(!view.window().get_reader_at_end(), "Home leaves it");

    key(&view, Key::Escape);
    let _ = intents(&mut view, &mut model);
    assert_eq!(
        pages(&lines.borrow()).last().map(String::as_str),
        Some("ui-trace page=conversation")
    );
}

/// While the reader is open its conversation is not shown: a message
/// that arrives then stays unread, and is read once the reader closes
/// (section 7, A10).
#[test]
fn a_message_arriving_while_the_reader_is_open_is_read_when_it_closes() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, &paragraphs(60)));
    open(&mut view, &mut model, &direct(&alice));
    view.set_window_focused(true);
    let _ = intents(&mut view, &mut model);
    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);

    model.received(received(2, &alice, "Meanwhile."));
    view.render(&model);
    let got = intents(&mut view, &mut model);
    assert!(
        !got.contains(&Intent::MarkRead(RowId::from_stored(2))),
        "not read behind the reader: {got:?}"
    );
    key(&view, Key::Escape);
    let got = intents(&mut view, &mut model);
    assert!(
        got.contains(&Intent::MarkRead(RowId::from_stored(2))),
        "read once shown: {got:?}"
    );
}

/// The reader is a screen over the conversation: the locked notice's
/// focus ask (unlock note §11, F11) waits under it, and Back lands on
/// the notice's Unlock rather than the row's Open the whole message --
/// in a narrow window and a wide one.
#[test]
fn the_locked_notice_takes_focus_once_the_reader_over_it_closes_narrow() {
    locked_notice_after_the_reader(390.0);
}

/// [`the_locked_notice_takes_focus_once_the_reader_over_it_closes_narrow`]
/// in a wide window, the list beside the reader.
#[test]
fn the_locked_notice_takes_focus_once_the_reader_over_it_closes_wide() {
    locked_notice_after_the_reader(1000.0);
}

/// One view per test: the testing platform is set once per thread.
fn locked_notice_after_the_reader(width: f32) {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(width, 4000.0));
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, &paragraphs(60)));
    open(&mut view, &mut model, &direct(&alice));
    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_reader_open(), "control: open at {width}");

    model.authentication_needed(true);
    view.focus_locked_notice();
    for _ in 0..3 {
        let _ = intents(&mut view, &mut model);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
    }
    assert_ne!(
        view.window().get_focus_id(),
        "notice",
        "nothing under the reader at {width}"
    );
    key(&view, Key::Escape);
    for _ in 0..3 {
        let _ = intents(&mut view, &mut model);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
    }
    assert!(!view.window().get_reader_open());
    assert_eq!(
        view.window().get_focus_id(),
        "notice",
        "the reader gone at {width}"
    );
}

/// A body whole formatted and long as source -- reference definitions
/// draw nothing formatted -- is cut only in source mode; its reader opens
/// as source, and closed after a switch to formatted leaves the row in
/// source mode with its Open the whole message (A1, A7).
#[test]
fn a_body_cut_only_as_source_opens_the_reader_as_source_and_closes_back_to_it() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    let defs = (1..=60)
        .map(|n| format!("[r{n}]: https://example.org/{n}"))
        .collect::<Vec<_>>()
        .join("\n");
    model.received(received(1, &alice, &format!("Intro.\n\n{defs}")));
    open(&mut view, &mut model, &direct(&alice));
    let row = rows(&view)[0].clone();
    assert_eq!(row.cut_line, "", "whole formatted");
    assert_ne!(row.source_cut_line, "", "cut as source");
    assert!(labelled(&view, text(UiText::OpenWholeMessage)).is_empty());

    choose(&view, &row.label, text(UiText::ShowSource));
    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_reader_as_source());
    assert!(
        reader_text(&view)
            .concat()
            .contains("[r60]: https://example.org/60")
    );

    view.window().invoke_reader_switch();
    assert!(!view.window().get_reader_as_source());
    assert!(reader_text(&view).contains(&"Intro.".to_owned()));
    key(&view, Key::Escape);
    let _ = intents(&mut view, &mut model);
    assert!(!view.window().get_reader_open());
    assert_eq!(
        labelled(&view, text(UiText::OpenWholeMessage)).len(),
        1,
        "the row is in source mode still"
    );
}

/// Copy message source gives the message's source byte for byte -- a
/// character past the cluster bound included, which every view draws as
/// one mark -- and, where the platform confirms nothing, the app's
/// snackbar says so (section 5.3, A9, A12).
#[test]
fn copy_gives_the_source_as_received_and_a_snackbar_confirms_it() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    let cluster = format!("a{}", "\u{301}".repeat(255));
    let source = format!("**{cluster}** and [a link](https://example.org)");
    model.received(received(1, &alice, &source));
    let key = direct(&alice);
    open(&mut view, &mut model, &key);
    let row = rows(&view)[0].clone();
    assert_eq!(
        view.window().invoke_message_source(row.handle),
        source.as_str()
    );

    choose(&view, &row.label, text(UiText::CopyMessage));
    let events = view.take_events(&model);
    assert!(
        events.contains(&ViewEvent::MessageCopied(key.clone())),
        "{events:?}"
    );
    model.message_copied(key);
    view.render(&model);
    assert!(model.snackbar().is_some(), "the model holds the snackbar");
    // The snackbar's text, and the window's announcement of it.
    assert!(!labelled(&view, text(UiText::MessageCopied)).is_empty());
}

/// Where the platform confirms a copy itself, the app adds nothing
/// (section 5.3: Android 13 and later).
#[test]
fn copy_says_nothing_where_the_platform_confirms_it() {
    let mut view = view();
    view.set_copy_feedback(interweave_human_ui_slint::CopyFeedback::Platform);
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, "Copied."));
    open(&mut view, &mut model, &direct(&alice));
    let row = rows(&view)[0].clone();
    choose(&view, &row.label, text(UiText::CopyMessage));
    assert!(
        !view
            .take_events(&model)
            .iter()
            .any(|e| matches!(e, ViewEvent::MessageCopied(_)))
    );
}

/// A row gone between its menu opening and the press has no source:
/// Copy writes nothing and nothing says it copied (section 5.3).
#[test]
fn copy_of_a_row_gone_confirms_nothing() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, "Gone soon."));
    model.read(RowId::from_stored(1));
    let key = direct(&alice);
    open(&mut view, &mut model, &key);
    let row = rows(&view)[0].clone();
    let _ = menu(&view, &row.label);
    let entry = ElementQuery::from_root(view.window())
        .find_all()
        .into_iter()
        .find(|e| {
            e.accessible_label().as_deref() == Some(text(UiText::CopyMessage))
                && e.accessible_description().as_deref() == Some(row.label.as_str())
        })
        .expect("the menu's entry");
    model.hide_conversation(&key);
    view.render(&model);
    assert_eq!(view.window().invoke_message_source(row.handle), "");
    entry.invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
    assert!(
        !view
            .take_events(&model)
            .iter()
            .any(|e| matches!(e, ViewEvent::MessageCopied(_)))
    );
}

/// The copy field Copy message source writes through is 0 x 0: that size
/// is what keeps a source's cluster past the bound from the desktop's
/// shaper, which answers a zero-size field's cursor before laying its
/// text out (app.slint, `peers-copy`).
#[test]
fn the_copy_field_has_no_size() {
    let view = view();
    let window = view.window();
    assert_eq!(
        (
            window.get_copy_field_width(),
            window.get_copy_field_height()
        ),
        (0.0, 0.0)
    );
}

/// The reader closes, with nothing said, when its row leaves the
/// conversation: the conversation hidden from the list beside it
/// (section 5.4, A7).
#[test]
fn the_reader_closes_when_its_row_leaves() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, &paragraphs(60)));
    // Read, so the conversation may be hidden.
    model.read(RowId::from_stored(1));
    let key = direct(&alice);
    open(&mut view, &mut model, &key);
    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_reader_open(), "control: open");

    model.hide_conversation(&key);
    view.render(&model);
    assert!(!view.window().get_reader_open());
}

/// The most a drawn item spans, in logical px, in either dimension
/// (HUMAN-CHAT.md, Bounds, architect-cto's DECISION 01a1ab12): four
/// times it is under the desktop renderer's 16-bit ceiling, so it holds
/// to scale 4.
const ITEM_EXTENT: f32 = 8_000.0;

/// A4's shapes at the bound: whole bodies, each in its tallest form, and
/// two cut ones, which the reader draws whole.
fn a4_shapes() -> Vec<(&'static str, String)> {
    let w50 = "abcdefghij".repeat(5);
    vec![
        ("paragraph", "word ".repeat(400).trim_end().to_owned()),
        ("list", vec!["- a"; 40].join("\n")),
        (
            "code",
            format!("```\n{}\n```", vec![w50.as_str(); 38].join("\n")),
        ),
        ("table", {
            let mut t = String::from("| h | i |\n|---|---|\n");
            for _ in 0..39 {
                t.push_str("| abcdefghijklmnopqrstu | abcdefghijklmnopqrstu |\n");
            }
            t
        }),
        (
            "headings",
            (0..40)
                .map(|_| format!("# {w50}"))
                .collect::<Vec<_>>()
                .join("\n\n"),
        ),
        (
            "heading autolinks",
            (0..20)
                .map(|i| format!("# <https://example.org/{i:02}/{}>", "p".repeat(20)))
                .collect::<Vec<_>>()
                .join("\n\n"),
        ),
        ("images", vec!["![](x)"; 110].join(" ")),
        (
            "cluster C1",
            format!("a{}{}", "\u{301}".repeat(255), "b".repeat(1999)),
        ),
        ("unbroken", "x".repeat(2000)),
        // Characters that draw wider than a Latin letter, each just under
        // the bound weighed (A4; HUMAN-CHAT.md Bounds, A 2026-11-07): a
        // count of characters bounds none of them, the measure does.
        ("wide letters", "W".repeat(2000)),
        ("CJK", "\u{6F22}".repeat(1000)),
        ("emoji", "\u{1F600}".repeat(1000)),
        ("U+FDFD", "\u{FDFD}".repeat(2000)),
        // A link control's label is never cut: drawn whole, in pieces by
        // measure inside one control (A 2026-11-07 (ii)), 1,979 clusters.
        (
            "a label of W",
            format!("[a](https://e.org/{})", "W".repeat(1960)),
        ),
        (
            "a label of U+FDFD",
            format!("[a](https://e.org/{})", "\u{FDFD}".repeat(1960)),
        ),
        // Past the bound, so the control is the reader's alone.
        (
            "a label of U+FDFD, cut",
            format!(
                "[a](https://e.org/{}) {}",
                "\u{FDFD}".repeat(1960),
                "x".repeat(100)
            ),
        ),
        // Cut, so that the reader draws them whole, in pieces.
        ("U+FDFD cut", "\u{FDFD}".repeat(4000)),
        ("emoji cut", "\u{1F600}".repeat(3000)),
    ]
}

/// What draws: an element that is, or inherits, one of the toolkit's
/// drawing items.
fn draws(e: &ElementHandle) -> bool {
    const DRAWING: [&str; 5] = ["Rectangle", "Text", "TextInput", "Path", "Image"];
    let mut names: Vec<String> = e.type_name().map(|t| t.to_string()).into_iter().collect();
    names.extend(e.bases().into_iter().flatten().map(|b| b.to_string()));
    // `Plain`, and what is built on it, is a Rectangle with no fill,
    // border or radius, which the renderer draws by no cast of its
    // geometry (app.slint's `Plain`).
    e.layout_kind().is_none()
        && !names.iter().any(|n| n == "Plain")
        && names.iter().any(|n| DRAWING.contains(&n.as_str()))
}

/// Every drawn item past the bound, named for the failure message: of
/// every kind, or text alone.
fn past_the_bound(view: &View) -> Vec<String> {
    past(view, &draws)
}

fn text_past_the_bound(view: &View) -> Vec<String> {
    past(view, &|e: &ElementHandle| {
        draws(e)
            && e.type_name()
                .is_some_and(|t| t.as_str() == "Text" || t.as_str() == "TextInput")
    })
}

/// The one Rectangle as tall as a row that is let be: the row's box,
/// which has no fill and is a Rectangle only to clip a closing row; a
/// clip is intersected with the window's in the software renderer
/// (`combine_clip`) before any geometry is cast, and the desktop probe at
/// scale 3 draws such a row.
const UNFILLED: &str = "AppWindow::row-box";

fn past(view: &View, kind: &dyn Fn(&ElementHandle) -> bool) -> Vec<String> {
    ElementQuery::from_root(view.window())
        .find_all()
        .into_iter()
        .filter(|e| kind(e))
        .filter(|e| e.id().as_deref() != Some(UNFILLED))
        .filter(|e| e.size().height > ITEM_EXTENT || e.size().width > ITEM_EXTENT)
        .map(|e| {
            format!(
                "{} {:?} {}x{}",
                e.type_name().unwrap_or_default(),
                e.id(),
                e.size().width,
                e.size().height
            )
        })
        .collect()
}

/// No drawn item spans more than 8,000 logical px, for every A4 shape at
/// the bound, at 200 % text on the narrowest layout, in the conversation
/// and in the reader, formatted and as source (design A4; the item
/// bound of HUMAN-CHAT.md Bounds).
#[test]
fn no_drawn_item_of_a_body_at_the_bound_passes_the_item_extent() {
    // One window, a conversation per shape: the platform is set once a
    // thread.
    let mut view = view();
    let mut model = UiModel::new();
    for (n, (name, body)) in a4_shapes().into_iter().enumerate() {
        view.window()
            .window()
            // As tall as a window may be under the bound: the panes the
            // window sizes are within it, and every row is in the tree with
            // the fills it spans; a body's own pieces, which the list clips
            // out of the tree below the window, are held by the cut's tests.
            .set_size(slint::LogicalSize::new(360.0, 7_900.0));
        i_slint_core::window::WindowInner::from_pub(view.window().window())
            .context()
            .set_platform_default_font_size(Some(i_slint_core::lengths::LogicalLength::new(28.0)));
        let alice = peer();
        model.received(received(i64::try_from(n).expect("few") + 1, &alice, &body));
        open(&mut view, &mut model, &direct(&alice));
        let row = rows(&view)[0].clone();
        // Every item, then, in a window tall enough that the list clips no
        // piece of the body out of the tree, every text item: there the
        // panes the window sizes are past the bound, and no text is.
        let tall = |view: &View, tall: bool| {
            view.window().window().set_size(slint::LogicalSize::new(
                360.0,
                if tall { 20_000.0 } else { 7_900.0 },
            ));
            slint::platform::update_timers_and_animations();
        };
        let check = |view: &View, at: &str| {
            assert_eq!(past_the_bound(view), Vec::<String>::new(), "{name}, {at}");
            tall(view, true);
            assert_eq!(
                text_past_the_bound(view),
                Vec::<String>::new(),
                "{name}, {at}, text"
            );
            tall(view, false);
        };
        // The reader, where the body is cut, in the mode the row is in.
        let reader = |view: &mut View, model: &mut UiModel, at: &str| {
            if let Some(read) = labelled(view, text(UiText::OpenWholeMessage)).first() {
                read.invoke_accessible_default_action();
                let _ = intents(view, model);
                check(view, at);
                key(view, Key::Escape);
                let _ = intents(view, model);
            }
        };
        check(&view, "row");
        reader(&mut view, &mut model, "reader");
        choose(&view, &row.label, text(UiText::ShowSource));
        check(&view, "source");
        reader(&mut view, &mut model, "reader as source");
    }
}

/// A link control's label past the extent is drawn whole inside one
/// control: its pieces, read in order, are its label, and there are
/// several; one element is named by it. A press on it draws nothing past
/// the extent, and a press's ripple grows no further than
/// `Tile.ripple-reach` (HUMAN-CHAT.md Bounds, A 2026-11-07 (ii)). Whether
/// a label is past the extent is the measure's, so the font's: what is
/// asserted of each shape follows what was drawn, and the shapes hold
/// one past it and one within it on any font the suite runs under (CI's
/// draws U+FDFD narrow).
#[test]
fn a_press_on_a_link_control_past_the_extent_is_drawn_in_pieces() {
    use slint::platform::PointerEventButton;
    let mut view = view();
    let mut model = UiModel::new();
    // The widest Latin letter at 300 % text: about twice the extent with
    // the desktop's fonts, so past it with a narrower W too.
    let shapes = [
        ("W".repeat(1960), Some(true)),
        ("\u{FDFD}".repeat(1960), None),
        // Within the extent, its diagonal past the ripple's reach.
        ("W".repeat(600), Some(false)),
    ];
    for (n, (destination, needed)) in shapes.into_iter().enumerate() {
        let window = view.window().window();
        window.set_size(slint::LogicalSize::new(360.0, 20_000.0));
        i_slint_core::window::WindowInner::from_pub(window)
            .context()
            .set_platform_default_font_size(Some(i_slint_core::lengths::LogicalLength::new(42.0)));
        let alice = peer();
        let destination = format!("https://e.org/{destination}");
        model.received(received(
            i64::try_from(n).expect("few") + 1,
            &alice,
            &format!("[a]({destination})"),
        ));
        open(&mut view, &mut model, &direct(&alice));
        let label = interweave_human_ui_model::body::link_label(&destination);
        let control = the(&view, &label);
        // A piece's Text is no accessible node of its own (the control is
        // named once), so what it draws is read from the row it is given.
        let links = rows(&view)[0].links.clone();
        assert_eq!(links.row_count(), 1, "{n}: one control");
        let pieces: Vec<String> = links
            .row_data(0)
            .expect("the link")
            .pieces
            .iter()
            .map(|p| p.to_string())
            .collect();
        assert_eq!(pieces.concat(), label, "{n}: the label, whole");
        let past = control.size().height > ITEM_EXTENT;
        if let Some(needed) = needed {
            assert_eq!(past, needed, "{n}: {:?}", control.size());
        }
        // A pinned shape is in pieces exactly when its control is past the
        // extent (neither is within the label's 9 px of padding of it); the
        // U+FDFD one, whatever its font draws, at least when past it.
        match needed {
            Some(_) => assert_eq!(pieces.len() > 1, past, "{n}: in pieces past the extent"),
            None => assert!(!past || pieces.len() > 1, "{n}: in pieces past the extent"),
        }
        let drawn = control
            .query_descendants()
            .match_type_name("Text")
            .find_all();
        // The tree holds only what the list shows: every piece of a control
        // the window holds, more than one of one it does not.
        let (top, height) = (control.absolute_position().y, control.size().height);
        if top >= 0.0 && top + height <= 20_000.0 {
            assert_eq!(drawn.len(), pieces.len(), "{n}: each piece drawn");
        } else {
            assert!(drawn.len() > 1, "{n}: pieces drawn, {}", drawn.len());
        }

        // Pressed in a window within the bound, as the extent test checks:
        // in a taller one the panes the window sizes are past it.
        view.window()
            .window()
            .set_size(slint::LogicalSize::new(360.0, 7_900.0));
        slint::platform::update_timers_and_animations();
        let (p, z) = (control.absolute_position(), control.size());
        // Halfway down what shows of it, clear of the header and composer.
        let shown = (p.y.max(400.0), (p.y + z.height).min(7_500.0));
        assert!(shown.0 < shown.1, "{n}: the control shows, {shown:?}");
        let position = slint::LogicalPosition::new(p.x + 20.0, f32::midpoint(shown.0, shown.1));
        let named = |view: &View, kind: &str| {
            ElementQuery::from_root(view.window())
                .match_type_name(kind)
                .find_all()
        };
        let tiled = named(&view, "Tiled").len();
        let window = view.window().window();
        window.dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(400));
        let named = |kind: &str| named(&view, kind);
        assert_eq!(past_the_bound(&view), Vec::<String>::new(), "{n}, pressed");
        // Past the extent the press shows, as one more fill in slices.
        assert_eq!(
            named("Tiled").len(),
            tiled + usize::from(past),
            "{n}: the pressed fill"
        );
        assert_eq!(named("StateLayer").len(), usize::from(!past), "{n}");
        let ripples = named("Ripple");
        assert_eq!(ripples.len(), usize::from(!past), "{n}: a ripple");
        for ripple in ripples {
            assert!(
                ripple.size().width <= 2_000.0 && ripple.size().width > 1_000.0,
                "{n}: grown to its reach, {:?}",
                ripple.size()
            );
        }
        window.dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
        // The press was the control's: released, it opens the link.
        assert_eq!(
            intents(&mut view, &mut model),
            vec![Intent::OpenLink(destination.clone())],
            "{n}: the press landed"
        );
    }

    // The reader draws a cut message's control the same way.
    let alice = peer();
    let destination = format!("https://e.org/{}", "W".repeat(1960));
    model.received(received(
        10,
        &alice,
        &format!("[a]({destination}) {}", "x".repeat(100)),
    ));
    open(&mut view, &mut model, &direct(&alice));
    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(360.0, 20_000.0));
    slint::platform::update_timers_and_animations();
    let label = interweave_human_ui_model::body::link_label(&destination);
    let rows = view.window().get_reader_rows();
    let links: Vec<_> = (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        // A link's row is the one row drawing pieces of a label.
        .filter(|r| r.pieces.row_count() > 0)
        .collect();
    assert_eq!(links.len(), 1, "the reader: one control");
    let pieces: Vec<String> = links[0].pieces.iter().map(|p| p.to_string()).collect();
    assert_eq!(pieces.concat(), label, "the reader: the label, whole");
    assert!(pieces.len() > 1, "the reader: in pieces");
    let drawn = the(&view, &label)
        .query_descendants()
        .match_type_name("Text")
        .find_all();
    assert!(drawn.len() > 1, "the reader: pieces drawn, {}", drawn.len());
}

/// A body measured at one text size is measured again at a larger one: a
/// piece within the extent at the first can be past it at the second
/// (HUMAN-CHAT.md Bounds, A 2026-11-07, `View::measured_for`).
#[test]
fn a_larger_text_size_measures_the_pieces_again() {
    let mut view = view();
    let mut model = UiModel::new();
    let font = |view: &View, size: f32| {
        i_slint_core::window::WindowInner::from_pub(view.window().window())
            .context()
            .set_platform_default_font_size(Some(i_slint_core::lengths::LogicalLength::new(size)));
    };
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(360.0, 20_000.0));
    font(&view, 14.0);
    let alice = peer();
    model.received(received(1, &alice, &"\u{1F600}".repeat(1000)));
    open(&mut view, &mut model, &direct(&alice));
    assert_eq!(text_past_the_bound(&view), Vec::<String>::new(), "at 100 %");
    // Three times the size: the pieces drawn at the first are past it.
    font(&view, 42.0);
    let _ = intents(&mut view, &mut model);
    assert_eq!(text_past_the_bound(&view), Vec::<String>::new(), "at 300 %");
}

/// A text size changed in place, as Android's fontScale is, wakes the
/// root, and the render it brings measures an open reader's rows again,
/// in both forms (review F1 of #583).
#[test]
fn a_text_size_changed_in_place_wakes_the_root_and_measures_the_reader_again() {
    let mut view = view();
    let mut model = UiModel::new();
    let font = |view: &View, size: f32| {
        i_slint_core::window::WindowInner::from_pub(view.window().window())
            .context()
            .set_platform_default_font_size(Some(i_slint_core::lengths::LogicalLength::new(size)));
    };
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(360.0, 20_000.0));
    font(&view, 14.0);
    let woken = std::rc::Rc::new(std::cell::Cell::new(0_u32));
    {
        let woken = std::rc::Rc::clone(&woken);
        view.set_wake(move || woken.set(woken.get() + 1));
    }
    let alice = peer();
    model.received(received(1, &alice, &"\u{1F600}".repeat(3000)));
    open(&mut view, &mut model, &direct(&alice));
    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    assert!(
        view.window().get_reader_open(),
        "control: the reader is open"
    );
    assert_eq!(text_past_the_bound(&view), Vec::<String>::new(), "at 100 %");
    let before = woken.get();
    font(&view, 42.0);
    slint::platform::update_timers_and_animations();
    assert!(woken.get() > before, "the change wakes the root");
    let _ = intents(&mut view, &mut model);
    assert_eq!(text_past_the_bound(&view), Vec::<String>::new(), "at 300 %");
    view.window().invoke_reader_switch();
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_reader_as_source(), "control: as source");
    assert_eq!(
        text_past_the_bound(&view),
        Vec::<String>::new(),
        "at 300 %, as source"
    );
}

/// The bubble says the cut line the mode shown draws, and only then: a
/// body whole formatted and cut as source says none formatted and the
/// source's once Show source is chosen (section 9; review F2 of #565).
#[test]
fn the_cut_line_is_said_in_the_mode_that_draws_it() {
    let mut view = view();
    let mut model = UiModel::new();
    let alice = peer();
    let defs = (1..=60)
        .map(|n| format!("[r{n}]: https://example.org/{n}"))
        .collect::<Vec<_>>()
        .join("\n");
    model.received(received(1, &alice, &format!("Intro.\n\n{defs}")));
    open(&mut view, &mut model, &direct(&alice));
    let row = rows(&view)[0].clone();
    let line = row.source_cut_line.to_string();
    assert!(!line.is_empty(), "control: cut as source");
    let said = |view: &View| -> String {
        ElementQuery::from_root(view.window())
            .match_accessible_role(AccessibleRole::ListItem)
            .find_all()
            .into_iter()
            .filter_map(|e| e.accessible_label().map(|l| l.to_string()))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    assert!(!said(&view).contains(&line), "formatted: no cut line said");
    choose(&view, &row.label, text(UiText::ShowSource));
    assert!(said(&view).contains(&line), "as source: {}", said(&view));
}

/// TAB and Shift+TAB from each focus stop of a conversation, the focus
/// id after each.
fn tabs(view: &mut View, model: &mut UiModel, pressed: Key, n: usize) -> Vec<String> {
    (0..n)
        .map(|_| {
            key(view, pressed);
            let _ = intents(view, model);
            view.window().get_focus_id().to_string()
        })
        .collect()
}

/// Shift+TAB from the composer onto a message taller than the list, the
/// list at its newest end so only the message's lower part is in view:
/// the message is shown from its top (`Reveal.tall`, `keyed`).
#[test]
fn tab_onto_a_tall_message_in_view_in_part_shows_its_top() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(411.0, 891.0));
    let mut model = UiModel::new();
    let alice = peer();
    let headings = (0..40)
        .map(|i| format!("# Heading {i} of a message taller than the list"))
        .collect::<Vec<_>>()
        .join("\n\n");
    model.received(received(1, &alice, &headings));
    open(&mut view, &mut model, &direct(&alice));
    let handle = rows(&view)[0].handle;
    let top = |view: &View, id: &str| {
        ElementHandle::find_by_element_id(view.window(), id)
            .next()
            .unwrap_or_else(|| panic!("{id} drawn"))
            .absolute_position()
            .y
    };
    let forward = tabs(&mut view, &mut model, Key::Tab, 8);
    assert!(
        forward.iter().any(|f| f == "composer"),
        "TAB reaches the composer: {forward:?}"
    );
    while view.window().get_focus_id() != "composer" {
        let _ = key_settled(&mut view, &mut model, Key::Tab);
    }
    // TAB showed the message from its top on the way: back to the
    // newest end, as the conversation opened, by the person's wheel.
    view.window()
        .window()
        .dispatch_event(WindowEvent::PointerScrolled {
            position: slint::LogicalPosition::new(200.0, 400.0),
            delta_x: 0.0,
            delta_y: -100_000.0,
        });
    slint::platform::update_timers_and_animations();
    let (list, before) = (
        top(&view, "AppWindow::messages-flick"),
        top(&view, "AppWindow::item"),
    );
    assert!(
        before < list - 1.0,
        "the message's top starts out of view: at {before}, the list at {list}"
    );
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Backtab),
        format!("item:{handle}")
    );
    let after = top(&view, "AppWindow::item");
    assert!(
        after >= list - 1.0,
        "shown from its top: the message at {after}, the list at {list}"
    );
}

/// A conversation on a phone-sized window whose messages are each
/// taller than the list: the toolkit reveals a focused item only when
/// it fits, and drops the focus of one it then finds out of view at the
/// next key event, the TAB's own release included, so TAB from such a
/// message began again at the top and never reached the next one, the
/// composer or Send (the A40, 2026-11-08). Each is shown from its top as
/// it gains focus (`Reveal`), and both directions cross every stop.
#[test]
fn tab_crosses_messages_taller_than_the_list() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(411.0, 891.0));
    let mut model = UiModel::new();
    let alice = peer();
    let headings = (0..40)
        .map(|i| format!("# Heading {i} of a message taller than the list"))
        .collect::<Vec<_>>()
        .join("\n\n");
    model.received(received(1, &alice, &headings));
    model.received(received(2, &alice, &headings));
    open(&mut view, &mut model, &direct(&alice));
    let handles: Vec<i32> = rows(&view).iter().map(|r| r.handle).collect();
    let [first, second] = handles[..] else {
        panic!("two rows: {handles:?}");
    };
    let stops = [
        "back-to-list".to_owned(),
        "header-title".to_owned(),
        "more-options".to_owned(),
        format!("item:{first}"),
        format!("item:{second}"),
        "composer".to_owned(),
        "send".to_owned(),
    ];
    let forward = tabs(&mut view, &mut model, Key::Tab, 2 * stops.len());
    let start = forward
        .iter()
        .position(|f| f == "back-to-list")
        .expect("TAB reaches the conversation's first stop");
    let cycle: Vec<_> = forward[start..start + stops.len()].to_vec();
    assert_eq!(cycle, stops, "TAB, in order: {forward:?}");
    let backward = tabs(&mut view, &mut model, Key::Backtab, stops.len());
    let mut reversed = stops.to_vec();
    reversed.reverse();
    let at = reversed
        .iter()
        .position(|s| s == &forward[start + stops.len() - 1])
        .expect("TAB ended on a stop");
    let expected: Vec<_> = reversed
        .iter()
        .cycle()
        .skip(at + 1)
        .take(stops.len())
        .cloned()
        .collect();
    assert_eq!(backward, expected, "Shift+TAB, in order");
}

/// A link control whose label is drawn far taller than the list is
/// crossed by TAB as its message is: shown from its top as it gains
/// focus, and the next TAB goes on.
#[test]
fn tab_crosses_a_link_control_taller_than_the_list() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(411.0, 891.0));
    let mut model = UiModel::new();
    let alice = peer();
    // "W" draws wide in any font with Latin letters (U+FDFD draws short
    // where no Arabic font is installed).
    let tall = format!("[a](https://e.org/{})", "W".repeat(1960));
    model.received(received(1, &alice, &tall));
    model.received(received(2, &alice, &tall));
    open(&mut view, &mut model, &direct(&alice));
    let list = ElementHandle::find_by_element_id(view.window(), "AppWindow::messages-flick")
        .next()
        .expect("the list drawn")
        .size()
        .height;
    let control = ElementHandle::find_by_element_type_name(view.window(), "LinkButton")
        .find(|e| e.accessible_label().is_some_and(|l| l.contains("e.org")))
        .expect("a link control drawn")
        .size()
        .height;
    assert!(
        control > list,
        "the case's premise: the link control ({control}) taller than the list ({list})"
    );
    let handles: Vec<i32> = rows(&view).iter().map(|r| r.handle).collect();
    let [first, second] = handles[..] else {
        panic!("two rows: {handles:?}");
    };
    let forward = tabs(&mut view, &mut model, Key::Tab, 12);
    let start = forward
        .iter()
        .position(|f| f == &format!("item:{first}"))
        .expect("TAB reaches the first message");
    assert_eq!(
        forward[start..start + 5],
        [
            format!("item:{first}"),
            format!("link:{first}:0"),
            format!("item:{second}"),
            format!("link:{second}:0"),
            "composer".to_owned(),
        ],
        "TAB, in order: {forward:?}"
    );
    // Back from the composer: each control above is out of view as it
    // gains focus, the first message's link wholly so.
    while view.window().get_focus_id() != "composer" {
        key(&view, Key::Tab);
        let _ = intents(&mut view, &mut model);
    }
    assert_eq!(
        tabs(&mut view, &mut model, Key::Backtab, 5),
        [
            format!("link:{second}:0"),
            format!("item:{second}"),
            format!("link:{first}:0"),
            format!("item:{first}"),
            "more-options".to_owned(),
        ],
        "Shift+TAB, in order"
    );
}

/// `pressed`, then the loop's turns and the timers of a second, the
/// focus id after.
fn key_settled(view: &mut View, model: &mut UiModel, pressed: Key) -> String {
    key(view, pressed);
    let _ = intents(view, model);
    for _ in 0..80 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        slint::platform::update_timers_and_animations();
    }
    view.window().get_focus_id().to_string()
}

/// A reader on a phone-sized window, open on `body`, focus on its
/// scroll area.
fn reader_on(body: &str) -> (View, UiModel) {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(411.0, 891.0));
    let mut model = UiModel::new();
    let alice = peer();
    model.received(received(1, &alice, body));
    open(&mut view, &mut model, &direct(&alice));
    the(&view, text(UiText::OpenWholeMessage)).invoke_accessible_default_action();
    let _ = intents(&mut view, &mut model);
    for _ in 0..3 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        slint::platform::update_timers_and_animations();
    }
    assert_eq!(view.window().get_focus_id(), "reader-scroll");
    (view, model)
}

/// A body of 200 paragraphs, then `n` links, each its own line.
fn paragraphs_then_links(n: usize) -> String {
    let links = (0..n)
        .map(|i| format!("[link {i}](https://e.org/{i:02})"))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("{}\n\n{links}", paragraphs(200))
}

/// Whether the link control `index` is drawn in the reader's view.
fn link_in_view(view: &View, index: usize) -> bool {
    let destination = format!("e.org/{index:02}");
    ElementHandle::find_by_element_type_name(view.window(), "LinkButton").any(|e| {
        e.accessible_label()
            .is_some_and(|label| label.contains(destination.as_str()))
    })
}

/// TAB in the reader reaches every link control of the body in body
/// order, laid out or not, then Back; Shift+TAB the reverse, to the
/// scroll area (section 9, ux-designer's ruling on #581): the reader
/// lays out only the rows near its view, and the toolkit's order
/// reached none of them from the top of a long body.
#[test]
fn tab_in_the_reader_reaches_every_link_laid_out_or_not() {
    let (mut view, mut model) = reader_on(&paragraphs_then_links(60));
    assert!(
        !link_in_view(&view, 0),
        "the links start far below the screen"
    );
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Tab),
        "reader-link:0"
    );
    assert!(link_in_view(&view, 0), "the first link shown");
    for i in 1..60 {
        assert_eq!(
            key_settled(&mut view, &mut model, Key::Tab),
            format!("reader-link:{i}")
        );
        assert!(link_in_view(&view, i), "link {i} shown");
    }
    assert_eq!(key_settled(&mut view, &mut model, Key::Tab), "reader-back");
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Backtab),
        "reader-link:59",
        "Shift+TAB from Back: the last link"
    );
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Backtab),
        "reader-link:58"
    );
    for i in (0..58).rev() {
        assert_eq!(
            key_settled(&mut view, &mut model, Key::Backtab),
            format!("reader-link:{i}")
        );
    }
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Backtab),
        "reader-scroll"
    );
    // Back from the top of the body, where the last link is not laid
    // out: Shift+TAB from Back still reaches it.
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Home),
        "reader-scroll"
    );
    assert!(!link_in_view(&view, 59));
    while view.window().get_focus_id() != "reader-back" {
        key_settled(&mut view, &mut model, Key::Backtab);
    }
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Backtab),
        "reader-link:59",
        "Shift+TAB from Back at the body's top: the last link"
    );
    assert!(link_in_view(&view, 59));
}

/// A link control taller than the screen, reached by TAB, is shown
/// from its top and keeps the focus: the toolkit reveals only what fits
/// the view and drops the focus of what it then finds out of view, so
/// Shift+TAB would begin again at an end.
#[test]
fn tab_in_the_reader_keeps_a_link_taller_than_the_screen() {
    let (mut view, mut model) = reader_on(&format!(
        "{}\n\n[a](https://e.org/{})",
        paragraphs(200),
        // "W" draws wide in any font with Latin letters; U+FDFD drew
        // short on a host without an Arabic font, so the link fitted the
        // screen and, the last row, could not be scrolled to the top.
        "W".repeat(1960)
    ));
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Tab),
        "reader-link:0"
    );
    let link = ElementHandle::find_by_element_type_name(view.window(), "LinkButton")
        .find(|e| e.accessible_label().is_some_and(|l| l.contains("e.org")))
        .expect("the link control drawn");
    let area = ElementHandle::find_by_element_id(view.window(), "AppWindow::reader-scroll")
        .next()
        .expect("the scroll area drawn");
    let scroll = area.absolute_position().y;
    assert!(
        link.size().height > area.size().height,
        "the case's premise: the link ({}) taller than the screen ({})",
        link.size().height,
        area.size().height
    );
    assert!(
        // The row's own 4 px padding sits above the control.
        (link.absolute_position().y - scroll - 4.0).abs() < 1.0,
        "shown from its top: the link at {}, the view at {scroll}",
        link.absolute_position().y
    );
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Backtab),
        "reader-scroll",
        "Shift+TAB from the link goes back to the scroll area"
    );
}

/// From the scroll area TAB goes to the first link control starting at
/// or below the top of the screen, not back to the first link.
#[test]
fn tab_from_the_reader_scroll_area_starts_at_the_screen() {
    let (mut view, mut model) = reader_on(&paragraphs_then_links(60));
    key(&view, Key::End);
    let _ = intents(&mut view, &mut model);
    let shown: Vec<usize> = (0..60).filter(|i| link_in_view(&view, *i)).collect();
    let first_shown = *shown.first().expect("End shows the last links");
    assert!(first_shown > 0, "End shows only the last links: {shown:?}");
    let focus = key_settled(&mut view, &mut model, Key::Tab);
    let index: usize = focus
        .strip_prefix("reader-link:")
        .and_then(|i| i.parse().ok())
        .unwrap_or_else(|| panic!("TAB went to {focus:?}"));
    assert!(
        index == first_shown || index == first_shown + 1,
        "the first link at or below the screen's top ({first_shown}, or the next if it starts above): {index}"
    );
    assert!(link_in_view(&view, index));
}

/// With no link control, TAB from the scroll area goes to Back.
#[test]
fn tab_from_a_reader_with_no_links_goes_to_back() {
    let (mut view, mut model) = reader_on(&paragraphs(200));
    assert_eq!(key_settled(&mut view, &mut model, Key::Tab), "reader-back");
}

/// The source form has no link control: TAB from its scroll area goes
/// to Back though the formatted form has links.
#[test]
fn tab_from_the_source_form_of_a_reader_goes_to_back() {
    let (mut view, mut model) = reader_on(&paragraphs_then_links(3));
    view.window().invoke_reader_switch();
    let _ = intents(&mut view, &mut model);
    assert!(view.window().get_reader_as_source());
    // Back to the scroll area, then TAB.
    while view.window().get_focus_id() != "reader-scroll" {
        key_settled(&mut view, &mut model, Key::Tab);
    }
    assert_eq!(key_settled(&mut view, &mut model, Key::Tab), "reader-back");
}

/// 200 paragraphs, every twentieth holding a link.
fn paragraphs_with_links() -> String {
    (1..=200)
        .map(|i| {
            if i % 20 == 0 {
                format!(
                    "Paragraph {i} [link {}](https://e.org/{:02}).",
                    i / 20 - 1,
                    i / 20 - 1
                )
            } else {
                format!("Paragraph {i}.")
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// In the reader each link's control comes right after the line that
/// holds it, never gathered after the body's last line (section 5.1 as
/// ux-designer amended it, e2a1959d6 on #581): a person pressing a link
/// halfway down a long body finds it there.
#[test]
fn a_reader_link_is_drawn_after_its_own_line() {
    let (view, _model) = reader_on(&paragraphs_with_links());
    let rows = reader_text(&view);
    for k in 0..10 {
        let at = rows
            .iter()
            .position(|r| r.contains(&format!("e.org/{k:02}")) && !r.starts_with("Paragraph"))
            .unwrap_or_else(|| panic!("link {k}'s control"));
        assert!(
            rows[at - 1].starts_with(&format!("Paragraph {} link {k}", (k + 1) * 20)),
            "link {k} follows its line: {:?}",
            &rows[at - 1..=at]
        );
    }
}

/// With links through the body, TAB from the scroll area goes to the
/// first one starting at or below the top of the screen, wherever the
/// person has scrolled to, and on in body order.
#[test]
fn tab_from_the_middle_of_a_reader_starts_at_the_screen() {
    let (mut view, mut model) = reader_on(&paragraphs_with_links());
    for _ in 0..8 {
        key(&view, Key::PageDown);
        let _ = intents(&mut view, &mut model);
    }
    let shown: Vec<usize> = (0..10).filter(|i| link_in_view(&view, *i)).collect();
    let first_shown = *shown.first().expect("a link on the screen");
    assert!(first_shown > 0, "scrolled past the first link: {shown:?}");
    let focus = key_settled(&mut view, &mut model, Key::Tab);
    assert!(
        focus == format!("reader-link:{first_shown}")
            || focus == format!("reader-link:{}", first_shown + 1),
        "TAB from the scroll area at link {first_shown}: {focus}"
    );
    let index: usize = focus["reader-link:".len()..].parse().expect("a link");
    assert_eq!(
        key_settled(&mut view, &mut model, Key::Tab),
        format!("reader-link:{}", index + 1)
    );
}

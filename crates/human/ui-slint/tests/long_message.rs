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

/// Choose `entry` from the open menu of the row labelled `label`.
fn choose(view: &View, label: &str, entry: &str) {
    let _ = menu(view, label);
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
/// message source and Show source last; a whole row's lacks Read the
/// whole message (section 4.3, message-swipe section 7 revised).
#[test]
fn a_rows_menu_has_read_the_whole_message_only_when_cut_and_copy_always() {
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
/// the notice's Unlock rather than the row's Read the whole message --
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
    the(&view, text(UiText::ReadWholeMessage)).invoke_accessible_default_action();
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

/// A4's shapes at the bound: whole bodies, each in its tallest form.
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
        check(&view, "row");
        choose(&view, &row.label, text(UiText::ShowSource));
        check(&view, "source");
        if let Some(read) = labelled(&view, text(UiText::OpenWholeMessage)).first() {
            read.invoke_accessible_default_action();
            let _ = intents(&mut view, &mut model);
            check(&view, "reader");
            key(&view, Key::Escape);
            let _ = intents(&mut view, &mut model);
        }
    }
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

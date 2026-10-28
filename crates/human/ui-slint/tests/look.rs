// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android look (design/2026-10-16-android-look.md) through the
//! compiled views: its colour roles, measured from the palette the window
//! draws with in both schemes, and what each screen shows -- section 11's
//! items a test can check.
//!
//! What this cannot prove: that a screenshot on the phone measures the
//! same (section 11 item 1 is a person's check there), or that every
//! element takes the role the section gives it -- the views tests hold
//! the places that matter.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use i_slint_backend_testing::ElementHandle;
use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{Destination, Origin, Received};
use interweave_human_core::RowId;
use interweave_human_ui_model::{
    ConversationKey, Intent, NewestRow, RuntimeHost, StoredSummary, Table, UiModel, UiText,
    ViewEvent, fill, placeholder_en, preview::Preview,
};
use interweave_human_ui_slint::{Palette, View};
use interweave_transport_api::{EndpointId, TransportIdentity};
use slint::{Color, ComponentHandle as _, Model as _};

/// WCAG 2.2's relative luminance of an sRGB colour.
fn luminance(c: Color) -> f64 {
    let channel = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(c.red()) + 0.7152 * channel(c.green()) + 0.0722 * channel(c.blue())
}

/// WCAG 2.2's contrast ratio.
fn contrast(a: Color, b: Color) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Every pair section 3's contrast table names, with its need: the
/// foreground's role, the background's, and the least ratio. One list,
/// as the section's table is one.
#[allow(clippy::too_many_lines)]
fn pairs(p: &Palette<'_>) -> Vec<(&'static str, Color, Color, f64)> {
    let white = Color::from_rgb_u8(0xff, 0xff, 0xff);
    let neutral_ink = if p.get_dark() {
        p.get_swipe_on_neutral()
    } else {
        white
    };
    vec![
        (
            "on-surface on surface",
            p.get_on_surface(),
            p.get_surface(),
            4.5,
        ),
        (
            "on-surface-variant on surface",
            p.get_on_surface_variant(),
            p.get_surface(),
            4.5,
        ),
        (
            "on-surface on surface-container-high",
            p.get_on_surface(),
            p.get_surface_container_high(),
            4.5,
        ),
        (
            "on-surface-variant on surface-container-high",
            p.get_on_surface_variant(),
            p.get_surface_container_high(),
            4.5,
        ),
        (
            "on-surface-variant on surface-container",
            p.get_on_surface_variant(),
            p.get_surface_container(),
            4.5,
        ),
        (
            "on-surface-variant on primary-container",
            p.get_on_surface_variant(),
            p.get_primary_container(),
            4.5,
        ),
        (
            "on-surface on primary-container",
            p.get_on_surface(),
            p.get_primary_container(),
            4.5,
        ),
        (
            "error on primary-container",
            p.get_error(),
            p.get_primary_container(),
            4.5,
        ),
        (
            "on-primary-container on primary-container",
            p.get_on_primary_container(),
            p.get_primary_container(),
            4.5,
        ),
        (
            "on-primary on primary",
            p.get_on_primary(),
            p.get_primary(),
            4.5,
        ),
        ("primary on surface", p.get_primary(), p.get_surface(), 4.5),
        (
            "primary on surface-container-high",
            p.get_primary(),
            p.get_surface_container_high(),
            4.5,
        ),
        (
            "on-surface code on surface",
            p.get_on_surface(),
            p.get_surface(),
            4.5,
        ),
        (
            "primary ring on primary-container",
            p.get_primary(),
            p.get_primary_container(),
            3.0,
        ),
        (
            "primary ring on surface-container",
            p.get_primary(),
            p.get_surface_container(),
            3.0,
        ),
        (
            "focus-ring-on-inverse on inverse-surface",
            p.get_focus_ring_on_inverse(),
            p.get_inverse_surface(),
            3.0,
        ),
        (
            "focus-ring-on-notice on notice-background",
            p.get_focus_ring_on_notice(),
            p.get_notice_background(),
            3.0,
        ),
        (
            "on-tertiary on tertiary",
            p.get_on_tertiary(),
            p.get_tertiary(),
            4.5,
        ),
        (
            "unread-dot on surface-container-high",
            p.get_unread_dot(),
            p.get_surface_container_high(),
            3.0,
        ),
        (
            "unread-dot on primary-container",
            p.get_unread_dot(),
            p.get_primary_container(),
            3.0,
        ),
        (
            "inverse-on-surface on inverse-surface",
            p.get_inverse_on_surface(),
            p.get_inverse_surface(),
            4.5,
        ),
        (
            "inverse-primary on inverse-surface",
            p.get_inverse_primary(),
            p.get_inverse_surface(),
            4.5,
        ),
        ("error on surface", p.get_error(), p.get_surface(), 4.5),
        // The card proposal (design 2026-10-21 section 8): its text on
        // surface-container, the same-name warning and a key change in
        // error, the PeerId in the code font, and the neutral avatar's
        // figure on surface-variant.
        (
            "on-surface on surface-container",
            p.get_on_surface(),
            p.get_surface_container(),
            4.5,
        ),
        (
            "error on surface-container",
            p.get_error(),
            p.get_surface_container(),
            4.5,
        ),
        (
            "on-surface-variant on surface-variant",
            p.get_on_surface_variant(),
            p.get_surface_variant(),
            3.0,
        ),
        (
            "error outline on surface",
            p.get_error(),
            p.get_surface(),
            3.0,
        ),
        ("outline on surface", p.get_outline(), p.get_surface(), 3.0),
        (
            "swipe-neutral on surface",
            p.get_swipe_neutral(),
            p.get_surface(),
            3.0,
        ),
        (
            "swipe-neutral on surface-container",
            p.get_swipe_neutral(),
            p.get_surface_container(),
            3.0,
        ),
        (
            "swipe-neutral on surface-container-high",
            p.get_swipe_neutral(),
            p.get_surface_container_high(),
            3.0,
        ),
        (
            "swipe-neutral on primary-container",
            p.get_swipe_neutral(),
            p.get_primary_container(),
            3.0,
        ),
        (
            "neutral ink on swipe-neutral",
            neutral_ink,
            p.get_swipe_neutral(),
            4.5,
        ),
        (
            "swipe-on-fill on swipe-keep",
            p.get_swipe_on_fill(),
            p.get_swipe_keep(),
            4.5,
        ),
        (
            "swipe-on-fill on swipe-remove",
            p.get_swipe_on_fill(),
            p.get_swipe_remove(),
            4.5,
        ),
        (
            "swipe-on-fill on swipe-retry",
            p.get_swipe_on_fill(),
            p.get_swipe_retry(),
            4.5,
        ),
        (
            "notice-text on notice-background",
            p.get_notice_text(),
            p.get_notice_background(),
            4.5,
        ),
        (
            "avatar teal",
            p.get_on_avatar_teal(),
            p.get_avatar_teal(),
            4.5,
        ),
        (
            "avatar amber",
            p.get_on_avatar_amber(),
            p.get_avatar_amber(),
            4.5,
        ),
        (
            "avatar navy",
            p.get_on_avatar_navy(),
            p.get_avatar_navy(),
            4.5,
        ),
    ]
}

#[test]
fn every_contrast_pair_of_section_3_meets_its_need() {
    i_slint_backend_testing::init_no_event_loop();
    let view = View::new(RuntimeHost::Daemon).expect("a view");
    let palette = view.window().global::<Palette<'_>>();
    let mut failed = Vec::new();
    for scheme in [
        slint::language::ColorScheme::Dark,
        slint::language::ColorScheme::Light,
    ] {
        palette.set_scheme(scheme);
        assert_eq!(
            palette.get_dark(),
            scheme == slint::language::ColorScheme::Dark
        );
        for (name, fg, bg, need) in pairs(&palette) {
            let ratio = contrast(fg, bg);
            if ratio < need {
                failed.push(format!("{scheme:?} {name}: {ratio:.2} < {need}"));
            }
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

// A canonical PeerId whose avatar is "h4" on the amber pair (ui-model's
// tests/look.rs pins the pick).
const ALICE: &str = "12D3KooWA284B2yjxoAAqAFwwVj6eRQ8DogF3t8wdpMzZ8Hh8wh4";

fn alice() -> TransportIdentity {
    TransportIdentity::parse(ALICE).expect("a PeerId")
}

fn human() -> EndpointId {
    EndpointId::parse("human").expect("an endpoint")
}

fn direct() -> ConversationKey {
    ConversationKey::Direct {
        peer: alice(),
        endpoint: Some(human()),
    }
}

fn envelope(serial: u128, text: &str, sent_at_ms: Option<u64>) -> HumanChatV2 {
    HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: format!("{serial:032x}"),
        text: text.to_owned(),
        reply_to: None,
        sent_at_ms,
        from_endpoint: None,
    }
}

fn received(row: i64, text: &str, at: u64) -> Received {
    Received {
        row: RowId::from_stored(row),
        origin: Origin::Direct {
            peer: alice(),
            endpoint: human(),
        },
        // The sender's clock says a year from now: never what is shown.
        envelope: envelope(
            u128::try_from(row).expect("positive") + 1_000,
            text,
            Some(at + 365 * DAY),
        ),
        received_at: at,
    }
}

fn send(model: &mut UiModel, row: i64, text: &str, at: u64) {
    model.sent(
        RowId::from_stored(row),
        &Destination::Direct {
            peer: alice(),
            endpoint: Some(human()),
        },
        envelope(u128::try_from(row).expect("positive"), text, None),
        at,
    );
}

const MINUTE: u64 = 60_000;
const DAY: u64 = 86_400_000;
// 2026-10-17 14:30 UTC.
const NOW: u64 = 20_743 * DAY + 14 * 60 * MINUTE + 30 * MINUTE;

fn view() -> View {
    thread_local! {
        static PLATFORM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    // The platform is set once per thread, and a test may make two views.
    if !PLATFORM.replace(true) {
        i_slint_backend_testing::init_no_event_loop();
    }
    let mut view = View::new(RuntimeHost::Embedded).expect("a view");
    view.set_clock(|| NOW, |_| 0);
    view.window().show().expect("shown");
    view
}

fn all(view: &View) -> Vec<ElementHandle> {
    let found = i_slint_backend_testing::ElementQuery::from_root(view.window()).find_all();
    assert!(
        !found.is_empty(),
        "the tree is not empty: built with debug info"
    );
    found
}

fn labelled(view: &View, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(view.window(), label).collect()
}

/// Every intent the view raises now, as the root takes them.
fn intents(view: &mut View, model: &UiModel) -> Vec<Intent> {
    view.take_events(model)
        .into_iter()
        .filter_map(|event| match event {
            ViewEvent::Intent(intent) => Some(intent),
            _ => None,
        })
        .collect()
}

/// Section 11 item 2, as #304 F12 changed it: the list's app bar holds
/// its title and the app's options as an icon button, Trust among them,
/// and nothing else in it.
#[test]
fn the_list_bar_holds_the_title_and_the_apps_options_as_its_one_button() {
    let mut view = view();
    view.render(&UiModel::new());
    let title = labelled(&view, placeholder_en::text(UiText::Conversations));
    assert_eq!(title.len(), 1, "the title, once");
    let trust = labelled(&view, placeholder_en::text(UiText::AppOptions));
    assert_eq!(trust.len(), 1, "one App options control (#304 F12)");
    assert_eq!(
        trust[0].accessible_role(),
        Some(i_slint_backend_testing::AccessibleRole::Button)
    );
    let (bar, button) = (title[0].absolute_position(), trust[0].absolute_position());
    assert!(
        (button.y - bar.y).abs() < 48.0,
        "the button sits in the title's bar, not at the list's foot: {bar:?} {button:?}"
    );
    assert!(trust[0].size().width >= 48.0 && trust[0].size().height >= 48.0);
}

/// A card someone sent, the newest row of a conversation listed from the
/// store, previews as a card someone sent -- never as one's own, and never
/// by the name it claims (contact-cards 5.4.5; #498 review F2). Mutation:
/// every card row previewed as one's own.
#[test]
fn a_received_card_previews_as_a_contact_card_not_ones_own() {
    let mut view = view();
    let mut model = UiModel::new();
    model.started(
        Vec::new(),
        vec![StoredSummary {
            conversation: direct(),
            unread: 1,
            newest: Some(NewestRow {
                table: Table::Unread,
                row: RowId::from_stored(7),
                received_at: NOW - MINUTE,
                preview: Preview::default(),
                card: true,
            }),
        }],
        None,
    );
    view.render(&model);
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row");
    assert_eq!(
        row.preview,
        placeholder_en::text(UiText::CardPreviewReceived)
    );
}

/// Section 11 items 3, 4 and 6: a row shows the avatar, the name, the
/// last message as plain text from its first line, the time from when it
/// reached this app, and the unread count -- and the name is bold exactly
/// while the count shows.
#[test]
fn a_row_shows_avatar_name_preview_time_and_unread_count() {
    let mut view = view();
    let mut model = UiModel::new();
    model.received(received(1, "first", NOW - 3 * MINUTE));
    model.received(received(
        2,
        "**Bold** and [the site](https://example.org)\nsecond line",
        NOW - 13 * 60 * MINUTE - 12 * MINUTE,
    ));
    view.render(&model);
    let rows = view.window().get_conversations();
    assert_eq!(rows.row_count(), 1);
    let row = rows.row_data(0).expect("a row");
    assert_eq!(row.avatar_letters, "h4");
    assert_eq!(row.avatar_hue, 1, "amber");
    // The later of the two by local time is "first", whatever the order
    // they arrived in.
    assert_eq!(row.preview, "first");
    assert_eq!(row.time, "14:27");
    assert_eq!(row.unread, "2");
    assert!(!row.pending);

    model.received(received(
        3,
        "**Bold** and [the site](https://example.org)\nsecond line",
        NOW,
    ));
    view.render(&model);
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row");
    assert_eq!(
        row.preview, "Bold and the site",
        "markup gone, the link's text kept, one line"
    );
    assert_eq!(row.time, "14:30");
    assert_eq!(row.unread, "3");
    let names: Vec<_> = all(&view)
        .into_iter()
        .filter(|e| e.accessible_label().as_deref() == Some("h4"))
        .collect();
    assert!(names.is_empty(), "the avatar is never read");
}

/// Section 11 item 3's clock: one's own pending last message, with
/// nothing unread, shows the clock in the badge's place and previews as
/// one's own.
#[test]
fn ones_own_pending_last_message_shows_the_clock_and_previews_as_ones_own() {
    let mut view = view();
    let mut model = UiModel::new();
    send(&mut model, 1, "See you at six", NOW - DAY);
    view.render(&model);
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row");
    assert_eq!(row.unread, "");
    assert!(row.pending);
    assert_eq!(
        row.preview,
        fill(
            placeholder_en::text(UiText::OwnLastMessage),
            &[("message", "See you at six")]
        )
    );
    assert_eq!(row.time, placeholder_en::text(UiText::Yesterday));
    // With something unread, the badge shows and never the clock: the
    // view draws the clock only for a row whose count is empty.
    model.received(received(2, "ok", NOW - DAY - MINUTE));
    view.render(&model);
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row");
    assert_eq!(row.unread, "1");
}

/// Section 11 item 5: a list that previews two unread messages raises no
/// read, and both stay unread for the store, until the conversation is
/// opened.
#[test]
fn a_preview_reads_nothing() {
    let mut view = view();
    let mut model = UiModel::new();
    model.received(received(1, "one", NOW - 2 * MINUTE));
    model.received(received(2, "two", NOW - MINUTE));
    view.set_window_focused(true);
    for _ in 0..3 {
        view.render(&model);
        slint::platform::update_timers_and_animations();
        assert!(
            intents(&mut view, &model).is_empty(),
            "the list raises nothing"
        );
    }
    let row = view
        .window()
        .get_conversations()
        .row_data(0)
        .expect("a row");
    assert_eq!(row.preview, "two");
    assert_eq!(row.unread, "2");
    // The control: opening it is what reads, both rows.
    view.select(direct());
    let raised = intents(&mut view, &model);
    view.render(&model);
    let raised = [raised, intents(&mut view, &model)].concat();
    assert!(
        raised.contains(&Intent::MarkRead(RowId::from_stored(1)))
            && raised.contains(&Intent::MarkRead(RowId::from_stored(2))),
        "{raised:?}"
    );
}

/// Press the one control labelled `label`, as a screen reader does.
fn press(view: &View, label: &str) {
    let found = labelled(view, label);
    assert_eq!(found.len(), 1, "one control labelled {label:?}");
    found[0].invoke_accessible_default_action();
    slint::platform::update_timers_and_animations();
}

/// Section 11 item 7: on a phone, the conversation's app bar has Back,
/// the name and the connectivity line, and an overflow button labelled
/// "More options" whose menu holds Details always and Hide conversation
/// exactly when it is offered; Details shows the exact `PeerId`.
#[test]
fn the_overflow_menu_holds_details_and_hide_exactly_when_offered() {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    let mut model = UiModel::new();
    let text = placeholder_en::text;
    // Something unread: Hide conversation is not offered.
    model.received(received(1, "hi", NOW));
    view.select(direct());
    let _ = view.take_events(&model);
    view.render(&model);
    slint::platform::update_timers_and_animations();
    assert_eq!(labelled(&view, text(UiText::BackToConversations)).len(), 1);
    assert_eq!(
        labelled(&view, &model.title(&direct())).len(),
        1,
        "the name"
    );
    press(&view, text(UiText::MoreOptions));
    assert_eq!(labelled(&view, text(UiText::Details)).len(), 1);
    assert!(
        labelled(&view, text(UiText::HideConversation)).is_empty(),
        "an unread message: not offered"
    );
    press(&view, text(UiText::Details));
    // #304 F11: the whole PeerId as text under its label, not a field.
    assert_eq!(labelled(&view, text(UiText::DetailsPeerId)).len(), 1);
    assert_eq!(labelled(&view, ALICE).len(), 1, "the PeerId, whole");
    assert!(
        labelled(&view, text(UiText::Details)).is_empty(),
        "the menu closed"
    );

    // Read: now Hide conversation is offered, in the same menu.
    model.read(RowId::from_stored(1));
    view.render(&model);
    view.window().set_details_open(false);
    press(&view, text(UiText::MoreOptions));
    assert_eq!(labelled(&view, text(UiText::HideConversation)).len(), 1);
    assert_eq!(labelled(&view, text(UiText::Details)).len(), 1);
}

/// Open the direct conversation in a phone-sized window.
fn opened(model: &UiModel, rtl: bool) -> View {
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 1600.0));
    view.window().set_rtl(rtl);
    view.select(direct());
    let _ = view.take_events(model);
    view.render(model);
    slint::platform::update_timers_and_animations();
    view
}

/// The left edge of the one element labelled `label`.
fn left_of(view: &View, label: &str) -> f32 {
    let found = labelled(view, label);
    assert_eq!(found.len(), 1, "one element labelled {label:?}");
    found[0].absolute_position().x
}

/// Section 11 item 8: received on the start side and sent on the end
/// side, swapped right to left; 2 px within a minute on one side, 8 px
/// otherwise; a day change shows its separator.
#[test]
fn bubbles_sit_on_their_side_grouped_by_the_minute_with_day_separators() {
    let mut model = UiModel::new();
    model.received(received(1, "yesterday's", NOW - DAY));
    model.received(received(2, "first", NOW - 3 * MINUTE));
    model.received(received(3, "second", NOW - 3 * MINUTE + 20_000));
    send(&mut model, 4, "mine", NOW - MINUTE);
    for rtl in [false, true] {
        let view = opened(&model, rtl);
        let (theirs, mine) = (left_of(&view, "first"), left_of(&view, "mine"));
        if rtl {
            assert!(theirs > mine, "right to left, received on the right");
        } else {
            assert!(theirs < mine, "received on the left");
        }
        let rows = view.window().get_messages();
        let gaps: Vec<f32> = rows.iter().map(|r| r.gap).collect();
        assert_eq!(gaps, [0.0, 8.0, 2.0, 8.0], "a run, then a change of side");
        let days: Vec<String> = rows.iter().map(|r| r.day.to_string()).collect();
        assert_eq!(
            days,
            [
                placeholder_en::text(UiText::Yesterday),
                placeholder_en::text(UiText::Today),
                "",
                ""
            ]
        );
        assert_eq!(
            labelled(&view, placeholder_en::text(UiText::Today)).len(),
            1
        );
    }
}

/// Section 11 item 9: in a channel each received bubble opens with its
/// author, described by the full `PeerId`; in a direct conversation none
/// does.
#[test]
fn a_channels_bubbles_name_their_author_and_a_direct_ones_do_not() {
    let general = interweave_transport_api::ChannelId::parse("general").expect("a channel");
    let mut model = UiModel::new();
    model.received(Received {
        row: RowId::from_stored(1),
        origin: Origin::Channel {
            channel: general.clone(),
            publisher: alice(),
        },
        envelope: envelope(1, "to all", None),
        received_at: NOW,
    });
    model.received(received(2, "to me", NOW));
    let mut view = view();
    view.select(ConversationKey::Channel(general));
    let _ = view.take_events(&model);
    view.render(&model);
    let short = interweave_human_ui_model::short_peer(ALICE);
    // The author line: a text of the short PeerId described by the full
    // one. The list's row and the app bar's heading title a direct
    // conversation with the same short PeerId (human-client-ui.md §4).
    let not_a_row = |e: &ElementHandle| {
        e.type_name().as_deref() == Some("Text")
            && e.accessible_description().as_deref() == Some(ALICE)
    };
    let author: Vec<_> = labelled(&view, &short)
        .into_iter()
        .filter(not_a_row)
        .collect();
    assert_eq!(author.len(), 1, "the author line");
    assert_eq!(author[0].accessible_description().as_deref(), Some(ALICE));

    let view = opened(&model, false);
    assert!(
        !labelled(&view, &short).into_iter().any(|e| not_a_row(&e)),
        "no author line"
    );
}

/// Section 11 item 10: each status shows its words and its icon, and an
/// accepted message a check, never the words "Read" or "Seen".
#[test]
fn each_status_has_its_words_and_its_icon() {
    use interweave_human_client_api::{ClientEvent, OutboundStatus, OutboundUpdate};
    use interweave_human_core::AppMessageId;
    use interweave_human_ui_model::LabelKey;
    let mut model = UiModel::new();
    model.received(received(1, "unread", NOW));
    send(&mut model, 2, "accepted", NOW);
    model.client_event(ClientEvent::Outbound(OutboundUpdate {
        row: RowId::from_stored(2),
        app_message_id: AppMessageId::parse(format!("{:032x}", 2)).expect("an id"),
        status: OutboundStatus::Accepted { endpoint: human() },
        last_code: None,
    }));
    send(&mut model, 3, "sending", NOW);
    let view = opened(&model, false);
    let rows: Vec<_> = view.window().get_messages().iter().collect();
    let by_status = |label: LabelKey| {
        rows.iter()
            .find(|r| r.status == placeholder_en::label(label))
            .unwrap_or_else(|| panic!("a row whose status is {label:?}"))
    };
    let unread = by_status(LabelKey::Unread);
    assert!(unread.status_dot, "the unread dot");
    let accepted = by_status(LabelKey::AcceptedByRemoteTransport);
    assert_eq!(
        accepted.status_glyph,
        interweave_human_ui_slint::Glyph::Check
    );
    assert!(!accepted.status_dot);
    let words = accepted.status.to_lowercase();
    assert!(
        !words.contains("read") && !words.contains("seen"),
        "{words}"
    );
    // A waiting message reads one status with its age (#304 F10).
    let sending = rows
        .iter()
        .find(|r| r.status == "Still sending, started just now")
        .expect("a row still sending");
    assert_eq!(
        sending.status_glyph,
        interweave_human_ui_slint::Glyph::Clock
    );
    for status in [unread, accepted, sending] {
        assert_eq!(
            labelled(&view, &status.status).len(),
            1,
            "its words are drawn"
        );
    }
}

/// The view's monotonic clock, stepped by [`step`] instead of slept
/// through: a leave is 170 ms of it.
fn stepped(view: &mut View) -> std::rc::Rc<std::cell::Cell<std::time::Duration>> {
    let start = std::time::Instant::now();
    let offset = std::rc::Rc::new(std::cell::Cell::new(std::time::Duration::ZERO));
    let read = std::rc::Rc::clone(&offset);
    view.set_monotonic(move || start + read.get());
    offset
}

fn step(clock: &std::cell::Cell<std::time::Duration>, ms: u64) {
    clock.set(clock.get() + std::time::Duration::from_millis(ms));
}

/// Section 8, a very long message: the bubble wraps and holds every line
/// of it, its status under the text, and nothing shows under it at rest:
/// its actions are in its menu (section 6, revised for #304 F2).
#[test]
fn a_wrapped_bubble_holds_its_status_and_nothing_under_it() {
    let mut model = UiModel::new();
    let long =
        "A long message with no link at all that goes past the width of the row for sure and wraps";
    model.received(received(6, long, NOW - 30_000));
    let view = opened(&model, false);
    let one = |label: &str| {
        let found = labelled(&view, label);
        assert_eq!(found.len(), 1, "one {label:?}");
        let (p, z) = (found[0].absolute_position(), found[0].size());
        (p.y, z.height, z.width)
    };
    let (text_y, text_h, text_w) = one(long);
    let (status_y, status_h, _) = one(placeholder_en::label(
        interweave_human_ui_model::LabelKey::Unread,
    ));
    assert!(text_h > 2.0 * status_h, "the text wrapped: {text_h}");
    assert!(text_w <= 390.0 * 0.8, "inside 80 % of the width: {text_w}");
    assert!(status_y >= text_y + text_h, "the status under the text");
    for action in [UiText::ShowSource, UiText::Keep, UiText::Dismiss] {
        assert!(
            labelled(&view, placeholder_en::text(action)).is_empty(),
            "no {action:?} at rest"
        );
    }
}

/// Review F1 on #295: the midnight wake is one timer, however many
/// renders asked for it. A timer per render woke the root once per render
/// at midnight, and each of those renders scheduled another.
#[test]
fn renders_wake_the_root_once_at_midnight() {
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW - MINUTE));
    let mut view = view();
    let woken = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = std::rc::Rc::clone(&woken);
    view.set_wake(move || count.set(count.get() + 1));
    for _ in 0..200 {
        view.render(&model);
    }
    assert_eq!(woken.get(), 0, "nothing before midnight");
    // 14:30 UTC: midnight is 9 h 30 min away.
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_secs(
        9 * 3600 + 30 * 60 + 1,
    ));
    assert_eq!(woken.get(), 1, "one wake at midnight");
}

/// #304 F10: a waiting message's one status says how long it has
/// waited since it was written, and the window wakes once, when that next
/// reads otherwise, to draw it.
#[test]
fn a_waiting_messages_age_wakes_the_root_when_it_next_reads_otherwise() {
    let mut model = UiModel::new();
    // Written 1 min 30 s before the view's clock: "1 minute ago" for 30 s.
    send(&mut model, 3, "waiting", NOW - 90_000);
    let mut view = view();
    let woken = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = std::rc::Rc::clone(&woken);
    view.set_wake(move || count.set(count.get() + 1));
    view.select(direct());
    let _ = view.take_events(&model);
    view.render(&model);
    let statuses: Vec<String> = view
        .window()
        .get_messages()
        .iter()
        .map(|r| r.status.to_string())
        .collect();
    assert_eq!(statuses, ["Still sending, started 1 minute ago"]);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_secs(29));
    assert_eq!(woken.get(), 0, "nothing before the next minute");
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_secs(2));
    assert_eq!(woken.get(), 1, "one wake when it reads 2 minutes");
}

/// Review F2 on #295, section 7: right to left the conversation's app bar
/// mirrors -- Back at the line's start on the right, More options at its
/// end on the left -- and the menu opens on More's side.
#[test]
fn the_conversation_bar_and_its_menu_mirror_right_to_left() {
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    model.read(RowId::from_stored(1));
    for rtl in [false, true] {
        let view = opened(&model, rtl);
        let back = left_of(&view, text(UiText::BackToConversations));
        let more = left_of(&view, text(UiText::MoreOptions));
        assert_eq!(back > more, rtl, "rtl={rtl}: back {back}, more {more}");
        press(&view, text(UiText::MoreOptions));
        let centre = |label: &str| {
            let found = labelled(&view, label);
            assert_eq!(found.len(), 1, "one {label:?}");
            found[0].absolute_position().x + found[0].size().width / 2.0
        };
        let (menu, button) = (
            centre(text(UiText::Details)),
            centre(text(UiText::MoreOptions)),
        );
        assert_eq!(
            menu < 195.0,
            button < 195.0,
            "rtl={rtl}: the menu on More's side"
        );
        view.window().set_menu_open(false);
    }
}

/// Review F3 on #295, as #304 F11 reshaped it: Details is a modal
/// dialog. Focus starts on Copy; Tab and Shift+Tab move between its two
/// actions -- never to the dialog itself, which draws no ring, nor to the
/// controls behind the scrim -- and Escape closes it, giving focus back
/// to More options.
#[test]
fn tab_stays_in_details_and_escape_closes_it() {
    use slint::platform::{Key, WindowEvent};
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    let view = opened(&model, false);
    // The field's ring is the one shown: focus is on a control that draws
    // it, never on the dialog, which draws none.
    let rings = || {
        all(&view)
            .into_iter()
            .filter(|e| e.type_name().as_deref() == Some("FocusRing"))
            .filter(|r| r.computed_opacity() > 0.0)
            .count()
    };
    let send = |key: Key| {
        let key: slint::SharedString = key.into();
        let window = view.window().window();
        window.dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
        window.dispatch_event(WindowEvent::KeyReleased { text: key });
        // A focus moves on a timer's turn and is reported a turn later.
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    };
    press(&view, text(UiText::MoreOptions));
    press(&view, text(UiText::Details));
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "details-copy",
        "on Copy, the dialog's act (#304 F11)"
    );
    // Tab either way moves between the dialog's two actions, never out.
    for (key, to) in [
        (Key::Backtab, "details-close"),
        (Key::Tab, "details-copy"),
        (Key::Tab, "details-close"),
        (Key::Backtab, "details-copy"),
    ] {
        send(key);
        assert_eq!(view.window().get_focus_id().as_str(), to, "{key:?}");
        assert_eq!(rings(), 1, "its ring shown");
        assert!(view.window().get_details_open());
    }
    // A press on the dialog's padding, beside the PeerId, leaves focus.
    let id = labelled(&view, ALICE);
    assert_eq!(id.len(), 1);
    let (at, window) = (id[0].absolute_position(), view.window().window());
    let position = slint::LogicalPosition::new(at.x - 12.0, at.y + 4.0);
    let button = slint::platform::PointerEventButton::Left;
    window.dispatch_event(WindowEvent::PointerPressed { position, button });
    window.dispatch_event(WindowEvent::PointerReleased { position, button });
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "details-copy",
        "a press beside it"
    );
    assert!(
        view.window().get_details_open(),
        "a press inside the dialog keeps it"
    );
    send(Key::Escape);
    assert!(!view.window().get_details_open(), "Escape closes it");
    assert_eq!(view.window().get_focus_id().as_str(), "more-options");
}

/// ux-designer-01's ruling on #295 (spec 025bb324): a bubble's label says
/// the clock time it shows, never its day, and the day separator heading
/// its run is an item a screen reader reaches before the bubble.
#[test]
fn an_older_bubble_reads_its_clock_time_after_its_day_separator() {
    let mut model = UiModel::new();
    model.received(received(1, "from yesterday", NOW - DAY - 2 * 60 * MINUTE));
    let view = opened(&model, false);
    let label = fill(
        placeholder_en::text(UiText::Item),
        &[
            ("author", &interweave_human_ui_model::short_peer(ALICE)),
            ("body", "from yesterday"),
            (
                "status",
                placeholder_en::label(interweave_human_ui_model::LabelKey::Unread),
            ),
            ("time", "12:30"),
        ],
    );
    let bubble = labelled(&view, &label);
    assert_eq!(bubble.len(), 1, "the label ends in the clock time: {label}");
    let day = labelled(&view, placeholder_en::text(UiText::Yesterday));
    assert_eq!(day.len(), 1, "the separator is an item of its own");
    let tree: Vec<_> = all(&view)
        .into_iter()
        .filter_map(|e| e.accessible_label())
        .collect();
    let at = |l: &str| tree.iter().position(|t| t == l).expect("in the tree");
    assert!(
        at(placeholder_en::text(UiText::Yesterday)) < at(&label),
        "the day before the bubble in reading order"
    );
}

/// Section 5.2 and ux-designer-01's ruling (REPLY 01a14817): every shown
/// time follows the platform's 12- or 24-hour setting, read at each render.
#[test]
fn times_follow_the_platforms_hour_setting() {
    use interweave_human_ui_slint::TouchSettings;
    let mut model = UiModel::new();
    // 14:29 UTC.
    model.received(received(1, "hi", NOW - MINUTE));
    let mut view = view();
    view.render(&model);
    let time = |view: &View| {
        view.window()
            .get_conversations()
            .row_data(0)
            .expect("a row")
            .time
            .to_string()
    };
    assert_eq!(time(&view), "14:29", "24-hour until told");
    view.set_touch_settings(TouchSettings {
        hour24: false,
        ..TouchSettings::default()
    });
    view.render(&model);
    assert_eq!(time(&view), "2:29 PM");
    view.set_touch_settings(TouchSettings::default());
    view.render(&model);
    assert_eq!(time(&view), "14:29");
}

/// Section 11 item 12, for the composer and the app bar: Send and Back
/// are 48 x 48, and Send does nothing while the field is empty.
#[test]
fn send_does_nothing_while_the_field_is_empty() {
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    let mut view = opened(&model, false);
    for label in [text(UiText::Send), text(UiText::BackToConversations)] {
        let found = labelled(&view, label);
        assert_eq!(found.len(), 1, "{label}");
        let size = found[0].size();
        assert!(
            size.width >= 48.0 && size.height >= 48.0,
            "{label} is {size:?}"
        );
    }
    let send = || labelled(&view, text(UiText::Send))[0].clone();
    // Inactive while empty: it says so to a screen reader, and is drawn
    // dimmed; the model refuses an empty send besides.
    assert_eq!(
        send().accessible_enabled(),
        Some(false),
        "inactive while empty"
    );
    send().invoke_accessible_default_action();
    assert!(
        intents(&mut view, &model).is_empty(),
        "nothing typed: nothing sent"
    );
    view.window().invoke_draft_edited("typed".into());
    let _ = view.take_events(&model);
    model.draft_changed(direct(), "typed".to_owned());
    view.render(&model);
    assert_eq!(
        labelled(&view, text(UiText::Send))[0].accessible_enabled(),
        Some(true),
        "active with text"
    );
    labelled(&view, text(UiText::Send))[0].invoke_accessible_default_action();
    let raised = intents(&mut view, &model);
    assert!(
        raised
            .iter()
            .any(|i| matches!(i, Intent::Send { draft, .. } if draft == "typed")),
        "the control: with text, Send sends: {raised:?}"
    );
}

/// Section 7: the snackbar on the Chats list sits 12 px from the screen's
/// sides, at least 48 px tall, its Undo a 48 x 48 target inside it.
#[test]
fn the_lists_snackbar_keeps_its_margins_and_its_targets() {
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    model.read(RowId::from_stored(1));
    let mut view = view();
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 800.0));
    view.render(&model);
    model.hide_conversation(&direct());
    view.render(&model);
    slint::platform::update_timers_and_animations();
    assert!(view.window().get_snack_shown() && view.window().get_snack_on_list());
    // The snackbar's text, not the window's announcement of it, which sits
    // in a point at the window's corner.
    let said: Vec<_> = labelled(&view, text(UiText::ConfirmConversationHidden))
        .into_iter()
        .filter(|e| e.absolute_position().y > 100.0)
        .collect();
    let undo = labelled(&view, text(UiText::Undo));
    assert_eq!((said.len(), undo.len()), (1, 1));
    let (s, u) = (said[0].absolute_position(), undo[0].absolute_position());
    let us = undo[0].size();
    assert!(s.x >= 12.0, "12 px from the start: {s:?}");
    assert!(
        u.x + us.width <= 390.0 - 12.0,
        "12 px from the end: {u:?} {us:?}"
    );
    assert!(us.width >= 48.0 && us.height >= 48.0, "Undo is {us:?}");
}

/// Section 7, motion: a press shows Material's state layer and, while the
/// system animates, a ripple from the press point; with the system's
/// animations off, the flat state layer alone (section 11 item 18).
#[test]
fn a_press_ripples_unless_animations_are_off() {
    use interweave_human_ui_slint::TouchSettings;
    use slint::platform::{PointerEventButton, WindowEvent};
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    model.read(RowId::from_stored(1));
    for reduced in [false, true] {
        let view = opened(&model, false);
        view.set_touch_settings(TouchSettings {
            reduced_motion: reduced,
            ..TouchSettings::default()
        });
        // A menu item's press, the row's menu opened as a screen reader
        // opens it.
        let row = view.window().get_messages().row_data(0).expect("a row");
        all(&view)
            .into_iter()
            .find(|e| {
                e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::ListItem)
                    && e.accessible_label().as_deref() == Some(row.label.as_str())
            })
            .expect("the row's item")
            .invoke_accessible_default_action();
        let chip = labelled(&view, text(UiText::Keep));
        assert_eq!(chip.len(), 1);
        let (p, z) = (chip[0].absolute_position(), chip[0].size());
        let position = slint::LogicalPosition::new(p.x + z.width / 2.0, p.y + z.height / 2.0);
        let window = view.window().window();
        window.dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(120));
        let count = |kind: &str| {
            all(&view)
                .into_iter()
                .filter(|e| e.type_name().as_deref() == Some(kind))
                .count()
        };
        assert_eq!(count("StateLayer"), 1, "reduced={reduced}: the state layer");
        assert_eq!(
            count("Ripple"),
            usize::from(!reduced),
            "reduced={reduced}: the ripple"
        );
        window.dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
        assert_eq!(count("StateLayer"), 0, "gone once released");
    }
}

/// Section 6: a message's swipe fill spans the list's width in the
/// bubble's height -- over its text and status.
#[test]
fn a_bubbles_swipe_fill_takes_the_bubbles_height() {
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "swipe me", NOW));
    model.read(RowId::from_stored(1));
    let view = opened(&model, false);
    let handle = view
        .window()
        .get_messages()
        .row_data(0)
        .expect("a row")
        .handle;
    view.window().set_swipe_key(format!("m:{handle}").into());
    view.window().set_swipe_offset(40.0);
    slint::platform::update_timers_and_animations();
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(300));
    let fill: Vec<_> = all(&view)
        .into_iter()
        .filter(|e| e.type_name().as_deref() == Some("SwipeReveal"))
        .collect();
    assert_eq!(fill.len(), 1, "the row's fill is drawn");
    let (top, height) = (fill[0].absolute_position().y, fill[0].size().height);
    let bottom = top + height;
    let body = labelled(&view, "swipe me");
    let status = labelled(
        &view,
        placeholder_en::label(interweave_human_ui_model::LabelKey::ReadNotKept),
    );
    let _ = text;
    assert!(top <= body[0].absolute_position().y, "over the text");
    assert!(
        bottom >= status[0].absolute_position().y + status[0].size().height,
        "over the status"
    );
    assert!(fill[0].size().width >= 390.0 - 40.0, "the list's width");
}

/// Section 7, "everything mirrors": inside a bubble, a quoted list item's
/// indent sits on the line's start side -- the left, left to right, and
/// the right, right to left -- measured against an unindented line of the
/// same bubble, whose box spans the bubble's width.
#[test]
fn a_quoted_list_line_mirrors_right_to_left() {
    let mut model = UiModel::new();
    model.received(received(
        1,
        "a plain line, as wide as the bubble\n\n> - quoted item",
        NOW,
    ));
    for rtl in [false, true] {
        let view = opened(&model, rtl);
        let bounds = |label: &str| {
            let found = labelled(&view, label);
            assert_eq!(found.len(), 1, "rtl={rtl}: {label}");
            let (p, z) = (found[0].absolute_position(), found[0].size());
            (p.x, p.x + z.width)
        };
        let (plain_left, plain_right) = bounds("a plain line, as wide as the bubble");
        let (quoted_left, quoted_right) = bounds("• quoted item");
        // Two levels deep (the quote and the list), 16 px each.
        if rtl {
            assert!(
                quoted_right <= plain_right - 32.0,
                "rtl: {quoted_right} vs {plain_right}"
            );
        } else {
            assert!(
                quoted_left >= plain_left + 32.0,
                "ltr: {quoted_left} vs {plain_left}"
            );
        }
    }
}

/// Section 7, "everything mirrors": right to left, the composer's Send
/// sits at the line's end, on the left of the field -- while Tab, in
/// either direction, reaches the field and then Send, one order in the
/// tree as the snackbar keeps (review F3 on #296).
#[test]
fn the_composer_mirrors_right_to_left() {
    use slint::platform::{Key, WindowEvent};
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    for rtl in [false, true] {
        let view = opened(&model, rtl);
        let send = left_of(&view, text(UiText::Send));
        let field = left_of(&view, text(UiText::Composer));
        assert_eq!(send < field, rtl, "rtl={rtl}: send {send}, field {field}");
        let tab = |view: &View| {
            let key: slint::SharedString = Key::Tab.into();
            let window = view.window().window();
            window.dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
            window.dispatch_event(WindowEvent::KeyReleased { text: key });
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(1));
            view.window().get_focus_id().to_string()
        };
        let mut reached = false;
        for _ in 0..40 {
            if tab(&view) == "composer" {
                assert_eq!(tab(&view), "send", "rtl={rtl}: the field, then Send");
                reached = true;
                break;
            }
        }
        assert!(reached, "rtl={rtl}: Tab reaches the field");
    }
}

/// Review F1 on #296: a change of the hour setting while the window is
/// shown wakes the root, so its next render redraws every time; the same
/// setting again wakes nothing.
#[test]
fn a_changed_hour_setting_wakes_the_root() {
    use interweave_human_ui_slint::TouchSettings;
    let view = view();
    let woken = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = std::rc::Rc::clone(&woken);
    view.set_wake(move || count.set(count.get() + 1));
    view.set_touch_settings(TouchSettings::default());
    assert_eq!(woken.get(), 0, "the same setting: nothing to redraw");
    view.set_touch_settings(TouchSettings {
        hour24: false,
        ..TouchSettings::default()
    });
    assert_eq!(woken.get(), 1, "a change wakes the root");
}

/// Review F2, N1 and N2 on #296, swipe spec section 5: when a message
/// leaves, its space closes over the leave rather than in one frame, and
/// the rows below only ever rise. Exactly one separator is drawn for the
/// leaving message's day throughout: counted from the rows as drawn, the
/// silent one included. Followed frame by frame on the message below, for
/// a message heading a run of its day (the next one within a minute, and a
/// minute or more later) and for a day's only message.
#[test]
fn a_leaving_message_closes_its_space_without_a_jump() {
    // Two days before NOW, a Saturday.
    let thursday = placeholder_en::text(UiText::Thursday);
    for (next_after, label) in [
        (20_000, "run head, a run"),
        (MINUTE, "run head, a new run"),
        (DAY, "only of its day"),
    ] {
        let mut model = UiModel::new();
        model.received(received(1, "leaving", NOW - 2 * DAY));
        model.received(received(2, "below", NOW - 2 * DAY + next_after));
        model.received(received(3, "last", NOW - 2 * DAY + next_after + 10_000));
        for row in 1..=3 {
            model.read(RowId::from_stored(row));
        }
        let mut view = opened(&model, false);
        let y = |view: &View| labelled(view, "last")[0].absolute_position().y;
        let drawn = |view: &View| {
            view.window()
                .get_messages()
                .iter()
                .filter(|row| row.day == thursday)
                .count()
        };
        let before = y(&view);
        assert_eq!(drawn(&view), 1, "{label}: control, one separator");
        let leaving = model.messages(&direct())[0].key;
        model.hide(leaving);
        view.render(&model);
        let mut seen = vec![before];
        // Where another message of the day follows, the day keeps exactly
        // one separator a screen reader reads, throughout.
        let read = |view: &View| labelled(view, thursday).len();
        let followed = next_after < DAY;
        for _ in 0..30 {
            i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
            assert!(drawn(&view) <= 1, "{label}: one separator for the day");
            if followed {
                assert_eq!(read(&view), 1, "{label}: the day's separator is read");
            }
            seen.push(y(&view));
        }
        view.render(&model);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        assert!(drawn(&view) <= 1, "{label}: one separator once it is gone");
        seen.push(y(&view));
        let total = before - seen.last().copied().unwrap_or(before);
        assert!(total > 10.0, "{label}: the space closed: {seen:?}");
        for step in seen.windows(2) {
            assert!(
                step[1] <= step[0] + 0.5,
                "{label}: the rows below only rise: {seen:?}"
            );
            assert!(
                step[0] - step[1] <= total / 2.0,
                "{label}: no jump: {seen:?}"
            );
        }
    }
}

/// Re-review on #296: two neighbouring messages of one day leaving close
/// together -- the second hidden while the first is still leaving -- hand
/// the day's separator on as one: it is drawn and read exactly once
/// throughout, and the rows below only rise.
#[test]
fn two_leaving_messages_hand_their_days_separator_on_as_one() {
    let thursday = placeholder_en::text(UiText::Thursday);
    let mut model = UiModel::new();
    let start = NOW - 2 * DAY;
    for (row, at) in [
        (1, start),
        (2, start + 20_000),
        (3, start + 40_000),
        (4, start + 50_000),
    ] {
        let text = ["first", "second", "below", "last"][usize::try_from(row - 1).expect("small")];
        model.received(received(row, text, at));
        model.read(RowId::from_stored(row));
    }
    let mut view = opened(&model, false);
    let monotonic = stepped(&mut view);
    let y = |view: &View| labelled(view, "last")[0].absolute_position().y;
    let drawn = |view: &View| {
        view.window()
            .get_messages()
            .iter()
            .filter(|row| row.day == thursday)
            .count()
    };
    let before = y(&view);
    let keys: Vec<_> = model.messages(&direct()).iter().map(|m| m.key).collect();
    model.hide(keys[0]);
    view.render(&model);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
    // The second goes 60 ms later, while the first is still leaving (a
    // leave is 170 ms of the view's monotonic clock, stepped here: real
    // sleeps under load outlasted the leave, and the two never overlapped):
    // the two leaves end 60 ms apart, and the renders below, every 5 ms,
    // fall between the two ends.
    step(&monotonic, 60);
    model.hide(keys[1]);
    view.render(&model);
    // Rendering through both leaves in real time, the day keeps exactly one
    // separator, drawn and read, at every render: the first, which hands the
    // day over, stays until the second has gone.
    let started = std::time::Instant::now();
    while view.window().get_messages().iter().any(|row| row.leaving)
        && started.elapsed() < std::time::Duration::from_secs(2)
    {
        view.render(&model);
        assert_eq!(
            drawn(&view),
            1,
            "one separator at every render of the leave"
        );
        assert_eq!(labelled(&view, thursday).len(), 1, "and it is read");
        step(&monotonic, 5);
    }
    let mut seen = vec![before];
    for frame in 0..30 {
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        if frame % 12 == 11 {
            view.render(&model);
        }
        assert_eq!(drawn(&view), 1, "frame {frame}: one separator drawn");
        assert_eq!(
            labelled(&view, thursday).len(),
            1,
            "frame {frame}: and read"
        );
        seen.push(y(&view));
    }
    view.render(&model);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
    seen.push(y(&view));
    assert_eq!(
        labelled(&view, thursday).len(),
        1,
        "the separator ends with the row below"
    );
    for step in seen.windows(2) {
        assert!(
            step[1] <= step[0] + 0.5,
            "the rows below only rise: {seen:?}"
        );
    }
}

/// Re-review 1d50f8fd on #296: a leaving run that spans a day change hands
/// over the separator of its last day -- the one the row after the run
/// takes -- and it is read at every render of the leave.
#[test]
fn a_leaving_run_across_a_day_hands_over_its_last_days_separator() {
    let thursday = placeholder_en::text(UiText::Thursday);
    let mut model = UiModel::new();
    // Wednesday's last message, Thursday's first, then Thursday's next.
    let thursday_at = NOW - 2 * DAY;
    for (row, at) in [
        (1, thursday_at - DAY),
        (2, thursday_at),
        (3, thursday_at + 20_000),
    ] {
        let text = ["wednesday", "thursday", "below"][usize::try_from(row - 1).expect("small")];
        model.received(received(row, text, at));
        model.read(RowId::from_stored(row));
    }
    let mut view = opened(&model, false);
    let monotonic = stepped(&mut view);
    let keys: Vec<_> = model.messages(&direct()).iter().map(|m| m.key).collect();
    model.hide(keys[0]);
    view.render(&model);
    step(&monotonic, 60);
    model.hide(keys[1]);
    view.render(&model);
    let started = std::time::Instant::now();
    while view.window().get_messages().iter().any(|row| row.leaving)
        && started.elapsed() < std::time::Duration::from_secs(2)
    {
        view.render(&model);
        assert_eq!(
            labelled(&view, thursday).len(),
            1,
            "Thursday's separator is read"
        );
        step(&monotonic, 5);
    }
    assert_eq!(
        labelled(&view, thursday).len(),
        1,
        "and ends on the row below"
    );
}

/// Section 6 and #304 F3: the composer is one line high whatever the free
/// space under the messages. In a tall window holding one message it had
/// swollen to fill the space the list left.
#[test]
fn the_composer_keeps_its_height_whatever_the_free_space() {
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    let view = opened(&model, false);
    let field = labelled(&view, text(UiText::Composer));
    assert_eq!(field.len(), 1, "the field");
    let height = field[0].size().height;
    assert!(
        height <= 56.0,
        "one line inside the 48 dp box, not the free space: {height}"
    );
    // Centred on Send's line, which is the row's 48.
    let send = labelled(&view, text(UiText::Send));
    let middle = |e: &ElementHandle| e.absolute_position().y + e.size().height / 2.0;
    assert!(
        (middle(&field[0]) - middle(&send[0])).abs() <= 1.0,
        "the field sits on Send's line"
    );
}

/// #304 F9: a landscape phone with the keyboard up. The list beside the
/// conversation, the network notice and the app bar step aside in that
/// order, as far as the space above the keyboard needs, leaving the
/// composer whole above it and the newest message in view; with the
/// keyboard closed every part is back. The keyboard is stood in for by
/// the height it covers, which the testing backend cannot raise.
#[test]
fn a_short_space_above_the_keyboard_keeps_the_composer_and_the_newest_message() {
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    // A conversation longer than any space above the keyboard, so being
    // anchored is something the list does, not where it starts (#304 §7:
    // the phone showed the top, the day line cut).
    for row in 1..20 {
        model.received(received(row, &format!("earlier {row}"), NOW));
    }
    model.received(received(20, "the newest", NOW));
    let view = opened(&model, false);
    let window = view.window();
    window
        .window()
        .set_size(slint::LogicalSize::new(915.0, 411.0));
    slint::platform::update_timers_and_animations();
    let field_left = |view: &View| {
        labelled(view, text(UiText::Composer))[0]
            .absolute_position()
            .x
    };
    let bar = |view: &View| labelled(view, text(UiText::MoreOptions)).len();
    assert!(field_left(&view) > 281.0, "control: the list beside it");
    assert_eq!(bar(&view), 1, "control: the app bar");
    assert!(window.get_has_notice(), "control: a network notice");

    for (covered, list, notice, app_bar) in [
        // The keyboard closed: the conversation opens at its end.
        (0.0, true, true, true),
        // H = 300: the list and the notice step aside, the bar stays.
        (111.0, false, false, true),
        // H = 191: the app bar too.
        (220.0, false, false, false),
        // Closed: everything is back.
        (0.0, true, true, true),
    ] {
        window.set_keyboard_overlap(covered);
        slint::platform::update_timers_and_animations();
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_secs(1));
        assert_eq!(
            field_left(&view) > 281.0,
            list,
            "covered {covered}: the list"
        );
        assert_eq!(
            !window.get_notice_aside(),
            notice,
            "covered {covered}: the notice"
        );
        assert_eq!(bar(&view) == 1, app_bar, "covered {covered}: the app bar");
        let field = &labelled(&view, text(UiText::Composer))[0];
        let above = 411.0 - covered;
        assert!(
            field.absolute_position().y + field.size().height <= above,
            "covered {covered}: the composer above the keyboard"
        );
        let newest: Vec<_> = all(&view)
            .into_iter()
            .filter(|e| {
                e.accessible_label()
                    .is_some_and(|l| l.contains("the newest"))
            })
            .collect();
        assert!(
            !newest.is_empty(),
            "covered {covered}: the newest message is drawn"
        );
        // Anchored at its newest message: its end is in view above the
        // keyboard, however little of it the space can hold (step 5).
        assert!(
            newest.iter().any(|e| {
                let end = e.absolute_position().y + e.size().height;
                end > 0.0 && end <= above + 0.5
            }),
            "covered {covered}: the newest message in view"
        );
    }
}

/// The spec's S3: a conversation opens at its end and stays there as
/// messages arrive, until the person scrolls away from it. Scrolled up,
/// a new message does not pull the list back down.
#[test]
fn the_list_follows_its_end_until_the_person_scrolls_away() {
    use slint::platform::WindowEvent;
    let settle = || {
        slint::platform::update_timers_and_animations();
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_secs(1));
        slint::platform::update_timers_and_animations();
    };
    let drawn = |view: &View, text: &str| {
        all(view)
            .into_iter()
            .any(|e| e.accessible_label().is_some_and(|l| l.contains(text)))
    };
    let mut model = UiModel::new();
    for row in 1..20 {
        model.received(received(row, &format!("earlier {row}"), NOW));
    }
    let mut view = opened(&model, false);
    view.window()
        .window()
        .set_size(slint::LogicalSize::new(390.0, 600.0));
    settle();
    assert!(drawn(&view, "earlier 19"), "it opens at its end");

    model.received(received(20, "arrived at the end", NOW));
    view.render(&model);
    settle();
    assert!(drawn(&view, "arrived at the end"), "and follows it");

    view.window()
        .window()
        .dispatch_event(WindowEvent::PointerScrolled {
            position: slint::LogicalPosition::new(195.0, 300.0),
            delta_x: 0.0,
            delta_y: 400.0,
        });
    settle();
    assert!(
        !drawn(&view, "arrived at the end"),
        "control: the scroll moved the list off its end"
    );
    model.received(received(21, "arrived while away", NOW));
    view.render(&model);
    settle();
    assert!(
        !drawn(&view, "arrived while away"),
        "scrolled away, a new message leaves the list where the person put it"
    );
}

/// Closing the keyboard keeps the composer focused, as a platform field
/// does: a letter typed after it closes still reaches the draft (#304
/// F16; F3's residual handle is the toolkit's).
#[test]
fn closing_the_keyboard_after_a_tap_keeps_the_composer() {
    use slint::platform::{PointerEventButton, WindowEvent};
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    let view = opened(&model, false);
    let window = view.window();
    let field = labelled(&view, text(UiText::Composer));
    let (p, z) = (field[0].absolute_position(), field[0].size());
    let position = slint::LogicalPosition::new(p.x + z.width / 2.0, p.y + z.height / 2.0);
    window.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    window
        .window()
        .dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
    let typed = |letter: &str| {
        let key: slint::SharedString = letter.into();
        window
            .window()
            .dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
        window
            .window()
            .dispatch_event(WindowEvent::KeyReleased { text: key });
        slint::platform::update_timers_and_animations();
    };
    window.set_keyboard_overlap(300.0);
    slint::platform::update_timers_and_animations();
    typed("a");
    assert_eq!(
        window.get_draft().as_str(),
        "a",
        "control: the tapped field types"
    );
    window.set_keyboard_overlap(0.0);
    slint::platform::update_timers_and_animations();
    typed("b");
    assert_eq!(window.get_draft().as_str(), "ab", "the field kept focus");
}

/// #304 F16: the field is its whole rounded box, as a Material text
/// field is. A tap on the box above or below the one line of text --
/// most of the box's 48 px -- focuses the field, as a tap on the line
/// does.
#[test]
fn a_tap_anywhere_on_the_composer_box_focuses_the_field() {
    use slint::platform::{PointerEventButton, WindowEvent};
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    let view = opened(&model, false);
    let window = view.window();
    let field = labelled(&view, text(UiText::Composer));
    let (p, z) = (field[0].absolute_position(), field[0].size());
    assert!(z.height < 40.0, "control: the line is shorter than the box");
    // Inside the box, above the line of text.
    let position = slint::LogicalPosition::new(p.x + z.width / 2.0, p.y - 8.0);
    window.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    window
        .window()
        .dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
    slint::platform::update_timers_and_animations();
    let key: slint::SharedString = "a".into();
    window
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    window
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text: key });
    slint::platform::update_timers_and_animations();
    assert_eq!(window.get_draft().as_str(), "a", "the tapped box types");
}

/// #304 F16: a conversation holding the person's own messages that need
/// attention ("May have reached the peer, send again or cancel") still
/// gives its composer focus on a tap, as one holding none does.
#[test]
fn a_tap_focuses_the_composer_beside_messages_needing_attention() {
    use interweave_human_client_api::{ClientEvent, OutboundStatus, OutboundUpdate, SendProblem};
    use interweave_human_core::AppMessageId;
    use slint::platform::{PointerEventButton, WindowEvent};
    let text = placeholder_en::text;
    let mut model = UiModel::new();
    model.received(received(1, "hi", NOW));
    for row in [2, 3] {
        send(&mut model, row, "w", NOW);
        model.client_event(ClientEvent::Outbound(OutboundUpdate {
            row: RowId::from_stored(row),
            app_message_id: AppMessageId::parse(format!("{row:032x}")).expect("an id"),
            status: OutboundStatus::NeedsAttention {
                problem: SendProblem::NoNetworkPath,
                may_have_reached: true,
            },
            last_code: None,
        }));
    }
    let view = opened(&model, false);
    let window = view.window();
    assert!(
        window
            .get_messages()
            .iter()
            .filter(|r| r.status.starts_with("May have reached the peer"))
            .count()
            == 2,
        "control: two messages need attention"
    );
    let field = labelled(&view, text(UiText::Composer));
    let (p, z) = (field[0].absolute_position(), field[0].size());
    let position = slint::LogicalPosition::new(p.x + z.width / 2.0, p.y + z.height / 2.0);
    window.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    window
        .window()
        .dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
    slint::platform::update_timers_and_animations();
    window.set_keyboard_overlap(300.0);
    slint::platform::update_timers_and_animations();
    let key: slint::SharedString = "a".into();
    window
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
    window
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text: key });
    slint::platform::update_timers_and_animations();
    assert_eq!(window.get_draft().as_str(), "a", "the tapped field types");
}

/// #304 F9 step 5: with the keyboard up in landscape, a conversation
/// longer than the space left shows its newest message, not its top.
#[test]
fn with_the_keyboard_up_a_long_conversation_shows_its_newest_message() {
    let mut model = UiModel::new();
    for n in 1..=14_u64 {
        let row = i64::try_from(n).expect("small");
        model.received(received(
            row,
            &format!("message number {n}"),
            NOW - 60_000 * (20 - n),
        ));
    }
    let view = opened(&model, false);
    let window = view.window();
    window
        .window()
        .set_size(slint::LogicalSize::new(915.0, 411.0));
    window.set_keyboard_overlap(200.0);
    // The list settles at its newest message within a few turns: its
    // content's height lags the change by one.
    for _ in 0..4 {
        slint::platform::update_timers_and_animations();
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(5));
    }
    let above = 411.0 - 200.0;
    // In view: its end is above the keyboard and below the list's top.
    let shown = |label: &str| {
        all(&view).into_iter().any(|e| {
            e.accessible_label().is_some_and(|l| l.contains(label)) && {
                let end = e.absolute_position().y + e.size().height;
                end > 0.0 && end <= above + 0.5
            }
        })
    };
    assert!(shown("message number 14"), "the newest message in view");
    assert!(!shown("message number 1."), "control: not the top");
}

/// #304 round 2 (ux-designer-01, REPLY 01a153b5): a long status, as a
/// pending message that may have reached the peer has, wraps inside the
/// bubble, and its time stays whole and inside it, on a phone-wide
/// window.
#[test]
fn a_long_status_wraps_and_its_time_stays_whole_inside_the_bubble() {
    use interweave_human_client_api::{ClientEvent, OutboundStatus, OutboundUpdate, SendProblem};
    use interweave_human_core::AppMessageId;
    let mut model = UiModel::new();
    send(&mut model, 1, "w1", NOW);
    model.client_event(ClientEvent::Outbound(OutboundUpdate {
        row: RowId::from_stored(1),
        app_message_id: AppMessageId::parse(format!("{:032x}", 1)).expect("an id"),
        status: OutboundStatus::NeedsAttention {
            problem: SendProblem::NoNetworkPath,
            may_have_reached: true,
        },
        last_code: None,
    }));
    let view = opened(&model, false);
    let row = view.window().get_messages().row_data(0).expect("a row");
    assert_eq!(
        row.status.as_str(),
        placeholder_en::label(
            interweave_human_ui_model::LabelKey::NeedsAttentionMayHaveBeenReceived
        ),
        "control: the long status"
    );
    let time = labelled(&view, row.time.as_str());
    assert_eq!(time.len(), 1, "the time is drawn once");
    let (at, size) = (time[0].absolute_position(), time[0].size());
    let status = labelled(&view, row.status.as_str());
    let words = status
        .iter()
        .map(|s| s.size().width)
        .fold(0.0_f32, f32::max);
    assert!(
        at.x + size.width <= 390.0 - 12.0,
        "the time inside the bubble: {at:?} {size:?}"
    );
    assert!(size.width >= 20.0, "the time is whole: {size:?}");
    assert!(
        words <= 390.0 * 0.8,
        "the status wraps inside the bubble: {words}"
    );
}

/// #304 F9 step 5, on the phone's own figures (ux-designer-01, round 3:
/// the Pixel in landscape, keyboard up): the strip left to the messages
/// ends at the composer, and the newest message's end, its status and
/// time, are in it, and a one-line message whole. An empty refused-send
/// line kept 25 px between them, most of that strip; the pane's top
/// padding and the heading's row took 20 px more above it.
#[test]
fn with_the_keyboard_up_in_landscape_the_newest_message_ends_at_the_composer() {
    let mut model = UiModel::new();
    model.received(received(1, "message number 1", NOW - 60_000));
    let view = opened(&model, false);
    let window = view.window();
    window
        .window()
        .set_size(slint::LogicalSize::new(923.0, 411.0));
    window.set_keyboard_overlap(270.0);
    for _ in 0..6 {
        slint::platform::update_timers_and_animations();
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(5));
    }
    let found = all(&view);
    let top_end = |e: &ElementHandle| {
        let y = e.absolute_position().y;
        (y, y + e.size().height)
    };
    let (list_top, _) = top_end(
        found
            .iter()
            .find(|e| e.type_name().as_deref() == Some("Flickable"))
            .expect("the message list"),
    );
    let (composer_top, _) = top_end(
        found
            .iter()
            .find(|e| e.id().as_deref() == Some("AppWindow::composer-box"))
            .expect("the composer"),
    );
    let (message_top, message_end) = top_end(
        found
            .iter()
            .find(|e| {
                e.accessible_label()
                    .is_some_and(|l| l.contains("message number 1"))
            })
            .expect("the message"),
    );
    // The strip is the pane's whole height above the composer: with the
    // bar aside, no top padding and no row for the heading, which took
    // 20 px of it on the phone (round 3).
    assert!(
        list_top <= 0.5,
        "the list begins at the pane's top: {list_top}"
    );
    // A one-line message fits whole in what is left, its text as well as
    // its status and time.
    assert!(
        message_top >= list_top,
        "the whole message in view: {message_top}..{message_end} from {list_top}"
    );
    // The list's padding (5 px) and the pane's spacing (8 px).
    assert!(
        composer_top - message_end <= 13.5,
        "the message ends at the composer: {message_end} against {composer_top}"
    );
    let time = found
        .iter()
        .find(|e| {
            e.accessible_label()
                .is_some_and(|l| l.len() == 5 && l.as_bytes()[2] == b':')
        })
        .expect("the message's time");
    let (time_top, time_end) = top_end(time);
    assert!(
        time_top >= list_top && time_end <= composer_top,
        "its time in view: {time_top}..{time_end} in {list_top}..{composer_top}"
    );
}

/// Android review F16 on a phone-wide window: an untrusted send's reason
/// sits inside the bubble, above its status, and the time stays whole on
/// the status's line, the bubble's last.
#[test]
fn a_needs_attention_reason_sits_above_the_status_inside_the_bubble() {
    use interweave_human_client_api::{ClientEvent, OutboundStatus, OutboundUpdate, SendProblem};
    use interweave_human_core::AppMessageId;
    use interweave_human_ui_model::ErrorClass;
    let mut model = UiModel::new();
    send(&mut model, 1, "w1", NOW);
    model.client_event(ClientEvent::Outbound(OutboundUpdate {
        row: RowId::from_stored(1),
        app_message_id: AppMessageId::parse(format!("{:032x}", 1)).expect("an id"),
        status: OutboundStatus::NeedsAttention {
            problem: SendProblem::PeerUntrusted,
            may_have_reached: false,
        },
        last_code: None,
    }));
    let view = opened(&model, false);
    let row = view.window().get_messages().row_data(0).expect("a row");
    let reason = placeholder_en::error(RuntimeHost::Embedded, ErrorClass::PeerNotTrusted);
    let one = |label: &str| {
        let found = labelled(&view, label);
        assert_eq!(found.len(), 1, "one element labelled {label:?}");
        (found[0].absolute_position(), found[0].size())
    };
    let (why, why_size) = one(reason);
    let (status, _) = one(row.status.as_str());
    let (time, time_size) = one(row.time.as_str());
    assert!(
        why.y + why_size.height <= status.y + 0.5,
        "the reason above the status: {why:?} {why_size:?} {status:?}"
    );
    assert!(
        why.x + why_size.width <= 390.0 - 12.0,
        "the reason inside the bubble: {why:?} {why_size:?}"
    );
    assert!(
        time.x + time_size.width <= 390.0 - 12.0 && time_size.width >= 20.0,
        "the time whole and inside the bubble: {time:?} {time_size:?}"
    );
}

/// A row with no reason draws no reason line, not an empty one: an empty
/// Slint Text keeps its height and the layout's spacing. Measured as the
/// gap below the body, to the status on an ordinary row and to the
/// reason on a row that has one: the same gap, so nothing sits between.
#[test]
fn a_row_with_no_reason_draws_no_empty_reason_line() {
    use interweave_human_client_api::{ClientEvent, OutboundStatus, OutboundUpdate, SendProblem};
    use interweave_human_core::AppMessageId;
    use interweave_human_ui_model::{ErrorClass, LabelKey};
    let mut model = UiModel::new();
    send(&mut model, 1, "plain", NOW);
    send(&mut model, 2, "refused", NOW);
    let update = |row: i64, status| OutboundUpdate {
        row: RowId::from_stored(row),
        app_message_id: AppMessageId::parse(format!("{row:032x}")).expect("an id"),
        status,
        last_code: None,
    };
    model.client_event(ClientEvent::Outbound(update(
        1,
        OutboundStatus::Cancelled {
            may_have_reached: false,
        },
    )));
    model.client_event(ClientEvent::Outbound(update(
        2,
        OutboundStatus::NeedsAttention {
            problem: SendProblem::PeerUntrusted,
            may_have_reached: false,
        },
    )));
    let view = opened(&model, false);
    let one = |label: &str| {
        let found = labelled(&view, label);
        assert_eq!(found.len(), 1, "one element labelled {label:?}");
        (found[0].absolute_position().y, found[0].size().height)
    };
    let (body, body_height) = one("plain");
    let (status, _) = one(placeholder_en::label(LabelKey::Cancelled));
    let (reason, reason_height) = one(placeholder_en::error(
        RuntimeHost::Embedded,
        ErrorClass::PeerNotTrusted,
    ));
    let (refused_body, refused_height) = one("refused");
    let (refused_status, _) = one(placeholder_en::label(LabelKey::NeedsAttention));
    assert!(
        reason + reason_height <= refused_status + 0.5,
        "control: the reason line is drawn"
    );
    let plain_gap = status - (body + body_height);
    let reason_gap = reason - (refused_body + refused_height);
    assert!(
        (plain_gap - reason_gap).abs() < 0.5,
        "no line between body and status: {plain_gap} against {reason_gap}"
    );
}

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
    ConversationKey, Intent, RuntimeHost, UiModel, UiText, ViewEvent, fill, placeholder_en,
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

/// Section 11 item 2: the list's app bar holds its title and Trust
/// settings as an icon button, and there is no other Trust settings
/// control.
#[test]
fn the_list_bar_holds_the_title_and_trust_settings_as_its_one_button() {
    let mut view = view();
    view.render(&UiModel::new());
    let title = labelled(&view, placeholder_en::text(UiText::Conversations));
    assert_eq!(title.len(), 1, "the title, once");
    let trust = labelled(&view, placeholder_en::text(UiText::TrustSettings));
    assert_eq!(trust.len(), 1, "one Trust settings control");
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
    let id = labelled(&view, text(UiText::ConversationId));
    assert_eq!(id.len(), 1);
    assert_eq!(id[0].accessible_value().as_deref(), Some(ALICE));
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
    let author = labelled(&view, &short);
    assert_eq!(author.len(), 1, "the author line");
    assert_eq!(author[0].accessible_description().as_deref(), Some(ALICE));

    let view = opened(&model, false);
    assert!(labelled(&view, &short).is_empty(), "no author line");
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
    let sending = by_status(LabelKey::Sending);
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

/// Section 8, a very long message: the bubble wraps and holds every line
/// of it, its status included, and the chips under it keep their 48 px.
/// A stretched chip row once took the room the wrapped text needed, so
/// the bubble's last line and its status ran past the fill, and its
/// Show source chip grew to 86 px (seen in the rendered window).
#[test]
fn a_wrapped_bubble_holds_its_status_above_its_chips() {
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
    let (chip_y, chip_h, _) = one(placeholder_en::text(UiText::ShowSource));
    assert!(text_h > 2.0 * status_h, "the text wrapped: {text_h}");
    assert!(text_w <= 390.0 * 0.8, "inside 80 % of the width: {text_w}");
    assert!(status_y >= text_y + text_h, "the status under the text");
    assert!(chip_y >= status_y + status_h, "the chips under the bubble");
    assert!(
        (chip_h - 48.0).abs() < 0.5,
        "the chip keeps its 48 px: {chip_h}"
    );
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

/// Review F3 on #295 and its re-review: Details is a modal dialog. Tab
/// and Shift+Tab stay on its identifier field -- never on the dialog
/// itself, which draws no ring, nor on the controls behind the scrim --
/// and Escape closes it, giving focus back to More options.
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
        "header-id",
        "in the field"
    );
    for key in [Key::Backtab, Key::Tab, Key::Tab, Key::Backtab] {
        send(key);
        assert_eq!(
            view.window().get_focus_id().as_str(),
            "header-id",
            "still in the field"
        );
        assert_eq!(rings(), 1, "its ring shown");
        assert!(view.window().get_details_open());
    }
    // A press on the dialog's padding, beside the field, leaves focus there.
    let field = labelled(&view, text(UiText::ConversationId));
    assert_eq!(field.len(), 1);
    let (at, window) = (field[0].absolute_position(), view.window().window());
    let position = slint::LogicalPosition::new(at.x - 12.0, at.y + 4.0);
    let button = slint::platform::PointerEventButton::Left;
    window.dispatch_event(WindowEvent::PointerPressed { position, button });
    window.dispatch_event(WindowEvent::PointerReleased { position, button });
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(10));
    assert_eq!(
        view.window().get_focus_id().as_str(),
        "header-id",
        "a press beside it"
    );
    assert_eq!(rings(), 1, "the field keeps focus and its ring");
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

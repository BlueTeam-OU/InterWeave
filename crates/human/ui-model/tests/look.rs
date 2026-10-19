// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the conversation list shows beside a name
//! (design/2026-10-16-android-look.md sections 5.1, 5.2 and 11 items 4-6),
//! through the model's public surface.

#![allow(clippy::expect_used, clippy::panic)]

use interweave_human_chat_protocol::{HumanChatV2, MessageKind};
use interweave_human_client_api::{
    ClientEvent, Destination, Origin, OutboundStatus, OutboundUpdate, SendProblem,
};
use interweave_human_core::{AppMessageId, RowId};
use interweave_human_ui_model::{
    Avatar, AvatarHue, ConversationKey, Day, Direction, Intent, LabelKey, ListedInbound, LocalTime,
    UiModel, Weekday, local_time,
};
use interweave_transport_api::{ChannelId, EndpointId, TransportIdentity};

// Three canonical PeerIds, one for each pair. Their byte sums modulo 3
// were computed apart from this crate (2, 0 and 1), so the test checks
// the rule rather than the code's own reading of it.
const NAVY_ID: &str = "12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e";
const TEAL_ID: &str = "12D3KooW9xCm2jWjNVrwh51SWCQBMYdMyeU3NpT85QhLVkF6PcNM";
const AMBER_ID: &str = "12D3KooWA284B2yjxoAAqAFwwVj6eRQ8DogF3t8wdpMzZ8Hh8wh4";

fn id(text: &str) -> TransportIdentity {
    TransportIdentity::parse(text).expect("canonical PeerId")
}

fn endpoint(name: &str) -> EndpointId {
    EndpointId::parse(name).expect("valid")
}

fn direct(peer: &TransportIdentity, route: Option<&str>) -> ConversationKey {
    ConversationKey::Direct {
        peer: peer.clone(),
        endpoint: route.map(endpoint),
    }
}

fn envelope(n: u32, text: &str, sent_at_ms: Option<u64>) -> HumanChatV2 {
    HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: format!("{n:032x}"),
        text: text.to_owned(),
        reply_to: None,
        sent_at_ms,
        from_endpoint: None,
    }
}

fn listed(row: i64, from: &TransportIdentity, envelope: HumanChatV2, at: u64) -> ListedInbound {
    ListedInbound {
        row: RowId::from_stored(row),
        origin: Origin::Direct {
            peer: from.clone(),
            endpoint: endpoint("human"),
        },
        envelope,
        received_at: at,
    }
}

#[test]
fn an_avatar_is_the_ids_last_two_characters_in_the_pair_its_byte_sum_picks() {
    for (text, letters, hue) in [
        (TEAL_ID, "NM", AvatarHue::Teal),
        (AMBER_ID, "h4", AvatarHue::Amber),
        (NAVY_ID, "3e", AvatarHue::Navy),
    ] {
        let peer = id(text);
        let expected = Avatar {
            letters: letters.to_owned(),
            hue,
        };
        // The endpoint is not part of the PeerId: the same id has the same
        // avatar whatever route it is reached by.
        assert_eq!(
            Avatar::of(&direct(&peer, Some("human"))),
            expected,
            "{text}"
        );
        assert_eq!(Avatar::of(&direct(&peer, None)), expected, "{text}");
    }
}

#[test]
fn a_channels_avatar_is_a_hash_on_the_navy_pair() {
    let channel = ConversationKey::Channel(ChannelId::parse("general").expect("channel"));
    assert_eq!(
        Avatar::of(&channel),
        Avatar {
            letters: "#".to_owned(),
            hue: AvatarHue::Navy,
        }
    );
}

#[test]
fn the_list_carries_each_conversations_avatar() {
    let peer = id(AMBER_ID);
    let mut model = UiModel::new();
    model.unread_listed(vec![listed(1, &peer, envelope(1, "hi", None), 10)]);
    let list = model.conversations();
    assert_eq!(list[0].avatar, Avatar::of(&direct(&peer, Some("human"))));
}

const MINUTE: u64 = 60_000;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
// 2026-10-17 (a Saturday), 14:30 UTC, in Unix milliseconds.
const NOW: u64 = 20_743 * DAY + 14 * HOUR + 30 * MINUTE;

fn utc(_: u64) -> i32 {
    0
}

#[test]
fn a_time_today_is_its_clock_time() {
    assert_eq!(
        local_time(NOW - 13 * HOUR - 12 * MINUTE, NOW, &utc),
        LocalTime {
            day: Day::Today,
            hour: 1,
            minute: 18,
        }
    );
}

#[test]
fn yesterday_this_week_and_earlier_each_have_their_day() {
    assert_eq!(local_time(NOW - DAY, NOW, &utc).day, Day::Yesterday);
    // Thursday, two days back: the first day named by its weekday.
    assert_eq!(
        local_time(NOW - 2 * DAY, NOW, &utc).day,
        Day::Weekday(Weekday::Thursday)
    );
    // Sunday, six days back: the last.
    assert_eq!(
        local_time(NOW - 6 * DAY, NOW, &utc).day,
        Day::Weekday(Weekday::Sunday)
    );
    assert_eq!(
        local_time(NOW - 7 * DAY, NOW, &utc).day,
        Day::Date {
            year: 2026,
            month: 10,
            day: 10,
            this_year: true,
        }
    );
    assert_eq!(
        local_time(NOW - 300 * DAY, NOW, &utc).day,
        Day::Date {
            year: 2025,
            month: 12,
            day: 21,
            this_year: false,
        }
    );
}

#[test]
fn a_time_after_now_is_today_never_a_day_to_come() {
    assert_eq!(local_time(NOW + 3 * DAY, NOW, &utc).day, Day::Today);
}

#[test]
fn the_day_is_the_local_zones_asked_at_each_instant() {
    // 23:30 UTC yesterday is 01:30 today two hours east: today, not
    // yesterday.
    let at = NOW - 15 * HOUR;
    assert_eq!(local_time(at, NOW, &utc).day, Day::Yesterday);
    let east = |_: u64| 2 * 3600;
    assert_eq!(
        local_time(at, NOW, &east),
        LocalTime {
            day: Day::Today,
            hour: 1,
            minute: 30,
        }
    );
    // An offset that changed between then and now (daylight saving): each
    // instant is read in its own.
    let changed = |ms: u64| if ms < NOW - HOUR { 3600 } else { 7200 };
    assert_eq!(local_time(at, NOW, &changed).hour, 0);
}

#[test]
fn the_last_message_is_the_latest_held_received_or_sent_by_local_time() {
    let peer = id(TEAL_ID);
    let mut model = UiModel::new();
    // The sender says it sent the second message long before the first:
    // the list goes by when each reached this app.
    model.unread_listed(vec![
        listed(
            1,
            &peer,
            envelope(1, "first", Some(NOW + 9 * DAY)),
            NOW - HOUR,
        ),
        listed(2, &peer, envelope(2, "second", Some(1)), NOW - MINUTE),
    ]);
    let last = model.conversations()[0].last.clone();
    assert_eq!(last.local_at, NOW - MINUTE);
    assert_eq!(last.direction, Direction::Inbound);
    assert_eq!(last.label, LabelKey::Unread);
    assert_eq!(
        local_time(last.local_at, NOW, &utc),
        LocalTime {
            day: Day::Today,
            hour: 14,
            minute: 29,
        }
    );

    model.sent(
        RowId::from_stored(3),
        &Destination::Direct {
            peer: peer.clone(),
            endpoint: Some(endpoint("human")),
        },
        envelope(3, "mine", None),
        NOW,
    );
    let last = model.conversations()[0].last.clone();
    assert_eq!(last.direction, Direction::Outbound);
    assert_eq!(last.local_at, NOW);
}

#[test]
fn previewing_the_list_reads_nothing() {
    let peer = id(TEAL_ID);
    let mut model = UiModel::new();
    model.unread_listed(vec![
        listed(1, &peer, envelope(1, "one", None), 1),
        listed(2, &peer, envelope(2, "two", None), 2),
    ]);
    for _ in 0..3 {
        let list = model.conversations();
        assert_eq!(list[0].unread, 2);
    }
    let key = direct(&peer, Some("human"));
    for message in model.messages(&key) {
        assert_eq!(message.label, LabelKey::Unread, "still unread");
    }
    // Both rows are still the store's to mark read, and only a focused
    // view of the conversation asks for it.
    assert_eq!(
        model.conversation_viewed(&key, true),
        [
            Intent::MarkRead(RowId::from_stored(1)),
            Intent::MarkRead(RowId::from_stored(2))
        ]
    );
}

#[test]
fn only_a_send_on_its_way_is_pending() {
    let peer = id(TEAL_ID);
    let mut model = UiModel::new();
    let row = RowId::from_stored(1);
    model.sent(
        row,
        &Destination::Direct {
            peer: peer.clone(),
            endpoint: Some(endpoint("human")),
        },
        envelope(1, "mine", None),
        1,
    );
    let last = |model: &UiModel| model.conversations()[0].last.clone();
    assert_eq!(last(&model).label, LabelKey::Sending);
    assert!(last(&model).pending());
    for (status, pending) in [
        (
            OutboundStatus::Unconfirmed {
                next_retry_at: None,
                last_problem: None,
            },
            true,
        ),
        (
            OutboundStatus::NeedsAttention {
                problem: SendProblem::NoNetworkPath,
                may_have_reached: false,
            },
            false,
        ),
        (
            OutboundStatus::Accepted {
                endpoint: endpoint("human"),
            },
            false,
        ),
    ] {
        let label = format!("{status:?}");
        model.client_event(ClientEvent::Outbound(OutboundUpdate {
            row,
            app_message_id: AppMessageId::parse(format!("{:032x}", 1)).expect("id"),
            status,
            last_code: None,
        }));
        assert_eq!(last(&model).pending(), pending, "{label}");
    }
}

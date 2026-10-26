// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the conversation list shows beside a name
//! (design/2026-10-16-android-look.md section 5): the avatar, and the
//! calendar a message's time falls in. Pure: the clock and the time zone
//! come from the caller, so the same inputs give the same answer on every
//! platform.

use crate::model::ConversationKey;
use interweave_transport_api::EndpointId;

/// Which colour pair an avatar takes (section 5.1). The values are the
/// view's; this names the pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AvatarHue {
    /// The teal pair.
    Teal,
    /// The amber pair.
    Amber,
    /// The navy pair, and every channel's.
    Navy,
}

impl AvatarHue {
    /// The pair an id's canonical text picks: the sum of its UTF-8 bytes
    /// modulo 3, in section 5.1's order -- teal, amber, navy.
    fn of_text(id: &str) -> Self {
        let sum: u64 = id.bytes().map(u64::from).sum();
        match sum % 3 {
            0 => Self::Teal,
            1 => Self::Amber,
            _ => Self::Navy,
        }
    }

    /// The hue of one of this device's identities: its `EndpointId`'s,
    /// by the same rule (contact-cards design 5.1 and 5.2 field 1). So it
    /// does not follow the `PeerId` and is known before the `PeerId` is;
    /// identities of one `PeerId` usually differ, but with three pairs a
    /// fourth always repeats one.
    #[must_use]
    pub fn of_identity(endpoint: &EndpointId) -> Self {
        Self::of_text(endpoint.as_str())
    }
}

/// A conversation's avatar: decoration beside the name, never a trust
/// signal -- two peers can share one, and the name stays the identity
/// (`human-client-ui.md` section 3). A screen reader never reads it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Avatar {
    /// What the circle shows: the last two characters of a direct
    /// conversation's `PeerId`, or "#" for a channel.
    pub letters: String,
    /// Its colour pair.
    pub hue: AvatarHue,
}

impl Avatar {
    /// The avatar of `key`. For a direct conversation, the letters are
    /// the last two characters of the `PeerId`'s canonical text -- the
    /// endpoint is not part of it, so they are those the list's short
    /// form ends with -- and the pair is the sum of that text's UTF-8
    /// bytes modulo 3, in section 5.1's order: teal, amber, navy. So the
    /// same id has the same avatar on every launch and every platform
    /// (tests/look.rs pins fixed ids).
    #[must_use]
    pub fn of(key: &ConversationKey) -> Self {
        match key {
            ConversationKey::Channel(_) => Self {
                letters: "#".to_owned(),
                hue: AvatarHue::Navy,
            },
            ConversationKey::Direct { peer, .. } => {
                let id = peer.as_str();
                let chars: Vec<char> = id.chars().collect();
                let letters = chars[chars.len().saturating_sub(2)..].iter().collect();
                Self {
                    letters,
                    hue: AvatarHue::of_text(id),
                }
            }
        }
    }
}

/// A day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

/// Which day a time falls on, seen from now (section 5.2): the same
/// words name the list's time and a conversation's day separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Day {
    /// Today, or a time later than now (a clock set back): never a day
    /// that has not come.
    Today,
    /// The day before today.
    Yesterday,
    /// One of the five days before yesterday: within the last seven days,
    /// so a weekday's name never also means today or yesterday.
    Weekday(Weekday),
    /// Earlier.
    Date {
        /// The year.
        year: i32,
        /// 1 to 12.
        month: u8,
        /// 1 to 31.
        day: u8,
        /// Whether it is the current year, which a short date may leave
        /// out.
        this_year: bool,
    },
}

/// A message's time as the list shows it: the clock time for today, else
/// its day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalTime {
    /// The day.
    pub day: Day,
    /// 0 to 23, in the local zone.
    pub hour: u8,
    /// 0 to 59.
    pub minute: u8,
}

const DAY_SECS: i64 = 86_400;

/// A local time, from the device's own clock (`local_at`, Unix
/// milliseconds), never a time a sender asserts. `now_ms` is the same
/// clock now, and `offset_at` the local zone's offset from UTC in seconds
/// at a given Unix millisecond, asked separately for each, so a daylight
/// saving change between the two moves neither.
#[must_use]
pub fn local_time(at_ms: u64, now_ms: u64, offset_at: &dyn Fn(u64) -> i32) -> LocalTime {
    let local = |ms: u64| i64::try_from(ms / 1000).unwrap_or(i64::MAX) + i64::from(offset_at(ms));
    let at = local(at_ms);
    let today = local(now_ms).div_euclid(DAY_SECS);
    let day_number = at.div_euclid(DAY_SECS);
    let secs = at.rem_euclid(DAY_SECS);
    let day = match today - day_number {
        ..=0 => Day::Today,
        1 => Day::Yesterday,
        2..=6 => Day::Weekday(weekday(day_number)),
        _ => {
            let (year, month, day) = civil(day_number);
            Day::Date {
                year,
                month,
                day,
                this_year: year == civil(today).0,
            }
        }
    };
    LocalTime {
        day,
        // Both below 60 and 24 by the remainder above.
        hour: u8::try_from(secs / 3600).unwrap_or(0),
        minute: u8::try_from(secs % 3600 / 60).unwrap_or(0),
    }
}

/// The weekday of a day counted from 1970-01-01, a Thursday.
fn weekday(day: i64) -> Weekday {
    const ORDER: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];
    // In 0..7 by `rem_euclid`.
    ORDER[usize::try_from((day + 3).rem_euclid(7)).unwrap_or(0)]
}

/// The proleptic Gregorian date of a day counted from 1970-01-01
/// (H. Hinnant's `civil_from_days`).
fn civil(day: i64) -> (i32, u8, u8) {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        i32::try_from(y).unwrap_or(i32::MAX),
        u8::try_from(m).unwrap_or(1),
        u8::try_from(d).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::{AvatarHue, civil};
    use interweave_transport_api::EndpointId;

    /// Contact-cards design 5.1: an identity's hue is its `EndpointId`'s,
    /// by section 5.1's rule over the id's text -- so "human" (bytes
    /// summing to 537), "work" (451) and "home" (425) take the three
    /// pairs in order, whatever `PeerId` holds them. Mutation: the hue
    /// taken from anything but the `EndpointId`'s text.
    #[test]
    fn an_identitys_hue_is_its_endpoint_ids() {
        let of = |id: &str| AvatarHue::of_identity(&EndpointId::parse(id).expect("an endpoint"));
        assert_eq!(of("human"), AvatarHue::Teal);
        assert_eq!(of("work"), AvatarHue::Amber);
        assert_eq!(of("home"), AvatarHue::Navy);
    }

    #[test]
    fn civil_dates_around_known_days() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert_eq!(civil(11_016), (2000, 2, 29));
        // 2026-10-17.
        assert_eq!(civil(20_743), (2026, 10, 17));
    }
}

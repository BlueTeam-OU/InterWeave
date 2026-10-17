// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A message's time as the views show it (design/2026-10-16-android-look.md
//! section 5.2): always the local time the message reached this app
//! (`local_at`), never the time a sender asserts, in the platform's zone
//! and its 12- or 24-hour setting. The calendar is ui-model's
//! [`local_time`]; the words are labels.rs's (ux-designer-01's ruling,
//! REPLY 01a14817: English through the label table, the phone's 12/24-hour
//! setting, other locales with the localisation contract).

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Local, Offset as _, TimeZone as _};
use interweave_human_ui_model::{
    Day, LocalTime, UiText, Weekday, fill, local_time, placeholder_en,
};

/// Where the time comes from: the system's clock and zone, or a test's.
pub(crate) struct Clock {
    now: Box<dyn Fn() -> u64>,
    offset: Box<dyn Fn(u64) -> i32>,
    /// The platform's 12- or 24-hour setting; 24 until it says.
    pub(crate) hour24: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            now: Box::new(system_now_ms),
            offset: Box::new(local_offset),
            hour24: true,
        }
    }
}

impl Clock {
    /// A clock that reads `now` and the zone `offset`, for a test or a
    /// root that pins them.
    pub(crate) fn new(
        now: impl Fn() -> u64 + 'static,
        offset: impl Fn(u64) -> i32 + 'static,
    ) -> Self {
        Self {
            now: Box::new(now),
            offset: Box::new(offset),
            hour24: true,
        }
    }

    /// Unix milliseconds now.
    pub(crate) fn now_ms(&self) -> u64 {
        (self.now)()
    }

    /// `at_ms` in the local zone, seen from now.
    pub(crate) fn local(&self, at_ms: u64) -> LocalTime {
        local_time(at_ms, self.now_ms(), &*self.offset)
    }

    /// The list's time for a message at `at_ms`: its clock time today,
    /// else its day.
    pub(crate) fn list_time(&self, at_ms: u64) -> String {
        let local = self.local(at_ms);
        match local.day {
            Day::Today => clock_text(local.hour, local.minute, self.hour24),
            day => day_text(day),
        }
    }

    /// How long until the day the list's times are reckoned from changes:
    /// the next local midnight, when "01:18" becomes "Yesterday".
    pub(crate) fn until_next_day_ms(&self) -> u64 {
        let now = self.now_ms();
        let local = self.local(now);
        let into_day =
            u64::from(local.hour) * 3_600_000 + u64::from(local.minute) * 60_000 + now % 60_000;
        86_400_000u64.saturating_sub(into_day).max(1_000)
    }
}

/// The system's clock, in Unix milliseconds.
fn system_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The local zone's offset from UTC at `at_ms`, in seconds: the system's
/// zone, read from the platform's tzdata on Android.
fn local_offset(at_ms: u64) -> i32 {
    i64::try_from(at_ms)
        .ok()
        .and_then(|ms| Local.timestamp_millis_opt(ms).single())
        .map_or(0, |t| t.offset().fix().local_minus_utc())
}

/// A clock time, in the 24- or the 12-hour form.
pub(crate) fn clock_text(hour: u8, minute: u8, hour24: bool) -> String {
    let minute = format!("{minute:02}");
    if hour24 {
        fill(
            placeholder_en::text(UiText::Time24),
            &[("hour", &format!("{hour:02}")), ("minute", &minute)],
        )
    } else {
        let period = placeholder_en::text(if hour < 12 {
            UiText::BeforeNoon
        } else {
            UiText::AfterNoon
        });
        let twelve = match hour % 12 {
            0 => 12,
            h => h,
        };
        fill(
            placeholder_en::text(UiText::Time12),
            &[
                ("hour", &twelve.to_string()),
                ("minute", &minute),
                ("period", period),
            ],
        )
    }
}

/// A day's words: a day separator's, and the list's for any day but
/// today.
pub(crate) fn day_text(day: Day) -> String {
    match day {
        Day::Today => placeholder_en::text(UiText::Today).to_owned(),
        Day::Yesterday => placeholder_en::text(UiText::Yesterday).to_owned(),
        Day::Weekday(weekday) => placeholder_en::text(match weekday {
            Weekday::Monday => UiText::Monday,
            Weekday::Tuesday => UiText::Tuesday,
            Weekday::Wednesday => UiText::Wednesday,
            Weekday::Thursday => UiText::Thursday,
            Weekday::Friday => UiText::Friday,
            Weekday::Saturday => UiText::Saturday,
            Weekday::Sunday => UiText::Sunday,
        })
        .to_owned(),
        Day::Date {
            year,
            month,
            day,
            this_year,
        } => {
            const MONTHS: [UiText; 12] = [
                UiText::January,
                UiText::February,
                UiText::March,
                UiText::April,
                UiText::May,
                UiText::June,
                UiText::July,
                UiText::August,
                UiText::September,
                UiText::October,
                UiText::November,
                UiText::December,
            ];
            let month = MONTHS[usize::from(month.clamp(1, 12)) - 1];
            let values = [
                ("month", placeholder_en::text(month)),
                ("day", &day.to_string()),
                ("year", &year.to_string()),
            ];
            let template = if this_year {
                UiText::DateThisYear
            } else {
                UiText::DateOtherYear
            };
            fill(placeholder_en::text(template), &values)
        }
    }
}

#[cfg(test)]
mod tests {
    use interweave_human_ui_model::{Day, Weekday};

    use super::{Clock, clock_text, day_text};

    #[test]
    fn a_clock_time_follows_the_hour_setting() {
        assert_eq!(clock_text(1, 18, true), "01:18");
        assert_eq!(clock_text(13, 5, true), "13:05");
        assert_eq!(clock_text(1, 18, false), "1:18 AM");
        assert_eq!(clock_text(0, 0, false), "12:00 AM");
        assert_eq!(clock_text(12, 30, false), "12:30 PM");
        assert_eq!(clock_text(23, 59, false), "11:59 PM");
    }

    #[test]
    fn each_day_has_its_words() {
        assert_eq!(day_text(Day::Today), "Today");
        assert_eq!(day_text(Day::Yesterday), "Yesterday");
        assert_eq!(day_text(Day::Weekday(Weekday::Thursday)), "Thursday");
        let date = |this_year| Day::Date {
            year: 2025,
            month: 12,
            day: 21,
            this_year,
        };
        assert_eq!(day_text(date(true)), "Dec 21");
        assert_eq!(day_text(date(false)), "Dec 21, 2025");
    }

    #[test]
    fn the_list_shows_todays_clock_time_and_any_other_days_name() {
        const DAY: u64 = 86_400_000;
        // 2026-10-17 14:30 UTC.
        let now = 20_743 * DAY + 14 * 3_600_000 + 30 * 60_000;
        let clock = Clock::new(move || now, |_| 0);
        assert_eq!(clock.list_time(now - 13 * 3_600_000 - 12 * 60_000), "01:18");
        assert_eq!(clock.list_time(now - DAY), "Yesterday");
        assert_eq!(clock.list_time(now - 2 * DAY), "Thursday");
        assert_eq!(clock.list_time(now - 7 * DAY), "Oct 10");
        assert_eq!(clock.until_next_day_ms(), 9 * 3_600_000 + 30 * 60_000);
    }
}

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

use chrono::{Datelike as _, Local, Offset as _, TimeZone as _};
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

    /// The calendar date of `at_ms` in the local zone, the year left out
    /// in the current one: what a `{date}` placeholder takes ("You shared
    /// your card here on {date}."), never the list's relative day, which
    /// would read "on 14:05" and go stale at midnight.
    pub(crate) fn date_text(&self, at_ms: u64) -> String {
        let local_date = |ms: u64| {
            let secs =
                i64::try_from(ms / 1000).unwrap_or(i64::MAX / 2) + i64::from((self.offset)(ms));
            chrono::DateTime::from_timestamp(secs, 0).map(|t| t.date_naive())
        };
        let (Some(date), Some(today)) = (local_date(at_ms), local_date(self.now_ms())) else {
            return String::new();
        };
        day_text(Day::Date {
            year: date.year(),
            month: u8::try_from(date.month()).unwrap_or(1),
            day: u8::try_from(date.day()).unwrap_or(1),
            this_year: date.year() == today.year(),
        })
    }

    /// The clock time of `at_ms` in the local zone, in the platform's
    /// hour setting.
    pub(crate) fn time_text(&self, at_ms: u64) -> String {
        let local = self.local(at_ms);
        clock_text(local.hour, local.minute, self.hour24)
    }

    /// When the day the list's times are reckoned from next changes, in
    /// Unix milliseconds: the next local midnight, when "01:18" becomes
    /// "Yesterday". Found in the zone's offset AT that midnight, not now's,
    /// so a daylight saving change during the day does not put it an hour
    /// off. At least a second ahead.
    pub(crate) fn next_day_ms(&self) -> u64 {
        const DAY_SECS: i64 = 86_400;
        let now = self.now_ms();
        let offset = |ms: i64| i64::from((self.offset)(u64::try_from(ms).unwrap_or(0)));
        let now_secs = i64::try_from(now / 1000).unwrap_or(i64::MAX / 2);
        let today = (now_secs + offset(now_secs * 1000)).div_euclid(DAY_SECS);
        let midnight = (today + 1) * DAY_SECS;
        // First guess in now's offset, then corrected by the offset in
        // force at that guess. Where the clocks jump over midnight, the
        // corrected instant is still the old day, an hour before the
        // change: then the new day begins at the change, which is the guess.
        let guess = midnight - offset(now_secs * 1000);
        let corrected = midnight - offset(guess * 1000);
        let on_next_day = |at: i64| (at + offset(at * 1000)).div_euclid(DAY_SECS) > today;
        let at = if on_next_day(corrected) {
            corrected
        } else {
            guess
        };
        u64::try_from(at.saturating_mul(1000))
            .unwrap_or(u64::MAX)
            .max(now.saturating_add(1_000))
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
        let template = if hour < 12 {
            UiText::TimeAm
        } else {
            UiText::TimePm
        };
        let twelve = match hour % 12 {
            0 => 12,
            h => h,
        };
        fill(
            placeholder_en::text(template),
            &[("hour", &twelve.to_string()), ("minute", &minute)],
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
        assert_eq!(clock_text(0, 5, false), "12:05 AM");
        assert_eq!(clock_text(12, 30, false), "12:30 PM");
        assert_eq!(clock_text(13, 5, false), "1:05 PM");
        assert_eq!(clock_text(23, 59, false), "11:59 PM");
    }

    // A `{date}` is the calendar date whatever the day: today's is a date
    // and not a clock time, and the year shows only outside the current
    // one. Mutations: `date_text` returning `list_time`; `this_year`
    // always true.
    #[test]
    fn a_date_is_the_calendar_date_today_and_every_day() {
        // 2026-10-24 12:00 UTC, at offset zero.
        const NOW: u64 = 1_792_843_200_000;
        let clock = Clock::new(|| NOW, |_| 0);
        assert_eq!(clock.date_text(NOW), "Oct 24");
        assert_eq!(clock.date_text(NOW - 86_400_000), "Oct 23");
        assert_eq!(clock.date_text(NOW - 365 * 86_400_000), "Oct 24, 2025");
        assert_eq!(clock.time_text(NOW), "12:00");
        // An offset that crosses midnight moves the date with it.
        let east = Clock::new(|| NOW, |_| 13 * 3600);
        assert_eq!(east.date_text(NOW), "Oct 25");
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
        assert_eq!(clock.next_day_ms(), now + 9 * 3_600_000 + 30 * 60_000);
    }

    #[test]
    fn where_midnight_does_not_exist_the_next_day_begins_at_the_change() {
        const HOUR: u64 = 3_600_000;
        const DAY: u64 = 24 * HOUR;
        // A zone at UTC-4 whose clocks go from 00:00 to 01:00 (UTC-3) as
        // the day changes, so local midnight never happens: the change is
        // at 04:00 UTC on 2026-10-18. Now is 22:00 local, 02:00 UTC.
        let change = 20_744 * DAY + 4 * HOUR;
        let now = change - 2 * HOUR;
        let clock = Clock::new(
            move || now,
            move |ms| if ms < change { -4 * 3_600 } else { -3 * 3_600 },
        );
        assert_eq!(
            clock.next_day_ms(),
            change,
            "the first instant of the new day"
        );
        // An hour before the change, still the same: never a second away.
        let later = change - HOUR / 2;
        let clock = Clock::new(
            move || later,
            move |ms| if ms < change { -4 * 3_600 } else { -3 * 3_600 },
        );
        assert_eq!(clock.next_day_ms(), change);
    }

    #[test]
    fn the_next_day_is_midnight_in_the_offset_then_in_force() {
        const HOUR: u64 = 3_600_000;
        const DAY: u64 = 24 * HOUR;
        // 2026-10-17 14:30 UTC; the zone is UTC+1 until 18:00 UTC, then
        // UTC+2, as on a spring-forward day.
        let now = 20_743 * DAY + 14 * HOUR + 30 * 60_000;
        let change = 20_743 * DAY + 18 * HOUR;
        let clock = Clock::new(
            move || now,
            move |ms| if ms < change { 3_600 } else { 7_200 },
        );
        // Local midnight is 2026-10-18 00:00 at UTC+2: 22:00 UTC. Now's
        // offset would have put it at 23:00, an hour late.
        assert_eq!(clock.next_day_ms(), 20_743 * DAY + 22 * HOUR);
    }
}

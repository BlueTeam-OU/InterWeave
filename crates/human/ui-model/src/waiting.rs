// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A waiting message's one status (#304 F10, language-culture-en's
//! REPLY 01a15453 and 01a15455): which of the two waits it is, how long
//! since it was written, and the last attempt's problem, as one whole
//! template. The facade retries it until it is through or cancelled, so
//! nothing here reads as failed.

use interweave_human_client_api::{OutboundStatus, SendProblem};

use crate::labels::{HostText, RuntimeHost, UiText, fill, placeholder_en};
use crate::model::ItemStatus;

const MINUTE_MS: u64 = 60_000;
const HOUR_MS: u64 = 60 * MINUTE_MS;
const DAY_MS: u64 = 24 * HOUR_MS;

/// How long a message written at `written_ms` has waited at `now_ms`,
/// rounded down to the coarse step messaging apps use. A written time
/// after now (the clock moved back) reads as just now.
#[must_use]
pub fn age_text(written_ms: u64, now_ms: u64) -> String {
    let elapsed = now_ms.saturating_sub(written_ms);
    let count = |n: u64| n.to_string();
    let (text, n) = match elapsed {
        e if e < MINUTE_MS => (UiText::AgeJustNow, None),
        e if e < 2 * MINUTE_MS => (UiText::AgeOneMinute, None),
        e if e < HOUR_MS => (UiText::AgeMinutes, Some(e / MINUTE_MS)),
        e if e < 2 * HOUR_MS => (UiText::AgeOneHour, None),
        e if e < DAY_MS => (UiText::AgeHours, Some(e / HOUR_MS)),
        e if e < 2 * DAY_MS => (UiText::AgeOneDay, None),
        e => (UiText::AgeDays, Some(e / DAY_MS)),
    };
    let template = placeholder_en::text(text);
    n.map_or_else(
        || template.to_owned(),
        |n| fill(template, &[("count", &count(n))]),
    )
}

/// When [`age_text`] next reads differently, in the same clock as
/// `now_ms`: always after it.
#[must_use]
pub fn next_age_change(written_ms: u64, now_ms: u64) -> u64 {
    let elapsed = now_ms.saturating_sub(written_ms);
    let step = match elapsed {
        e if e < HOUR_MS => MINUTE_MS,
        e if e < DAY_MS => HOUR_MS,
        _ => DAY_MS,
    };
    let base = written_ms.min(now_ms);
    base.saturating_add((elapsed / step + 1).saturating_mul(step))
}

/// The last attempt's problem, as the waiting status names it. The
/// classes a retried row can carry; any other names none, since the
/// facade stops retrying on it and the row needs attention instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Problem {
    NoPath,
    Route,
    Busy,
    Unavailable,
}

const fn problem(last: Option<SendProblem>) -> Option<Problem> {
    match last {
        Some(SendProblem::NoNetworkPath) => Some(Problem::NoPath),
        Some(SendProblem::RouteUnavailable) => Some(Problem::Route),
        Some(SendProblem::Busy) => Some(Problem::Busy),
        Some(SendProblem::ServiceUnavailable) => Some(Problem::Unavailable),
        _ => None,
    }
}

/// What a waiting status says apart from its age: whether an earlier
/// attempt may have reached the peer, and the last problem.
const fn waiting(status: &OutboundStatus) -> Option<(bool, Option<Problem>)> {
    match status {
        OutboundStatus::Sending { last_problem, .. } => Some((false, problem(*last_problem))),
        OutboundStatus::Unconfirmed { last_problem, .. } => Some((true, problem(*last_problem))),
        _ => None,
    }
}

/// The status of a message written at `written_ms` that waits, at
/// `now_ms`, as it reads where the runtime runs; `None` for one that
/// does not wait.
#[must_use]
pub fn waiting_text(
    status: &OutboundStatus,
    written_ms: u64,
    now_ms: u64,
    host: RuntimeHost,
) -> Option<String> {
    let (maybe, problem) = waiting(status)?;
    let template = match (maybe, problem) {
        (false, None) => placeholder_en::text(UiText::WaitingSending),
        (false, Some(Problem::NoPath)) => placeholder_en::text(UiText::WaitingSendingNoPath),
        (false, Some(Problem::Route)) => placeholder_en::text(UiText::WaitingSendingRoute),
        (false, Some(Problem::Busy)) => placeholder_en::text(UiText::WaitingSendingBusy),
        (false, Some(Problem::Unavailable)) => {
            placeholder_en::host_text(host, HostText::WaitingSendingUnavailable)
        }
        (true, None) => placeholder_en::text(UiText::WaitingMaybe),
        (true, Some(Problem::NoPath)) => placeholder_en::text(UiText::WaitingMaybeNoPath),
        (true, Some(Problem::Route)) => placeholder_en::text(UiText::WaitingMaybeRoute),
        (true, Some(Problem::Busy)) => placeholder_en::text(UiText::WaitingMaybeBusy),
        (true, Some(Problem::Unavailable)) => {
            placeholder_en::host_text(host, HostText::WaitingMaybeUnavailable)
        }
    };
    Some(fill(template, &[("age", &age_text(written_ms, now_ms))]))
}

/// Whether a message's status changed in a way a person is told of: a
/// waiting message's attempt counter, retry time and age are not, only
/// the wait it is and its problem; any other change is.
#[must_use]
pub fn announced_change(was: &ItemStatus, now: &ItemStatus) -> bool {
    match (was, now) {
        (ItemStatus::Outbound(was), ItemStatus::Outbound(now)) => {
            match (waiting(was), waiting(now)) {
                (Some(was), Some(now)) => was != now,
                _ => was != now,
            }
        }
        _ => was != now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRITTEN: u64 = 1_000_000_000;

    #[test]
    fn the_age_steps_round_down_and_change_when_the_next_step_begins() {
        for (elapsed, text, next) in [
            (0, "just now", MINUTE_MS),
            (MINUTE_MS - 1, "just now", MINUTE_MS),
            (MINUTE_MS, "1 minute ago", 2 * MINUTE_MS),
            (2 * MINUTE_MS, "2 minutes ago", 3 * MINUTE_MS),
            (HOUR_MS - 1, "59 minutes ago", HOUR_MS),
            (HOUR_MS, "1 hour ago", 2 * HOUR_MS),
            (2 * HOUR_MS + 5, "2 hours ago", 3 * HOUR_MS),
            (DAY_MS - 1, "23 hours ago", DAY_MS),
            (DAY_MS, "1 day ago", 2 * DAY_MS),
            (9 * DAY_MS, "9 days ago", 10 * DAY_MS),
        ] {
            assert_eq!(age_text(WRITTEN, WRITTEN + elapsed), text, "{elapsed}");
            assert_eq!(
                next_age_change(WRITTEN, WRITTEN + elapsed),
                WRITTEN + next,
                "{elapsed}"
            );
        }
        assert_eq!(
            age_text(WRITTEN, WRITTEN - 5),
            "just now",
            "a clock moved back"
        );
        assert!(next_age_change(WRITTEN, WRITTEN - 5) > WRITTEN - 5);
    }

    #[test]
    fn a_waiting_message_reads_its_wait_its_age_and_its_last_problem() {
        let sending = |last_problem| OutboundStatus::Sending {
            attempts: 3,
            next_retry_at: Some(1),
            last_problem,
        };
        let maybe = |last_problem| OutboundStatus::Unconfirmed {
            next_retry_at: Some(1),
            last_problem,
        };
        let at = WRITTEN + 3 * HOUR_MS;
        let read = |s: &OutboundStatus, host| waiting_text(s, WRITTEN, at, host);
        assert_eq!(
            read(&sending(None), RuntimeHost::Embedded).as_deref(),
            Some("Still sending, started 3 hours ago")
        );
        assert_eq!(
            read(
                &maybe(Some(SendProblem::NoNetworkPath)),
                RuntimeHost::Embedded
            )
            .as_deref(),
            Some(
                "May have reached the peer. Still sending, started 3 hours ago. Last attempt: the peer could not be reached"
            )
        );
        assert_eq!(
            read(
                &sending(Some(SendProblem::ServiceUnavailable)),
                RuntimeHost::Daemon
            )
            .as_deref(),
            Some(
                "Still sending, started 3 hours ago. Last attempt: the transport daemon was not available"
            )
        );
        assert_eq!(
            read(
                &sending(Some(SendProblem::ServiceUnavailable)),
                RuntimeHost::Embedded
            )
            .as_deref(),
            Some(
                "Still sending, started 3 hours ago. Last attempt: the network service was not available"
            )
        );
        assert_eq!(
            read(&sending(Some(SendProblem::TooLarge)), RuntimeHost::Embedded).as_deref(),
            Some("Still sending, started 3 hours ago"),
            "a class a retried row does not carry names no problem"
        );
        assert_eq!(
            read(&OutboundStatus::Published, RuntimeHost::Embedded),
            None
        );
    }

    #[test]
    fn only_a_change_of_wait_or_problem_is_announced_never_a_retry() {
        let out = |s| ItemStatus::Outbound(s);
        let sending = |attempts, last_problem| {
            out(OutboundStatus::Sending {
                attempts,
                next_retry_at: Some(u64::from(attempts) * 1_000),
                last_problem,
            })
        };
        assert!(!announced_change(&sending(1, None), &sending(2, None)));
        assert!(!announced_change(
            &sending(2, Some(SendProblem::Busy)),
            &sending(3, Some(SendProblem::Busy))
        ));
        assert!(announced_change(
            &sending(2, None),
            &sending(3, Some(SendProblem::Busy))
        ));
        assert!(announced_change(
            &sending(2, None),
            &out(OutboundStatus::Unconfirmed {
                next_retry_at: None,
                last_problem: None,
            })
        ));
        assert!(announced_change(
            &sending(2, None),
            &out(OutboundStatus::Published)
        ));
    }
}

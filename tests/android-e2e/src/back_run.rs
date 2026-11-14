// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A5's Back runs while the reader opens (design/2026-11-04-long-message.md
//! A5 and section 5.4), judged on the phone's uptime clock.
//!
//! The presses come from the androidTest APK's `KeySender`, run by the
//! phone's shell: one process sends Open the whole message's Enter and
//! every Back on a schedule and prints each press's send time, its
//! uptime the moment before injection ([`sent_times`]). The trace's
//! lines are read again from a monotonic logcat, which stamps them on the
//! same clock ([`monotonic_trace`]). [`judge`] holds the two together.
//!
//! What a trace time is: when the app logged the line, which for a page
//! is when its state changed, not when the frame showing it reached the
//! screen. A close timed here is the app's close, a frame or so before
//! the screen's; the recording is what shows the screen.

use std::time::Duration;

use crate::screen::TRACE;

/// A step's result for a void round: neither `pass` nor `fail`.
pub const VOID: &str = "void";

/// The test package that carries `KeySender`.
pub const SENDER_PACKAGE: &str = "org.interweave.human.test";

/// The phone shell's command that sends `keys`, each `(key code name,
/// delay after the previous press was sent)`.
#[must_use]
pub fn sender_command(keys: &[(&str, Duration)]) -> String {
    let schedule: Vec<String> = keys
        .iter()
        .map(|(key, after)| format!("{key}:{}", after.as_millis()))
        .collect();
    format!(
        "CLASSPATH=$(pm path {SENDER_PACKAGE} | sed -n '1s/^package://p') app_process /system/bin {SENDER_PACKAGE}.KeySender {}",
        schedule.join(" ")
    )
}

/// The sender's `sent <KEY> <uptime ms>` lines, in order. An `unsent`
/// line, or any other, gives no time: a press not injected has none.
#[must_use]
pub fn sent_times(out: &str) -> Vec<(String, u64)> {
    out.lines()
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            match (words.next(), words.next(), words.next(), words.next()) {
                (Some("sent"), Some(key), Some(at), None) => {
                    at.parse().ok().map(|at| (key.to_owned(), at))
                }
                _ => None,
            }
        })
        .collect()
}

/// The trace's lines in a `logcat -v threadtime -v monotonic` of the
/// app's tag, each with its uptime in ms, in order: `<secs>.<ms> <pid>
/// <tid> <level> interweave: <line>`. A line of any other shape is
/// skipped, so the caller holds the result against the raw trace it
/// already read before trusting it.
#[must_use]
pub fn monotonic_trace(log: &str) -> Vec<(u64, String)> {
    log.lines()
        .filter_map(|l| {
            let (_, message) = l.split_once(" interweave: ")?;
            let message = message.trim_end();
            if !message.starts_with(TRACE) {
                return None;
            }
            let (secs, millis) = l.split_whitespace().next()?.split_once('.')?;
            let secs: u64 = secs.parse().ok()?;
            if millis.len() != 3 {
                return None;
            }
            let millis: u64 = millis.parse().ok()?;
            Some((secs * 1000 + millis, message.to_owned()))
        })
        .collect()
}

/// Which of A5's runs a round is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    /// One Back; the reader closes within `within` of it. Void when the
    /// first screen was drawn before the Back.
    Single {
        /// The close's limit after the Back.
        within: Duration,
    },
    /// Two Backs about 0.3 s apart: neither is lost, nothing more.
    Pair,
    /// Two Backs pressed together, both before the close: exactly two
    /// steps, the reader's close within `within` of the first Back. Void
    /// when a send time is missing, the second was sent after the close,
    /// or the first screen was drawn before the first Back.
    Together {
        /// The close's limit after the first Back.
        within: Duration,
    },
}

/// A round's verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// A5's conditions held.
    Pass,
    /// One did not, and why.
    Fail(String),
    /// Neither: A5 cannot judge it, and it goes back to ux-designer.
    Void(String),
}

/// What a round measured, on the uptime clock, for its report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Times {
    /// Open the whole message's Enter, sent.
    pub enter: Option<u64>,
    /// The first Back, sent.
    pub back: Option<u64>,
    /// The second Back, sent.
    pub back2: Option<u64>,
    /// The reader's `reader=drawn` line: its first screen.
    pub drawn: Option<u64>,
    /// The conversation's page after the reader's: the close.
    pub closed: Option<u64>,
    /// The chat list's page after that: the second step.
    pub left: Option<u64>,
}

impl Times {
    /// Back to close, in ms.
    #[must_use]
    pub fn back_to_close(&self) -> Option<u64> {
        self.closed?.checked_sub(self.back?)
    }

    /// How long the first screen showed before the close, in ms.
    #[must_use]
    pub fn first_screen_shown(&self) -> Option<u64> {
        self.closed?.checked_sub(self.drawn?)
    }
}

/// The round's times: `sent` from the sender, `trace` the run's trace
/// lines (from Open the whole message on) with their times.
#[must_use]
pub fn times(sent: &[(String, u64)], trace: &[(u64, String)]) -> Times {
    let backs: Vec<u64> = sent
        .iter()
        .filter(|(k, _)| k == "BACK")
        .map(|(_, at)| *at)
        .collect();
    let page = |name: &str, after: Option<usize>| {
        let line = format!("{TRACE}page={name}");
        trace
            .iter()
            .enumerate()
            .skip(after.map_or(0, |i| i + 1))
            .find(|(_, (_, l))| *l == line)
            .map(|(i, (at, _))| (i, *at))
    };
    let reader = page("reader", None);
    let closed = reader.and_then(|(i, _)| page("conversation", Some(i)));
    let left = closed.and_then(|(i, _)| page("conversations", Some(i)));
    let drawn_line = format!("{TRACE}reader=drawn ");
    Times {
        enter: sent.iter().find(|(k, _)| k == "ENTER").map(|(_, at)| *at),
        back: backs.first().copied(),
        back2: backs.get(1).copied(),
        drawn: trace
            .iter()
            .find(|(_, l)| l.starts_with(&drawn_line))
            .map(|(at, _)| *at),
        closed: closed.map(|(_, at)| at),
        left: left.map(|(_, at)| at),
    }
}

/// The page lines among `trace`, in order: the steps the Backs took.
fn pages(trace: &[(u64, String)]) -> Vec<&str> {
    trace
        .iter()
        .filter_map(|(_, l)| l.strip_prefix(TRACE)?.strip_prefix("page="))
        .collect()
}

/// A5's verdict on a round. Void is decided first: a round A5 cannot
/// judge is void whatever the app did.
#[must_use]
pub fn judge(run: Run, t: &Times, trace: &[(u64, String)]) -> Verdict {
    let Some(back) = t.back else {
        return Verdict::Void("the Back has no send time".to_owned());
    };
    if let Some(drawn) = t.drawn.filter(|d| *d < back && run != Run::Pair) {
        return Verdict::Void(format!(
            "the first screen was drawn {} ms before the Back",
            back - drawn
        ));
    }
    if let Run::Together { .. } | Run::Pair = run {
        let Some(back2) = t.back2 else {
            return Verdict::Void("the second Back has no send time".to_owned());
        };
        if let (Run::Together { .. }, Some(closed)) = (run, t.closed)
            && back2 > closed
        {
            return Verdict::Void(format!(
                "the second Back was sent {} ms after the close",
                back2 - closed
            ));
        }
    }
    let Some(closed) = t.closed else {
        return Verdict::Fail("the reader never closed".to_owned());
    };
    let steps = pages(trace);
    match run {
        Run::Single { within } => {
            if steps != ["reader", "conversation"] {
                return Verdict::Fail(format!("the pages were {steps:?}"));
            }
            within_of(within, back, closed)
        }
        Run::Pair => {
            if t.left.is_none() {
                return Verdict::Fail(format!("a Back was lost: the pages were {steps:?}"));
            }
            Verdict::Pass
        }
        Run::Together { within } => {
            if steps != ["reader", "conversation", "conversations"] {
                return Verdict::Fail(format!(
                    "not exactly two steps (the reader closed, then the conversation left): the pages were {steps:?}"
                ));
            }
            within_of(within, back, closed)
        }
    }
}

fn within_of(within: Duration, back: u64, closed: u64) -> Verdict {
    let took = closed.saturating_sub(back);
    if u128::from(took) <= within.as_millis() {
        Verdict::Pass
    } else {
        Verdict::Fail(format!(
            "the reader closed {took} ms after the Back, past {} ms",
            within.as_millis()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(at: u64, what: &str) -> (u64, String) {
        (at, format!("{TRACE}{what}"))
    }

    fn sent(keys: &[(&str, u64)]) -> Vec<(String, u64)> {
        keys.iter().map(|(k, at)| ((*k).to_owned(), *at)).collect()
    }

    const FAST: Run = Run::Together {
        within: Duration::from_millis(150),
    };

    fn together(back: u64, back2: u64, trace: &[(u64, String)]) -> Verdict {
        let s = sent(&[("ENTER", 1000), ("BACK", back), ("BACK", back2)]);
        judge(FAST, &times(&s, trace), trace)
    }

    fn two_steps(close: u64, left: u64) -> Vec<(u64, String)> {
        vec![
            line(1010, "page=reader"),
            line(close, "page=conversation"),
            line(left, "page=conversations"),
        ]
    }

    #[test]
    fn the_sender_command_names_the_schedule_in_ms() {
        let c = sender_command(&[
            ("ENTER", Duration::ZERO),
            ("BACK", Duration::from_millis(600)),
            ("BACK", Duration::from_millis(30)),
        ]);
        assert!(c.ends_with(".KeySender ENTER:0 BACK:600 BACK:30"), "{c}");
        assert!(
            c.starts_with("CLASSPATH=$(pm path org.interweave.human.test "),
            "{c}"
        );
    }

    #[test]
    fn only_a_sent_line_gives_a_time() {
        let out = "sent ENTER 5000\nunsent BACK refused\nsent BACK 5630\nsent BACK x\nnoise\nsent BACK 1 2\n";
        assert_eq!(sent_times(out), sent(&[("ENTER", 5000), ("BACK", 5630)]));
    }

    #[test]
    fn a_monotonic_logcat_gives_each_trace_line_its_uptime() {
        let log = "--------- beginning of main\n\
                   \u{20}  1234.567  4321  4330 I interweave: ui-trace page=reader\n\
                   \u{20}  1234.600  4321  4330 I interweave: not the trace\n\
                   \u{20}  1235.004  4321  4330 I interweave: ui-trace reader=drawn ms=412  \n\
                   11-14 10:00:00.000  4321  4330 I interweave: ui-trace page=conversation\n";
        assert_eq!(
            monotonic_trace(log),
            vec![
                (1_234_567, format!("{TRACE}page=reader")),
                (1_235_004, format!("{TRACE}reader=drawn ms=412")),
            ]
        );
    }

    #[test]
    fn a_pressed_together_round_of_two_steps_in_time_passes() {
        assert_eq!(together(1600, 1630, &two_steps(1700, 1760)), Verdict::Pass);
    }

    #[test]
    fn a_pressed_together_round_with_a_missing_send_time_is_void() {
        let trace = two_steps(1700, 1760);
        let s = sent(&[("ENTER", 1000), ("BACK", 1600)]);
        assert!(matches!(
            judge(FAST, &times(&s, &trace), &trace),
            Verdict::Void(_)
        ));
        let s = sent(&[("ENTER", 1000)]);
        assert!(matches!(
            judge(FAST, &times(&s, &trace), &trace),
            Verdict::Void(_)
        ));
    }

    #[test]
    fn a_second_press_sent_after_the_close_is_void_even_when_one_was_lost() {
        let lost = vec![line(1010, "page=reader"), line(1700, "page=conversation")];
        assert!(matches!(together(1600, 1701, &lost), Verdict::Void(_)));
        // At the close itself it was pending: judged, and here it failed.
        assert!(matches!(together(1600, 1700, &lost), Verdict::Fail(_)));
    }

    #[test]
    fn a_first_screen_before_the_first_press_is_void() {
        let mut trace = two_steps(1700, 1760);
        trace.insert(1, line(1599, "reader=drawn ms=599"));
        assert!(matches!(together(1600, 1630, &trace), Verdict::Void(_)));
        trace[1] = line(1600, "reader=drawn ms=600");
        assert_eq!(together(1600, 1630, &trace), Verdict::Pass);
    }

    #[test]
    fn fewer_or_more_steps_or_no_close_fail() {
        let merged = vec![line(1010, "page=reader"), line(1700, "page=conversation")];
        assert!(matches!(together(1600, 1630, &merged), Verdict::Fail(_)));
        let mut more = two_steps(1700, 1760);
        more.push(line(1800, "page=conversation"));
        assert!(matches!(together(1600, 1630, &more), Verdict::Fail(_)));
        let open = vec![line(1010, "page=reader")];
        assert!(matches!(together(1600, 1630, &open), Verdict::Fail(_)));
    }

    #[test]
    fn a_close_past_its_limit_fails() {
        assert!(matches!(
            together(1600, 1630, &two_steps(1751, 1800)),
            Verdict::Fail(_)
        ));
        assert_eq!(together(1600, 1630, &two_steps(1750, 1800)), Verdict::Pass);
    }

    #[test]
    fn a_single_back_is_void_after_the_first_screen_and_timed_otherwise() {
        let run = Run::Single {
            within: Duration::from_secs(1),
        };
        let s = sent(&[("ENTER", 1000), ("BACK", 1090)]);
        let mut trace = vec![line(1005, "page=reader"), line(1400, "page=conversation")];
        assert_eq!(judge(run, &times(&s, &trace), &trace), Verdict::Pass);
        // Drawn after the Back and shown before the close: a pass, and
        // reported.
        trace.insert(1, line(1180, "reader=drawn ms=180"));
        let t = times(&s, &trace);
        assert_eq!(judge(run, &t, &trace), Verdict::Pass);
        assert_eq!(
            (t.back_to_close(), t.first_screen_shown()),
            (Some(310), Some(220))
        );
        trace[1] = line(1089, "reader=drawn ms=89");
        assert!(matches!(
            judge(run, &times(&s, &trace), &trace),
            Verdict::Void(_)
        ));
        let late = vec![line(1005, "page=reader"), line(2091, "page=conversation")];
        assert!(matches!(
            judge(run, &times(&s, &late), &late),
            Verdict::Fail(_)
        ));
    }

    #[test]
    fn a_pair_checks_only_that_neither_back_was_lost() {
        let s = sent(&[("ENTER", 1000), ("BACK", 1600), ("BACK", 1900)]);
        // Drawn before the first Back and closed late: still only the
        // two steps count.
        let trace = vec![
            line(1010, "page=reader"),
            line(1500, "reader=drawn ms=500"),
            line(1880, "page=conversation"),
            line(1950, "page=conversations"),
        ];
        assert_eq!(judge(Run::Pair, &times(&s, &trace), &trace), Verdict::Pass);
        let lost = &trace[..3];
        assert!(matches!(
            judge(Run::Pair, &times(&s, lost), lost),
            Verdict::Fail(_)
        ));
    }
}

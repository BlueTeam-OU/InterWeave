// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Render cost stays linear in the input (`HUMAN-CHAT.md`: "a renderer
//! must not superlinearly amplify pathological input").
//!
//! Each pathological shape is rendered at `n` and at `4n` bytes, both
//! inside the decoded ceiling, and the larger may take at most
//! [`ALLOWED_RATIO`] times the smaller. Linear is 4; quadratic is 16. The
//! two sizes are timed in alternation and the best of each kept, so a
//! burst of load on a busy host falls on both sides rather than one; a
//! ratio over the bound is measured again, up to [`ATTEMPTS`] times, and
//! only one over it every time fails. That a quadratic cost still fails
//! and a linear one passes is held for a synthetic workload of each by
//! `the_harness_fails_a_quadratic_cost_and_passes_a_linear_one`; the
//! file's two timed tests take [`TIMING`] in turn, so neither is timed
//! while the other burns the CPU. (Timed one size after the other, the test failed under a full
//! parallel workspace run while passing alone, 2026-10-25.)

#![allow(clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use interweave_human_chat_protocol::render;

/// Four times the input may cost at most this many times the time.
const ALLOWED_RATIO: f64 = 9.0;

/// Held by each timed test for its whole run: libtest runs a file's tests
/// in parallel, and the control's quadratic loop beside the render timing
/// would load one side of it (#450 review, risk 1).
static TIMING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// How many times a ratio over the bound is measured before it fails.
const ATTEMPTS: usize = 3;

/// The size of the smaller input; the larger is four times it, still
/// under the 196,608-byte ceiling so both are parsed.
const BASE: usize = 40_000;

/// The ratio of `work` on `large` to `work` on `small`: each timed
/// `runs` times in alternation, the best of each kept.
fn ratio(work: &impl Fn(&str), small: &str, large: &str, runs: usize) -> (f64, Duration, Duration) {
    let time = |source: &str| {
        let start = Instant::now();
        work(std::hint::black_box(source));
        start.elapsed()
    };
    let (mut t_small, mut t_large) = (Duration::MAX, Duration::MAX);
    for _ in 0..runs {
        t_small = t_small.min(time(small));
        t_large = t_large.min(time(large));
    }
    let t_small = t_small.max(Duration::from_micros(50));
    (
        t_large.as_secs_f64() / t_small.as_secs_f64(),
        t_small,
        t_large,
    )
}

/// Whether `work` stays within the bound: measured again while it is
/// over, up to [`ATTEMPTS`] times; the last measurement when it never is.
fn within_bound(work: &impl Fn(&str), small: &str, large: &str) -> Result<(), String> {
    let mut last = String::new();
    for _ in 0..ATTEMPTS {
        let (ratio, t_small, t_large) = ratio(work, small, large, 5);
        if ratio <= ALLOWED_RATIO {
            return Ok(());
        }
        last = format!("4x the input took {ratio:.1}x the time ({t_small:?} -> {t_large:?})");
    }
    Err(last)
}

/// `unit` repeated to `len` bytes.
fn fill(unit: &str, len: usize) -> String {
    unit.repeat(len / unit.len())
}

#[test]
fn every_pathological_shape_renders_in_linear_time() {
    let _timing = TIMING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let shapes: [(&str, &str); 7] = [
        ("unclosed link brackets", "["),
        ("unclosed emphasis runs", "*a"),
        ("alternating delimiters", "*_~"),
        ("backtick runs", "`a``"),
        ("nested blockquote markers", "> > > > a\n"),
        ("raw html openers", "<a "),
        ("a long table", "| x | y |\n"),
    ];
    for (name, unit) in shapes {
        let small = fill(unit, BASE);
        let large = fill(unit, BASE * 4);
        let source_small = if name == "a long table" {
            format!("| h | h |\n| - | - |\n{small}")
        } else {
            small
        };
        let source_large = if name == "a long table" {
            format!("| h | h |\n| - | - |\n{large}")
        } else {
            large
        };
        let rendered = |source: &str| {
            std::hint::black_box(render(source));
        };
        if let Err(why) = within_bound(&rendered, &source_small, &source_large) {
            panic!("{name}: {why}");
        }
    }
}

/// The control the retries need: a cost quadratic in the input is over
/// the bound on every attempt and fails, a linear one passes. Mutation:
/// `within_bound` passing on any attempt's measurement but the last.
#[test]
fn the_harness_fails_a_quadratic_cost_and_passes_a_linear_one() {
    let _timing = TIMING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let small = fill("a", 2_000);
    let large = fill("a", 8_000);
    let quadratic = |source: &str| {
        let bytes = source.as_bytes();
        let mut sum = 0u64;
        for a in bytes {
            for b in bytes {
                sum = sum.wrapping_add(u64::from(*a ^ *b));
            }
        }
        std::hint::black_box(sum);
    };
    let linear = |source: &str| {
        let mut sum = 0u64;
        for _ in 0..200 {
            for b in source.as_bytes() {
                sum = sum.wrapping_add(u64::from(*b));
            }
        }
        std::hint::black_box(sum);
    };
    assert!(
        within_bound(&quadratic, &small, &large).is_err(),
        "a quadratic cost fails"
    );
    assert!(
        within_bound(&linear, &small, &large).is_ok(),
        "a linear cost passes"
    );
}

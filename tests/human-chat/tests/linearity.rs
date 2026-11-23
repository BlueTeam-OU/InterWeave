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
//! `the_harness_fails_a_quadratic_cost_and_passes_a_linear_one`.
//!
//! Time is the timed thread's CPU time, not the wall clock: load beside
//! it — the file's other timed test, which libtest runs on a thread
//! beside it and nextest in a process beside it, or any other test
//! process — takes the CPU away between its instructions, which the wall
//! clock counts and the thread's own clock does not
//! (`the_clock_counts_this_threads_work_alone`). Contention while the
//! thread runs, on a shared core or its caches, still slows both sides,
//! which the alternation and the retries absorb. (Timed one size
//! after the other, the render test failed under a full parallel
//! workspace run while passing alone, 2026-10-25; on the wall clock, the
//! control measured a quadratic cost within the bound under nextest on
//! CI, 2026-11-23, when the lock the two tests took in turn serialised
//! nothing across its processes.)

#![allow(clippy::expect_used, clippy::panic)]

use std::time::Duration;

use interweave_human_chat_protocol::render;

/// Four times the input may cost at most this many times the time.
const ALLOWED_RATIO: f64 = 9.0;

/// How many times a ratio over the bound is measured before it fails.
const ATTEMPTS: usize = 3;

/// The size of the smaller input; the larger is four times it, still
/// under the 196,608-byte ceiling so both are parsed.
const BASE: usize = 40_000;

/// The CPU time the calling thread has used.
#[cfg(unix)]
fn thread_cpu_time() -> Duration {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    Duration::new(
        u64::try_from(t.tv_sec).expect("a thread's CPU time is not negative"),
        u32::try_from(t.tv_nsec).expect("nanoseconds are under a second"),
    )
}

/// Off unix there is no thread clock to hand; the wall clock since the
/// first call stands in, and the two timed tests, which no lock keeps
/// apart, can then load each other's measurement.
#[cfg(not(unix))]
fn thread_cpu_time() -> Duration {
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    EPOCH.get_or_init(std::time::Instant::now).elapsed()
}

/// The ratio of `work` on `large` to `work` on `small`: each timed
/// `runs` times in alternation, the best of each kept.
fn ratio(work: &impl Fn(&str), small: &str, large: &str, runs: usize) -> (f64, Duration, Duration) {
    let time = |source: &str| {
        let start = thread_cpu_time();
        work(std::hint::black_box(source));
        thread_cpu_time().saturating_sub(start)
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

/// The clock [`ratio`] reads leaves out time this thread is not running:
/// a sleep, and another thread's work, advance it by far less than this
/// thread's own work does. The work is a fixed count of steps, not a
/// span of wall time, so what it costs this thread does not shrink when
/// load beside it takes the CPU away. Mutation: the monotonic or the
/// process CPU clock in `thread_cpu_time`.
#[cfg(unix)]
#[test]
fn the_clock_counts_this_threads_work_alone() {
    let work = || {
        let mut sum = 0u64;
        for step in 0..30_000_000u64 {
            sum = std::hint::black_box(sum.wrapping_add(step));
        }
    };
    let used_by = |run: &dyn Fn()| {
        let before = thread_cpu_time();
        run();
        thread_cpu_time().saturating_sub(before)
    };
    let own = used_by(&work);
    let sleeping = used_by(&|| std::thread::sleep(own));
    let joined = used_by(&|| {
        std::thread::spawn(work)
            .join()
            .expect("the working thread ends");
    });
    assert!(
        own >= Duration::from_millis(5),
        "this thread's own work took only {own:?}"
    );
    assert!(
        sleeping < own / 5,
        "a sleep as long as the work took {sleeping:?} of the thread's CPU, the work {own:?}"
    );
    assert!(
        joined < own / 5,
        "another thread's work took {joined:?} of this one's CPU, its own {own:?}"
    );
}

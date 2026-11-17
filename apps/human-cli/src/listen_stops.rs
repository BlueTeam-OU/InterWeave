// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The `stop` a test hands `listen`: the condition the test waits for,
//! under a bound that FAILS the test when it runs out.
//!
//! A bound that ends `listen` as a stop does lets a loaded host cut the
//! output short, and the test then fails on the list it was handed --
//! rows 1 to 28 of 60 under `cargo test --workspace` (j122) -- or, worse,
//! passes on what it saw in the time it had. Running out is said instead,
//! naming what was awaited and, where the test can say it, what was
//! printed (`running_out_fails_naming_the_wait`,
//! `a_stop_never_sent_fails_naming_the_wait`).

use std::time::Duration;

/// Far past what any of these waits takes on a loaded host: it exists
/// only so a broken `listen` cannot hang the suite.
pub(crate) const BOUND: Duration = Duration::from_secs(120);

const POLL: Duration = Duration::from_millis(50);

/// Ends once `done` holds, checked every 50 ms.
///
/// # Panics
/// When [`BOUND`] runs out first, naming `what`.
pub(crate) async fn when(what: &str, done: impl FnMut() -> bool) {
    within(BOUND, what, done, String::new).await;
}

/// [`when`], whose failure also says what `listen` printed: the
/// assertions the test makes after it, which would have shown it, are
/// never reached.
///
/// # Panics
/// When [`BOUND`] runs out first, naming `what` and quoting `printed`.
pub(crate) async fn when_showing(
    what: &str,
    done: impl FnMut() -> bool,
    printed: impl Fn() -> String,
) {
    within(BOUND, what, done, printed).await;
}

/// Ends when `stopped` is sent or dropped.
///
/// # Panics
/// When [`BOUND`] runs out first, naming `what`.
pub(crate) async fn on(what: &str, stopped: tokio::sync::oneshot::Receiver<()>) {
    on_within(BOUND, what, stopped).await;
}

async fn on_within(bound: Duration, what: &str, stopped: tokio::sync::oneshot::Receiver<()>) {
    assert!(
        tokio::time::timeout(bound, stopped).await.is_ok(),
        "listen never reached {what} within {bound:?}"
    );
}

async fn within(
    bound: Duration,
    what: &str,
    mut done: impl FnMut() -> bool,
    printed: impl Fn() -> String,
) {
    let deadline = tokio::time::Instant::now() + bound;
    while !done() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "listen never reached {what} within {bound:?}; it printed:\n{}",
            printed()
        );
        tokio::time::sleep(POLL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The panic a spawned wait ended in, as text.
    async fn panic_of(waited: tokio::task::JoinHandle<()>) -> String {
        let failed = tokio::time::timeout(Duration::from_secs(10), waited)
            .await
            .expect("the bound ended the wait")
            .expect_err("running out is a failure");
        failed
            .into_panic()
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default()
    }

    /// Mutation: the deadline's assert removed -- the wait then ends on
    /// `done` alone, and this test fails at its own 10 s timeout.
    #[tokio::test]
    async fn running_out_fails_naming_the_wait() {
        let message = panic_of(tokio::spawn(within(
            Duration::from_millis(100),
            "the drain",
            || false,
            || "row 1".to_owned(),
        )))
        .await;
        assert!(
            message.contains("listen never reached the drain"),
            "{message}"
        );
        assert!(message.ends_with("it printed:\nrow 1"), "{message}");
    }

    /// The same for a stop sent by the test, the sender held alive.
    /// Mutation: the timeout's result discarded.
    #[tokio::test]
    async fn a_stop_never_sent_fails_naming_the_wait() {
        let (held, stopped) = tokio::sync::oneshot::channel::<()>();
        let message = panic_of(tokio::spawn(async move {
            on_within(Duration::from_millis(100), "its line", stopped).await;
        }))
        .await;
        drop(held);
        assert!(
            message.contains("listen never reached its line"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn the_condition_ends_the_wait() {
        let mut polls = 0;
        within(
            Duration::from_secs(10),
            "the third poll",
            || {
                polls += 1;
                polls == 3
            },
            String::new,
        )
        .await;
        assert_eq!(polls, 3);
    }
}

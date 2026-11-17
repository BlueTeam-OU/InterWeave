// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The `stop` a test hands `listen`: the condition the test waits for,
//! under a bound that FAILS the test when it runs out.
//!
//! A bound that ends `listen` as a stop does lets a loaded host cut the
//! output short, and the test then fails on the list it was handed --
//! rows 1 to 28 of 60 under `cargo test --workspace` (j122) -- or, worse,
//! passes on what it saw in the time it had. Running out is said instead,
//! naming what was awaited (`running_out_fails_naming_the_wait`).

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
    within(BOUND, what, done).await;
}

/// Ends when `stopped` is sent or dropped.
///
/// # Panics
/// When [`BOUND`] runs out first, naming `what`.
pub(crate) async fn on(what: &str, stopped: tokio::sync::oneshot::Receiver<()>) {
    assert!(
        tokio::time::timeout(BOUND, stopped).await.is_ok(),
        "listen never reached {what} within {BOUND:?}"
    );
}

async fn within(bound: Duration, what: &str, mut done: impl FnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + bound;
    while !done() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "listen never reached {what} within {bound:?}"
        );
        tokio::time::sleep(POLL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mutation: the deadline's assert removed -- the wait then ends on
    /// `done` alone, and this test fails at its own 10 s timeout.
    #[tokio::test]
    async fn running_out_fails_naming_the_wait() {
        let waited = tokio::spawn(within(Duration::from_millis(100), "the drain", || false));
        let failed = tokio::time::timeout(Duration::from_secs(10), waited)
            .await
            .expect("the bound ended the wait")
            .expect_err("running out is a failure");
        let message = failed.into_panic();
        let message = message
            .downcast_ref::<String>()
            .map(String::as_str)
            .unwrap_or_default();
        assert!(
            message.contains("listen never reached the drain"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn the_condition_ends_the_wait() {
        let mut polls = 0;
        within(Duration::from_secs(10), "the third poll", || {
            polls += 1;
            polls == 3
        })
        .await;
        assert_eq!(polls, 3);
    }
}

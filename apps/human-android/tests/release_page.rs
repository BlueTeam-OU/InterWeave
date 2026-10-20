// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The release page owes Slint's attribution: the Royalty-free License
//! 2.0 is taken on the condition that the page the binaries are
//! downloaded from carries the "Made with Slint" badge (ADR-0057 decision
//! 3; plan section 22's carry from section 18). One page per release
//! serves every platform (architect-cto's ruling on j30's Q2,
//! 2026-10-20), so the badge is there once, for the desktop binaries and
//! the APK alike.
//!
//! What this holds is the TEMPLATE; a published page is the release
//! run's, and until the first release is published the carry stays open
//! as a `limit` ("not yet published").

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the repository root")
        .to_owned()
}

fn template() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("release/release-page.md.in"),
    )
    .expect("the release page's template")
}

/// The `slint` version the lockfile resolves, which the shipped binaries
/// are built with.
fn locked_slint() -> String {
    let lock = std::fs::read_to_string(root().join("Cargo.lock")).expect("Cargo.lock");
    let mut versions = lock
        .split("[[package]]")
        .filter(|block| block.lines().any(|l| l.trim() == "name = \"slint\""))
        .map(|block| {
            block
                .lines()
                .find_map(|l| l.trim().strip_prefix("version = \""))
                .and_then(|v| v.strip_suffix('"'))
                .expect("a version")
                .to_owned()
        });
    let version = versions.next().expect("slint is in the lockfile");
    assert!(versions.next().is_none(), "one slint in the lockfile");
    version
}

#[test]
fn the_release_page_carries_the_slint_badge_once() {
    let page = template();
    let badge = format!(
        "[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/v{}/logo/\
         MadeWithSlint-logo-light.svg)](https://slint.dev)",
        locked_slint()
    );
    assert_eq!(
        page.matches(&badge).count(),
        1,
        "the page carries the badge, linked to slint.dev, its image from the \
         Slint release the binaries are built with:\n{badge}"
    );
    assert_eq!(
        page.matches("Made with Slint").count(),
        2,
        "one heading, one badge"
    );
    // The words may wrap anywhere, so they are compared with each run of
    // whitespace as one space.
    let prose = page.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        prose.contains("under the Slint Royalty-free License 2.0"),
        "the page names the licence the badge is owed under"
    );
}

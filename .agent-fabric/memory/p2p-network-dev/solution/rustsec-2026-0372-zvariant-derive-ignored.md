---
role: "p2p-network-dev"
class: solution
topic: "rustsec-2026-0372-zvariant-derive-ignored"
description: "why deny.toml ignores RUSTSEC-2026-0372 (zvariant_derive unmaintained) and when to drop it; how a new unmaintained advisory turns every PR red"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c30f3a4c90fc66e5
---

## why deny.toml ignores RUSTSEC-2026-0372 (zvariant_derive unmaintained) and when to drop it; how a new unmaintained advisory turns every PR red

2026-10-18: RUSTSEC-2026-0372 (zvariant_derive 5.15.0, informational unmaintained, patched none) turned `tree checks` and `tool self-tests` red on every PR, whatever the PR changed: an advisory published after main's last green run fails check_dependencies.sh and test_check_dependencies.sh's "repository satisfies its own policy" assertion. The path is slint 1.18.1 -> i-slint-backend-winit (names zbus 5.19.0 directly) -> accesskit_winit -> accesskit_unix 0.22.1 -> zbus/zvariant 5, plus atspi 0.29 as a desktop-e2e dev-dependency. Not on the Android graph. zbus 6.0.0 (2026-10-16) is the fix, and the latest Slint cannot reach it, so the answer was an ignore entry like the RUSTSEC-2026-0192 one: commit 2aa72262 on for/architect-cto-01/rustsec-2026-0372, supplied to architect-cto's #297 (REPLY 01a14e88). Drop the entry at the next Slint or accesskit bump that takes zbus 6. This account has no cargo-deny, so the fold's CI is the first run of the check.

Correction, same day: the advisory was one of a batch of three (z-galaxy/zbus#2043). RUSTSEC-2026-0370 (zvariant) and -0371 (zbus_names) failed #297's first cargo-deny run after the fold. Lesson: when one crate of a family that was just split up or merged gets an advisory, scan the whole advisory-db against Cargo.lock first (a shallow clone of rustsec/advisory-db plus a python loop over lock packages, about 11 MB). Fixed in bae39f8a. Review r3 P3 (dde0a007): tests/desktop-e2e names zbus 5.19 and atspi 0.29 directly as dev-dependencies, so "no update of ours removes it" was false. The revisit is the next Slint or accesskit bump together with those pins.

*Observed 2026-10-18 (p2p-network-dev)*

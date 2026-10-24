---
role: "devex-tooling"
class: solution
topic: "ignored-row-ci-nextest-exact"
description: "wiring one #[ignore]d test into CI? use nextest -E 'test(=name)' --no-tests=fail, never cargo test -- --exact"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4da7713ba4db996d
---

## wiring one #[ignore]d test into CI? use nextest -E 'test(=name)' --no-tests=fail, never cargo test -- --exact

To run ONE #[ignore]d case in the InterWeave `rust` job, use
`cargo nextest run <Tests' own selection> --run-ignored only --no-tests=fail -E 'binary_id(<pkg>::<bin>) & test(=<path::name>)'`.
`cargo test -- --ignored --exact <name>` exits 0 with zero tests when the
case is renamed, so the step stays green while the row runs nowhere.
Measured on nextest 0.9.148 (2026-10-20, for/ branch
develop-qzapp/devex-tooling/for/rust-ui-dev-02/rollback-row-ci, a31cd702):
a non-matching filter gives exit 4 "no tests to run". Keeping Tests'
`--workspace --all-targets --profile ci` selection is meant to reuse its
build, because a `-p` run unifies features differently. A step like this
fails on any tree without the test, so it lands with the test, as a for/
branch onto the test owner's branch.

**Two such tests in one step: one nextest run each, never one ORed
`-E`.** With an OR, `--no-tests=fail` stays green when one name is renamed,
as long as the other still matches. Done for TM-49 beside S9 in
#353 (fe067e9e, 2026-10-22), where the request had asked for an OR.

**Why:** a guard that passes because it never ran is this role's most
common defect class ([[grep-q-pipefail-silent-pass]]).
**How to apply:** also, never wait with `until ! pgrep -f <script>`:
the waiter's own command line contains the pattern, so it never exits.
Wait on a sentinel line in a log file instead.

*References: grep-q-pipefail-silent-pass*

*Anchors: `.github/workflows/ci.yml::--no-tests=fail`*

*Observed 2026-10-20 (devex-tooling)*

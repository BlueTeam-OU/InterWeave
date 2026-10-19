---
role: "devex-tooling"
class: solution
topic: "rust-job-cache-and-nextest-measured"
description: "InterWeave rust CI job — cargo target cache sizes, nextest gains, and how a warm cache was measured before merge (PR #288, 2026-10-16)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5dae63d84f29ef28
---

## InterWeave rust CI job — cargo target cache sizes, nextest gains, and how a warm cache was measured before merge (PR #288, 2026-10-16)

InterWeave's `rust` job on main ran 19m22s on 2026-10-16: clippy 141 s, test build 7m05s, and 204 test binaries one after another 432 s. The job cap is 25 minutes.

PR #288 (j138) added a main-only cargo cache and `cargo nextest run --profile ci`. Measured results:
- Cold: 14m29s.
- Warm: 10m03s. Clippy took 29 s, the test build 3m36s, and 3206 tests 233 s.

Cache size:
- An unpruned `target/` with debuginfo is 49 GB.
- `tools/ci/prune_cargo_target.sh` drops the workspace's own builds. That leaves 4.7 GB, which saves as about 1.24 GB compressed.
- The repo's Actions cache was already over its 10 GB limit. Most of that is CodeQL default-setup java dependency caches, 508 MB each, mostly on pull refs.

The remaining 3m36s of the warm test build is workspace crates plus linking 204 debuginfo binaries. Debuginfo is the next lever (for example `line-tables-only` in CI), and it was not tried.

How the warm run was measured before merge:
1. A temporary commit saved the cache from `pull_request`.
2. `gh run rerun --job` gave an exact hit.
3. The temporary commit was reverted, and the PR-scoped cache was deleted with `gh api -X DELETE .../actions/caches?key=…&ref=refs/pull/N/merge`.

Two places read the CI test command line, and both had to change with it:
- `tools/checks/check_root_funnel_precondition.sh`
- xtask's `the_test_task_is_ci_s_tests_step`, which now strips `--profile ci`. xtask itself keeps `cargo test`.

Merged 2026-10-17 as 420465dd. The first main push run (39006585812, rust 13m32s) saved key rust-cargo-Linux-…-2026-W42-… on refs/heads/main at 1,244,084,044 bytes. Open follow-up: p2p-network-dev-01's j74 makes free_port cross-process and removes the end-to-end group (both e2e packages).

Related: [[ci-command-line-is-read-by-guards]].

*References: ci-command-line-is-read-by-guards*

*Anchors: `.github/workflows/ci.yml::cargo nextest run --workspace --all-targets --locked --profile ci`, `tools/ci/prune_cargo_target.sh::is_workspace`, `xtask/src/main.rs::the_test_task_is_ci_s_tests_step`*

*Observed 2026-10-16 (devex-tooling)*

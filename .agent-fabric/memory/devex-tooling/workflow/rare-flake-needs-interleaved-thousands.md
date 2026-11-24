---
role: "devex-tooling"
class: workflow
topic: "rare-flake-needs-interleaved-thousands"
description: "Before calling a rare test failure a regression of a bump, run both arms' binaries interleaved, thousands of fresh processes each; 0/1000 on the base was luck"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5d67a7ee9d49ab5e
---

## Before calling a rare test failure a regression of a bump, run both arms' binaries interleaved, thousands of fresh processes each; 0/1000 on the base was luck

j328 (#637, rusqlite 0.42 / SQLCipher 4.20): after the bump, `crates/human/store/tests/trust_boundary.rs` failed once in a run of the store suites. These were the measurements, in order:
- branch 4/1300, main 0/1000. That looked like a regression, and I said so to the store owner.
- Interleaved, 3000 runs each: main 6, branch 8, with the same error on both. So it was pre-existing, and the earlier 0/1000 was luck.

What made it tractable:
- Run the built test binary directly (`target/debug/deps/<test>-<hash>`), not through cargo.
- Run each arm's binary alternately in one loop, so machine load hits both equally.
- `--test-threads=1` as a control: 0/1500 showed it needed two threads.
- Build the base arm in a scratch `git worktree` of origin/main with its own CARGO_TARGET_DIR (~4 min for the store crate). Remove both afterwards.

The cause, found by rust-ui-dev-01 (their j263): SQLite's `sqlite3_initialize` sets isInit before SQLCipher's extra init finishes, outside the init mutex. So it races once per process, which is why fresh processes find it and an in-process loop never does.

**Why:** a regression claim sent on thin numbers sends another role chasing the wrong cause.
**How to apply:** for any intermittent failure that appears with a dependency bump, measure base against branch interleaved, at least 3000 fresh processes per arm, before saying "regression". Report the numbers you have as unconfirmed until then. See [[lock-bump-stale-index-and-range-edges]].

*References: lock-bump-stale-index-and-range-edges*

*Anchors: `crates/human/store/tests/trust_boundary.rs::a_desktop_profiles_store_is_the_human_directorys_store_file`*

*Observed 2026-11-12 (devex-tooling)*

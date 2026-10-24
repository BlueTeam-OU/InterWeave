---
role: "p2p-network-dev"
class: workflow
topic: "run-whole-crate-not-a-filter"
description: "before pushing a change to a crate, run its whole lib suite and clippy -D warnings, never a test-name filter — source-pin and guard tests live elsewhere in the crate"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - b7106d1681bfaa65
---

## before pushing a change to a crate, run its whole lib suite and clippy -D warnings, never a test-name filter — source-pin and guard tests live elsewhere in the crate

On #280 (j30, 2026-10-14) I ran `cargo test -p interweave-transport-libp2p --lib -- live_dns resolver` and pushed. Two unrelated tests in the same crate broke and the filter hid them. One was a source pin that `find`s a function by name (`the_runtime_transport_is_the_port_fallback_s`): it broke when I renamed `start_with_resolver`'s body. The other was the dialing guard, which refuses an out-of-file `#[cfg(test)] mod x;` in runtime/mod.rs. CI's clippy also refused `Err(_) =>` in a new test, because my local clippy had run before the test file existed.

**Why:** this crate pins its own source text and module layout in tests, so a rename or a module move breaks a test whose name has nothing to do with the change.

**How to apply:** before the first push, run `cargo clippy -p <crate> --all-targets -- -D warnings` and the crate's whole `--lib` suite, no filter. Keep runtime test modules inline in runtime/mod.rs. Then run `cargo xtask ci` before arming (see [[full-workspace-tests-before-arm]]).

*References: full-workspace-tests-before-arm*

*Observed 2026-10-14 (p2p-network-dev)*

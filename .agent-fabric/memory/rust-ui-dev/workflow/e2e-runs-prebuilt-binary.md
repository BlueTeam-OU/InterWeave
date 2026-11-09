---
role: "rust-ui-dev"
class: workflow
topic: "e2e-runs-prebuilt-binary"
description: "desktop-e2e launches target/<profile>/<bin> as last built; reverting source and running one e2e test tests the OLD binary unless the app crate is rebuilt"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cb5a516fbd15c218
---

## desktop-e2e launches target/<profile>/<bin> as last built; reverting source and running one e2e test tests the OLD binary unless the app crate is rebuilt

`tests/support/src/e2e/mod.rs` finds the app as `target/<profile>/<name>`, next to the test's
own `deps/` directory. `cargo test -p interweave-desktop-e2e-tests` does NOT rebuild
`human-desktop`. On 2026-10-26 (#453), I reverted sources twice to bisect a `human_app` failure,
and both runs tested the head binary built at 00:05. That sent me down a wrong lead (timezone,
midnight) for half an hour.

**Why:** a bisect that does not rebuild what it runs measures nothing, and it reads as a real
result.

**How to apply:**
- After changing source, run `cargo build -p interweave-human-desktop` (or the app in question)
  before a single e2e test.
- Check the binary's mtime (`ls -la --time-style=+%H:%M target/debug/human-desktop`) against the
  edit.
- A full `cargo test --workspace` builds it.

Related: [[no-commit-during-suite]].

*References: no-commit-during-suite*

*Anchors: `tests/support/src/e2e/mod.rs::current_exe`*

*Observed 2026-10-25 (rust-ui-dev)*

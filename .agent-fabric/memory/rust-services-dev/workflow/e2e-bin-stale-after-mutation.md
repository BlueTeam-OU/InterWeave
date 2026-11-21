---
role: "rust-services-dev"
class: workflow
topic: "e2e-bin-stale-after-mutation"
description: "desktop-e2e runs the prebuilt claude-channel binary; after reverting a crate mutation, rebuild it before the next run or every later run tests the mutant"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - d030ba2c34c498be
---

## desktop-e2e runs the prebuilt claude-channel binary; after reverting a crate mutation, rebuild it before the next run or every later run tests the mutant

`tests/desktop-e2e` finds the bridge (and the daemon) through `workspace_binary`: the file beside the test in `target/<profile>/`. `cargo test -p interweave-desktop-e2e-tests` does NOT rebuild another package's binary. So after `git checkout` reverts a mutation in `crates/human/chat-protocol` (or any crate the bridge links), the binary is still the mutant until `cargo build -p interweave-claude-channel` runs again.

Seen on PR #647 (j69, 2026-11-12). After mutation 2 (the parser admitting 65 scalars) was reverted, the next "normal" run still read `dropped_cards.name: 0`. It passed once the bridge was rebuilt.

**Why:** a stale mutant makes a correct test look broken, or a broken one look correct, and nothing in the output says which binary ran.

**How to apply:** script every mutation as build, then run, then revert, then build again. Begin every control run with `cargo build -j 2 -p interweave-claude-channel -p interweave-transport-daemon`. A second lesson from the same PR: a check that verifies a signature against the same variable that was signed is a tautology. Verify against bytes rebuilt from what the envelope carries. Related: [[mutation-runs-save-the-fix-first]].

*References: mutation-runs-save-the-fix-first*

*Anchors: `tests/support/src/e2e/mod.rs::workspace_binary`*

*Observed 2026-11-12 (rust-services-dev)*

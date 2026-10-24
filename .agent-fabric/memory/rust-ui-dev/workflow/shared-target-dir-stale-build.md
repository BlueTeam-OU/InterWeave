---
role: "rust-ui-dev"
class: workflow
topic: "shared-target-dir-stale-build"
description: one CARGO_TARGET_DIR shared by two InterWeave worktrees can link a stale workspace crate; a compile error naming a variant that exists in source is the sign
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 416868337c5c5864
---

## one CARGO_TARGET_DIR shared by two InterWeave worktrees can link a stale workspace crate; a compile error naming a variant that exists in source is the sign

On 2026-10-19, #313's head 04605e70 in ~/projects/interweave-j111-f10 was built with CARGO_TARGET_DIR pointing at the j111-android-recovery worktree's target. desktop-e2e then failed to compile: E0599 on UiText::StateShortDirect and AppOptions, although both are in crates/human/ui-model/src/labels.rs at that head. Cargo had reused the other worktree's ui-model build. Its fingerprint is mtime-based, and the two worktrees evidently share unit hashes. After `touch crates/human/ui-model/src/labels.rs`, ui-model was recompiled, and human_app passed 19/19.

**Why:** a stale unit gives false failures and, worse, false passes.

**How to apply:** give each worktree its own target (the shared one in interweave-j111-android-recovery was removed 2026-10-20 at 133G when /home hit 98%), or touch the changed crates before a suite run on a shared one. Don't believe a red or a green from a shared target until it has rebuilt the changed crates (look for their "Compiling" lines). Remove a target once its work lands.

*Observed 2026-10-20 (rust-ui-dev)*

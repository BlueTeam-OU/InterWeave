---
role: "rust-ui-dev"
class: workflow
topic: "ui-slint-change-runs-desktop-app-tests"
description: "A ui-slint or app.slint change must also run apps/human-desktop's own test binaries (window_faults etc.), not only the ui-slint and desktop-e2e suites"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d03fd45b128bb517
---

## A ui-slint or app.slint change must also run apps/human-desktop's own test binaries (window_faults etc.), not only the ui-slint and desktop-e2e suites

On #521 (2026-10-31) I ran the whole ui-slint suite, desktop-e2e, app-core and store before pushing, and CI's `rust` job still went red on `apps/human-desktop/tests/window_faults.rs` (`at_the_session_bound_the_limit_row_asks_and_asks_again_after_a_reopen`): that package has test binaries of its own, through the facade thread, which no other suite runs.

**Why:** a fix that ended a needless per-render row reset (dddea518) exposed a real view defect (a row arriving above the top's line kept out of view by the anchor at content-y -5 px, the list's padding) that only that binary exercised; the PR had already been armed on architect-cto's word and the push to fix it took the arming off.

**How to apply:** before pushing a change under crates/human/ui-slint or app.slint, run `tools/ci/with_display.sh cargo test -p interweave-human-desktop -p interweave-human-ui-slint --features interweave-human-ui-slint/desktop --no-fail-fast` plus desktop-e2e (binaries rebuilt). A temporary worktree sharing CARGO_TARGET_DIR left a stale path-dependency artifact (E0599 on a new store method): `cargo clean -p <crate>` after. See [[slint-row-init-reads-layout]], [[e2e-runs-prebuilt-binary]].

*References: e2e-runs-prebuilt-binary, slint-row-init-reads-layout*

*Anchors: `apps/human-desktop/tests/window_faults.rs::at_the_session_bound_the_limit_row_asks_and_asks_again_after_a_reopen`*

*Observed 2026-10-31 (rust-ui-dev)*

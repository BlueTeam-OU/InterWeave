---
role: "rust-ui-dev"
class: solution
topic: "j284-held-call-pr695"
description: "j284 / PR #695 — acceptance 10 superseded cases on a held admin call (desktop-e2e superseded.rs) + TM-45's four log lines; state and how the hold works"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3dfc05547f9bf0ea
---

## j284 / PR #695 — acceptance 10 superseded cases on a held admin call (desktop-e2e superseded.rs) + TM-45's four log lines; state and how the hold works

j284 (architect-cto REQUEST 01a1e54b, 2026-11-16) is PR #695, branch develop-qzapp/rust-ui-dev-01/feat/j284-interweave-next-job-the-hold-the.

- The hold is test-crate only: `tests/desktop-e2e/tests/human_app/superseded.rs` wraps `IpcBinding` as an `AdminBinding` (`Holding`/`HeldPort`). The real port opens first (the daemon stamps admission order), the write waits on a oneshot until the case has run `transportctl`'s write, then goes on the older connection → refused `superseded`. Armed with `fail_read_back`, the first list/trust read after the write returns `BackendUnavailable` without reaching the daemon.
- Witness: the daemon's `interweave::audit` line with `outcome="superseded"` in `world.a_daemon.log()`. Without it, add-vs-add, remove-vs-remove, trust read back trusted and undo read back listed pass identically when the write is simply applied.
- Control: move `hold.wait` into `Holding::admin` before the port opens → every case fails at the audit assertion.
- A one-block step-2 Trust decided trusted closes the add page with `PeersSnack::PeerTrusted` (§5.3); `blocks()` is then empty.
- TM-45: app-core `test-hooks` gained StartListing/UnreadListing/OwnCards/ShareRecord faults; `apps/human-desktop/tests/log_lines.rs` holds all seven `App::pump` failure lines.
- DONE 2026-11-16: #695 merged 0ac77f044 (5 work, 2 fix) on architect-cto AUTHORIZES 01a1e619; eleven superseded cases run, five kinds named not run; round-2 re-review clean. Risk carried: the failed-read-back add case passes the same if the failure stopped reaching it.

*Anchors: `tests/desktop-e2e/tests/human_app/superseded.rs::Holding`, `crates/human/app-core/src/test_hooks.rs::set_share_record`*

*Observed 2026-11-16 (rust-ui-dev)*

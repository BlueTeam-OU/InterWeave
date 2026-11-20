---
role: "rust-ui-dev"
class: solution
topic: "j283-held-start-cases-tm45"
description: "j283 / PR #688 — held-start host tests (full queue loses the stop) and TM-45 desktop log-line tests via App::with_log; what is unreachable on the host"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 305c19e4aff7bce5
---

## j283 / PR #688 — held-start host tests (full queue loses the stop) and TM-45 desktop log-line tests via App::with_log; what is unreachable on the host

DONE: PR #688 merged f99496967 (2026-11-16) on architect-cto's word (5 work, 4 fix). architect-cto: no new hook for the 4 unreached lines; the TM-45 "one log" wording is theirs to fix; the full-queue finding awaits their ruling.

- Held start (add-a-peer §5.1), tests in `crates/human/android-platform/src/view_side.rs`, driven through the real Hub:
  - A FULL hub queue (`TO_VIEW` 256) loses the stop. `Viewer::send` drops the viewer, so `retire`'s `Running(false)` is never queued for it. The reattached view hears `Running(false)` first and records no stop.
  - So `start_ended` stays false, and the return's start is judged by the held screen, unlike #678's rule. Reported to architect-cto (REPLY 01a1e276), NOT changed. A possible fix: the hub keeps the last stop's count and says it at attach.
  - Held-start tests must use a model that owes Connect (`connect_owed()`). Otherwise `start_decided` is true as soon as the service runs (#688 review F1).
- TM-45: `App` (`apps/human-desktop/src/app.rs`) logs through `log: fn(&str)`, and `App::with_log` lets a test read the lines. The tests are in `apps/human-desktop/tests/log_lines.rs`.
  - No hook reaches ListingFailed, UnreadNotListed, CardsNotRead or ShareNotRecorded: no app-core test hook makes `start()` or a later store read fail. Forcing them needs a new test hook.

Related: [[j280-held-start-ended]].

*References: j280-held-start-ended*

*Anchors: `crates/human/android-platform/src/view_side.rs::a_stop_lost_with_a_full_queue_leaves_the_return_to_the_held_screen`, `apps/human-desktop/src/app.rs::with_log`*

*Observed 2026-11-16 (rust-ui-dev)*

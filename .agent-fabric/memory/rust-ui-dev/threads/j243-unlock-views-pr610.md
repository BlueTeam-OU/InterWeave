---
role: "rust-ui-dev"
class: threads
topic: "j243-unlock-views-pr610"
description: "j243+j245 / PR #610 unlock note views; armed 386217175 (10 work, 2 fix); APK a host scratch path; lessons"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cab516d2f5769fac
---

## j243+j245 / PR #610 unlock note views; armed 386217175 (10 work, 2 fix); APK a host scratch path; lessons

PR #610 (j243, ux REQUEST 01a1c3b5; j245 ux OBSERVATION 01a1c3c6 rides it) armed 2026-11-10 at
386217175 (10 work, 2 fix). Review r1 F1-F3 real (adjudicated), fixed cdf56973b, re-reviewed fixed;
format-only 386217175 re-reviewed. Final English from language-culture-en (REPLY 01a1c43c, their j263).
Debug APK at cdf56973b: A host scratch path (sha e5837110…), INFO 01a1c48b to ux; A11 is ux's.

Lessons:
- A subagent/slint test of a FocusNote made by the same render: the testing backend instantiates
  repeated elements only on layout; query the tree (ElementQuery::find_first) before settle().
- init_no_event_loop panics if called twice on one thread; a test making several views needs a
  thread_local guard (views.rs `unlock_view`).
- Rust's "shown" for a notice must copy the window's own placement (narrow/list-shown/offline), or
  focus handover and the trace disagree with what is drawn (r1 F2/F3).
- ViewSide::show_kept must re-say every hub-held fact (r1 F1).
- cargo fmt per crate missed app-core tests: run `cargo fmt --all -- --check` before pushing.
- A session commits only in its own job's working copy (hook); j244 left uncommitted, see [[j244-608-f3-f4]].

*References: j244-608-f3-f4*

*Observed 2026-11-10 (rust-ui-dev)*

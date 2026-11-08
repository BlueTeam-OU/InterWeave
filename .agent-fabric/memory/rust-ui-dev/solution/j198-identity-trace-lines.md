---
role: "rust-ui-dev"
class: solution
topic: "j198-identity-trace-lines"
description: "ui-trace vocabulary for identity/locked state (connectivity=, notice=, identity=, prompt=); supplied on for/p2p-network-dev-02/identity-trace"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 92e01174d4cf85e1
---

## ui-trace vocabulary for identity/locked state (connectivity=, notice=, identity=, prompt=); supplied on for/p2p-network-dev-02/identity-trace

j198 (2026-11-03) supplied 603c9c8a..0b1b64b3 onto p2p-network-dev-02's feat/j17 (REPLY 01a1a299).
- `connect=` is onboarding Connect's phase; network state is `connectivity=`; `notice=` is the session notice.
- `identity=<screen>` comes from onboard.rs render via `View::trace_identity` (show_identity takes a page, not the flow); store.rs's screens are not traced.
- `prompt=phrase|unlock|granted|cancelled` are logged by onboard.rs; the cold-launch unlock prompt is the Activity's own (MainActivity onStart) and has no asked line.
- No distinct unlock-cancelled or locked-return state: the notice holds from launch, so the trace (change-only) says nothing on return.
- A fresh UiModel's session notice is `reconnecting`, not none: a host test of "unlocked" sees notice=reconnecting.
- Settings' Show recovery phrase focus key is `show-phrase`; it opens S1 before any prompt.

*Anchors: `interweave:crates/human/ui-slint/src/trace.rs`, `interweave:apps/human-android/src/onboard.rs`*

*Observed 2026-11-03 (rust-ui-dev)*

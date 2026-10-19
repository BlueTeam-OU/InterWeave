---
role: "rust-ui-dev"
class: rationale
description: "where Android's Stay reachable choice lives (architect-cto DECISION 01a124ed, 2026-10-10) and what the shell must do with it"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8607220a14525f3a
---

## where Android's Stay reachable choice lives (architect-cto DECISION 01a124ed, 2026-10-10) and what the shell must do with it

Ruled by architect-cto on 2026-10-10 (DECISION 01a124ed-a425), in answer to my QUESTION 01a124ec-8971.

Where the choice lives:
- Stay reachable is a PERSISTED OVERLAY entry in the profile's state directory, a sibling of the trust overlay.
- config.yaml `runtime.android.availability_mode` is only the authored default. It is never rewritten.
- One write path: an embedded-host API (EmbeddedHost::set_availability, or whatever name is agreed with p2p-network-dev-01).
- profile-config exposes `effective_availability_mode()` = overlay if present, else config. Everything reads that function: the Service's start decision, background_restart_requires_user_authentication, and the diagnostic.

What the shell must do:
- The switch writes only on the person's tap.
- Turning it off REMOVES the entry. It never writes foreground-only.
- The Service reads the effective mode at start, and on a change event from the host. A flip while running restarts the Service in the new mode.

Not ruled: an operator policy forbidding persistent reachability.

**Why:** two sources (a config field and an app preference) would disagree exactly where ADR-0041's explicit opt-in must be provable.

**How to apply:** architect-cto supplies the record text: human-client-android.md, an ADR-0041 amendment, and an ADR-0028 sentence. Until the host API lands, only the debug-only stand-in profile sets the mode, for the ADR-0041 run.

Related: [[embedded-host-seam]] [[android-shell-build]]

*References: android-shell-build, embedded-host-seam*

*Observed 2026-10-10 (rust-ui-dev)*

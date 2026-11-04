---
role: "rust-ui-dev"
class: threads
topic: "j159-stage21-gate-items"
description: "j159 — architect-cto's five s29 gate items (REQUEST 01a1716e); all five merged in #436 (08ad14be, 2026-10-25); CodeQL alert 95 still open on the owner's choice"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - aed0e4269339c990
---

## j159 — architect-cto's five s29 gate items (REQUEST 01a1716e); all five merged in #436 (08ad14be, 2026-10-25); CodeQL alert 95 still open on the owner's choice

j159 (REQUEST 01a1716e from architect-cto-01, 2026-10-25). PR #436, branch
develop-qzapp/rust-ui-dev-01/feat/j159-stage-21-close-five-gate-items
(worktree interweave-j159-stage-21-close-five-gate-items).

All five items on #436 at 402f8a80 (8 work, 3 fix, 2 merge) — at the
minimum, so no architect-cto word needed to arm:
- items 1, 2: manifest camera required="false"; allow-list and camera-API
  rules in check_android_contacts_and_backup.py (fail closed on a missing
  root or unreadable source; reviews r1-r3).
- items 4, 5: desktop-e2e human_app/codes.rs (ModelSide/FacadeSide over IPC;
  Show read from window pixels).
- item 3: apps/human-android/device-runs/2026-10-25-samsung-a40-camera.md
  (A40 API 30, owner's "go ahead" for pm revoke; API 30 pm has no
  clear-permission-flags).
- 96cc5791: external v0.1.0 review P3 (via p2p-network-dev-01, 01a17319):
  desktop clipboard image reader removed, Paste ends cancelled; replied
  01a1731c, sha sent 01a1738c.

A40 driving lessons: each step's adb connect puts a "Wireless debugging
connected" banner over the top ~200 px for a few seconds; a tap there opens
Developer options — wait 8 s before tapping. The app requests
POST_NOTIFICATIONS at every launch even on API 30 (no-op window).
Merged 2026-10-25 15:58 UTC as 08ad14be (armed on the count rule after a
clean round 4; REPLY to architect-cto 01a173bb). Still open: CodeQL alert 95
(codes.rs:158) — the owner to choose dismiss or print the length. Follow-up
j161: askNotifications requests POST_NOTIFICATIONS below API 33.
Related: [[a40-own-adb-server]] [[pixel-no-destructive]] [[key-driven-ui-settled-focus]]

*References: a40-own-adb-server, key-driven-ui-settled-focus, pixel-no-destructive*

*Observed 2026-10-25 (rust-ui-dev)*

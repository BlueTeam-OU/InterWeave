---
role: "architect-cto"
class: threads
topic: "interweave-521-records-to-place-2026-10-31"
description: "InterWeave #521 (rust-ui-dev-02, armed on my word 2026-10-31 at 75229a78): two records set aside for me to place — the j78 unread-cards figures beside SPIKE-011's read-walk row, and the Samsung A40 row of…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ac4168cde3ffdeb5
  - fa884fb83d52e50e
---

## InterWeave #521 (rust-ui-dev-02, armed on my word 2026-10-31 at 75229a78): two records set aside for me to place — the j78 unread-cards figures beside SPIKE-011's read-walk row, and the Samsung A40 row of device-runs/2026-10-30-windowed-start.md not re-measured for j77 (Android unverified)

#521 (6 work, 2 fix), armed from my session on 2026-10-31 06:42Z after
reading head 75229a78 and its three blind reviews (r1 F1 fixed, r2 F2 P3
fixed, r3 clean); boundary files crates/human/store/src/store.rs and its
test: one new read, `unread_cards_page` over the v10 kind index, no schema
change (agreed in 01a18ecf). REPLY 01a190b2 carried the AUTHORIZES header.

That arm did not land: CI's `rust` check failed one window_faults test,
bisected to dddea518 (rows no longer re-set each render had hidden a view
defect: focus onto the top's line leaves the 5 px padding, so the anchor
follow read the list as away from its top). a7cbcb0f adds `top-shown`
(padding-tolerant) for the anchor branch only, at-top untouched; 613496c7
its comment (F3). Re-armed on my word at 613496c7 2026-10-31 08:2xZ,
#521 (7 work, 3 fix) green, queued at 1; REPLY 01a1910a. Lesson for the
requester, theirs: run the package's own test binaries before a push.

Two records rust-ui-dev-02 named as mine to place, not part of the arm:

- j78 figures beside SPIKE-011's read-walk row: full unread walk 5.8 ms at
  2 000 rows (baseline 5.3) and 57 ms at 20 000; the card read 0.08 / 0.11 ms.
  PLACED 2026-10-31 in SPIKES.md's SPIKE-011 bullet "The baseline's read,
  restated on the window (A 2026-10-31)", on IW #523 (j458, 4c2dfc50),
  labelled as a scratch harness's and not the spike's.
- The Samsung A40 row in apps/human-android/device-runs/2026-10-30-windowed-start.md
  (rows about 10 s after a tap) is not re-measured for j77; Android unverified.
  A device window, when one opens, re-measures it.

**How to apply:** the A40 row stays owed to the device-runs record when
it is next touched (or a device window re-measures it); the plan's P2-1
close (#523) names it as open, so it is no longer a silent carry.

*Observed 2026-10-31 (architect-cto)*

---
role: "ux-designer"
class: threads
topic: "j88-identity-keyboard-answers"
description: "j88 state — answers to rust-ui-dev-01's j199 keyboard questions on android-identity s7/s8 (Back on the discard dialog, Enter/Space typing vs navigating, R2 grid order), owed as a spec revision"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 012a7d283cd7f711
---

## j88 state — answers to rust-ui-dev-01's j199 keyboard questions on android-identity s7/s8 (Back on the discard dialog, Enter/Space typing vs navigating, R2 grid order), owed as a spec revision

j88 (rust-ui-dev-01 QUESTION 01a1a324, their j199 branch
feat/j199-identity-screens-design-8-keyboard-reach at 69254de8).
Answered by REPLY 01a1a331-9c3f (2026-11-03), read against InterWeave
main 647e37e3, design/2026-10-19-android-identity.md. Written into the spec as #543 (1e21a0ba, 2026-11-03; adds Backspace =
delete key, a screen opens as typing, focus after Delete words). Was owed: write these
into the spec (s7, s8, s10 acceptance 15) on branch
feat/j88-identity-design-s7-s8-three-keyboard, then name the PR to them.
- Q1: Back (platform or Escape) or a tap outside on the open discard
  dialog = Continue entering; only Delete words drops. Focus returns to
  the control focused when Back was pressed (a key opened it, not a
  control).
- Q2: keyed to the last input. Typing (hardware letter, Backspace, or
  Enter by the rule): Enter takes the only suggestion or the whole-word
  chip, else nothing; Space does nothing. Navigating (Tab, arrow, tap,
  press of a control): Enter and Space press the focused control. Focus
  ring drawn in both.
- Q3: the spec already answers. s4 puts the grid under the keypad (item
  4), s6 the checksum banner above the grid, and s8's focus order follows
  that reading order. The build draws the grid above, which is a
  deviation. Order: chips, keys, delete, clear, banner, grid (or Show all
  my words over 130 %), primary, secondary, tertiary. If they name a
  constraint (the 200 % fit in acceptance 13, the scrolling area), revise
  s4/s8 openly.
- C3's Back dialog was not dropped (s7 "R2, C3"); they build it as a
  separate job.
Related: [[j39-add-peer-unknown-retries]]
- 2026-11-03 later: rust-ui-dev-01 REPLY 01a1a333 named the Q3 height
  constraint (~800 dp vs ~570 at 100 %); spec revised on #543 4ea6c843:
  portrait R2 = scrolling area (notice, grid) above a fixed picker.
  Arrows now required (spatial). Answered by REPLY 01a1a33f (provisional
  until #543's review clean). Carried on #543: Back from K1-opened R2,
  first focus on R3/C4/R2-from-R3, focus after a C3 wrong word.
- #543 round 6 clean on 720abbac (2 work, 5 fix). Arm needs the owner's
  word (under 8) and `--defer "720abbac:F5" "<why>"`: F5's rest (R3, C4,
  R2-from-R3 first focus) is pre-existing, carried in PR comment
  6615610379. After merge: tell rust-ui-dev-01 the merge commit.
- j202 (QUESTION 01a1a34d, "Show all my words" list): specified on #543
  bfe71023 (secure full-screen list, rows edit, app bar arrow not a
  Close button); REPLY 01a1a34f sent, provisional until round 7 clean.
- #543 round 9 clean on 7f5e0b31 (3 work, 7 fix); j202 replies were provisional — confirm to rust-ui-dev-01 once merged.
- IdBack: spec reuses AddClose (51519bc3); OBSERVATION 01a1a383 + correction INFO 01a1a384 (KeyMakeNew precedent) await rust-ui-dev-01. Owed next spec change: soften the AddClose row's 'test does not allow for' reason.
- rust-ui-dev-01 REPLY 01a1a385: drops IdBack, reuses AddClose (their j205). Settled.
- rust-ui-dev-01 INFO 01a1a3b4: acceptance 15 focus rules built on feat/j203 11dc8a51 (host traces only). Unbuildable yet: C2 PeerId page missing (Back from C3 focuses IdWrittenDown), no R2/C3 landscape. Device review owed once built.
- DELIVERED 2026-11-04: #543 merged caca4154 on the owner's word (4 work, 8 fix; F5 remainder deferred by --defer 1e21a0ba:F5 — a defer names the head the finding was RAISED on). INFO 01a1a3ec to rust-ui-dev-01. Owed: device run of acceptance 13/15 when C2's PeerId page and R2/C3 landscape exist; soften the AddClose row's reason in the next change.

*References: j39-add-peer-unknown-retries*

*Observed 2026-11-04 (ux-designer)*

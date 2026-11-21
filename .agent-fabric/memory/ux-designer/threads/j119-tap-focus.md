---
role: "ux-designer"
class: threads
topic: "j119-tap-focus"
description: "j119 — delivered: #645 merged 6c9327055 (tap is typing, no focus move); carried C3 chip vs banner focus, 24th-word target"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5ca90d338edbff62
---

## j119 — delivered: #645 merged 6c9327055 (tap is typing, no focus move); carried C3 chip vs banner focus, 24th-word target

rust-ui-dev-01 QUESTION 01a1d095 (2026-11-12, #644): on R2/C3 a tap presses a key and sets navigating, but focus stays
(fresh R2: on q), so a hardware Return after tapping t presses q. Options to decide: move focus to the tapped control, or
treat a tapped letter/delete/clear as typing (Enter then takes the single suggestion/whole-word chip). Also write focus
after a tap on a chip (already: first enabled key) and on a position. Q2: §4 item 2 says "a chip the person taps": make it
"tap or Enter" (intent: nothing fills itself). My REPLY 01a1d097-ccc0 promised the answer in
design/2026-10-19-android-identity.md on a PR after #642, and said to keep current focus behaviour until then.
Related: [[j88-identity-keyboard-answers]], [[answer-in-spec-before-message]].

2026-11-12: answered on PR #645 (1 work, 1 fix). Rule: a tap on an enabled key/delete/clear = typing; no tap moves focus;
a tap on text or nothing leaves the mode; a chip fill or position choice by any input = typing, focus to first enabled key;
a focused chip removed by letters -> first enabled key. Round 1 had 5 real findings (my Android touch-mode claim was false:
Views clear focus on touch). Round 2 clean; risks carried in a PR comment. Provisional REPLY 01a1d0cd sent to rust-ui-dev-01.
Next: LC-en REQUEST 01a1d0ce, fold, re-review the fold, ask the owner to arm (under 8 work commits).
Later 2026-11-12: LC j299 (a83cdc253) and j300 (ea79c410c) were folded, and my answers to LC questions a-c went in at 7230131ea.
Round 3 caught the revision note's flat "a tap does not move focus", fixed in 8b7d40754. Round 4 was clean. Armed on the
owner's word, asked in session. Carried: C3 chip-focus vs banner-focus, no target after the 24th word, the device run.

*References: answer-in-spec-before-message, j88-identity-keyboard-answers*

*Observed 2026-11-12 (ux-designer)*

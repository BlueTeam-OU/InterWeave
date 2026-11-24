---
role: "ux-designer"
class: rationale
topic: "sr-activate-is-press"
description: "Identity §8 modes — a screen reader's activate (TalkBack double-tap, Orca activate) on a keypad key counts as a press (navigating), not a tap; write into §8 in j121"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c0ad7c874906fb9f
---

## Identity §8 modes — a screen reader's activate (TalkBack double-tap, Orca activate) on a keypad key counts as a press (navigating), not a tap; write into §8 in j121

2026-11-13, answer to rust-ui-dev-01 QUESTION 01a1d261 (my REPLY 01a1d262): the accessibility
default action on a letter key, delete or clear sets navigating, as the build already does
(branch develop-qzapp/rust-ui-dev-01/feat/j266-j119-tap-focus-and-4-item). Why: typing mode is for
a finger tap that chooses a key without moving focus; a screen-reader user moves focus first, then
activates, which is Enter's pattern; with Orca passing Enter through, the "pp" case must hold.
Chips, grid positions and Try again keep their own exceptions however activated.
Built on rust-ui-dev-01 #654 (head 40b1d6a31, REPLY 01a1d2c2); my phone review of it is owed once on main or that head.
Written into §8 on #655 (j121, [[j121-launch-rule-pr655]]); TalkBack (j105) and Orca runs stay hypotheses.
list in j121's change (j121's title names only add-a-peer and item 15; add this).
Related: [[j119-tap-focus]], [[connect-owed-since-launch]].

2026-11-13: #654 merged as 41bb631b9 (REPLY 01a1d3b6). Its host test for acceptance 15 is `a_screen_reader_activate_then_enter_presses_the_key_again` in trace.rs: "ra", b activated, "rabb". The test's WORDS fixture has no word starting "ap". The spec's "ap"/"app" example still holds for the device, because the real BIP39 list has apart, apple, april and others; no spec change.
Owed: a phone review of #654 from any main build: a tap on a disabled control, and a tap followed by Return.

*References: connect-owed-since-launch, j119-tap-focus, j121-launch-rule-pr655*

*Observed 2026-11-13 (ux-designer)*

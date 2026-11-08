---
role: "rust-ui-dev"
class: solution
topic: "j201-c3-discard-dialog"
description: "j201 C3 Back asks the discard dialog, pushed 90cd9adb on feat/j201 stacked on j199 c3bbb1c5; rides j199's PR; ux 01a1a33f added acceptance-15 items (j203)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e48f003b0b5c083c
---

## j201 C3 Back asks the discard dialog, pushed 90cd9adb on feat/j201 stacked on j199 c3bbb1c5; rides j199's PR; ux 01a1a33f added acceptance-15 items (j203)

j201 (2026-11-03): branch develop-qzapp/rust-ui-dev-01/feat/j201-identity-design-s7-c3-s-back, pushed at 90cd9adb, stacked on j199's c3bbb1c5 (which merges e87e1462, not yet on main). It joins j199's PR, one PR per agent.

- ui-model d8100416: `Check` carries `discard_asked`; C3 "words entered" = a right word taken (`done > 0`) or letters typed. Back on the open dialog = Continue entering. Delete words → C2, and C3 asks both positions again. `discard()` now acts only on an open dialog (R2 too).
- ui-slint 90cd9adb: trace test `c3_back_with_words_entered_asks_the_discard_dialog`. The view needed no change because the dialog binds `flow.discard_asked()` on any screen.
- Mutants: 4 of 4 caught by the unit test; the view test catches 2.
- ux-designer-01 REPLY 01a1a33f answered j199 Q3: the grid stays above the picker. The spec is revised on #543 (provisional until its review is clean). It adds acceptance-15 work: closing the dialog restores the typing/navigating mode of the moment Back was pressed; R2/C3 open with focus on the first enabled key in typing mode; Back from C3 focuses IdPeerIdWritten, and from R2 (via R1) IdEnterWords; arrow-key directional focus; Space/no-op Enter/Back keep the mode. Queued as j203.

Related: [[j199-identity-keyboard-reach]]

*References: j199-identity-keyboard-reach*

*Anchors: `crates/human/ui-model/src/identity_flow.rs::Check`, `crates/human/ui-slint/tests/trace.rs::c3_back_with_words_entered_asks_the_discard_dialog`*

*Observed 2026-11-03 (rust-ui-dev)*

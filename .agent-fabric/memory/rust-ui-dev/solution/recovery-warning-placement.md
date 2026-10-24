---
role: "rust-ui-dev"
class: solution
topic: "recovery-warning-placement"
description: "Where the Android identity flow's pre-phrase statements go (R1/K1, not R2) and who edits the matrix row that cites their tests"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4c94cd8b468f7eed
---

## Where the Android identity flow's pre-phrase statements go (R1/K1, not R2) and who edits the matrix row that cites their tests

The Android identity flow's statements owed "before the phrase is taken"
(human-client-ui.md §9: ADR-0043's active-elsewhere warning, item 11; the
new-keys line, item 12) sit on R1 and on K1 when it offers a restore — the
screens every path into R2 passes — never on R2, whose keypad, suggestions
and primary must fit at 200 % (design acceptance 13). The model says which
screen carries one (`IdentityFlow::warns_active_elsewhere`), and
ui-slint's `identity::page` appends it as the last paragraph. ux-designer
confirmed this placement on #391 (2026-10-23); #392 built item 11.

Tests that hold it: `every_path_into_r2_passes_a_screen_that_warns` and
`no_screen_that_shows_or_takes_a_word_warns` (ui-model identity_flow.rs),
`r1_and_k1_show_the_active_elsewhere_warning_last` (ui-slint identity.rs).
A positive-path test alone let the predicate widen to word screens
unnoticed (#392 review F1): assert absence on C1-C4, R2-R4, S1, S2 too.

The Stage 18 matrix (tests/SECURITY-MATRIX.md) rows TM-48 etc. that
architect-cto asks about are edited by architect-cto on their docs PR:
send the test names; don't edit the row on the code PR (#392 F2).

language-culture-en may send a corrected text minutes after the first
delivery when a neighbouring key changes (IdActiveElsewhere, 01a16713 →
01a16717): read the inbox before arming. Item 12 is job j46 after #391
lands. Related: [[labels-keep-is-reserved]], [[identity-key-rulings-j37]].

*References: identity-key-rulings-j37, labels-keep-is-reserved*

*Anchors: `crates/human/ui-model/src/identity_flow.rs::warns_active_elsewhere`, `crates/human/ui-slint/src/identity.rs::IdActiveElsewhere`*

*Observed 2026-10-23 (rust-ui-dev)*

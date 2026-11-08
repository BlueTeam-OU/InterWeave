---
role: "rust-ui-dev"
class: solution
topic: "j202-show-all-my-words"
description: "j202 R2 'Show all my words' over 130 % font, built and pushed 2fbf0388 on feat/j202 stacked on j201; rides j199's PR; spec at #543 bfe71023 (ux REPLY 01a1a34f); no device run"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 89a0bbc74f2bf0b5
---

## j202 R2 'Show all my words' over 130 % font, built and pushed 2fbf0388 on feat/j202 stacked on j201; rides j199's PR; spec at #543 bfe71023 (ux REPLY 01a1a34f); no device run

j202 (2026-11-03): branch develop-qzapp/rust-ui-dev-01/feat/j202-identity-design-s4-r2-s-grid, fast-forwarded onto j201 90cd9adb, then 3 work commits, pushed at 2fbf0388. It rides j199's one PR, which waits on e87e1462 (j200's supply) reaching main.

- My QUESTION 01a1a34d proposed the full list. ux REPLY 01a1a34f took it, with two changes: no Close button (the app bar's back arrow is the way out), and Tab starts at the arrow. Spec on #543 bfe71023, provisional until its review is clean.
- ui-model 1056f8af: R2 has `reviewing`. `review_words(open)` never opens over the dialog or a check. Back on the list closes it and asks no dialog. `edit` closes it.
- ui-model ad5fd171: new key IdBack "Back" (PLACEHOLDER). It is paired with AddClose in ONE_ENGLISH_WORD, because the labels test refuses duplicate texts. language-culture-en was asked to finalise it (REQUEST 01a1a37d).
- ui-slint 2fbf0388: `large-text: 1rem > 18.25px`. The line is not 18.2: as a float, 1.3 × 14 comes out just over 18.2, so the strict "over 130 %" test fails at exactly 130 %. The list is an overlay. R2's main layout gets `visible: false`, so its stops leave the focus chain and the accessibility tree. While the list shows, segment 0 is the arrow and segment 1 is the rows. A focused row scrolls into view (Slint's try_scroll_into_visible on FocusIn). Rows the Flickable clips are absent from ElementHandle searches.
- Two trace tests; 8 view mutants, all caught. M1 needed "Tab from the button reaches the key": the button's focus-wanted ignores the index.
- Not done: any device or rendered run (identity screens are Android-only). The app bar arrow on the other identity screens is j204.

Related: [[j199-identity-keyboard-reach]] [[j201-c3-discard-dialog]] [[slint-focus-new-element]]

*References: j199-identity-keyboard-reach, j201-c3-discard-dialog, slint-focus-new-element*

*Anchors: `crates/human/ui-slint/tests/trace.rs::r2s_grid_is_a_full_list_behind_a_button_over_130_percent`, `crates/human/ui-model/src/identity_flow.rs::review_words`*

*Observed 2026-11-03 (rust-ui-dev)*

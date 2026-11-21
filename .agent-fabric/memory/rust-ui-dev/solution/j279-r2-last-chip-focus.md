---
role: "rust-ui-dev"
class: solution
topic: "j279-r2-last-chip-focus"
description: "j279 (ux OBSERVATION 01a1d83f): R2's 24th chip leaves focus on the keypad; spec §8 line 649's navigating-on-IdCheckWords exception was never built"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9151711bde5466d6
---

## j279 (ux OBSERVATION 01a1d83f): R2's 24th chip leaves focus on the keypad; spec §8 line 649's navigating-on-IdCheckWords exception was never built

Cause (read on origin/main 5608d8ea7): `changed page` in crates/human/ui-slint/ui/app.slint sent focus to the keypad for every chip on R2/C3; design/2026-10-19-android-identity.md §8 line 649's exception (a chip leaving no R2 position empty -> navigating, focus on IdCheckWords) was never built. Acceptance case: line 1043.

DONE 2026-11-14: c52581ec3 on develop-qzapp/rust-ui-dev-01/feat/j279-interweave-r2-24th-chip-leaves-focus (pushed, no PR; j277's record fix cherry-picked on top as 4084d5b9d; both ride the next PR). Rule: `to-check = chip-taken && screen == r2 && primary-enabled` -> typing=false, go(6, 0). Tests in tests/trace.rs (Identity::at_r2_but_24); mutation-checked. REPLY 01a1d875 to ux-designer-01; the A40 repeat is theirs.

Test traps met: R2's check is async, so after Primary the flow stays R2 with waiting() true (assert that, not a screen change); take() the typed letter's input before asserting the chip's.

2026-11-15 agreement (REPLY 01a1dc86-6041 to rust-ui-dev-02): their j110/j111 ui-slint PRs go first; I open no PR touching ui-slint or TM-58's row until theirs merge, then merge main into j279 and resolve TM-58 (4084d5b9d conflicts with their fb822d109 there) myself.

2026-11-15 (j281): merged origin/main eb72c4f4b onto j279+j277 as 7fefaf8dd on develop-qzapp/rust-ui-dev-01/feat/j281-interweave-open-the-pr-for-j279 (pushed, no PR yet; fleet hold stopped the test run). architect-cto #680 edits the same TM-58 row: whoever lands second folds.

2026-11-16 (j281): PR #682 opened. 2 work, 3 fix. Review r1 at 7fefaf8dd found F1 (stale comments) and F2 (the chip-taken conjunct was untested: the mutation survived all 56 trace tests). Both fixed in 01b2dc442 and 00b381cf9; the new test is a_position_chosen_on_a_full_r2_hands_focus_to_the_keypad. r2 found F3 (the summaries lacked "no check running"), fixed in bcba3117a. r3 at bcba3117a clean, posted. Arm word asked of architect-cto-01 (REQUEST 01a1e181-efad); j281 is blocked on it. ux REPLY 01a1e181-5b30 (provisional): while R2's check runs, the words are fixed and focus stays on the disabled IdCheckWords. That is job j282, to build after ux's note PR lands.
2026-11-16: #682 ARMED at bcba3117a on architect-cto-01's AUTHORIZES 01a1e182-f6b8; wait-merged running. Deliver j281 after the merge (deliver-after-merge). architect-cto folds the TM-58 wording into #680.
2026-11-16: #682 MERGED dbc969d7e; j281 delivered.

*Anchors: `crates/human/ui-slint/ui/app.slint::to-keypad`, `crates/human/ui-slint/src/identity.rs::primary_enabled`*

*Observed 2026-11-14 (rust-ui-dev)*

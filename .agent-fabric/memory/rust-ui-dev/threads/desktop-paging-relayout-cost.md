---
role: "rust-ui-dev"
class: threads
topic: "desktop-paging-relayout-cost"
description: "The desktop's first draw after a page cost ~7 ms per held row (2-2.8 s at 256-320); cause and fix found in j77 (2026-10-31), ~0.25 ms/held row left"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a7492b270488e7d4
---

## The desktop's first draw after a page cost ~7 ms per held row (2-2.8 s at 256-320); cause and fix found in j77 (2026-10-31), ~0.25 ms/held row left

CAUSE (j77, 2026-10-31): each message row read its place (`in-view`, `item-top`) in `init` and in its change handlers' first evaluation, which runs as the repeater makes the row; each read laid out the whole column and re-measured every row's WRAPPED text (`fn_layoutinfo_v_with_constraint` is uncached). Mechanism and Slint traps: [[slint-row-init-reads-layout]]. A second, unrelated defect found on the way: [[slint-image-equality]] (every held row re-set each render; fixing it changed no timing).

FIX: PR #521 MERGED 2026-10-31 as 097a5b83 (7 work, 3 fix; j77 and j78 closed; armed below the floor on architect-cto's word, the owner's choice; first arm went red on window_faults, fixed by a7cbcb0f top-shown -- see [[ui-slint-change-runs-desktop-app-tests]]), commits dddea518 (image) and d3bf9c58 (row `placed` gate + Timer, test `crates/human/ui-slint/tests/draw_cost.rs`, debug real window: 32 rows vs 1 above 224 held, gated 1.2x, mutants 10-12x, threshold 4).

Release window, first draw after a 64-row page: before 0.2/1.2/2.6 s at 64/128/192 held (2.3-2.8 s at 320); after 33-90 ms to 320, 190 ms at 448-640, 290 ms at 1024 (HELD_ROW_CAP); redraw 37 ms at 1024. What remains is per-held-row layout/traversal without virtualization -- a possible follow-up, not taken.

Contacts list has the same read-at-creation shape but no wrapped text; measured 30 vs 300 contacts identical with and without the gate, so left ungated.

NOT measured: Android. The Samsung A40 row (apps/human-android/device-runs/2026-10-30-windowed-start.md: pane read "Select a conversation" 3 s after a tap, rows ~10 s later; Pixel <3 s) shares this view and should be re-measured on the device.

*References: slint-image-equality, slint-row-init-reads-layout, ui-slint-change-runs-desktop-app-tests*

*Observed 2026-10-31 (rust-ui-dev)*

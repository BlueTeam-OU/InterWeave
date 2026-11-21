---
role: "rust-ui-dev"
class: solution
topic: "j95-reader-open-cost"
description: "Where the reader's opening time went (A5) and what brought it down: conversations() preview, per-build measure cache, deferred source form; text layout is the floor"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 023479de246d541a
---

## Where the reader's opening time went (A5) and what brought it down: conversations() preview, per-build measure cache, deferred source form; text layout is the floor

Measured 2026-11-10 on the desktop testing backend at opt-level 3
(CARGO_PROFILE_DEV_OPT_LEVEL=3, own CARGO_TARGET_DIR; release has no
element tree, so ElementHandle finds nothing there). Figures track the A40's
of 2026-11-09 closely. Host noise is ~30 %: A/B inside one run, alternating.

- `UiModel::conversations()` rebuilt each item's list preview
  (`body::first_line` walks the WHOLE body) on every render: ~200 ms a render
  for 28k links. Fixed: `Item.preview: OnceLock` (commit 1229c9175).
- Reader rows measured every line/label (`measured` ~65-95 us fixed cost
  each): 13k identical labels. Fixed: `Measures` cache per build
  (438ce9947), key = only what measure-text reads (15a99c072).
- After those: shapes 72/76/122/166 ms (paragraph/images/heading
  autolinks/reference link), from 283/710/1391/1922.
- Worst cases the cache cannot help (distinct link destinations ~8k,
  distinct headings ~6k): 0.7-1.8 s. Floor is text layout, ~1.5 us per
  character, per form. Stacking lines into one measure (sum fits => each
  fits) cut measure COUNT 20x but not time: tried and dropped.
- The source form was a second full layout at open; now built after the
  first frame or on Show source (reader::Forms::source).

Related: [[j95-plan]], [[j95-a5-device-case]], [[j95-design-choices]].

*References: j95-a5-device-case, j95-design-choices, j95-plan*

*Anchors: `crates/human/ui-model/src/model.rs::preview: OnceLock`, `crates/human/ui-slint/src/lib.rs::struct Measures`, `crates/human/ui-slint/src/reader.rs::fn source`*

*Observed 2026-11-10 (rust-ui-dev)*

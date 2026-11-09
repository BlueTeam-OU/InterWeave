---
role: "rust-ui-dev"
class: solution
topic: "flickable-clips-focus-ring"
description: "A Flickable clips; a control filling its width draws a focus ring with no sides; common::ring_cut checks every Tab walk"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 03e9d8e1aebc7dad
---

## A Flickable clips; a control filling its width draws a focus ring with no sides; common::ring_cut checks every Tab walk

In InterWeave's ui-slint, a focusable control laid out edge to edge inside a Flickable shows its FocusRing (5 px outside) without the sides that fall outside the scroll area. Seen rendered on the refusal screen's path field in j83 (PR #547). The fix there: the body inside the Flickable gets padding of `FocusMode.ring-reach`, and the outer margin is reduced by the same amount so the visible 24 px is unchanged.

The check: `common::ring_cut(window)` reports each Flickable holding the ring's centre that the ring falls outside of. The refusal, views (`tab_each_draws_one_ring`) and cards (`tab_each_ring_covers_nothing`) walks ask it. A control (forcing it to report every held ring) showed the views walk and four card walks do reach rings inside a Flickable. It does not see `clip: true` Rectangles.

Related: [[focus-ring-reach-spacing]], [[slint-textinput-no-focus-reason]]

*References: focus-ring-reach-spacing, slint-textinput-no-focus-reason*

*Anchors: `crates/human/ui-slint/tests/common/mod.rs::ring_cut`*

*Observed 2026-11-04 (rust-ui-dev)*

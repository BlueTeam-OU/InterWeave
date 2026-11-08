---
role: "rust-ui-dev"
class: solution
topic: "focus-ring-reach-spacing"
description: "Text beside a focusable control sits at least FocusMode.ring-reach (5 px) from it; tests/common ring_overdraws checks every Tab walk"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 79c08b75a86db639
---

## Text beside a focusable control sits at least FocusMode.ring-reach (5 px) from it; tests/common ring_overdraws checks every Tab walk

In InterWeave's ui-slint, every FocusRing is placed 5 px outside its control (a 2 px gap plus the 3 px stroke). Any Text laid out nearer than that gets drawn over when the control holds keyboard focus. The A40 run of 2026-11-03 showed it on the fingerprint's label.

The rule: spacing beside a focusable control is `FocusMode.ring-reach` (app.slint global, exported for tests). #546 (merged 2026-11-04, 2e78f885) applied it at 15 layouts.

The check: `crates/human/ui-slint/tests/common/mod.rs` `ring_overdraws` (shared by views.rs, cards.rs and refusal.rs). It returns `None` when no ring is shown, so a walk records only the stops it really checked and asserts its stops of interest are among them. Text clipped to a 1 px point is screen-reader-only and excluded. `cards.rs` `ring_reach_is_where_the_ring_is_drawn` holds the global, `common::REACH` and one drawn ring equal.

How to apply: a new screen with a label above a focusable control uses ring-reach spacing, and its Tab walk goes through `tab_each_ring_covers_nothing` (cards.rs) or `tab_each_draws_one_ring` (views.rs). The first review of #546 found the first version vacuous, because a stop with no ring shown passed silently.

Related: [[slint-textinput-no-focus-reason]]

*References: slint-textinput-no-focus-reason*

*Anchors: `crates/human/ui-slint/ui/app.slint::ring-reach`, `crates/human/ui-slint/tests/common/mod.rs::ring_overdraws`*

*Observed 2026-11-04 (rust-ui-dev)*

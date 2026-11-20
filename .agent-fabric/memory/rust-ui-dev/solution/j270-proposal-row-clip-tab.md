---
role: "rust-ui-dev"
class: solution
topic: "j270-proposal-row-clip-tab"
description: "j270 — proposal Decline/Update unreachable by TAB on the A40 was the proposal row's unconditional clip; fixed fa946c195 on #658; host-test traps met"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d23436b77a4a2cde
---

## j270 — proposal Decline/Update unreachable by TAB on the A40 was the proposal row's unconditional clip; fixed fa946c195 on #658; host-test traps met

j270 (2026-11-13): on the A40 a second card's note and link pushed Decline and Update contact below the screen and TAB went link → composer. Cause: `ProposalEntry::proposal-row` had `clip: true` for its swipe; Slint 1.18.1 TAB skips a control hidden by any clipping ancestor that is not a Flickable (i-slint-core window.rs `move_focus`, item_tree.rs `is_visible_or_clipped_by_flickable`). Fix fa946c195 on #658's branch: `clip: self.offset != 0px`. Test `tests/cards.rs::every_control_of_a_proposal_taller_than_the_window_is_a_tab_stop` (390x640, 28 px text, card taller than window), mutation `clip: true` fails. 5e8dee8df puts cluster_bound.rs's comment in the past tense. Not run on a phone yet.

Host-test traps met: (1) the testing backend's ElementHandle queries SKIP items `is_visible()` false (clipped by the Flickable too), so a label lookup of an off-screen control returns nothing; (2) focusing an off-screen control by focus request and then TAB lands on the window's first stop — walk TAB from the window's start instead; (3) the conversation anchors at its end, so only a card taller than the list puts controls below it on the host.

Related: [[slint-rendered-window-lessons]] (same class as message rows' `clip: row.leaving`), [[j268-connect-launch-count]]

*References: j268-connect-launch-count, slint-rendered-window-lessons*

*Anchors: `crates/human/ui-slint/ui/app.slint::clip: self.offset != 0px`, `crates/human/ui-slint/tests/cards.rs::every_control_of_a_proposal_taller_than_the_window_is_a_tab_stop`*

*Observed 2026-11-13 (rust-ui-dev)*

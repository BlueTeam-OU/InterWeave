---
role: "ux-designer"
class: solution
description: "Which message kinds the InterWeave human client shows and which per-row actions each has — the base for any gesture/swipe design"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 678fbda2fbda9c83
---

## Which message kinds the InterWeave human client shows and which per-row actions each has — the base for any gesture/swipe design

As of origin/main 522dd997 (2026-10-16), `UiModel::actions` in
`crates/human/ui-model/src/model.rs` offers per message row:
- received, read, not kept -> Keep (only after read, human-client-ui.md §6)
- received, kept -> Unkeep (deletes the durable copy; content stays in RAM this session, so a Keep can undo it)
- sent, pending/failed -> Retry, Cancel
- sent, accepted (terminal) -> nothing

MarkRead is never a row action: `conversation_viewed` raises it when a
focused conversation is shown, so an inbound row in the list is
already read. There are no reactions ("Like"), no request kind
(accept/reject), and no "hide" intent. Trust changes are settings-only
with an exact-PeerId confirmation (§8), and remote content cannot
trigger them (§4), so a message gesture can never be a trust action.
The Slint view (`crates/human/ui-slint/ui/app.slint`) renders each
row's actions as ActionButtons — the existing non-gesture equivalent.

Related: [[swipe-actions-j1]]

*References: swipe-actions-j1*

*Anchors: `crates/human/ui-model/src/model.rs::pub fn actions`, `crates/human/ui-model/src/model.rs::pub fn conversation_viewed`*

*Observed 2026-10-16 (ux-designer)*

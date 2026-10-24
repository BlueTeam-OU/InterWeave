---
role: "rust-ui-dev"
class: solution
topic: "accept-carries-shown-context"
description: "Why Command::AcceptCard carries the ProposalContext the person saw, and what the facade does when the store disagrees (ProposalChanged)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b833ea31c3ac5258
---

## Why Command::AcceptCard carries the ProposalContext the person saw, and what the facade does when the store disagrees (ProposalChanged)

Since #359 (plan §29 step 2, review round 1 F2, 2026-10-22), `Command::AcceptCard { row, shown: Box<ProposalContext> }` carries the context the person was shown. model_side takes it from `UiModel::proposal_context(row)`.

The facade (`crates/human/app-core/src/facade.rs`, `accept_card`) recomputes `cards::context` from the store. Where the two differ, it:
- writes nothing;
- puts the card's read copy back;
- gives the proposal again (`Update::ReceivedCard`);
- answers `Failure::ProposalChanged`.

**Why:** two cards on one route, arriving before any Accept, were both shown as "new key". The second Accept then replaced the recorded key unseen, which human-client-ui.md §13 forbids on every path, the public Command API included. Refreshing the model alone could not close it, because a queued second command runs before the refresh arrives.

Two related rules from the same review:
- Every exit before the write puts the copy back (F3).
- A Keep of a copy whose envelope is a card by its kind is refused, whether or not it verifies (`cards::holds_card`, F4, F9).

The model caps read proposals at `READ_PROPOSAL_CAP`, which a test holds equal to app-core's `READ_COPY_CAP` (F5).

The step 4 views must keep calling `UiModel::proposals_viewed` only where they actually render proposals; `conversation_viewed` never reads a card. Related: [[j40-step2-scope-rulings]].

*References: j40-step2-scope-rulings*

*Anchors: `crates/human/app-core/src/facade.rs::ProposalChanged`, `crates/human/app-core/src/cards.rs::holds_card`, `crates/human/ui-model/src/model/cards.rs::READ_PROPOSAL_CAP`*

*Observed 2026-10-22 (rust-ui-dev)*

---
role: "rust-ui-dev"
class: solution
topic: "human-store-realistic-size"
description: "InterWeave human store sizing: 500 kept messages is realistic, 2000 already large; unread and unsent are marginal; 20k is out of scope"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 70af4640d6b00c82
---

## InterWeave human store sizing: 500 kept messages is realistic, 2000 already large; unread and unsent are marginal; 20k is out of scope

The owner, 2026-10-21 (during SPIKE-011): the human store's realistic size
is **500 kept messages**; **2000 kept** is already a big number for a
privacy messenger; unread and pending outbound are marginal (a read,
unkept message is released, so the store is mostly what the person kept);
20 000 is beyond the product's case.

**Why:** the store holds only pending_outbound, unread_inbound and
kept_inbound (ADR-0044); there is no history table.

**How to apply:** size store benchmarks and budgets at 500 (realistic) and
2000 (large), kept-dominated with a small unread backlog; a larger size is
a labelled stress row, never the bound a criterion is judged at.
Related: [[sqlite-deletion-residue]].

*References: sqlite-deletion-residue*

*Anchors: `interweave:crates/human/store/Cargo.toml::THREE CONTENT TABLES`, `interweave:spikes/spike-011/harness/src/main.rs::UNREAD_BACKLOG`*

*Observed 2026-10-21 (rust-ui-dev)*

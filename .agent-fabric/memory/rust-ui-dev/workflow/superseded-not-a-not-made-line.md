---
role: "rust-ui-dev"
class: workflow
topic: "superseded-not-a-not-made-line"
description: "a refusal caused by another writer (IPC 2.10 superseded) needs its own line; reusing \"not made / still in the list / try again\" can be false"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4d0e6e9849e46437
---

## a refusal caused by another writer (IPC 2.10 superseded) needs its own line; reusing "not made / still in the list / try again" can be false

On j152 (2026-10-24) I mapped an address add, remove or Undo refused `superseded` to the add-a-peer page's existing not-made lines (AddFailed, RemoveFailed, UndoFailed). ux-designer-01 (REPLY 01a16f8c) showed why that fails. Another client may already have made the same change, so "It is still in the list" or "still removed" can be false over the list read back. "Nothing was changed" reads as "the list is as it was". And Try again would undo another person's deliberate change.
**Why:** those lines describe this client's write failing on a list nobody else touched. A refusal caused by another writer changes what the list says.
**How to apply:** when a contract adds a refusal caused by another actor, ask ux-designer for its state before mapping it to an existing line; give it no automatic retry. The follow-up is j153 ([[j153-superseded-address-state]]). The trust side had architect-cto's rule (TrustSuperseded).

*References: j153-superseded-address-state*

*Observed 2026-10-24 (rust-ui-dev)*

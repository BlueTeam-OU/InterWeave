---
role: "architect-cto"
class: workflow
topic: "reflection-read-the-crate-s-visibility-before-an-adr-names-a-mechanism-site"
description: "An ADR rule that says where a mechanism runs inside a vendored or third-party crate is written only after the implementer has read what the crate exposes at that point; otherwise the rule follows the implementation through several…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e358db0cc997d38e
---

## An ADR rule that says where a mechanism runs inside a vendored or third-party crate is written only after the implementer has read what the crate exposes at that point; otherwise the rule follows the implementation through several corrections

InterWeave ADR-0052 rule 5, 2026-09-19: "rule 5 corrected to your
mechanism… whole-list verdict at the hook" (01a0b6c0-6bed), "corrected
again" (01a0b6da-e29c), because "the crate's dial list is not readable
before the pending hook (DialOpts::get_addresses is pub(crate))"
(01a0b6f2-1f6b). Extends the test-seam rule from seams to mechanism
sites.

*Observed 2026-10-23 (architect-cto)*

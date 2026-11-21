---
role: "architect-cto"
class: threads
topic: "j686-adr-0068-keepalive-promotion-2026-11-14"
description: "ADR-0068 (libp2p keepalive + held connection, promoted from CONNECTIVITY.md §14) is InterWeave #667; state, what the owner must accept, what is owed after"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d7201e778201be22
---

## ADR-0068 (libp2p keepalive + held connection, promoted from CONNECTIVITY.md §14) is InterWeave #667; state, what the owner must accept, what is owed after

j686: ADR-0068 promotes §14's keepalive and hold rules (eight rules; README row, digest entry, §14 pointer paragraph, plan §21 two sentences, ADR-0055 decision 2 amended 2026-11-14 to cite it). InterWeave #667, head 5f60b5db on 2026-11-14: 1 work, 7 fix; three review rounds posted, F1-F6 fixed, round 3 `Finding: none`; unarmed.

**Waiting on:** the owner's word to arm (my own PR, below the minimum) and the owner's acceptance of what the record states for the first time: rule 7 (liveness is never authentication or authorization) and three alternatives marked "recorded with this promotion". Asked in the session on 2026-11-14.

**How to apply:** on the owner's word, `fabric-pr arm 667 --basis "the owner's word ..."` only if the head is still 5f60b5db; a later push needs a re-review first. #666 (p2p-network-dev-01) edits §14's 2026-11-13 paragraph; the two merge clean in either order. `cargo-machete` is not installed in this account, so `check_unused_dependencies.sh` exits 2 locally.

*Observed 2026-11-14 (architect-cto)*

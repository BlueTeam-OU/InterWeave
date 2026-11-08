---
role: "fabric-coordinator"
class: workflow
topic: "head-pinned-word-leaves-local-fold"
description: "When the owner's arming word (or an AUTHORIZES) names a head, arm that head and leave any later local fold of main unpushed"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - b4bfc90b35793fd2
---

## When the owner's arming word (or an AUTHORIZES) names a head, arm that head and leave any later local fold of main unpushed

InterWeave #529 (j504, 2026-11-03): the owner said "merge 529" for head 637d5777 and architect-cto-01's AUTHORIZES was pinned to it; a session switch left a local, unpushed merge of origin/main (72680175c) on the branch. Pushing it would have disarmed the PR (disarm-on-push), needed a re-review and a fresh word. The ruleset is non-strict and the merge queue tests its own ref against main, so I armed 637d5777 as said; it merged as 57a4186e with the fold never pushed.

**Why:** the word and the authorization cover a head, not a branch; a fold after it is a new head nobody approved.

**How to apply:** before arming on a head-pinned word, compare local HEAD with the PR's headRefOid; if they differ by a clean fold only (`git merge-tree --write-tree` tree equals the fold's tree), arm the PR's head and drop the fold with the branch. Related: [[arm-defer-carried-findings]].

*References: arm-defer-carried-findings*

*Observed 2026-11-03 (fabric-coordinator)*

---
role: "architect-cto"
class: workflow
topic: "reflection-prose-that-describes-behaviour-you-changed-grep-before-sending"
description: "W43: When a ruling changes what a contract text claims, grep the whole tree for the old claim's distinctive words before the supply leaves, not after each review round names one"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 91546e41e59c9c18
---

## W43: When a ruling changes what a contract text claims, grep the whole tree for the old claim's distinctive words before the supply leaves, not after each review round names one

When a ruling changes a claim my texts made (2026-10-24, session.losses "lower bound / permanent"), run one tree-wide grep for the old claim's distinctive words — across architecture/, crates/, tests/, apps/, the history notes and the digest — and fix every hit in the first supply. Three supply rounds (39e6bc41, 109070b0, 276220ce) each removed one more sentence a reviewer found; the lane's grep ended it, which was mine to run.

**Why:** a review names one instance per round; a grep names them all at once, and each round costs a fold, a re-review and a message.
**How to apply:** before `git commit` of a correction, `git grep -n '<old words>' -- architecture crates tests apps` and read every hit, including the history note the correction itself wrote.

*Observed 2026-10-24 (architect-cto)*

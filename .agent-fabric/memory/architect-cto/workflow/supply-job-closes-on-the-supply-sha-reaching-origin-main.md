---
role: "architect-cto"
class: workflow
topic: "supply-job-closes-on-the-supply-sha-reaching-origin-main"
description: "A supply job (commits handed to another lane's PR) closes only when the supply sha itself is an ancestor of origin/main, not when the PR shows merged"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 20139f9ade66b37b
---

## A supply job (commits handed to another lane's PR) closes only when the supply sha itself is an ancestor of origin/main, not when the PR shows merged

A supply job's artifact is the supply commit on the other lane's PR, so the job is done when `git merge-base --is-ancestor <supply sha> origin/main` holds, checked after a fetch. The PR's MERGED state alone is not enough: the integrator folds the for/ branch, and a fold can leave the supply commit out or rebase it to a new sha, in which case the body's words are grepped on origin/main instead.

**Why:** j207 (ADR-0057 decision 4, supply 873175a0 onto p2p-network-dev-01's for/ branch, folded into BlueTeam-OU/InterWeave#324, merged 2026-10-21) was closed in a later session by that one check; the job's own note named the PR, and the PR alone would have said "merged" even had the fold dropped the commit.

**How to apply:** when a supply job's blocker PR merges, run the ancestor check on the supply sha, then `fabric-jobs deliver <id> <pr> <sha>` and `done`; remove the job's worktree and its branch (it holds nothing past origin/main, so `-D` is the right form from another checkout). See [[reflection-what-a-supply-reply-says-it-contains-is-read-from-the-pushed-sha]].

*References: reflection-what-a-supply-reply-says-it-contains-is-read-from-the-pushed-sha*

*Observed 2026-10-25 (architect-cto)*

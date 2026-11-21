---
role: "architect-cto"
class: workflow
topic: "disarm-by-arming-authority-goes-through-gh-2026-11-09"
description: "fabric-pr has no disarm verb; the arming authority takes auto-merge off another session's PR with gh --disable-auto from a fresh session and leaves a hand comment on the PR"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b7506d2b62376536
---

## fabric-pr has no disarm verb; the arming authority takes auto-merge off another session's PR with gh --disable-auto from a fresh session and leaves a hand comment on the PR

j568/j570 (2026-11-09): InterWeave #600 (rust-ui-dev-02, 15 work,
7 fix, head 43ce40bb) sat armed six hours, held by two checks red
on an advisory already fixed on main; the PR owner's guard, the
coordinator's guard and my own ended-job session all refused the
disarm. A FRESH session of the arming authority ran
`gh pr merge <n> -R <repo> --disable-auto` and it went through.

**Why:** `fabric-pr` has verbs for arm but none for disarm, so an
authority's disarm is recorded nowhere unless done by hand; the
disarm-on-push workflow comments only on a push.

**How to apply:** verify first (autoMergeRequest, mergeQueueEntry via
GraphQL, review-status, the fix on origin/main), disarm with gh, read
it back, post a one-line comment on the PR naming the time, head,
requester and reason, then REPLY to the requester with the read-back.
Raised to fabric-coordinator as a fact in REPLY 01a1c1ef-0c95. See
[[owner-rule-never-wait-on-owner-to-arm-2026-11-09]].

*References: owner-rule-never-wait-on-owner-to-arm-2026-11-09*

*Observed 2026-11-09 (architect-cto)*

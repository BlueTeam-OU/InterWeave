---
role: "rust-services-dev"
class: workflow
topic: "deferred-finding-new-pr-is-work"
description: "A finding deferred from a merged PR's review, fixed on a new PR, is Kind: work with no Answers: trailer; and a release-blocking PR may be armed by architect-cto before the owner speaks"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 18a5cf4e36031945
---

## A finding deferred from a merged PR's review, fixed on a new PR, is Kind: work with no Answers: trailer; and a release-blocking PR may be armed by architect-cto before the owner speaks

A finding deferred at arming (`--defer "<head8>:F<n>"`) and fixed later on a NEW PR is that PR's work: commit it `Kind: work`, citing the finding in the body ("#612 review F7 (2bc7c1b0), deferred there"), never with an `Answers:` trailer, which marks a review-fix and is excluded from the work count that arms the PR. Done this way on InterWeave #618 (2026-11-10): 5 work commits, F7/F8 of #612.

Same PR: under the minimum, I asked the owner for "arm 618", but architect-cto (the repository's `below_min_authority`) had already armed it on its own word because the v0.3.0 tag waited on the INSTALL.md fix; it merged before the owner's word arrived.

**Why:** an `Answers:` trailer on a fresh PR would have cost a work commit from the count; and stating "waits on the owner's word" was stale within the hour.
**How to apply:** before reporting a PR as waiting on a word, re-read `gh pr view <n> --json state` — another role's arming authority may have acted. Related: [[blocked-job-refuses-arm]].

*References: blocked-job-refuses-arm*

*Anchors: `agent-fabric:AGENTS.md::Kind: work`, `agent-fabric:AGENTS.md::Answers:`*

*Observed 2026-11-10 (rust-services-dev)*

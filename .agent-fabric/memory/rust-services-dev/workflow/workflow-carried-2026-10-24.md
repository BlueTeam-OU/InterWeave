---
role: "rust-services-dev"
class: workflow
topic: "workflow-carried-2026-10-24"
description: "To keep two open InterWeave PRs off one library crate, withdraw a small self-contained change from your own PR and re-land it after the other merges, instead of asking for an exception"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 3c8c5c228c8ede30
---

## To keep two open InterWeave PRs off one library crate, withdraw a small self-contained change from your own PR and re-land it after the other merges, instead of asking for an exception

#375 carried a 21-line unit test in crates/claude/channel-core/src/meta.rs, and #376 (rust-services-dev-02) was open in the same crate. The remit keeps one open PR per library crate (architect-cto's ruling 01a1631f relaxes this only for root test suites).

What I did:
- Reverted the file to main's version in my own PR (3a986d58).
- Tracked the re-land as a job blocked on `pr:BlueTeam-OU/InterWeave#376`.
- Re-landed it as #379 once #376 merged, folding the two cherry-picks into one work commit so that #375's `Answers:` trailer did not make it count as a review fix there.

**Why:** the rule guards against hunks meeting. Withdrawing my own small piece costs a later one-commit PR, and costs the other holder nothing.

**How to apply:**
- Before folding main or a supply, check whether any open PR touches the same library crate (`gh pr list --json files`).
- If one does, move your piece out with a job blocked on the other PR. Don't ask for an exception, and don't wait unannounced.

*Observed 2026-10-22 (rust-services-dev)*

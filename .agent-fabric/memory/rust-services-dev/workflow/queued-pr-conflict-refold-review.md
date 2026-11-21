---
role: "rust-services-dev"
class: workflow
topic: "queued-pr-conflict-refold-review"
description: "An armed PR that main makes conflict in the merge queue: fold main, push (disarms), full review of merge-base..head, re-arm with every open P3 deferred by owner"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - d198c14b73da4aa3
---

## An armed PR that main makes conflict in the merge queue: fold main, push (disarms), full review of merge-base..head, re-arm with every open P3 deferred by owner

On #612 (S32, 2026-11-10), `fabric-pr wait-merged` ended "BLOCKED — conflicts with the base; fold origin/main in (exit 5)". Another role's prose commits on main had touched the same long lines of the README and plan §19.

What worked:
1. Fold main and resolve.
   - Where both sides edit one long line, apply main's edit onto ours by exact substring.
   - Assert `base.replace(old, new) == theirs`, so nothing else of theirs is lost.
2. Push. The push disarms the PR.
3. Run a full `mode: review` over `$(git merge-base HEAD origin/main)..HEAD`, with `review_round` set by hand ([[review-range-with-main-merge-is-partial]]).
4. Re-arm.

A P3 that a review raises in prose spread across several owners' files does not need another round. Pass `fabric-pr arm --defer "<head8>:F<n>" "<why>"`, have the hunk's owner take its share into a PR they already plan, and queue your own share as a job.

**Why:** each review round cost 80-250k tokens and an hour of queue time. Gate 4c keeps even a P3 open until it is judged or deferred.

**How to apply:** for each finding, name who fixes which file, and the job that holds it, in the defer reason. Send the owner the finding with the record that proves it.

*References: review-range-with-main-merge-is-partial*

*Anchors: `agent-fabric:bin/fabric-pr::wait-merged`, `agent-fabric:bin/fabric-pr::arm`*

*Observed 2026-11-10 (rust-services-dev)*

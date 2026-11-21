---
role: "rust-services-dev"
class: workflow
topic: "post-review-json-error-retry"
description: "fabric-pr post-review \"unexpected end of JSON input\" — check the PR's reviews, then retry the bare command"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - ecf25dc3afb8db7b
---

## fabric-pr post-review "unexpected end of JSON input" — check the PR's reviews, then retry the bare command

On 2026-11-12, `fabric-pr post-review 649` failed twice with "the review was rejected by GitHub (PR #649): unexpected end of JSON input". `--dry-run` rendered the body correctly, and `gh api repos/BlueTeam-OU/InterWeave/pulls/649/reviews` showed nothing posted. A third bare call, a few minutes later, posted it. So the error was transient on GitHub's side, not in the report. What to do: list the PR's reviews first (a timeout may have posted it), then rerun the same bare command. Never post by hand, and never wrap the call: the hook refuses `cd …&&`, pipes and `2>&1` on post-review.

Related: [[review-brief-rounds-mechanics]]

*References: review-brief-rounds-mechanics*

*Anchors: `agent-fabric:tools/fabric/github/post_review.py::rejected by GitHub`*

*Observed 2026-11-12 (rust-services-dev)*

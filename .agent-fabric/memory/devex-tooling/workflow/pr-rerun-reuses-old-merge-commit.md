---
role: "devex-tooling"
class: workflow
topic: "pr-rerun-reuses-old-merge-commit"
description: "After a base-side fix lands (lock, advisory, workflow), other PRs' red checks stay red on re-run; only folding origin/main turns them green"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 98228a7bc871bc98
---

## After a base-side fix lands (lock, advisory, workflow), other PRs' red checks stay red on re-run; only folding origin/main turns them green

GitHub re-runs a job with the original event's GITHUB_SHA and GITHUB_REF. For a pull_request event that SHA is the merge commit made when the event fired. So a re-run of a red check after a fix lands on main tests the same old tree and stays red. On 2026-11-12, #637 (fd4679a7f) fixed RUSTSEC-2026-0502 on main, while #634, #635 and #636 kept their red tree checks and tool self-tests.

Before I knew this I told one owner "re-run them once it lands", and had to correct it (INFO 01a1cf76-05ae).

**Why:** the wrong advice costs another session a wasted CI round, and a PR that looks stuck.
**How to apply:** when a fix on main un-reds other PRs, tell each owner to fold origin/main. For an armed PR, say that the push disarms it (disarm-on-push), so the fold needs a review of the new range and `fabric-pr arm` again. That is the owner's call; touch none of their branches.

*Anchors: `.github/workflows/disarm-on-push.yml::disarm`, `.github/workflows/ci.yml::pull_request`*

*Observed 2026-11-12 (devex-tooling)*

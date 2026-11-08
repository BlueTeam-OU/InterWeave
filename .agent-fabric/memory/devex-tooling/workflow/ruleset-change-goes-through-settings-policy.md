---
role: "devex-tooling"
class: workflow
topic: "ruleset-change-goes-through-settings-policy"
description: "Before changing a managed repo's GitHub ruleset (required checks, queue): change its policy in agent-fabric-settings first, then fabric-repo-policy apply"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 98872f6606186ca2
---

## Before changing a managed repo's GitHub ruleset (required checks, queue): change its policy in agent-fabric-settings first, then fabric-repo-policy apply

A managed repository's required checks are pinned twice: the GitHub
ruleset and `repos/<repo>.json` in `BlueTeam-OU/agent-fabric-settings`
(`required_checks`). `fabric-pr arm` refuses EVERY PR of the repository
while the two differ ("GitHub's settings differ from its arming policy").
Measured 2026-11-08 (j297, InterWeave #591): a hand PUT adding
`android apk (assembleDebug)` to ruleset 20756630 blocked all InterWeave
arming until reverted minutes later.

**Why:** the policy is what arm reads; GitHub follows it, not the reverse.

**How to apply:** settings PR first (devex-tooling maintains that repo;
`tools/validate_policies.py --fabric ../agent-fabric --repo <owner/name>`),
merge, then `fabric-repo-policy apply <owner/repo>` and `verify`. The
settings clone is another working copy, so it is its own job (the
one-working-copy hook refuses a commit there from another job's session).
Save `gh api .../rulesets/<id>` before any change so a revert is exact.

*Anchors: `agent-fabric:tools/fabric/github/repo_policy.py::required_status_checks`*

*Observed 2026-11-08 (devex-tooling)*

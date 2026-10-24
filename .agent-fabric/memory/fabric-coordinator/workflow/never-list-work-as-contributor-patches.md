---
role: "fabric-coordinator"
class: workflow
topic: "never-list-work-as-contributor-patches"
description: "When python-devs are idle and the only Python work is on CONTRIBUTOR_NEVER, they write it as a patch the coordinator commits"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 2aefc3611056da32
---

## When python-devs are idle and the only Python work is on CONTRIBUTOR_NEVER, they write it as a patch the coordinator commits

On 2026-10-22 the owner chose (option "1") that idle python-dev logins
take fabric-coordinator's never-list jobs (guards, runtime/claude-code
hooks) as patches: the contributor commits the tests (in its entry) on
`<host>/<login>/for/user/<what>`, leaves the never-list file's change
uncommitted and sends `git diff -- <file>` verbatim in a GZCoord REPLY;
fabric-coordinator applies it as its own commit, folds the for/ branch
unrebased, and the blind review covers both. First used for j303 (to
python-dev-02), j319 (python-dev-04), j334 (python-dev-03).

**Why:** the fence (policies/AUTHORITY.md, CONTRIBUTOR_NEVER) refuses a
contributor's commit on those paths in hook and CI; the owner had once
chosen entries over patches (ADR-018 §5 rule 8), so this widens how
work reaches them, not what they may commit.

The request must say PUSH: on 2026-10-22 all three suppliers (j303,
j319, j334) replied with a commit id on a branch that was not on origin,
because the delivery terms said "commit" and not "push"; each cost a
round trip. Fold only what `git fetch origin <branch>` returns.

That suppliers run no review of their own (the caller folds, then one
blind review) is a fabric rule since 2026-10-22: agent-fabric ADR-020
§5 rule 11, held by the dispatch guard (#354). Not repeated here.

**How to apply:** before offering contributors work, read the tree, not
an ADR status table: on 2026-10-21 ADR-040's Wave 9 table still said
"in progress" while every batch was already forwarders or gone (#318),
and the owner said yes to work that did not exist. See
[[verification-pipelines-must-fail-loudly]].

*References: verification-pipelines-must-fail-loudly*

*Anchors: `agent-fabric:tools/fabric/lint.py::CONTRIBUTOR_NEVER`, `agent-fabric:policies/AUTHORITY.md::The contributor opens no pull request`*

*Observed 2026-10-22 (fabric-coordinator)*

---
role: "language-culture"
class: workflow
topic: "folded-supply-branch-kept"
description: "in a branch sweep, a for/ branch above 0 with no PR is kept while the caller's PR is unopened; prove the fold first"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 022fe2e5afa066a3
---

## in a branch sweep, a for/ branch above 0 with no PR is kept while the caller's PR is unopened; prove the fold first

A `for/<caller>/…` branch shows in `fabric-branches` as commits off main with "pull request: none",
because the supplier opens no PR. That is not an orphan. Before calling it one, find the caller's
REPLY naming the fold (`fabric-history <branch name>`), then prove it on origin:
`git merge-base --is-ancestor <for/ tip> <caller branch sha from git ls-remote>`. While the
caller's PR is unopened or unmerged, keep the branch: owed-supply tracks it, and the caller may
still need it. The owner's "open the PR, fold it, or delete it" is already met by the fold.

Seen in j241 (2026-11-04): the gzapp branch `for/flutter-dev-01/j34-point-keys` (9d2bfd237) was
folded into flutter-dev-01's `feat/j34-completion-plan-17-items-1-and` at 8038bed3e, and that PR
was not yet open. See [[supply-base-moves-merge]].

*References: supply-base-moves-merge*

*Anchors: `agent-fabric:tools/fabric/branches.py::pull request: {pr or 'none'}`*

*Observed 2026-11-04 (language-culture)*

---
role: "rust-ui-dev"
class: workflow
topic: "herdr-conventional-commits"
description: "agent-fabric-herdr (FleetDeck) CI requires conventional-commit PR titles AND commit subjects; title edits do not retrigger CI"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-12"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ca11636f489f4b0f
---

## agent-fabric-herdr (FleetDeck) CI requires conventional-commit PR titles AND commit subjects; title edits do not retrigger CI

In BlueTeam-OU/agent-fabric-herdr, `scripts/conventional_commits.py` checks the PR title on pull_request and every commit subject on a push to master: `type(scope): subject`, types chore, ci, docs, feat, fix, perf, refactor, release, test. Run `python3 scripts/conventional_commits.py --range origin/master..HEAD` before pushing.

**Why:** #15 (2026-10-11) went red with `fleet-deck/install: ...` subjects; the title fix needed a close/reopen to rerun (CI triggers on opened/synchronize/reopened only), and the commit subjects could not be fixed without a force-push. The fork's master has no required checks, so an arm merges before CI finishes.

**How to apply:** write herdr commit subjects as `fix(fleet-deck): ...` from the first commit; tell the owner to arm only after every check leg is green.

2026-10-11: the repo is renamed BlueTeam-OU/agent-fabric-fleetdeck (GitHub redirects). Write the new name in the fork's text. Do NOT set-url the clone's origin until fabric-coordinator says the registry update reached this account (01a12a2c).

*Observed 2026-10-11 (rust-ui-dev)*

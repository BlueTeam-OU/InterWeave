---
role: "fabric-coordinator"
class: workflow
topic: "per-job-auto-mode-entry-procedure"
description: "How a per-job auto-mode allow entry (policies/auto-mode.json) is landed and what bit last time — owner's choice, own PR, revert elsewhere, review against the driver"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 43e27bc949d545ca
---

## How a per-job auto-mode allow entry (policies/auto-mode.json) is landed and what bit last time — owner's choice, own PR, revert elsewhere, review against the driver

A per-job auto-mode allow entry (s19 in 68e7730f, s22 in agent-fabric #357, merged 9342e397, 2026-10-23) is the owner's choice by the `_comment`'s rule: ask the owner before writing it, and record "(the owner's choice, <date>)" in the entry.

- **Own PR:** land it on a PR of its own when the requester is waiting. The owner chose that for #357, armed on their word with 1 work commit. If the entry also sits on another open PR, revert it there (#356, cf230447). Otherwise that PR merging after the entry's removal brings the entry back.
- **Review:** the review class checks each clause against the driver's code. The first draft of s22 claimed less than the driver did. It never listed the driver's writes, the raw output and evidence.md committed into the public repository, so read literally it covered nothing. It also overstated two details: one run writes no settings file, and /etc symlinks are recreated rather than bound. Spell out what the driver writes, and exclude any binary-selecting environment variable such as SPIKE_CLAUDE.
- **Local suite refused:** the auto-mode classifier refused `tests/run.sh` on the branch that adds the entry ("Permission Grant"). Run the suite on the branch where the commit is first made, before cherry-picking; CI covers the PR.
- **Removal:** queue the removal as its own job at delivery (j361 for s22).

*Anchors: `agent-fabric:policies/auto-mode.json::an entry the owner chose for one role's job`*

*Observed 2026-10-23 (fabric-coordinator)*

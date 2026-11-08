---
role: "rust-ui-dev"
class: workflow
topic: "supplier-docs-a-pr-makes-false"
description: "When a PR makes another role's record (contract, matrix row, plan carry) false, that record's fix lands in the same PR as a supply, before arming"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3ce0f81623c74b5f
---

## When a PR makes another role's record (contract, matrix row, plan carry) false, that record's fix lands in the same PR as a supply, before arming

architect-cto-01 on #522 (2026-10-31) declined to arm until the stale records the PR contradicted were amended IN the PR: they supplied the human-client-ui.md §13 sentence (95b39f29) and, after a re-review found TM-56 and the plan carry still saying "owed", a second supply (06cbde22). Each supply moved the head and needed its own re-review round.

**Why:** a doc the PR's code contradicts must not land after it; the record and the code land together.

**How to apply:** before asking for the arming word, grep the tree for the old behaviour's words in other roles' records (tests/SECURITY-MATRIX.md TM rows, plan carries, human-client-ui.md §13) and list each in the request, so the owner supplies them all at once — one re-review round, not two. Related: [[j60-twin-titles-and-name-key]].

*References: j60-twin-titles-and-name-key*

*Observed 2026-10-31 (rust-ui-dev)*

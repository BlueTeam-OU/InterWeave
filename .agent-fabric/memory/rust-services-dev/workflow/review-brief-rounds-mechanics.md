---
role: "rust-services-dev"
class: workflow
topic: "review-brief-rounds-mechanics"
description: "fabric-review brief across rounds: re-review reads the last posted review, a base off main needs card_base, adjudication takes previous_findings, and a handback report is extracted from the task JSONL"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 09cb37f6413f4c39
---

## fabric-review brief across rounds: re-review reads the last posted review, a base off main needs card_base, adjudication takes previous_findings, and a handback report is extracted from the task JSONL

Learned on InterWeave #531 (2026-11-03, six review rounds):

- `mode: re-review` with `pr:` refuses `previous_findings`: it reads the previous report from the PR's last posted review, so post each review before rendering the next brief.
- When the range's base is not on origin/main (any round after the first), the brief refuses unless `card_base:` names a full commit main contains (the first round's base).
- `mode: finding-adjudication` does take `previous_findings: <file>`; judge before answering, then fix.
- More than 3 lenses (general counts) draws a dilution warning.
- The code-review agent hands back via a SubagentHandback call: post its text by walking the task's JSONL for the string holding the dispatch block and the last `Finding:`/`Judgement:` line, then `fabric-pr post-review <n> < file` as the bare command (anything wrapped is refused).
- An architecture fold supplied by architect-cto onto the PR gets its own re-review; findings in its text go back to architect-cto as a REQUEST, never fixed in their files.

**Why:** each of these cost a refused render or a wasted round.
**How to apply:** any PR that goes past one review round. Related: [[review-range-with-main-merge-is-partial]].

*References: review-range-with-main-merge-is-partial*

*Observed 2026-11-03 (rust-services-dev)*

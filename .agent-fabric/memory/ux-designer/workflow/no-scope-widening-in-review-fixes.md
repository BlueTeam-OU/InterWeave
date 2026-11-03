---
role: "ux-designer"
class: workflow
topic: "no-scope-widening-in-review-fixes"
description: "during a PR's review rounds, fix only what a finding names; never settle a reviewer's \"risk\" by adding a new rule"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9b18bd785e443973
---

## during a PR's review rounds, fix only what a finding names; never settle a reviewer's "risk" by adding a new rule

While a PR is in review, a fix answers the finding it names and nothing more. A reviewer's "Risks (unproven)" section is not a work list: note it on the PR or in the thread, and leave it for a later change.

**Why:** on InterWeave #469 (2026-10-27), after a clean re-review (round 3), I added a rule nobody had asked for, for a card whose Cancel was still waiting for its answer when Make a new key ran. It was wrong, and its own fixes cost three more review rounds (F3 to F10). The owner: "stop widening scope; arm it if round 7 is clean".

**How to apply:** when a re-review comes back clean, arm it (subject to the gates). Carry the listed risks in a PR comment or a memory as follow-ups. Related: [[review-fixes-open-new-questions]].

*References: review-fixes-open-new-questions*

*Observed 2026-10-27 (ux-designer)*

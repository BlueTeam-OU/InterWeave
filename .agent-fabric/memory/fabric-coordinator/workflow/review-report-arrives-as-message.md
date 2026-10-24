---
role: "fabric-coordinator"
class: workflow
topic: "review-report-arrives-as-message"
description: "a code-review subagent's report comes back as a hand-back message, not in its task output file; post-review needs it copied from there"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 2a67fe1a48208064
---

## a code-review subagent's report comes back as a hand-back message, not in its task output file; post-review needs it copied from there

A code-review subagent's final report reaches the session as a hand-back message. Its task output JSONL held no assistant text containing the report, so piping an extract of that file into `fabric-pr post-review` sent an empty body ("the review body is empty", InterWeave #406, 2026-10-24).

**Why:** the harness delivers the report as a message and the completion notice says "not repeated here".

**How to apply:** copy the report verbatim from the hand-back message into a scratchpad file, then run `fabric-pr post-review <n> < file`. Do not parse the output file.

*Anchors: `agent-fabric:bin/fabric-pr::post-review`*

*Observed 2026-10-24 (fabric-coordinator)*

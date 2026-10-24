---
role: "architect-cto"
class: workflow
topic: "reflection-gate-evidence-is-a-committed-artifact-never-a-message-or-scratch-log"
description: "A stage-gate clause cites a committed log with the device, API level, build ids, commits and rows; a message reporting a run is history, not evidence, and a log left in /var/tmp or a scratch directory is wiped by the host"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 903c4a9816aeb2b8
---

## A stage-gate clause cites a committed log with the device, API level, build ids, commits and rows; a message reporting a run is history, not evidence, and a log left in /var/tmp or a scratch directory is wiped by the host

InterWeave 2026-10-12..17: DECISION 01a12770-540b names a committed
real-device log as gate (c)'s evidence; "a message cannot stand in the
closing record as gate (c)'s evidence; a committed log can"
(01a15395-17be); "the 2026-10-16 log… sat in a /var/tmp scratchpad
that the host's wipe… removed" (01a15394-9045). The plan states it at
gate (c) only. Beside
[[reflection-evidence-for-the-owner-names-its-build-and-source]].

*References: reflection-evidence-for-the-owner-names-its-build-and-source*

*Observed 2026-10-23 (architect-cto)*

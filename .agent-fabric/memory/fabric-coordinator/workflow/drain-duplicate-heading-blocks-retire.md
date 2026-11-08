---
role: "fabric-coordinator"
class: workflow
topic: "drain-duplicate-heading-blocks-retire"
description: "a section heading standing in two files of one topic (a stale copy beside its -2 part) makes a retire claim's merge_target ambiguous; curate by removing the stale copy, then the retire resolves"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 867f4f83af56af9d
---

## a section heading standing in two files of one topic (a stale copy beside its -2 part) makes a retire claim's merge_target ambiguous; curate by removing the stale copy, then the retire resolves

In InterWeave, architect-cto/threads/threads-carried-2026-10-18.md and its -2 part carried one
section under the same heading: the first an earlier copy ending at "Still awaited", the -2 the
same text carried on to 2026-10-21. architect-cto-01's empty-body retire claim
(threads-carried-2026-10-18-retire-duplicate) named that heading and was held as MERGE TARGET
AMBIGUOUS in the 2026-10-28 and 2026-10-31 drains. j504 (PR #529, curation commit ec20fc94)
deleted the stale copy and its INDEX line in a commit of its own before the drain; the drain
then retired the section and removed the file (last-drain-report retired_in_siblings).

**Why:** a retire takes exactly one heading, so a duplicate across a topic's files has no single
target, and the hold repeats every drain until someone curates.

**How to apply:** when a hold says MERGE TARGET AMBIGUOUS, grep the topic's files for the
heading; if one copy is a prefix of the other, remove the older copy as a curation commit, then
run the drain from that commit without the hold. See [[drain-hold-keys-are-claim-topics]].

*References: drain-hold-keys-are-claim-topics*

*Anchors: `agent-fabric:tools/fabric/assembler/slices.py::is_retire`*

*Observed 2026-11-03 (fabric-coordinator)*

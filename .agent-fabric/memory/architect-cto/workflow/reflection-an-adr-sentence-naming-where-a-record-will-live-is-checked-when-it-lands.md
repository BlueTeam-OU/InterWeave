---
role: "architect-cto"
class: workflow
topic: "reflection-an-adr-sentence-naming-where-a-record-will-live-is-checked-when-it-lands"
description: "When an ADR says a lane's record \"will be recorded in <dir> when it lands\", the landing PR's verification checks that sentence against where the record actually went, and amends the pointer in the same round"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 866534087e9c04db
---

## When an ADR says a lane's record "will be recorded in <dir> when it lands", the landing PR's verification checks that sentence against where the record actually went, and amends the pointer in the same round

ADR-0059 decision 9 said the command-line client's name, argument
shape and output format would be "recorded in `architecture/clients/
human/` when it lands". The client landed over four PRs (#368, #369,
#404, #429) with its record in `apps/human-cli/README.md`, and nothing
under the named directory ever mentioned it; the sentence stayed wrong
for three days until batch 4's verification read decision 9 against
the tree (fixed as ADR-0059 A 2026-10-25 (v) on IW #442).

**Why:** a forward-looking location in an ADR is a promise the lane
never sees — their PR puts the record where their crate is — and a
reader following the ADR looks in an empty place and concludes the
record does not exist.

**How to apply:** when verifying a landed step against its ADR, grep
the ADR for "recorded in", "lives in", "when it lands" and "owed", and
check each named path exists and names the thing; a wrong pointer is a
small amendment in that round, not a later one. Related:
[[reflection-an-owed-or-built-after-clause-is-false-the-moment-its-pr-merges]].

*References: reflection-an-owed-or-built-after-clause-is-false-the-moment-its-pr-merges*

*Observed 2026-10-25 (architect-cto)*

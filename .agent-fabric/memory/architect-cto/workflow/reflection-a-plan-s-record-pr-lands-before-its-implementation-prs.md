---
role: "architect-cto"
class: workflow
topic: "reflection-a-plan-s-record-pr-lands-before-its-implementation-prs"
description: "W43: When a plan's first PR is \"the ADR + proposed contracts\", land it before the implementation PRs; gzapp's stop-call ADR slipped a month and PR 9 could not open until ADR-079 was written in one night"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - bb091d93be1532f6
---

## W43: When a plan's first PR is "the ADR + proposed contracts", land it before the implementation PRs; gzapp's stop-call ADR slipped a month and PR 9 could not open until ADR-079 was written in one night

A completion plan whose PR 1 is "the governing ADR and the proposed contract drafts" needs that PR landed before PR 2 opens, or the record is written last, from the code, under time pressure (gzapp ADR-079, 2026-10-25: six implementation PRs landed under the plan's authority; the three event kinds PR 9 emits had no record to cite and `ban_unknown_event_kinds` would refuse them; the governing ADR ADR-063 §5 r1 requires for a new policy field set did not exist).

**Why:** a record written first is a target the code conforms to and reviewers can judge against; a record written last is a description that can only agree with what shipped, and every design question the lanes asked in between (8b, 13a, 13b, PR 9) was answered by message instead of by the record.
**How to apply:** when I sequence a plan, the record PR is mine and goes first, with its number reserved on main; when a lane's PR 1 slips, I write the record myself before the next implementation PR is requested, not after the last.

*Observed 2026-10-25 (architect-cto)*

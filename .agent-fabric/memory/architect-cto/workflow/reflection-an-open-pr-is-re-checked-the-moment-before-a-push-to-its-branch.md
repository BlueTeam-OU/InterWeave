---
role: "architect-cto"
class: workflow
topic: "reflection-an-open-pr-is-re-checked-the-moment-before-a-push-to-its-branch"
description: "before pushing new work onto an existing PR's branch, read the PR's merge state in the same minute, not from a listing read earlier; an armed PR merges between two commands"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - acd80cfb134f3235
---

## before pushing new work onto an existing PR's branch, read the PR's merge state in the same minute, not from a listing read earlier; an armed PR merges between two commands

2026-11-05, j539: `fabric-pr sessions /OPEN` listed #571 open; while I
folded two commits onto its branch (one-open-PR rule) the queue merged it
(1aadab26f, 18:20:31Z), and my push landed on a merged branch. GitHub
kept reporting the PR's old head, which was the first sign. The branch
delete was then refused by the classifier, so the stray commits stayed
on origin.

**Why:** an armed PR is standing consent; "open" read a minute ago is not
"open" now, and a push to a merged branch is noise nobody reviews.

**How to apply:** read `gh pr view <n> --json state,mergedAt,autoMergeRequest`
immediately before the push; if armed or queued, do not add to it — commit
on your own branch off origin/main and open the next PR when it merges.
Build the merged head on your own branch first and push to the PR branch
as the last step. See [[reflection-the-arming-word-is-given-on-a-finished-head-only]].

*References: reflection-the-arming-word-is-given-on-a-finished-head-only*

*Observed 2026-11-05 (architect-cto)*

---
role: "architect-cto"
class: workflow
topic: "reflection-commit-before-the-active-job-leaves"
description: "delivering the active job and switching through a queued reply-only job leaves no active job, and the commit hook then refuses every commit in that session"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 956ee0ae754a6673
---

## delivering the active job and switching through a queued reply-only job leaves no active job, and the commit hook then refuses every commit in that session

While a PR of mine still waits on a reply I will fold (a language-culture
read, a supply), keep its job ACTIVE in the session: do not `fabric-jobs
deliver` it and `switch` through a reply-only job to tidy the list.

**Why:** 2026-11-13, InterWeave #663: I delivered j666/j676, switched to
j677 (a word already sent) and closed it; no job was active when
language-culture-en's text arrived, so the PreToolUse hook refused
`git commit`, and `fabric-jobs switch` refused the dirty tree. The way out
was a tagged stash, a new job and a fresh session.

**How to apply:** fold and commit first, deliver after; a reply-only job
is switched to only when the tree is clean and nothing more is to be
committed in this session. See [[arming-request-arriving-after-delivery-is-answered-in-place]].

`fabric-jobs drop <id>` takes a reason as its second argument; a drop
without one is refused, and a `switch` run after it activates the job
meant to be dropped (j678, 2026-11-13).

*References: arming-request-arriving-after-delivery-is-answered-in-place*

*Observed 2026-11-13 (architect-cto)*

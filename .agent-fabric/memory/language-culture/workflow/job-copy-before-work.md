---
role: "language-culture"
class: workflow
topic: "job-copy-before-work"
description: "check a job's recorded working copy against its project before working it; a mismatch blocks the commit"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3b8bb31a4218da11
---

## check a job's recorded working copy against its project before working it; a mismatch blocks the commit

Before starting a job, compare `fabric-jobs show <id>` "project" with "working copy". On 2026-11-10 job j253
(gzapp, a REQUEST for product/i18n/en-US.json) was recorded with the InterWeave worktree; the job-copy guard
(agent-fabric ADR-037 rule 3) refused the commit in ~/projects/gzapp after the copy was finished.

**Why:** a session commits only in its job's copy; the work was done and then had to be carried to another session.

**How to apply:** on a mismatch, settle the job first (a REQUEST that has an amendment queued as its own job:
drop it as superseded and reply), then `fabric-jobs switch` to the job whose copy is right; keep an
unfinished edit as a patch in the notes directory and name it in `fabric-fresh --note`.
When the mismatched job came from a REQUEST and already records a branch (j314, 2026-11-14): `fabric-jobs project`
refuses (branch recorded), `drop` refuses until the REQUEST is answered, and `add --request` refuses a message that
is already a job. What works: compose and validate from a detached scratch worktree of the right clone (reading and
`git worktree add` are allowed), save the patch in the notes directory, `pause` the job with the reason, `add` a
carrier job with `--project`/`--working-copy` naming the right clone and the patch, `switch` to it; the fresh
session commits, sends the REPLY with IN-REPLY-TO the request, then drops the paused job.
If the paused job comes back as active in a later session (j314 did, 2026-11-14, after j315 and j316 were
delivered), check the carrier's commit on origin and its reply, then `fabric-jobs deliver <paused> commit:… branch:…
reply:…` with the carrier's artifacts. Do not redo the work or send a second reply.
Related: [[gzapp-english-holder]].

*References: gzapp-english-holder*

*Anchors: `agent-fabric:runtime/claude-code/hooks/job-copy-guard.py::outside J's working copy and worktree`*

*Observed 2026-11-14 (language-culture)*

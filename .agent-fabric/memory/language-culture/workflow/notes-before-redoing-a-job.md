---
role: "language-culture"
class: workflow
topic: "notes-before-redoing-a-job"
description: "read the day's notes for a job before starting it; a held commit may be saved as a patch with a promised next step"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 948c5983f17eacae
---

## read the day's notes for a job before starting it; a held commit may be saved as a patch with a promised next step

Before working a job, grep the day's notes (`notes/YYYY-MM-DD.md`) for its id. On 2026-11-12, j290 had already been answered in an earlier session. Its REPLY (seq 1052608) promised a branch name and a text, its commit was held back by the one-job-per-session rule, and its edit was saved as `notes/j290-pass.patch`. The new session reworded the paragraph from scratch before finding that record, and then dropped its own variant.

**Why:** a second wording would contradict the text the caller already holds from the earlier reply.

**How to apply:** if the notes hold a saved patch and a promised branch, apply that patch on the named branch and deliver it unchanged. Re-read it only if the tree has moved under it. Related: [[check-commit-before-reply]], [[job-copy-before-work]].

*References: check-commit-before-reply, job-copy-before-work*

*Observed 2026-11-12 (language-culture)*

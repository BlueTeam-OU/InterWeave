---
role: "architect-cto"
class: workflow
topic: "job-artifacts-name-the-repository"
description: "fabric-jobs sweep closes a delivered job only when an artifact reads `<project> #N` or `owner/repo#N`; a bare `#N` or a commit on a for/ branch stays forever"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 20a19d96f7d43aa8
---

## fabric-jobs sweep closes a delivered job only when an artifact reads `<project> #N` or `owner/repo#N`; a bare `#N` or a commit on a for/ branch stays forever

`fabric-jobs sweep` resolves a delivered job's evidence only from an artifact written as `<project> #N` or `owner/repo#N`. A bare `#N` is "unresolved" and blocks the sweep even when a later artifact is well-formed; a commit named on a `for/` branch resolves nothing. On 2026-11-01, 69 delivered jobs had sat since mid-October in that state; each closed with `fabric-jobs done` after I checked the PR merged (gh) or the supply sha on origin/main.

**Why:** a job that never closes hides the real open set, and `switch` cannot tell what is owed.

**How to apply:** on `fabric-jobs deliver`, name the repository in the PR reference (`interweave #492`, `gzapp #1183`); for a supply, add the caller's PR once it opens; a message-only job (DECISION, REPLY) closes with `done` once sent. See [[supply-job-closes-on-the-supply-sha-reaching-origin-main]].

*References: supply-job-closes-on-the-supply-sha-reaching-origin-main*

*Observed 2026-11-01 (architect-cto)*

---
role: "architect-cto"
class: threads
topic: "adr-0058-build-dependency-sources-2026-11-09"
description: "ADR-0058 A 2026-11-09 (a -src build-dependency's compiled sources are the artifact's crates; notice_sources.json; PR-time check) SUPPLIED to devex-tooling's j307 as d44a93f47 on a for/ branch from 0b12b777d; REPLY 01a1c129; job j566…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - af18430591d6f95a
---

## ADR-0058 A 2026-11-09 (a -src build-dependency's compiled sources are the artifact's crates; notice_sources.json; PR-time check) SUPPLIED to devex-tooling's j307 as d44a93f47 on a for/ branch from 0b12b777d; REPLY 01a1c129; job j566 closes when the sha reaches origin/main

devex-tooling's QUESTION 01a1c0a5 (2026-11-09): #602's review found release_notices.py walked normal dependencies only, so SQLCipher's BSD-3-Clause and OpenSSL's Apache-2.0 notices were missing from both artifacts' notices; their branch develop-qzapp/devex-tooling/feat/j307-release-notices-omit-sqlcipher-s-bsd (tip 4b79501fe at supply time) adds tools/ci/notice_sources.json (tools by name with reason / sources by name+version+licence+files; unclassified or version mismatch refuses) and tools/checks/check_release_notices.sh in tree checks. Ruled (REPLY 01a1c0a6): amend ADR-0058 (principle stands, text incomplete: decision 1 class, 2.i subdirectory files by path, decision 3 second list, Consequences/Operational PR-time check); gl_generator as a tool holds.

**State (j566, CLOSED 2026-11-09):** #604 merged cb36a1e70 with the supply; amendment committed d44a93f47 on develop-qzapp/architect-cto-01/for/devex-tooling/adr-0058-build-dependency-sources (from 0b12b777d, pushed 2026-11-09), four ADR files only; names/versions/paths checked against their tip; REPLY 01a1c129 sent with the sha. Job delivered; closes when d44a93f47 is an ancestor of origin/main (their PR folds it) — see [[supply-job-closes-on-the-supply-sha-reaching-origin-main]].

**Lesson:** the one-job-per-session hook keys on the job's WORKING COPY, not the branch: a supply for/ branch is created inside the job's worktree (`git checkout -b for/... <their base>`), never in a worktree of its own; a stray worktree's uncommitted work moves as `git diff > patch` / `git apply`, verified identical before the old worktree is removed.

*References: supply-job-closes-on-the-supply-sha-reaching-origin-main*

*Observed 2026-11-09 (architect-cto)*

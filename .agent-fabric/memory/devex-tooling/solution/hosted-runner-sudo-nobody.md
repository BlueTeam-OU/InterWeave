---
role: "devex-tooling"
class: solution
topic: "hosted-runner-sudo-nobody"
description: "need a second uid in an InterWeave CI job? sudo -n -u nobody works on ubuntu-latest, python3 included, TMPDIR=/tmp traversable"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e764aa795b4cce97
---

## need a second uid in an InterWeave CI job? sudo -n -u nobody works on ubuntu-latest, python3 included, TMPDIR=/tmp traversable

On GitHub's ubuntu-latest runner the job user can `sudo -n -u nobody -- <cmd>`
with no setup: `id -u` answers 65534, and `python3` runs as nobody from the
workspace cwd under ~/work. A test whose scratch comes from TMPDIR
needs `TMPDIR: /tmp` pinned so nobody can traverse it. Measured on PR #338
(run 39612744784, 2026-10-21): the Stage 18 foreign-uid socket test ran 1/1
passed in 0.35 s as nobody. This host (develop-qzapp) has no sudo, so the
precheck lines were tested locally with stand-in `sudo` scripts on PATH
(refused, same uid, no python3, working).

**Why:** the first CI run was the only place the switch could be measured.
**How to apply:** reuse `nobody` rather than creating an account; related
[[ignored-row-ci-nextest-exact]] for the selection.

*References: ignored-row-ci-nextest-exact*

*Anchors: `.github/workflows/ci.yml::INTERWEAVE_FOREIGN_UID_USER`*

*Observed 2026-10-21 (devex-tooling)*

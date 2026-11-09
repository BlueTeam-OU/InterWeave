---
role: "devex-tooling"
class: solution
topic: "runs-on-ubuntu-2604-actionlint"
description: "runs-on ubuntu-26.04 fails actionlint's runner-label rule up to v1.7.12; .github/actionlint.yaml declares it (InterWeave #534)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 60241279d3574c47
---

## runs-on ubuntu-26.04 fails actionlint's runner-label rule up to v1.7.12; .github/actionlint.yaml declares it (InterWeave #534)

ubuntu-latest has pointed at the ubuntu-26.04 image since 2026-10-19, and every
job printed a notice saying so. Naming `runs-on: ubuntu-26.04` ends the notice
(InterWeave #534, j279), but actionlint up to v1.7.12 lists hosted labels only
up to 24.04 and fails check_workflows_lint with `[runner-label]`. The fix is
`.github/actionlint.yaml` with `self-hosted-runner: labels: [ubuntu-26.04]`:
the fabric's workflows_lint.py runs actionlint at the root with no -config-file,
so it reads that file. Drop the entry once the pinned actionlint knows the label.
The same will hit gzapp or any other repository that pins the label.
Related: [[ci-command-line-is-read-by-guards]].

*References: ci-command-line-is-read-by-guards*

*Observed 2026-11-04 (devex-tooling)*

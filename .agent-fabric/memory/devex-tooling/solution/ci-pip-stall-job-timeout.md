---
role: "devex-tooling"
class: solution
topic: "ci-pip-stall-job-timeout"
description: "A CANCELLED required job with no failed test: read the check-run annotations first — 'exceeded the maximum execution time' is the job's own timeout-minutes, not the queue"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f114fbb22c8246b0
---

## A CANCELLED required job with no failed test: read the check-run annotations first — 'exceeded the maximum execution time' is the job's own timeout-minutes, not the queue

BlueTeam-OU/InterWeave merge-group run 41862961434 (2026-11-04, #553):
`tool self-tests` ended CANCELLED, log "The operation was canceled" inside
the self-test step, and the queue evicted the PR (`failed_checks`, 12 min
later). The reporter read the 35-min timeout of the `rust` job; the
self-tests job's own `timeout-minutes` is 15, and
`gh api repos/<r>/check-runs/<job id>/annotations` said "The job has
exceeded the maximum execution time of 15m0s". The cancel arrives as
"canceled" in the log either way, so the annotations are the first read.

Cause: "Install validator dependencies" (`pip install --quiet
--require-hashes -r tools/requirements.txt`) took 575 s, printing nothing.
Over 100 self-tests jobs since 2026-10-28 that step is median 3 s, p90 5 s,
next-worst 28 s; job median 270 s, max 454 s. A trickling download, which
pip's socket --timeout does not cut (same shape as
[[ci-apt-stall-measured]]).

Fix (j287, on PR #534): tools/ci/pip_install.sh — three attempts, each
under `timeout --kill-after=10` of PIP_INSTALL_ATTEMPT_SECONDS (120), pip
not --quiet, --disable-pip-version-check (else the upgrade notice lands in
the log the CI-warnings scan reads, [[ci-warnings-scan-reads-text]]); both
ci.yml install steps (tree checks, tool self-tests) call it.

*References: ci-apt-stall-measured, ci-warnings-scan-reads-text*

*Anchors: `tools/ci/pip_install.sh::PIP_INSTALL_ATTEMPT_SECONDS`, `.github/workflows/ci.yml::bash tools/ci/pip_install.sh tools/requirements.txt`*

*Observed 2026-11-04 (devex-tooling)*

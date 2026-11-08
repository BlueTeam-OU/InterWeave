---
role: "rust-services-dev"
class: workflow
topic: "spike-driver-commit-before-cited-runs"
description: "SPIKE-001 runs: commit the driver first, then re-run every case at that commit, so the README cites one revision"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 7e3cab8e04b8c2b2
---

## SPIKE-001 runs: commit the driver first, then re-run every case at that commit, so the README cites one revision

On S25 (2026-10-24, PR #427), a driver fix mid-campaign (a string `message` crashed the timeline) left runs made by two driver states. Committing the drivers and the bridge fix, then re-running all nine cases at that commit (9e333ab9), gave one revision to cite, and every run passed again.

**Why:** a spike record names the tree that produced it. Mixed driver revisions are a review finding ([[spike-facts-cite-committed-runs]]).

**How to apply:** treat the first runs as probes. Once their output shape is settled, commit the drivers, delete `runs/<campaign>-*`, re-run everything at that commit, and only then write facts. Record a wall-clock `started_at` in run.json whenever a fact will compare an event's own timestamp (such as a channel tag's `received_at`) against the run's timings.

*References: spike-facts-cite-committed-runs*

*Anchors: `spikes/spike-001/s25_print.py::started_at`, `spikes/spike-001/s25_print.py::def timeline`*

*Observed 2026-10-24 (rust-services-dev)*

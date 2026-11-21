---
role: "rust-services-dev"
class: workflow
topic: "commit-driver-before-first-run"
description: A spike run records HEAD as its tree; commit a new driver before its first run, or the run names a tree that lacks it
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 7d28c2a56d596cb7
---

## A spike run records HEAD as its tree; commit a new driver before its first run, or the run names a tree that lacks it

SPIKE-001's drivers write `git rev-parse HEAD` into each run's `tree`. In S33 (InterWeave j66, PR #628, 2026-11-11) the trial run and the first campaign run were made before `s33_reply.py`/`s33_proxy.py` were committed, so they record `c6e80e04b`, a tree without the driver, and the README had to say so by hand.

**Why:** a run's record should name the tree that produced it; an uncommitted driver makes that unverifiable.

**How to apply:** commit the driver (even as a trial) before the first run; if a fix follows, commit it before re-running. Committing while runs are going is safe (no checkout), but runs already started keep the old tree. Related: [[mutation-runs-save-the-fix-first]], [[figures-carry-their-run-set]].

*References: figures-carry-their-run-set, mutation-runs-save-the-fix-first*

*Anchors: `interweave:spikes/spike-001/s32_reply.py::rev-parse`*

*Observed 2026-11-11 (rust-services-dev)*

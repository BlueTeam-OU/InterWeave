---
role: "rust-ui-dev"
class: workflow
topic: "xtask-checks-spike-locks-slow"
description: cargo xtask checks in a fresh InterWeave worktree runs past 30 minutes on check_spike_locks; run the relevant checks by name instead
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0fa95f2931605d05
---

## cargo xtask checks in a fresh InterWeave worktree runs past 30 minutes on check_spike_locks; run the relevant checks by name instead

In a fresh InterWeave worktree, `cargo xtask checks` spends more than 30 minutes in `check_spike_locks`, which compiles every spike harness at its pinned revision. It was stopped at the 30-minute background limit on 2026-10-19 (j29, PR #315), leaving about 2.8 GB of `spikes/**/target`.

**Why:** a change that touches no spike gains nothing from that compile, and the leftover targets are a test run's debris.

**How to apply:** for a change outside `spikes/`, run the checks that cover its files by name. For test and matrix changes those are:
- `tools/checks/check_license_headers.sh`
- `check_guards_are_wired.sh`
- `check_domain_fns_are_called.sh`
- `check_human_layering.sh`
- `fabric-python tools/checks/check_security_matrix.py`

If the full run is wanted, start it with `run_in_background` and a timeout over 30 minutes, and remove `spikes/*/target`, `spikes/*/*/target` and `spikes/*/*/*/target` afterwards (measure them first). Related: [[fabric-review-brief-usage]].

*References: fabric-review-brief-usage*

*Anchors: `xtask/src/main.rs::check_spike_locks.sh`*

*Observed 2026-10-19 (rust-ui-dev)*

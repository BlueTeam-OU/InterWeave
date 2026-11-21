---
role: "rust-ui-dev"
class: workflow
topic: "job-may-be-done-elsewhere"
description: "Before starting a fresh-session job, search my sent replies for its source message; the work may already sit on another branch's PR"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 54c5f8742ed1d6e6
---

## Before starting a fresh-session job, search my sent replies for its source message; the work may already sit on another branch's PR

A job's worktree can be opened for work that is already done on another branch: j97 (launch_to_identity unlock check) got a fresh branch on 2026-11-09, but its fix fe0c55e8f had been on j95's device-runs branch since 2026-11-08, in PR #600 with an A40 record.

**Why:** the job note named only the new branch. My own REPLY of 2026-11-08 said where the fix was. A relay replay had aged out (500 messages), but `~/.local/state/agent-fabric/agents/<login>/episodic.db` (table `episodes`, column `content`) still held both sides of the thread.

**How to apply:** at the start of a job, search episodic.db for the source MESSAGE-ID, then `git branch -r --contains <commit>` for any commit the replies name. If the work is done, deliver the job with those artifacts (`fabric-jobs deliver`) and leave the empty branch for the sweep.

*Anchors: `tests/android-e2e/src/screen.rs::launch_to_identity`*

*Observed 2026-11-09 (rust-ui-dev)*

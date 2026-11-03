---
role: "rust-ui-dev"
class: workflow
topic: "keep-full-output-of-flake"
description: "when reporting another lane's test failure, keep the whole panic output, not grep'd location lines"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 23d9cf8ebcf17994
---

## when reporting another lane's test failure, keep the whole panic output, not grep'd location lines

When a full-suite run fails in another role's test, save that test's complete output (panic message, state rows, logs after it) to a named file before the next run overwrites the log, and quote its essentials in the OBSERVATION.

**Why:** pairing.rs:183 (38e78a40) did not reproduce for its owner (p2p-network-dev-01, 2026-10-27), who asked for the step and state row; my log kept only grep'd location lines and was overwritten, so the evidence was lost.

**How to apply:** pipe suites to a per-run log (suite-<sha>.log), never filter at capture; extract with grep afterwards.

*Observed 2026-10-27 (rust-ui-dev)*

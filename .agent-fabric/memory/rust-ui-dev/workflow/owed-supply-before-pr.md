---
role: "rust-ui-dev"
class: workflow
topic: "owed-supply-before-pr"
description: "Run fabric-pr owed-supply before opening or arming a PR; a supply handed off by message can sit unfolded for hours"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 40328222efeb9b0e
---

## Run fabric-pr owed-supply before opening or arming a PR; a supply handed off by message can sit unfolded for hours

Before opening or arming a PR, run `fabric-pr owed-supply` in the clone. On InterWeave j111 (2026-10-19) p2p-network-dev-01's custody supply (REPLY 01a152d9, branch for/rust-ui-dev-01/custody-recovery-api) was delivered in the morning and stayed unfolded until owed-supply listed it while the PR body was being written; the branch's own custody doc already described the API it brought.

**Why:** the rule is fold every supply, then the full suite, then the review; a missed supply means a partial landing and a rerun of the suite.

**How to apply:** run owed-supply first, then fold each listed branch unrebased, then run the suite. `fabric-pr counts` needs a PR number, so it only works after opening.

*Observed 2026-10-19 (rust-ui-dev)*

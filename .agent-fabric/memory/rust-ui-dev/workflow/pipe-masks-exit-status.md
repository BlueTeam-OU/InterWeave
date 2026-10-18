---
role: "rust-ui-dev"
class: workflow
topic: "pipe-masks-exit-status"
description: "cargo clippy/test piped into grep in a && chain: grep's exit status decides, so a failing lint lets the commit through"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8eadfddda6d227f7
---

## cargo clippy/test piped into grep in a && chain: grep's exit status decides, so a failing lint lets the commit through

`cargo clippy ... | grep ... ; git commit` (or `&&` after the pipe) commits even when clippy fails: the pipeline's status is grep's. Happened on j82 (ui-model commit, 2026-10-16), fixed by amending before push.

**Why:** a lint failure that reaches the remote costs a review-fix commit and CI red.
**How to apply:** check `${PIPESTATUS[0]}` after a filtered cargo run, or run the gate unfiltered before `git commit`; never chain a commit after a filtered pipe.

*Observed 2026-10-16 (rust-ui-dev)*

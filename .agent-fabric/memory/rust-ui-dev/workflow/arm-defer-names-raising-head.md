---
role: "rust-ui-dev"
class: workflow
topic: "arm-defer-names-raising-head"
description: "fabric-pr arm --defer names a finding by the head of the review that RAISED it, not the PR's current head"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 01b5c46cf125cf9d
---

## fabric-pr arm --defer names a finding by the head of the review that RAISED it, not the PR's current head

`fabric-pr arm <n> --defer "<head8>:F<n>" "<why>"` takes the head8 of the review that first raised the finding, not the head the chain has reached. On #652 (2026-11-13), `--defer "dad016cc:F2"` (the current head) was refused with "names no open finding (open: bd0c8b64:F2)". `--defer "bd0c8b64:F2"` then armed. The refusal names the right key, so read it and do not guess.

The same arm took a below-floor word as `--authorization <message id>` from architect-cto-01's REPLY, which carried `AUTHORIZES: BlueTeam-OU/InterWeave#652@dad016cc45f5` in its header. A "partly" judgement whose remainder is in another role's file (here the plan's dated entry) stays open across clean re-reviews. Hand it to that role and defer it, naming them. Related: [[armed-pr-after-its-job]].

A queued job whose work landed inside another job's PR cannot be closed with `done` while it is still queued. Run `fabric-jobs start jN` first, then `done jN <artifact>`.

*References: armed-pr-after-its-job*

*Anchors: `agent-fabric:bin/fabric-pr::--defer`*

*Observed 2026-11-13 (rust-ui-dev)*

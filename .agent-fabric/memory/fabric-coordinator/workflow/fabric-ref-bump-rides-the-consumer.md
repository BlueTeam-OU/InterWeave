---
role: "fabric-coordinator"
class: workflow
topic: "fabric-ref-bump-rides-the-consumer"
description: "How an InterWeave .agent-fabric/fabric-ref bump is delivered and checked when another role's PR needs it"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - a4d7d138ff5d04ca
---

## How an InterWeave .agent-fabric/fabric-ref bump is delivered and checked when another role's PR needs it

An InterWeave fabric-ref bump is one work commit, below the floor of eight, so it does not
get a PR of its own. It rides a PR that is already open: a drain PR (#302 carried b3f2dcb5),
or the PR that needs it, as a supply branch `develop-qzapp/user/for/<caller>/fabric-ref-<sha>`
that the caller folds unrebased (j335, 2026-10-21: 71a2263d for p2p-network-dev-01's #324,
commit 51b0792c).

To check a pin before pushing it, add an agent-fabric worktree at the new commit in the
scratchpad and run `cargo xtask checks` and the CI self-test loop (run_suite.sh over
tools/{gh,checks,ci,host/android}/test_*.sh) with AGENT_FABRIC_ROOT pointing at that worktree.
On develop-qzapp three failures come from the host and not from the fabric: check_rustc_pin
(Fedora rustc 1.99 against the 1.98.1 pin, no rustup) and check_unused_dependencies plus its
self-test (cargo-machete is not installed).

The full id of a GZCoord message that a job title abbreviates (e.g. 01a15f21) is in
episodic.db, table episodes, column message_id; gzcoord-compose --in-reply-to refuses the
short form.

*Anchors: `.github/workflows/ci.yml::.agent-fabric/fabric-ref`, `tools/checks/run_suite.sh`*

*Observed 2026-10-21 (fabric-coordinator)*

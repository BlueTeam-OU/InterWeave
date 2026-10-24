---
role: "rust-ui-dev"
class: workflow
topic: "arming-tool"
description: "How to arm, gate, post reviews in InterWeave — fabric-pr verbs since the tools/gh forwarders were retired (4a010c6b)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b52e5f1f9a6e0e9f
  - d227b3f7b631f978
---

## How to arm a PR in InterWeave since 2026-10-04: tools/gh/arm.sh, not gh pr merge --auto by hand

Since InterWeave #177 (devex-tooling broadcast seq 11455, 2026-10-04): arm with `tools/gh/arm.sh <n> --basis "<one line>"` (agent-fabric runtime/github/arm.sh), never `gh pr merge --auto` by hand (CLAUDE.md §9). Security-boundary paths come from agent-fabric projects/interweave/integration/gh/arm.json; a missing path is proposed to fabric-coordinator, `--boundary` forces the gate meanwhile. Exit 2 = nothing armed. `pr-gate.sh` also exists. #176 was armed by hand just before this landed.
- Rule from 2026-10-04 (seq 11520): `arm.sh --no-boundary` waiver needs architect-cto-01 approval (DECISION/REPLY naming the PR), never my own reading. A boundary PR without waiver still needs review of head + no open thread + owner word. arm.sh will take --waiver <MESSAGE-ID>.

*Observed 2026-10-04 (rust-ui-dev)*

## How to arm, gate, post reviews in InterWeave — fabric-pr verbs since the tools/gh forwarders were retired (4a010c6b)

InterWeave's tools/gh forwarders that had a fabric-pr verb were retired on main (4a010c6b, seen 2026-10-11):
tools/gh/post-review.sh, pr-gate.sh, arm.sh, pr-review-status.sh no longer exist. Use
`../agent-fabric/bin/fabric-pr <verb>` from the working copy: `gate <n>`, `post-review <n>` (body on stdin),
`arm <n> --basis "..."`, `review-status <n>`. Never `gh pr merge --auto` by hand.
Under 8 work commits the basis needs the owner's word.

*Observed 2026-10-11 (rust-ui-dev)*

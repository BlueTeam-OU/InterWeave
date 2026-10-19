---
role: "architect-cto"
class: workflow
topic: "gateway-boundary-authority-architect-cto"
description: "architect-cto is commits.boundary_authority for BlueTeam-OU/agent-fabric-gateway (and agent-fabric): how an arm-below-minimum REQUEST is judged and answered, and that the AUTHORIZES DECISION send passes auto mode since 2026-10-19"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2ae2c7a18fdefa15
---

## architect-cto is commits.boundary_authority for BlueTeam-OU/agent-fabric-gateway (and agent-fabric): how an arm-below-minimum REQUEST is judged and answered, and that the AUTHORIZES DECISION send passes auto mode since 2026-10-19

agent-fabric-settings `repos/agent-fabric-gateway.json` (read through the
GitHub API; no clone on this login) sets `commits.min: 1` and
`commits.boundary_authority: architect-cto`: a security-boundary PR under
boundary_min arms on this role's DECISION carrying the header field
`AUTHORIZES: BlueTeam-OU/agent-fabric-gateway#<n>@<head8>` (a header, not
a body line; FleetDeck #21 taught that). The review gate still applies.
`waiver_role` is null there: only the owner waives a boundary.

Judging one (gateway #70, 2026-10-19, DECISION 01a15277-3816, seq 331456):
`gh pr diff` the head and read every hunk; `fabric-pr review-status <n>
--repo BlueTeam-OU/agent-fabric-gateway` for the review class's review ON
the head and zero threads; `fabric-pr arm <n> --repo … --any-owner
--dry-run --basis x` runs gates 1-4b from any clone (the count rule needs
the repo's own clone and is the only gate that refuses); `gh pr checks`.
Say in the DECISION what was not verified (no live run without a clone).

The send passed auto mode this time (fresh session after
fabric-coordinator's allow entry, agent-fabric d085351d); the 2026-10-18
refusal is history. Compose from the gateway's project: a skeleton
composed in the interweave checkout carries PROJECT/REPOSITORY
interweave and must be corrected by hand.

A REQUEST queued by the inbox gets no working copy: `fabric-jobs switch`
refuses it, and `add --request` cannot be re-run for the same message
once it exists (dropped included). Add a plain job with
`--working-copy` and `--priority blocking` instead.

**Why:** three different refusals (header field, auto mode, no working
copy) on three successive authorizations; none should be rediscovered.
**How to apply:** on the next arm-authority REQUEST from a gateway or
fabric lane, follow the list above in order. See [[rustsec-zbus-family-ignores-and-arming-below-minimum]].

*References: rustsec-zbus-family-ignores-and-arming-below-minimum*

*Observed 2026-10-19 (architect-cto)*

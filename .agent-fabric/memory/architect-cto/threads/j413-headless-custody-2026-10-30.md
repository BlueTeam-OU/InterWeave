---
role: "architect-cto"
class: threads
topic: "j413-headless-custody-2026-10-30"
description: "j413 (R4 of the j422 control-plane design): ADR-0060 d3 and ADR-0038 amended 2026-10-30 on IW #509 — a headless custody/unlock for an automated client's (control) profile when SPIKE-007's envelope lands; who runs store discard, identity…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2b35892e16d53746
---

## j413 (R4 of the j422 control-plane design): ADR-0060 d3 and ADR-0038 amended 2026-10-30 on IW #509 — a headless custody/unlock for an automated client's (control) profile when SPIKE-007's envelope lands; who runs store discard, identity restore and a lost endpoint key's new identity on such a profile

2026-10-30: the fleet hold lifted (DECISION 01a18b15); j413 taken. Precondition held: the j422 design's on-host records landed on agent-fabric main 2026-10-28 (#417: ADR-029 rule 18 + ADR-064 rule 12/14 rows "2026-10-28"); its §8 second-host amendment still waits for a named host. R4's constraint: a lingering control agent cannot answer a passphrase. Written on IW #509 (branch develop-qzapp/architect-cto-01/feat/j413-interweave-adr-0060-d3-adr-0038, head 570159d7, 1 work): ADR-0060 d3 desktop bullet keeps `filesystem-only` selectable for an unattended profile and the envelope unlocks from a credential source no person types; SPIKE-007 never makes the envelope the only desktop custody. ADR-0038 Decision: the source is the account's own secret store handed to the process environment at start, SPIKE-007 names it before the option is selectable; Operational: identity restore is the operator's, a lost endpoint key = ADR-0067 d5's new identity minted by the deployment (`transportctl endpoint init`, roster re-render, owner's word) — the key change the design named (`identity new-key --confirm key-change`) was withdrawn by ADR-0067 on 2026-10-28; Revisit keeps the headless mode through a keychain/HSM adoption; first Amendments table + history/0038-amendments.md. ADR-0060 Operational: `store discard --confirm <scope>` is the operator's over the host's administrative path, never the control agent's, which drives listen/send and no deleting or minting verb. SPIKES.md SPIKE-007 evidence line carries the source. Owed: the review's findings, then the owner's word ("arm 509"); tell fabric-coordinator where R4 landed (REPLY in thread 01a17bff) once merged.

**LANDED 2026-10-30:** #509 (1 work, 1 fix) merged 9d8389f2 on the owner's word ('arm 509'); j413 done; fabric-coordinator told where R4 landed (INFO 01a18b54-5fd3).

*Observed 2026-10-30 (architect-cto)*

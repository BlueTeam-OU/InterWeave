---
role: "p2p-network-dev"
class: threads
topic: "pr452-j126-landed"
description: "#452 (Stage 18 S7/S8 misbehaving relay + AutoNAT server, relayed keepalive) merged e31760d0 2026-10-25; one comment P3 carried to the next relay_driver change"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d1ddac2866c7fdfa
---

## #452 (Stage 18 S7/S8 misbehaving relay + AutoNAT server, relayed keepalive) merged e31760d0 2026-10-25; one comment P3 carried to the next relay_driver change

#452 merged at e31760d0 on 2026-10-25 (14 work, 5 fix), closing j126 and j129. Round 1 of its review gave one P2 (the keepalive's rules for relayed connections had no test that failed when reverted: fixed with two crate tests over a real circuit) and three P3s (the refusal flag outliving its relay listener, fixed by moving it onto `Listening`; a dead set; stale comments). Round 2 found all four fixed and nothing new.

**Carry to the next relay_driver change:** the doc comment on `RelayState::listeners` says an entry leaves on the close or at the request horizon. It also leaves through `abandon_listener` (release/forget). Comment only, and older than #452.

**Why:** the stage record is architect-cto's; told in INFO 01a1753c.
**How to apply:** fix that sentence when relay_driver.rs is next touched. See also [[tightening-a-peer-rule-sweep-every-test-peer]].

*References: tightening-a-peer-rule-sweep-every-test-peer*

*Observed 2026-10-25 (p2p-network-dev)*

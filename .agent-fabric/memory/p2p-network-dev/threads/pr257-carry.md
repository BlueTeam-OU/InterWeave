---
role: "p2p-network-dev"
class: threads
topic: "pr257-carry"
description: "#257 (NetworkDenied + DCUtR crate rebuild/departed IPs) armed 2026-10-11 on the owner's word at 6 work; what carries to the next PR"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - be09f747b228f9bc
---

## #257 (NetworkDenied + DCUtR crate rebuild/departed IPs) armed 2026-10-11 on the owner's word at 6 work; what carries to the next PR

#257 armed 2026-10-11 on the owner's word (6 work, 9 fix). Carry to the next PR:
- P3: no test covers HolePunchScope::network_added with an IPv4-mapped IPv6 addition lifting a departed plain IPv4 (hole_punch.rs network_added's to_canonical()).
- Risk (re-review round 6): the module note says "a relayed connection is the peer's path until a punch lands"; under step 9 the relay stays announced until the direct connection holds for direct_stability_period.
- Accepted limits (owner): the replay of public observations into the rebuilt crate is counted beside the call, not observed in the crate; a relayed connection kept across a change answers every inbound CONNECT from its stale candidate copy for its life.
Lesson: a same-invariant review loop on DCUtR ran 4 rounds before the class fix (departed IPs + one predicate at both doors); enumerate the crate's inputs at round 2, not 4.

LANDED 74437fc6 (2026-10-11). Both carries done on develop-qzapp/p2p-network-dev-01/feat/endpoint-overlay (6de47961 mapped-addition test, mutation-caught; 0dee91ca step-9 wording), unpushed, to share a PR with j62 (plan §20 gate (e), endpoint overlay persistence: proposal (A) persist / (B) re-carry sent to architect-cto seq 108262, awaiting ruling).

*Observed 2026-10-11 (p2p-network-dev)*

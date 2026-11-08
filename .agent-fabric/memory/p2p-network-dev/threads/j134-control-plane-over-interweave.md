---
role: "p2p-network-dev"
class: threads
topic: "j134-control-plane-over-interweave"
description: "j134 control plane over InterWeave (1 of 3) — the request, the branch, case 1 and case 2 (two-host overlay) as planned, the limits 5b names; DELIVERED 2026-10-28, see j134-two-host-and-case-1-built and j134-v0-2-0-release"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b4ccfe316e18727b
---

## j134 control plane over InterWeave (1 of 3) — the request, the branch, case 1 and case 2 (two-host overlay) as planned, the limits 5b names; DELIVERED 2026-10-28, see j134-two-host-and-case-1-built and j134-v0-2-0-release

REQUEST 01a17c02 (architect-cto, 2026-10-27, owner's priority): fabric control plane over InterWeave.
Branch develop-qzapp/p2p-network-dev-01/feat/j134-control-plane-over-interweave-the-one (correction sent, seq 606207); architect-cto commits ADR-0055 5b onto it, fold unrebased; one PR for both cases.

- Case 1 (tests/automated-client): one profile, carrier on gzcoord + `interweave-human listen --input --consume --endpoint fabric-control`; no human-store row after; expired ephemeral send never on the wire. Do NOT start until rust-ui-dev-02's verbs PR is merged.
- Case 2 (X1) as promised: two daemons, each in its own network namespace joined by a veth /30, listening on a WILDCARD (see below; the overlay literal alone never closes on loss), peer's address as a static entry (rule 9's door admits any class, outside rule 2's floor — no transport change). Checks: direct each way + broadcast; link down/up closes and redials; positive control without the static entry never connects. Rootless `unshare -Urn` + veth measured OK here (note: `ip link add a ...` parses "a" as address; use names like va/vb). CI's rust job already lifts the AppArmor userns restriction (ci.yml, the mdns_bounds step), so no devex step was expected; #475's CI is the check.
- Limits to name in 5b: one kernel (no tunnel latency/MTU/skew, no real WireGuard), no NAT, no second uid; ADR-0034 §7 / SPIKE-004 unchanged; owner decides acceptance.
- Item 3: the release run is mine once both lanes land. j125 still blocked on owner (authorized_keys).

Continued in j134-two-host-and-case-1-built (#475, #478) and j134-v0-2-0-release (#487, #499).

*Observed 2026-11-03 (p2p-network-dev)*

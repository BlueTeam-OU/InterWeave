---
role: "p2p-network-dev"
class: solution
topic: "j165-data-plane-hold-handover"
description: "j165 landed: #657 merged c94f6be69 2026-11-13 — data-plane hold, dialer-first agreed selection, late refusal close; what was carried"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - eb0410be71fde442
---

## j165 landed: #657 merged c94f6be69 2026-11-13 — data-plane hold, dialer-first agreed selection, late refusal close; what was carried

j165 (the owner's word 2026-11-13): #657 merged c94f6be69 on 2026-11-13, armed below the minimum on the owner's word after a clean re-review of head 3349fa57a.

What landed: `SubstrateConfig::hold_data_plane` (on in every profile via composition `translate.rs`) holds one direct data-plane connection per peer and pings it; a missed ping closes it. `dialing::held_connections` prefers the connection the LOWER peer id dialled (`OpenConnection.dialer`, from `holds_dialer_role`, role override applied), then oldest, then lower id, and is recomputed after every event reaching `settle_outcome`. `RelayKeepalive.refused` remembers a kept refusal so a peer that later becomes a control peer is closed and reported. The rule is `architecture/transport/libp2p/CONNECTIVITY.md` §14 A 2026-11-13 (written by architect-cto, j667/j668).

Carried, named in §14 as not proved: same-connection choice over two real Swarms (no test crosses two real dials), real NAT expiry, idle past 180 s on two real hosts, the phone's 15 s ping cost (device measurement owed).

Related: [[hold-choice-must-agree-at-both-ends]].

*References: hold-choice-must-agree-at-both-ends*

*Anchors: `crates/transport/libp2p/src/runtime/dialing.rs::held_connections`, `crates/transport/libp2p/src/runtime/dialing.rs::holds_dialer_role`, `crates/transport/libp2p/src/relay_keepalive.rs::refused`*

*Observed 2026-11-13 (p2p-network-dev)*

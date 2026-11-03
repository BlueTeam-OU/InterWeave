---
role: "p2p-network-dev"
class: solution
topic: "wildcard-listener-loopback-first-refuses-cache-route"
description: "A 0.0.0.0 listener answers listen() with 127.0.0.1 and reports the LAN address later; a private discovery route offered in between is refused by ADR-0052 rule 3 and the 5-min discovery memo kept it refused (j133)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 28ab8ea1363f790f
---

## A 0.0.0.0 listener answers listen() with 127.0.0.1 and reports the LAN address later; a private discovery route offered in between is refused by ADR-0052 rule 3 and the 5-min discovery memo kept it refused (j133)

Measured 2026-10-27 (j133); landed in BlueTeam-OU/InterWeave#468, merged bdc7ac89 the same day (9 work, 8 fix, six review rounds).

Symptom: android-e2e `on_stand_ins_with_the_cache_on_the_cache_alone_reconnects` failed "A after the restart", row all None, log `reconnect refused peer=B class=no_known_address` and no dial. Reproduced 1 in 24 only with THREE pairing binaries running at once; CPU contention alone (taskset + busy loops, 0/15) and ten test threads in one binary (0/10) did not.

Mechanism: the stand-in listens on `/ip4/0.0.0.0/tcp/<port>`. libp2p reports one NewListenAddr per interface; the first (loopback) answers `listen` (`commands.rs` translate), so composition spawns its driver before the LAN address is in `active`. The first discovery round offers the peer cache's private route; `Learn` judges rule 3 against `own_listeners` with no private listener -> PrivateWithoutPrivateListener; composition's `changed_candidates` memo then held the offer for RELEARN_INTERVAL_MS (300 s). Only CacheAlone fails because the overlay's address enters by the operator door (rule 9), admitted whatever its class. composition.rs's cache-restart test listens on the private IP explicitly, so it never raced.

Fix: NetworkSet::publish counts each new offered private-range listener IP (HeldListeners::widenings, read via SwarmRuntime::listener_widenings); the memo stores the count read before each offer and re-offers when it moves. After: 72/72 under the same harness.

The Identify sibling was fixed in the same PR: `HeldBack` holds what rule 3 refused per connected peer (distinct routes by `canonical_dial_address`, capped per family, dropped on last close) and re-judges at a widening. Still open by choice: mDNS re-judges only at its one-minute refresh; re-judgements are counted again in the store refusal counts (a rule 5 counting-unit question).

How to apply: a memo of what was OFFERED to a judge whose verdict depends on local state must be invalidated when that state moves (trust already did this in set_trust). A wildcard listener's first reported address is not its whole set.

*Anchors: `crates/transport/composition/src/discovery.rs::changed_candidates`, `crates/transport/libp2p/src/runtime/network_change.rs::fn publish`, `crates/transport/libp2p/src/held_listeners.rs::ListenerWidenings`*

*Observed 2026-10-27 (p2p-network-dev)*

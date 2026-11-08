---
role: "p2p-network-dev"
class: threads
topic: "j49-network-change-binding-ruled-part-2"
description: "§20 step 5 runtime side (j49) ruled by architect-cto seq 33736: NetworkView snapshot of IPs into the ONE detector, one redial per allowlisted peer on an addition, wildcard-only listeners for embedded-android (2 of 2)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ecda657603e317a7
---

## §20 step 5 runtime side (j49) ruled by architect-cto seq 33736: NetworkView snapshot of IPs into the ONE detector, one redial per allowlisted peer on an addition, wildcard-only listeners for embedded-android (2 of 2)

Part 2 of [[j49-network-change-binding-ruled]].

PAUSED 2026-10-09 ~19:10Z by the owner (host restart; fabric-coordinator seq 75835). Branch pushed at e70747ce, clean. Folded architect's supply 2fa69380 (merge c88908ba): ADR-0011/0052 A 2026-10-09 rulings.
Done after the re-review: N1/N3/N4 prose (9bb9975d, 18eef8f3, 6e683189); gap B relay ladder skips a relay the gate still holds (af803ee3 holds_off + 35029a68 wiring, wire test an_addition_inside_the_gates_floor_leaves_the_relay_ladder_alone); gap A just-bound listener offered after observe, if held (e70747ce, dcutr test extended).
RESUME WITH: (1) mutation-check gap B wiring (pass |_| false in mod.rs lift_held_off -> wire test must fail) and gap A (drop .filter(holds) on just_bound -> dcutr test must fail); (2) gap C, architect seq 74818: ADR-0052 rule 3 own-listener test reads bound AND known IP at EVERY instance -- RootFunnel (NewListenAddr/ExpiredListenAddr own listeners), Kademlia set_own_listeners, relay state set_own_listeners, commands.rs:535/1129, mod.rs own_listeners sites (~2554/2600/2639/2706/2786) -- with tests; (3) pre-existing: AutoNAT holds filters (mod.rs ~187 change turn, ~2210 tick) have no test; add one; (4) tell architect the shas for A/B/C so they append dated paragraphs; (5) full xtask ci, merge origin/main after dev-02's PR merges (connection_manager overlap), open PR with counts, blind review of the whole range.
RESUMED 2026-10-10: gap A/B mutations caught; gap C done 7c1eaf7b (HeldListeners shared record published by NetworkSet, RootFunnel ctor arg; NetworkSet::own_listeners at every runtime rule-3 site incl. mdns_tick own param and commands.rs Learn/OfferRoutingPeer; wire test a_departed_ips_listener_admits_no_private_candidate); architect told seq 91460. AutoNAT holds filters untestable on this host (only probeable/public listeners offered). Remaining: xtask ci on 7c1eaf7b, wait for dev-02's PR merge, merge main, open PR with counts, blind review of the whole range.
2026-10-10: merged main (1ee90a9c, ADR same-day amendments kept both; 0011 body order network-addition before relay-hop, architect agreed); re-review 2 fixes b2f154c5/22d6498f (funnel wiring test kademlia_driver a_walk_after...)/a605e63a; xtask ci GREEN on a605e63a. PR #250 OPEN (21 work, 10 fix), body has supply range lines + AWAITING-SUPPLY: architect-cto-01 (their j75: strike "owed/at this writing" clauses; they cut from a605e63a when their pause lifts) -- add its range line when folded, then re-review that range before arming. Blind review dispatched in two parts (A substrate, B surroundings) on a605e63a; post both with post-review.sh, answer findings on the PR.
2026-10-10 later: two-part blind review of a605e63a posted on #250 (no P1; B-F1 P2 = architect j75). Judged and fixed: B-F2 expired backoff not a lift + B-F3 claimed retry guard (f5730cb1), A-F1/A-F3/B-F4/A-F2 docstring (8f95064a, 8c752c68, 22f31edb, ccfb645e, 11ffc4ad); dispositions comment posted; PR body limits gained the DCUtR crate candidate-cache gap (pre-existing follow-up: rebuild dcutr inner on removal). Re-review of a605e63a..11ffc4ad dispatched. Still: architect j75 supply -> add range line, re-review it, then arm via arm.sh.
Re-review of a605e63a..11ffc4ad posted (no P1/P2); its P3 fixed in code (network_lifts pruned by class too) + LRU nit, pushed a8bed51b. #250 (21 work, 18 fix) unarmed. NEXT: when architect's j75 supply lands -> fold, add range line under the AWAITING-SUPPLY in the PR body, dispatch ONE re-review of 11ffc4ad..<new head>, post, then arm.sh once checks are all SUCCESS.
ARMED 2026-10-10 ~07:00Z: #250 (22 work, 18 fix) at 388f2bda via arm.sh, queued at 1; all 9 checks SUCCESS; final re-review clean. arm.sh noted 22 > 16: smaller batches next time. On merge: fabric-jobs deliver/done j49, clean target, start j53 (branch off origin/main).
LANDED 2026-10-10 07:22Z: #250 merged ef73b0b1; j49 done; architect told (seq 95393) with proved/not-proved list. Follow-up owed: DCUtR crate candidate cache (rebuild inner on removal).

*References: j49-network-change-binding-ruled*

*Observed 2026-11-04 (p2p-network-dev)*

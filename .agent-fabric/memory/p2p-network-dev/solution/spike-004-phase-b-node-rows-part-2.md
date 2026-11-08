---
role: "p2p-network-dev"
class: solution
topic: "spike-004-phase-b-node-rows-part-2"
description: "SPIKE-004 phase B's five items as node rows on the podman NAT matrix (PR #127) — the four harness facts a row needs before it measures anything, and the six findings (2 of 2)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6d99d589f35508cc
---

## SPIKE-004 phase B's five items as node rows on the podman NAT matrix (PR #127) — the four harness facts a row needs before it measures anything, and the six findings (2 of 2)

Part 2 of [[spike-004-phase-b-node-rows]].

**SUPERSEDES the rule text above (architect-cto seq 6196, #128 at 1bd94834, reviewed):**
- Hop gate: NOT "as for an unauthorized class" — `class_gate.rs` decides once at
  establishment (239-265) and its poll never revisits, so open connections would keep hop and
  renewals would be accepted (`handler.rs:671` marks a re-ask a renewal). Build a WRAPPING
  connection handler whose `listen_protocol` substitutes `DeniedUpgrade` while the served set is
  empty: refuses RESERVE and CONNECT per request, new or renewal, on new or open connections;
  client sees `ReserveError::Unsupported`/`ConnectError::Unsupported` (the crate's denial status
  is pub(crate)). Not: withholding only ReservationReqReceived, `set_status(Disable)`, or closing
  open connections.
- Close-on-removal: close a connection whose local IP is in `removed` AND in no address still
  bound (`network_change.rs` compares full multiaddrs; two listeners can share an IP).
Cite RELAY.md §8 ("Before the first AutoNAT verdict") and CONNECTIVITY.md §14 item (5).

**2026-09-26 evening: the code PR is #129** (head 3d110bab, not armed): hop gate (read at negotiation and at arrival), close-on-removal (kernel route for outbound local IP; named dials close on any departure), keepalive pinging at BOTH reservation ends, every end answering. early and ifchange re-run at 680bbe10, both PASS (REPRODUCTION-2026-09-26-rebuild.log) — BEFORE the review fixes F1-F3, so a re-run at the final head is owed. Doc points sent to architect-cto seq 6697.

**2026-09-26, the owner: "3 deferrer the 4 point"** — SPIKE-004 phase B's four remaining limits DEFERRED (hole-punch rates in the wild, public VM, carrier CGNAT, independently operated services). Relayed to architect-cto. Phase B now waits only on the ifchange row met at #129's final head and #129 landing (after architect-cto's docs PR).

**Carried after #129 (for the next PR's test list):** (1) P3-1: a disconnected `relay_hop_counters` handle still passes the runtime assertion (loopback cannot move the count). (2) architect-cto's unproven risk: "a former holder is pinged until its last connection closes" rests on the runtime's `open` map dropping the closing connection before the relay's ReservationClosed is handled (runtime/mod.rs ~2306; libp2p-swarm 0.48 lib.rs 901-914/1200 orders it so) — pin with a test closing a holder's ONLY connection and asserting set_reserved gets the empty set. Both need a runtime relay that GRANTS, i.e. the hop gate open at runtime level on loopback — no such seam exists; decide the seam first. #130 (architect-cto, 33ce0236) lands after #129 and carries phase B's closing record.

**#129 MERGED 2026-09-27 07:33Z as af489d38** on the owner's "arm 129". #130 (architect-cto) next: phase B's closing record, the owner arms it. Then mine: SPIKE-010 node rows (mDNS deadline) and the carried runtime-relay-grant test seam.

**SPIKE-010 domain/node rows: PR #131** (2026-09-27, head 718c51e2, not armed, 3 work commits → owner's word), recorded a1b37dd2 at pin af489d38, all six PASS. Guarantee 13's two doors are observed via root_funnel_counters around a dial (0->0; control 0->2 while connected; mutation 2 and 1). Carried P3s: push_expired false labelled PUSHREFUSED; "b not yet connected" unasserted; WORK absolute note; header control list; main.rs:41 wording. Harness facts: interface multicast flag blocks nothing; nft output drop → EPERM → MdnsInterfaceFailed; crate rewrites announced first host to packet source (loopback cannot be crafted from IPv4).

**#131 MERGED 2026-09-27 10:28Z as 5eb28b95** on the owner's "land 131": SPIKE-010 PASS, the Stage 11 mdns deadline MET (architect-cto's a060abe3). Stage 11 now waits only on architect-cto's closing record (owner approves). Mine next: promote SPIKE-010 rows into tests/discovery-conformance ("not run: no multicast domain" where absent) with the five carried P3s; the runtime-relay-grant test seam for #129's two carried checks.

*References: spike-004-phase-b-node-rows*

*Observed 2026-09-26 (p2p-network-dev)*

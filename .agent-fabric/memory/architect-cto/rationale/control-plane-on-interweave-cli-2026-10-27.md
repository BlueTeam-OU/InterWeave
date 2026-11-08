---
role: "architect-cto"
class: rationale
topic: "control-plane-on-interweave-cli-2026-10-27"
description: "owner's direction 2026-10-27 — \"control plane on interweave should be implemented on interweave cli\"; control plane (ADR-029) stays on the relay today (ADR-064 rule 12); relayed to fabric-coordinator; InterWeave side = whatever verbs the…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6b701dbdf06eea9d
  - ba19850a2dee813c
---

## owner's direction 2026-10-27 — "control plane on interweave should be implemented on interweave cli"; control plane (ADR-029) stays on the relay today (ADR-064 rule 12); relayed to fabric-coordinator; InterWeave side = whatever verbs the CLI lacks

2026-10-27. The owner asked "control plane is already using interweave?" Answer from the tree: no. ADR-064 (agent-fabric, accepted 2026-10-23) runs the GZCoord DATA plane over InterWeave on one host beside the relay (every login has an `interweave-daemon@gzcoord` + `interweave-client@gzcoord` unit; roster `runtime/interweave/roster.json`; relay stays transport of record, rule 11; parity live-checks 2026-10-24/25). The CONTROL plane (fabric:control, ADR-029: control agent per account answering signed actions over the relay) stays on the relay by ADR-064 rule 12. The owner corrected my first answer ("gzcoord is about data plane, not control plane") and then directed, verbatim: "control plane on interweave should be implemented on interweave cli".
Relayed verbatim to fabric-coordinator as a DECISION (REPLY-EXPECTED yes) with my reading kept apart: the control agent drives `interweave-human` (apps/human-cli: send, listen, reads with --json, ms times, script-holdable exit codes) as its carrier, ADR-029's signed-action grammar unchanged. InterWeave's side: any verb/output the CLI lacks comes to me as a REQUEST and goes to rust-ui-dev with the record. Also raised: ADR-033 §7 "nothing in this tree integrates it" is stale since ADR-064 (OBSERVATION to fabric-coordinator).
**2026-10-27 06:00Z, fabric-coordinator REPLY 01a17bf0:** the owner confirmed in their session: plan it now, the InterWeave CLI as the carrier (each control agent driving send/listen with --json), not a library binding or a second bridge. Their job j422: design + ADR-029/ADR-064 amendment draft, a dual run with the relay as fallback as GZCoord had, ADR-033 §7 corrected in the same amendment. CLI verbs/outputs the design needs come to me as a REQUEST once named.

[… 7 earlier entries of this thread moved to control-plane-on-interweave-cli-2026-10-27-log.md on 2026-10-28, not drained …]
**08:29Z:** fabric-coordinator accepted R1-R12 into the j422 design and the ADR-064 drafts; R7 already on agent-fabric #400 (python-dev-03: wildcard listener, loopback for same-host rows, overlay for other-host rows). OWED BY ME: carry R4 (headless custody for an account's control profile) into ADR-0060 d3 / ADR-0038 revisit once the design is accepted (their proposal: operator op for discard, owner's word for a key change). R8/R9 on #477 are mine to see through.
**08:45Z:** #475 MERGED cf7089b5 (5b, wildcard, decision 5 measured all on main). Note (ii) supplied 85565595 on for/p2p-network-dev-01/adr-0055-note-ii from 35a324f3 (their follow-up branch feat/j134-case-1-and-review-carries; third case a_listener_on_the_overlay_literal_keeps_its_connection_past_the_close_window). #474 (5 work, 3 fix) MERGED: rule 10 second case + rule 3 on main; j404/j408 closed.
**08:50Z:** p2p-01 carried risk: `peers add` (admin.peers.add_address) refuses 100.64/10 via special_use_host (ADR-0062 d1 floor), so a 5b overlay peer is a config.yaml entry only. Ruled: floor stays (person's door; the fleet re-renders config from the roster, ADR-064 r2); revisit = a person's mesh peer on that range, with rule 10's reading. My first 5b text said rule 9's door admits "whatever its class" — wrong for the add half. Supplied a8fe9ea1 (revises unlanded note (ii) in place) from 5d551a69. Lesson: rule 9's operator door has two halves (configuration vs AddAddress) with different floors; name the half.
**10:15Z:** #477 (5 work, 11 fix) ARMED on the owner's word at 44de4c90 with G1-G6 incl. G5 (`problem` on every outbound event) and the G3 drain/proposal decline (c7b5f684, 7fa781e9). Store facts binding the carrier (unchanged): inbound rows record no receiving endpoint (drained rows print local_endpoint null → rely on R3, one human endpoint); pending rows record no source endpoint (a send naming --endpoint uses --no-pending). Sent to fabric-coordinator (INFO).
**11:00Z:** #477 MERGED 73579a46. Case 1 built on #478 (p2p-01): tests/automated-client/tests/control_plane.rs the_carrier_and_the_control_cli_share_one_profile_and_leave_no_row — two profiles, gzcoord (automated) + fabric-control (human) rows, both leases held at once, traffic reaches only its own endpoint, no store row left (--consume), an expired --no-pending send never arrives after the peer returns; both negatives mutation-checked. Release run after #478 lands.
**14:15Z:** #478 (7 work, 7 fix) MERGED 79c84b29: control_plane.rs case 1 + ADR-0055 note (ii) + add-floor correction on main; j412 closed. Item 3 (release): p2p-01 prepares the version bump + cases taking released binaries; the TAG is the owner's to sign by hand (ADR-0066); evidence under packaging/linux/evidence; deliverable to fabric-coordinator = tag + test paths.

*Observed 2026-10-28 (architect-cto)*

---
role: "p2p-network-dev"
class: threads
topic: "stage-15-r1-r2-shapes-part-2"
description: "Stage 15 work for rust-ui-dev (jobs j10, j11): the shapes architect-cto gave for R1a ServerState, R1b PeerPathChanged, R2 admin.trust.*, and ready(); replay relay seq 11166 for the full text (2 of 2)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0b9bb26bf6bb81b7
---

## Stage 15 work for rust-ui-dev (jobs j10, j11): the shapes architect-cto gave for R1a ServerState, R1b PeerPathChanged, R2 admin.trust.*, and ready(); replay relay seq 11166 for the full text (2 of 2)

Part 2 of [[stage-15-r1-r2-shapes]].

Catalogue rows (architect-cto, 2026-10-03, seq 11291): #175 carries admin.trust.list, admin.trust.set and peer.path_changed as approved PROSE LISTS beside LOCAL-IPC.md's Method/Event tables, because schema_agreement.rs holds the Rust tables to every table row. R1b and R2 each move their shape into its table as a row (Domain, Capability, Params, Result, Since 2.1) in the SAME commit as the schema, the ipc-protocol mirror variant and the agreement test; the list line goes when the row lands. R1a needs no row: server_state is an existing frame.

#175 fix 2f49c2a1 (architect-cto, seq 11448), what R2 builds to: a capability introduced at minor m is requested only in a hello sent AFTER a hello_response from the same daemon showed it selects >= m. A first hello names only 2.0 capabilities. A hello naming a capability above the minor it negotiates is the client's protocol violation: close{ProtocolViolation}, from a 2.0 daemon and from a 2.1 daemon for a minor-0 hello alike. The client re-learns the minor and retries without it; a restart may change the minor. The server keeps its closed parse and adds no leniency. The cost is one probe connection per daemon instance. ServerState is latest-state coalesced, and item 10 is "one at open or when first known, never two pending, the drained one is current". server_state receivers: I proposed (seq after 11448) data connections holding `events`, with R1a changing connection.rs:137 plus a test; the contract line :460 "holding commands" is architect-cto's to fix.

**R1 landed (2026-10-04):** R1a #180, R1b #184 (merged 2026-10-04T17:47Z, head 649665f8, clean final re-review); j10 done. Risk carried from #184's last review, a contract question for architect-cto: ipc-client does not check that the daemon's selected minor is <= the one it proposed (unknown event types are refused anyway, so T4's fix does not depend on it). R2's contract text is on main since #175 (LOCAL-IPC.md:146, :308-331); j11 is next, in a fresh session.

**R2 built (2026-10-04)** on develop-qzapp/p2p-network-dev-01/feat/stage-15-r2-admin-trust, 13 work commits. Two architect-cto rulings shaped it:
- **The list shape** (msg 01a1081f-4319): `local_peer` is a named field, optional, absent when unset; `allowed` rows are `{peer, persisted: false}`; no default and no decision on the wire.
- **The list pages** (msg 01a10820-b85d), because 4096 rows of about 82 bytes is about 336 KB against the 128 KiB body:
  - `trust-list-params {after?}`, where `after` is exclusive and is a position, not a row;
  - `trust-list {local_peer? (first page only), allowed ≤1024 ascending, next?}`.

What was built:
- **Contract and protocol.** The schemas are approved; method 1.1.0, capability 1.2.0, request 1.1.0. The hello gate closes a capability above the negotiated minor with ProtocolViolation on either socket.
- **Client.** It probes the daemon's minor once and relearns it after a ProtocolViolation.
- **Driver.** The composition driver holds the one copy of the policy and publishes it to the substrate and to discovery.
- **Directory cache.** SetTrust forgets the cached directory of every peer that left the data plane.
- **Audit.** Each set is logged under target interweave::audit at INFO. That level is a limit: a profile with log_level warn or error drops the audit line.

**R2 landed 2026-10-04:** #186 merged 2561ee78 after 4 review rounds plus one bot thread. The fixes it took: the audit target is admitted at every log level; the fake revokes both ways; a too-low learnt minor is re-probed; a self-listing allowlist drops the self row. Two questions are open with architect-cto (seq 12418): LOCAL-CLIENT §7 item 11 for trust, and whether the audit requirement holds for embedded hosts. j11 is done.

*References: stage-15-r1-r2-shapes*

*Observed 2026-11-04 (p2p-network-dev)*

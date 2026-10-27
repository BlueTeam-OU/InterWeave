# ADR-0055 — amendment history

### Amendment 2026-10-23 — The fleet's deployment record is agent-fabric ADR-064

The Context cited "agent-fabric ADR-062" as the record of the fleet's
own deployment of InterWeave. That number is agent-fabric's hidden-
Unicode record; the deployment record, written at architect-cto's
proposal of 2026-10-22 and accepted by the owner on 2026-10-23, is
ADR-064 (GZCoord over InterWeave, one host, dual run beside the
relay). Found by the review of that record (fabric-coordinator's REPLY
01a1674d). Propagation: the Context sentence; this note; the log row;
the digest entry.

### Amendment 2026-10-27 — Two hosts over a private overlay the deployment provides

The owner directed on 2026-10-27, confirmed in fabric-coordinator's
session, that the fabric's control plane rides InterWeave through the
command-line client, so that an agent can run on a remote host — the
first likely a hosted edge. fabric-coordinator's REQUEST 01a17bff
asked InterWeave for a two-host decision over a private overlay with
operator-supplied static addresses, no Kademlia and no connectivity
roles, and a two-host case in `tests/automated-client`.
p2p-network-dev-01 (REPLY 01a17c04) read the transport: ADR-0052
rule 9's operator door already admits a static entry of any class,
so an overlay literal needs nothing of the transport; what changes is
the deployment's shape — the listen address and the static entry —
and that a real second host cannot run in CI, where a job gets one
runner and jobs cannot reach each other. Decision 5b states the
shape, puts the overlay on the deployment's side of the line decision
7 draws, and names the case that proves it: two daemons in two
network namespaces on one runner over a point-to-point private link,
with the limits that leaves. The owner's word on those limits for the
first remote host, when given, is recorded here; ADR-0034 §7 and
SPIKE-004's deferred limits stay where decision 5 left them.
Propagation: decision 5b; this note; the log row; the digest entry;
the README row. The case is p2p-network-dev-01's (j134, with the
one-profile carrier-and-client case); the command-line client's verbs
are rust-ui-dev-02's.


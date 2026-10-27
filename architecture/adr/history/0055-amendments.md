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
with the limits that leaves. The first text offered the listen address as
"its overlay address, or a wildcard"; p2p-network-dev-01 measured
the two (REPLY 01a17c34, 2026-10-27): with wildcard listeners the
link's deletion closes the connection at both ends within 10 s and
its return redials, with the overlay literal the loss closes nothing,
since a listener bound to one address reports no change and a daemon
has no platform view, so the decision names the wildcard alone and
says why. The owner's word on those limits for the
first remote host, when given, is recorded here; ADR-0034 §7 and
SPIKE-004's deferred limits stay where decision 5 left them.
Propagation: decision 5b; this note; the log row; the digest entry;
the README row. The case is p2p-network-dev-01's (j134, with the
one-profile carrier-and-client case); the command-line client's verbs
are rust-ui-dev-02's.

### Amendment 2026-10-27 — Decision 5's cross-account port rule is measured

Decision 5 said that another account's daemon being refused the port
this one holds rested on the kernel's `SO_REUSEPORT` rule and was not
measured, the suite having one uid. p2p-network-dev-01 measured it on
the host on 2026-10-27 (REPLY 01a17c4a, their job j125, asked in
architect-cto's REQUEST 01a1722a): two accounts, each under its own
user manager with the installed v0.1.0 programs and `DEPLOYING.md`'s
unit and configuration, one holding `127.0.0.1:47990`, the other's
daemon exiting 1 on "Address already in use" with its unit retrying
every 5 s; the control, once the holder stopped, the next retry
listening within a second. The second account was another agent's,
by fabric-coordinator's route (no shell into another uid). Not run:
a fresh install from the archive in an empty account, which needs an
account the owner opens by hand, carried by name; two hosts, which
decision 5b covers. The sentence now says what was measured and
where; the suite's one-uid limit stands as written. Propagation: the
decision 5 sentence; this note; the log row; the digest entry.

### Amendment 2026-10-27 (ii) — The two amendments of the day read against each other

The blind review of #475 (head 38db9e38) read the two amendments of
2026-10-27 together and found what neither had checked against the
other (F1, F2, both P3; p2p-network-dev-01's OBSERVATION 01a17c62).
Decision 5 had been amended to say the cross-account port rule was
measured while decision 5b's limit sentence, written hours earlier,
still said it stayed unmeasured; decision 5's last sentence and the
Revisit condition still spoke of "a second host" as the day the
release gate and SPIKE-004's limits are consumed, which 5b's private
overlay is not; and 5b's wildcard parenthesis cited a measurement the
kept case did not render, `two_hosts.rs` running the wildcard shape
alone. #475 landed on the owner's word without waiting; this note
follows on p2p-network-dev-01's next branch, which adds the third
case the parenthesis now cites
(`a_listener_on_the_overlay_literal_keeps_its_connection_past_the_close_window`,
35a324f3: both daemons on their overlay literal, the link deleted,
the connection still reported 10 s later; mutated to the wildcard it
fails at "closed within 10s"). A fourth sentence came from the same review's carried risk
(p2p-network-dev-01's OBSERVATION 01a17c8a): the first 5b text said
the operator door admits the overlay literal "whatever its class",
but rule 9's door has two halves, and only the configuration half
is outside rule 2's floor; the run-time add (`peers add`,
`admin.peers.add_address`) is a person's door and keeps ADR-0062
decision 1's special-use floor, which refuses `100.64.0.0/10`
(`reachability::special_use_host` and its unit test
`the_add_floor_refuses_every_special_use_class_and_admits_private_and_public`).
Ruled: the floor stays; a 5b peer is a configuration entry changed by
a re-render, which is the fleet's own mechanism (agent-fabric ADR-064
rule 2), so the deployment loses nothing; a person's mesh peer on
that range is the revisit, with ADR-0052 rule 10's reading of the
range. `DEPLOYING.md` says the same (7d7b65e5). No decision changes;
the sentences now agree. Propagation: decision 5's last sentence; 5b's limit sentence
and parenthesis; the Revisit condition; this note; the log row; the
digest entry. The lesson for the record's author: two amendments of
one record on one day are read against each other before the second
leaves.

### Amendment 2026-10-27 (iii) — The owner accepts decision 5b's limits for the first remote host

Decision 5b's note of the same day said the owner's word on the
two-namespace case's limits would be recorded here when given. It
was given in architect-cto's session on 2026-10-27, verbatim: "accept
the 5b limits but remember me about this after we have the second
host". So the limits the case leaves — one kernel, so no real tunnel
latency, loss, MTU or clock skew; no NAT; no second uid; no real
second host in CI — are accepted for the first remote host, with
ADR-0034 §7's release gate and SPIKE-004's deferred limits unchanged,
and the Revisit conditions gain the second host's running under
decision 5b as the point at which the owner is reminded of what was
accepted, so that a real second host is measured rather than
assumed. Propagation: the Revisit conditions; this note; the log row;
the digest entry.


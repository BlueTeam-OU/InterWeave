# A connection the profile relies on while idle carries a keepalive, a missed liveness closes it, and one direct connection per data-plane peer is held past the idle timeout

**Status:** Accepted — a promotion of rules already in force; what this record states for the first time is named below. Rule 1 is the owner's word of 2026-09-26 ("keepalive yes"); rules 2 and 3 are architect-cto's ruling of 2026-10-25, except rule 3's data-plane clause and its reach-back, which came with the hold on 2026-11-13 as corrected in the review of #657; rules 4 and 5 are the owner's word of 2026-11-13, after the idle measurement; rules 6 and 8 state what the three share, from §14's own words; rule 7 and three of the alternatives are stated here for the first time and are marked so. Rules 1 to 6 and 8 each landed with their build and were in force in `architecture/transport/libp2p/CONNECTIVITY.md` §14 before this record; the plan's §21 record ordered their promotion into one ADR when §14 gained a fourth.

## Context

The substrate has three kinds of connection whose loss the profile would otherwise not notice, because nothing is sent on them for long periods and an idle TCP connection does not report that its path died.

A **relay control connection** carries a reservation. A client that believes it holds a reservation advertises a circuit address through that relay; when the path to the relay dies without an interface event — a NAT rebinding, a carrier drop — the client goes on advertising an address nobody can reach it through. SPIKE-004 phase B measured it on 2026-09-26: after an interface removal the client lost and rebuilt no reservation and neither relay saw its connection end.

A **relayed connection** to a data-plane peer runs over a circuit whose relay the dialling end holds no reservation on. With the keepalive on control connections alone, the dialling end was pinged nowhere on that path: when the relay stopped under the circuit the reserving end was told within the keepalive's bound while the dialling end kept the peer `Connected` and paid the direct timeout on every send (`tests/security/tests/relay_double.rs`, measured as S21-07's limit a). And the pinned ping handler gives up for good the first time the other end refuses the protocol, so a relay that does not speak ping was kept and never found dead (limit b).

A **direct connection** between two data-plane peers was kept past the Swarm's 60 s idle timeout by nothing but a side effect of the pinned GossipSub handler, which parks one outbound stream on a peer's first connection (measured on main a04400b39). A peer that cannot dial back — behind a NAT, or holding no address for this profile — reaches the profile over a connection the profile dialled and over nothing else, so that connection closing while idle left the peer unable to send until the next redial, and a dependency bump could have removed the stream with every check green.

Each gap was closed where it was found, in a dated passage of §14. Three such passages made a decision that binds three behaviours, two connection classes and every profile, with its alternatives and its cost recorded in a transport specification's amendment prose and in no decision record. This record states it once. The mechanics, the measured figures, the crate facts and the named tests stay in §14, which this record cites and does not restate.

## Decision

1. **A relay control peer's connections carry a keepalive, at both ends.** The client pings the relay it holds an active reservation on; the relay pings each peer that holds one of its reservations, or held one until that peer's last connection closes. The keepalive is the bounded ping that `CONNECTIVITY.md` §4's matrix already admits on infrastructure connections, and every end answers ping on them. The unit is the relay control PEER: the relay crate names no connection for a reservation, so every connection to such a peer is pinged whatever its class.

2. **Every relayed connection carries the keepalive end to end.** A connection whose path is a circuit is pinged toward its data-plane peer by both ends. The unit is the CONNECTION: a far peer's direct connection is not pinged under this rule, and a relayed connection retired once a direct path is stable stops being pinged with its closing. The ping's bytes count against the circuit's byte budget.

3. **A relay control peer that refuses the ping protocol is treated as a missed liveness, at both ends.** The client closes the connection and reports the cause once by name; a relay sheds a reservation whose holder refuses. Required, not preferred: a static relay so refusing stays configured, is asked at its ladder step and is refused with that cause. A data-plane peer that refuses ping on a relayed or held connection is counted and kept, with no liveness check — until it becomes a control peer by a reservation in either direction, when the refusal it already gave is a control peer's missed liveness.

4. **A profile holds one direct connection per data-plane peer past the idle timeout.** For each peer it holds a direct connection carrying the data plane to, the runtime names one and keeps it whatever the idle timeout says. Both ends must name the same connection, since either end's idle timeout closes it; the order that makes them agree is §14's. A peer's second direct connection is not held; a relayed connection is not held (rule 2 covers it and its relay's limits end it); a connection to an infrastructure-only peer is not held, its retention being its origin's (ADR-0036). The hold admits and retains nothing the root gate and the trust classification did not already admit as data-plane, and a peer that leaves the data plane has its connection closed as before. The hold is on in every profile through the composition root, the embedded Android runtime included, and off in a bare substrate.

5. **A held connection is pinged.** A connection kept open and never tested would outlive its path, with the peer `Connected` and every send into it lost.

6. **A missed liveness closes the connection whose own ping failed, and nothing else.** On a control connection the close takes the reservation ladder exactly as a removed address does (§8); on a relayed or held connection the peer leaves `Connected` — `Disconnected` to the consumer, or `PeerPathChanged` where another path stands. Who redials after the close, and when, is the retry scheduler's and the discovery round's as for any closed connection (`CONNECTIVITY.md` §12 and §14); this record promises no redial.

7. **The keepalive is liveness, never authentication or authorization** (stated with this record; §14 says it only of the hold, which "retains and admits nothing", and ADR-0017 says it of the IPC keepalive). An answered ping proves a path and a process, not who the peer is or what it may do; it widens no class, extends no reservation, and is no application-level acknowledgement. No built behaviour reads a ping as any of those, so the rule changes nothing that runs; it forbids a later change from doing so without an amendment here.

8. **The interval and the miss count are the ping crate's defaults**, stated in §14 with the pinned version they were read at, and are the same on all three kinds of connection. This record sets no number; a different interval for one kind, or for one platform, is an amendment here with the measurement that asks for it.

## Alternatives considered

**No keepalive; rely on interface events and send failures.** The network-change detector sees an address leave the host and nothing else; a path that dies with every local address intact is invisible to it. A send failure finds the dead path only when someone sends, and for a reservation nobody on this side sends at all — the cost falls on the peers dialling a circuit address that no longer works. Rejected by the owner on 2026-09-26, the traffic and battery cost named as the accepted price.

The next two, and the last of this section, were not weighed in §14: they are recorded with this promotion, as the reasons the rules stand against options a reader would otherwise propose.

**TCP keepalive at the socket (recorded with this promotion).** It tests the kernel's path to the next hop that answers for the socket, not the far process, and it cannot run end to end over a circuit, where the two TCP connections each terminate at the relay. It would also leave a far peer dying behind a live relay undetected. One probe at the protocol layer covers the relay's death and the far peer's alike.

**Raise or remove the idle timeout instead of holding one connection (rule 4; recorded with this promotion).** It would keep every connection of every class, a peer's redundant second connection and a stranger's pre-classification inbound included, and it would keep them untested. The hold names one connection per data-plane peer and rule 5 tests it.

**Ping the circuit's relay rather than the far peer (rule 2).** The dialling end holds no control relationship with that relay, and a relay that answers says nothing about the peer behind it.

**Prefer, rather than require, a relay that answers ping (rule 3).** A preference keeps the refusing relay in the ladder as its last rung, which is the reachability claim rule 1 exists to withdraw: a relay that can never be found dead. No interoperability cost was accepted for it — this profile's relays answer ping on every handler, and a foreign relay that does not is one this profile declines to depend on.

**Leave the GossipSub stream as what keeps a connection (rule 4).** A side effect of a pinned dependency's handler is not a rule; it held with no channel joined and would have gone with a version bump.

**Hold the oldest connection.** As first written on 2026-11-13. Age does not agree between the two ends of two crossing dials — each end's oldest is its own — so each end closed the connection only the other held; the review of #657 found it and the order was corrected before it landed.

**Close a data-plane peer that refuses ping (recorded with this promotion).** A refusal from a peer the profile trusts for application traffic is not evidence its path is dead, and closing would make a peer running a foreign ping-less build unreachable where today it is reachable and merely untested. Kept and counted; the close on refusal stays the control peers' rule.

## Consequences

- A client stops advertising a circuit address through a relay it cannot reach within the keepalive's bound, about a minute for a stopped relay at the defaults; each send through it meanwhile ends `PeerUnreachable`.
- The relay's own ping is what lets a client rebuild a reservation after an interface change: without it the relay keeps the dead reservation and refuses the new one for per-peer room until it lapses.
- A peer that cannot dial back stays able to send for as long as the pair's held connection lives, and ADR-0055 decision 2 promises exactly that and no more: no redial from the side that holds no address.
- Every profile now sends a ping per held peer and per relayed connection at the crate's interval. On a phone each wakes the radio. The phone holds at the same interval as every profile until a device measurement says otherwise, and that measurement is owed before a release note calls the phone's idle cost known.
- An operator's small circuit byte budget is shortened by the ping's share; at the defaults the share is about 0.02 %.
- A foreign relay that declines ping cannot serve this profile.
- `ping` is in the protocol set of every profile the composition root builds: the pinger on the connections rules 1, 2 and 5 name, an answer-only echo on every other connection it is offered.
- A connection count is no longer bounded by activity: a profile with N allowed data-plane peers online holds N connections indefinitely. The bound is the trust set and the connection bounds of ADR-0026 and ADR-0035, not the idle timeout.

## Security implications

The keepalive adds one protocol to infrastructure connections and one long-lived connection per data-plane peer; it adds no authority. Rule 7 is the invariant: nothing reads an answered ping as identity or permission, and the hold applies after the root gate and the trust classification, so it cannot retain a connection of a class they refuse (`exactly_one_direct_data_plane_connection_per_peer_is_held`, `crates/transport/libp2p/src/runtime/dialing.rs`, whose infrastructure-only peer is not held). A peer that loses data-plane trust has its held connection closed by the gating change, as before.

What an attacker gains: an allowed data-plane peer can pin one connection of this profile open for as long as it answers ping, which it could already do by sending; an unauthorized peer gains nothing, since its connection is closed at establishment and never held. A relay can force a client off itself by refusing or dropping ping, which a relay can do by closing the connection. A peer that answers ping while dropping application streams is kept — the keepalive detects a dead path, not a misbehaving peer; each send to such a peer ends at the direct protocol's timeout, and no rule here or elsewhere closes it for that.

Residual risk accepted: the two ends can still rank two connections dialled by the same end within one handshake of each other differently and lose both to the idle timeout (§14, by name); and the regular ping is a traffic pattern an on-path observer can read as "this profile holds a session with that address", which the long-lived connection already disclosed.

## Operational implications

There is no profile setting for the keepalive or the hold: both are on in every profile the composition root builds, and a relay-configured profile answers ping on its infrastructure connections. An operator sees a relay that refuses the keepalive by name, once, as the cause of a lost or refused reservation, and sees a held data-plane peer's refusal as a count. A stopped relay is found within about a minute. A relay operator sizing `max_circuit_bytes` small should know the ping draws on it. The idle timeout still governs every connection this record does not name.

## Implementation implications

All built, and inside `crates/transport/libp2p` (ADR-0045): the keepalive in `src/relay_keepalive.rs`, the hold's choice in `src/runtime/dialing.rs` (`held_connections`), the switch `SubstrateConfig::hold_data_plane` set by the composition root's `translate.rs`. The tests are `crates/transport/libp2p/tests/relay_keepalive.rs` and `tests/held_data_plane.rs`, `tests/security/tests/relay_double.rs`, and `tests/automated-client/tests/two_hosts.rs`; §14 names each case against the clause it pins, and what is not proved. Nothing here is gated on a later stage. A change to any of the eight rules is made here and §14 follows; a change to the mechanics that leaves the rules standing is §14's alone.

## Revisit conditions

- A device measurement of the phone's idle cost that the current interval makes unacceptable: rule 8 then takes a per-platform interval, or rule 4 a platform exception.
- A deployment that must use relays it does not operate and that do not answer ping: rule 3's "required" is reopened with that deployment's evidence.
- A profile whose allowed data-plane set is large enough that one held connection per online peer presses on the connection bounds of ADR-0026 and ADR-0035: rule 4 then needs a bound and an order of who is held.
- An upstream change that gives a reservation a connection outside the relay crate, which would let rule 1's unit become the connection; or one that gives the idle timeout a per-connection override that both ends negotiate, which would replace rule 4's ordering.
- A measured loss of both connections in the same-dialler case rule 4 leaves open.

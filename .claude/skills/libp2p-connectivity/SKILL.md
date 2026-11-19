---
name: libp2p-connectivity
description: What Stage 11 (AutoNAT, Circuit Relay, DCUtR) established about the libp2p connectivity code and still binds it — the three routes to a dialled or retained infrastructure-only connection, the D1-D3 violations and their fixes, the class gate, the gating-change close, and what each connectivity test pins. Use before changing anything under crates/transport/libp2p or tests/connectivity, before touching the outbound gate, the inbound arm, a connectivity behaviour's wrapper or its switch, and whenever a comment cites a "route 1/2/3". Rule of thumb: read the route list itself, never a summary of it.
---

# What Stage 11 left binding on the connectivity code

This text stood in the repository's instruction file, §1, until
2026-11-19, and is kept whole because its findings bind what is built
on the stage. A comment that cites "route 1", "route 2" or "route 3",
or the instruction file's §1 on AutoNAT, relay or DCUtR, means the
list below. The record of the stage itself is the plan's §14 closing
record and `architecture/roadmap/SPIKES.md`; where this text and a
contract disagree, the contract wins and this text is what gets fixed.

**Stage 11 closed 2026-09-27** on SPIKE-004 phase
B's evidence with four deferred limits carried by name — the
plan's §14 closing record. The stage's history, kept because its findings still bind
what is built on it: SPIKE-004's **phase A closed 2026-09-01: PASS
for implementation**, so the AutoNAT/Relay/DCUtR work was authorized.
Two things it did NOT settle bound everything built then: phase B,
and its findings. **Phase B — the real-NAT matrix — ran as a containerised NAT row (ruled satisfied on 2026-09-09 with
three deferrals: the population claim, a public VM, a
carrier's CGNAT); its other five items ran as node rows on 2026-09-26:
relay loss, capacity denial and the two services PASS; success
rates and cost MEASURED; interface change MET at #129's final code once the §14
rules landed and the row re-ran (log 3e3edb4d) — PHASE B CLOSED
by the record of 2026-09-26, effective on its landing, four
deferred limits named; the STAGE closed on 2026-09-27 with
those limits carried** — and no production code may assume server-mode
reachability evidence for ADR-0034 §7's release gate exists: that gate
is a separate decision, consuming phase B's rows with the limits stated. And its findings bind rather
than inform. Attribution must precede enabling any of the three
behaviours; a gate refusal of a behaviour dial is INVISIBLE, so the
gate must record its own refusals; `AUTONAT.md` §7 must be implemented
here at the PENDING hook; `RelayCircuit` is a command-path origin;
`RELAY.md` §8's budgets are not the crate's defaults in any row that
matters, and a per-peer ceiling admits one more than it says
(measured for circuits, read for reservations); DCUtR has no knobs, so
§13's bounds must be built by an adapter tracking the attempt
lifecycle — one punch is a dial at both ends, so no single gate sees
the attempt and the outcome reaches the behaviour rather than the
gate; and `contracts/CONNECTIVITY.md` §5's "no second
`PeerConnected`" is work, because the Swarm reports a second
connection for a peer already connected — **done in step 7**: the
consumer's `Connected`, `PeerPathChanged` and `Disconnected` are
derived once per logical peer from the open set
(`dialing::path_events`), never from the Swarm's per-connection
events. It found **three
violations sitting in already-shipped code, none ever reachable in a
shipped build**, and **step 2 fixed all three (D1 and D2 on
2026-09-04, D3 on 2026-09-05); the harness reports zero divergences.**
**`autonat`, `relay` and `dcutr` are now IN the workspace libp2p
features**, added after step 2 in a change that constructs nothing.
**Since steps 3 to 8, ALL THREE behaviours — AutoNAT and Circuit
Relay each in both roles, and DCUtR — have a constructor and a
switch**: `SubstrateBehaviour.autonat_client` is built when
`SubstrateConfig.autonat_client` is `Some`,
`SubstrateBehaviour.autonat_server` when `autonat_server` is,
`SubstrateBehaviour.relay_client` — together with the relay
TRANSPORT, which the Swarm builder composes beside it and nowhere
else — when `relay_client` is, `SubstrateBehaviour.relay_server`
when `relay_server` is, and `SubstrateBehaviour.dcutr` when `dcutr`
is; all five are `None` by default — gated off by the 2026-09-07
ruling; the composition root (Stage 12) is where a profile's block
becomes a `Some`. **A configuration path EXISTS and reaches
the switch only through that root.**
`profile-config` models and validates the whole
`transport.connectivity` block, and its `infrastructure.allowed_peers`
is the first production site that builds an `InfrastructureSet`; the
libp2p crate translates the client's block
(`AutonatClientSettings::from_profile`); until Stage 12 nothing called
that translation from a profile, so a profile setting
`autonat.client.enabled` or `relay.client.enabled` constructed
nothing, and the validated config was a document shape, not a
switch — since Stage 12 (b28af0ed, 2026-09-27) the composition root's
`translate.rs` calls every `from_profile`: the client roles run in
every valid profile (their `enabled` is `literal[true]`, so the
setting named above cannot be false), and a server role's block,
or Kademlia's discovery entry, is the switch. That ended the era in which §3's promise was kept by the
compiler: a behaviour can now be switched on by writing code rather
than by editing a manifest, so from here the guarantee is the outbound
gate, the trust classification and their tests. **Two
infrastructure-only states must not be confused.** An inbound connection
from such a peer has ALWAYS been ESTABLISHED and then closed — neither
gate denies at the established inbound hook — so it needs no relay
code and is reachable today through ordinary configuration;
`tests/connectivity/tests/advertised_protocol_set.rs` pins it. **What
is advertised in that window is NOT pinned and must not be relied on**:
it measured empty, five runs out of five, but that rests on the
subject's handler not getting CPU before the close takes effect, and
`transport/libp2p/CONNECTIVITY.md`'s matrix gives Identify a `yes` for
this class anyway. The test records what it sees there and asserts only
the establish-then-close. A connection DIALLED or
RETAINED as infrastructure-only CAN now exist, by all three routes
and only when a client or a server is configured. There are
**three** routes to one; step 3's adapter reaches routes 2 and 3 (see
below) and step 4's server role reaches route 1. "Reached" is
deliberately weaker than "blocked": route 2 is a grep and not a guard,
as `behaviour.rs` says in as many words. **Read the list below rather
than any summary of it** — including this one: which route is which has
been written down wrong in both directions, more than once.
Retention is decided by `authorizes_for(class, origin)`, so any route
that supplies `RelayReservation` or `AutonatProbe` reaches it.

1. **A WRAPPED BEHAVIOUR whose classifier announces one.** The
   intended route for both: `Attributing` announces the origin from
   the behaviour's own `poll`, and `OutboundAdmission`'s pending hook
   resolves it, mints the ticket and deposits it — no `attempt_dial`
   anywhere. **The feature list barred this route FOR THESE THREE
   BEHAVIOURS ONLY** — you cannot wrap what you cannot construct — and
   not in general: `Attributing<B>` is generic over every
   `NetworkBehaviour` and `always` is exported from the crate root, so
   wrapping an already-compiled dialling behaviour (`request-response`
   since Stage 6, `kad` since Stage 10) with `always(AutonatProbe)`
   would have reached retention with the connectivity features off.
   Enabling them removed the narrow barrier; the AutoNAT client is
   constructed when configured but is NOT wrapped in `Attributing`
   (it never dials, below). **The AutoNAT SERVER IS, and this route
   is reached by step 4**: `autonat_server` is
   `Toggle<ClassGated<Attributing<ProbeServer>>>` (the wrapper owns
   the vendored server; the toggle is the switch) with
   `always(AutonatProbe)`, so its dial-back — the one dial AutoNAT v2
   makes — is announced, admitted by the root policy at the gate's
   pending hook, and retained under `authorizes_for(class,
   AutonatProbe)` when it establishes. What stands between that dial
   and an arbitrary target is `ProbeServer`'s pending hook, after
   the gate's: `AUTONAT.md` §7's literal-IP, source-equality and
   address-class rule, refusing before any socket (the gate takes the
   refused dial's ticket back, PR #91). **The relay CLIENT is wrapped
   the same way (step 5)**: `relay_client` is
   `Toggle<ClassGated<Attributing<ReservationScope<..>>>>` with
   `always(RelayReservation)`, so the control dial the client makes
   for a reservation — to the relay as peer, at the configured
   address plus whatever the other behaviours hold for it, when it
   holds no connection to it — is announced, admitted by the root policy,
   and retained under `authorizes_for(class, RelayReservation)`;
   what the Swarm advertises for it is `ReservationManager`'s set,
   the crate's own confirmation swallowed. **The relay SERVER (step
   6) dials nothing** — a reservation rides the requester's inbound,
   which the inbound arm retains under `RelayReservation` when the
   server is on, and a circuit's far end is reached over the
   connection the destination already holds — so it is class-gated
   for the infrastructure service and not wrapped in `Attributing`;
   and since 2026-09-26 it offers the hop protocol only while it
   holds a verified direct address (`RELAY.md` §8's hop gate,
   `hop_gate.rs`), refused PER REQUEST below the class gate, a
   renewal on an open connection included;
   `tests/connectivity/tests/relay_server.rs` pins the retention,
   the gate shut on loopback (a reservation and a circuit refused as
   an unsupported protocol, the connection kept) and the stranger
   closed, and `crates/transport/libp2p/tests/relay_hop_gate.rs` the
   gate opening and shutting and the two reservation ceilings exact
   on the wire (the crate's per-peer ones are handed over one below,
   since the crate admits one more than told; the circuit ceilings
   are the unit test's). **DCUtR (step 8) is wrapped the same way**:
   `dcutr` is `Toggle<ClassGated<Attributing<HolePunchScope>>>`
   with `always(DcutrHolePunch)` under the DATA-PLANE class gate, so
   a non-data-plane peer is offered no DCUtR handler at all (D1's
   rule at the handler, beside the gate's) and every punch dial —
   one at EACH end, as SPIKE-004 measured — is announced, admitted
   by the root policy toward the data-plane far end, and retained
   under `authorizes_for(class, DcutrHolePunch)`; `HolePunchScope`
   is §13's attempt lifecycle the crate lacks (four in flight, one
   per peer, the five-minute cooldown, an attempt horizon), and a
   direct connection that comes up while an attempt is in flight is
   the punch whichever end dialled it — and the peer's PATH only once
   it has held for `direct_stability_period` (step 9: the relay stays
   the announced path meanwhile, and a relayed connection is retired
   when safe once any stable direct connection — the punch past its
   interval, or a dialled one — is the path), `DialPeer` reusing a
   direct connection that carries the data plane, dialling direct
   first and a circuit route only after the 750 ms head-start; and a
   NETWORK CHANGE (step 10) — an address leaving the runtime's known
   IP set (the listeners' bound set, or the platform's view on
   Android), seen once by the runtime, with the AutoNAT client off
   too; an addition is reported, invalidates nothing and lifts the
   held-off peers' backoff once per lift floor — gives up
   every attempt (ended `Abandoned`, no cooldown, once the crate is
   done) and lifts every cooldown, sends the
   AutoNAT verdict to `unknown` with a jittered re-test, and --
   since 2026-09-26 -- closes every connection that ran from an IP
   the change took off the host and nothing else
   (the detector's removed IP set in `network_change`, fed by the
   listeners and the platform's view; two listeners on one IP losing
   one close nothing), while relay control connections carry a
   keepalive whose missed ping closes them too (`relay_keepalive`).
   What stands between a punch
   dial and an arbitrary target is `DCUTR.md` §6's address-class
   boundary (ADR-0052) at the wrapper's pending hook, after the
   gate's: a candidate outside it — loopback, link-local, a DNS
   name, a private range on a host with no private listener of that
   family — is removed from the dial before any socket (the crate's
   dial denied, the survivors reissued as the wrapper's), and a dial
   with no survivor ends the attempt `refused_by_class`; the same
   boundary keeps such a
   candidate out of what this profile learns and offers, so what it
   sends in a CONNECT is inside it. `tests/connectivity/tests/
   dcutr.rs` pins the upgrade at both ends as `PeerPathChanged {
   HolePunched }` with no punch dial refused (over the host's
   private-range pair, since loopback is refused), a bare initiator's
   loopback candidate refused before any socket and, beside a private
   one, removed with the punch made, a loopback-only
   subject sending no candidate at all, an infrastructure-only source
   over a circuit starting no attempt, and a peer that does not punch
   failing the attempt and having its next circuit declined for the
   cooldown. `tests/connectivity/tests/relay_client.rs` pins the
   reservation, the class-gated protocol set on the retained
   connection, the gate's refusal of an unauthorized static relay
   under the same origin, the withdrawal within a second of the
   loss, and learning under the opt-in.
2. **AN `attempt_dial` CALL SITE passing one.** `attempt_dial` takes
   an origin from any in-crate caller, so one line suffices with no
   behaviour anywhere. The feature list never guarded this, and nothing
   passes either reachability origin today. (The command path is how
   `RelayCircuit` ARRIVES since step 7 — `Dial`, `DialPeer` and the
   retry scheduler classify a `/p2p-circuit` address under it through
   `dialing::origin_for`, since the transport
   rather than a behaviour dials a circuit — that is the same
   MECHANISM, but `RelayCircuit` names an application destination and
   so cannot produce a retained infrastructure-only connection at
   all: `tests/connectivity/tests/relayed_paths.rs` pins the refusal
   before any socket.)
3. **A RELAXATION OF THE INBOUND ARM.** `dialing.rs` retains an
   inbound connection only if `ConnectionManager::authorizes`, which
   asks under `DialOrigin::Manual` and so refuses this class outright.
   The feature list never guarded this either. **Since step 7 the
   PATH is asked first, in both directions** (`retention_origin`): a
   connection that came up over a circuit is judged under
   `RelayCircuit` whatever dialled or answered it, so a relayed
   inbound is retained only for a data-plane source even when the
   closure would name an infrastructure origin — the servers on —
   and a reservation ask that reached its relay THROUGH a relay is
   refused at establishment. That is ADR-0036's inbound relayed
   clause, which SPIKE-004 found had no implementation site.

**Step 3 REACHES routes 2 and 3**, which is why the restriction below
had to land first — it did, and step 3's adapter keeps it true.
Route 2 is `autonat_driver::reconcile` dialling a server the profile
holds no outbound connection to — a static one, or under
`use_authorized_identify_servers` an authorized peer whose Identify
advertised the protocol — through `attempt_dial` under
`AutonatProbe`. Route 3 is the inbound arm in `dialing.rs` asking
`authorizes_for(class, AutonatProbe)` for a peer the adapter holds as
a server — one this profile DIALLED that advertised the dial-request
protocol — and `Manual` for everyone else; keyed on "is a server"
rather than "has a probe outstanding" because the crate emits no
probe-start event and nothing tracks probes in flight (ruled
2026-09-17). A dial-back arrives as an inbound from the
infrastructure-only server and the CLIENT serves
`/libp2p/autonat/2/dial-back` on it; the retained connection is
class-gated and carries Identify and that protocol and nothing else
(measured). `tests/connectivity/tests/autonat_client.rs` pins both
routes over real sockets with an infrastructure-only bystander as the
control, which is still established-then-closed. What that test
cannot show on loopback — a real probe and a real dial-back, since §6
refuses a loopback candidate — is SPIKE-004 phase B's.
**Step 4's server role widens route 3 and reaches route 1.** With
`autonat_server` configured, the inbound arm asks
`authorizes_for(class, AutonatProbe)` for EVERY inbound — §7 lets
every authorized peer ask for a probe, and a request arrives on the
asker's inbound — so an infrastructure-only client's connection is
retained, class-gated for the infrastructure service and offered
Identify and `/libp2p/autonat/2/dial-request` and nothing else
(measured); `Unauthorized` is refused under either origin. The
dial-back is route 1 (above). `tests/connectivity/tests/
autonat_server.rs` pins the retention, the protocol set, a loopback
target refused before any socket with the gate's ticket released,
and two controls: the bystander, and the same client against a
subject with the server off, closed at establishment.
**NOT route 1 for the CLIENT, and this was written down wrong until it
was measured.**
The AutoNAT v2 CLIENT never dials: every `ToSwarm` it emits is
`ExternalAddrConfirmed`, `GenerateEvent` or `NotifyHandler`
(libp2p-autonat 0.16.0, the vendored copy under `third_party/`
per ADR-0051: `v2/client/behaviour.rs`, lines 201, 251 and 315 —
first read at 0.15.0 as 202, 238 and 302, only the indices moved),
because a probe is a request over an ALREADY-OPEN connection.
The dial in AutoNAT v2 belongs to the SERVER — the dial-back at
`v2/server/behaviour.rs:123` — which is step 4's. So wrapping the
client in `Attributing` announces an origin for a dial that never
happens, and the outbound gate sees no probe traffic at all: whatever
enforces `AUTONAT.md` §3 and §6 sits where the CONNECTION is made,
not at the dial hook — and specifically where the OUTBOUND connection
is made, since the client installs its dial-request handler only on a
connection this profile dialled. **A guard written as
a grep over `attempt_dial` call sites would see none of that.** Do not
read the feature change as evidence those paths are live. The exposure
`BOTTOM-UP-IMPLEMENTATION-PLAN.md` §14 names — every data-plane
behaviour installed uniformly on every connection — was about the
RETAINED case, and **it is CLOSED**: `ClassGated<B>` wraps all four,
so a connection of any other class is offered no data-plane protocol
at all. Measured on a retained infrastructure-only inbound, its
Identify carries `/ipfs/id/1.0.0` and `/ipfs/id/push/1.0.0` and
nothing else.
**So the ordering constraint the routes above were written for is
DISCHARGED**, and what replaces it is weaker but not nothing: step 3's
adapter is the first code that produces a retained infrastructure-only
connection, and it keeps that restriction true rather than merely not
preceding it — the retained server inbound in
`tests/connectivity/tests/autonat_client.rs` is offered Identify and
the dial-back protocol and nothing else, and step 4's retained client
inbound in `autonat_server.rs` Identify and the dial-request protocol
and nothing else. The 2026-09-07 ruling: the connectivity behaviours ship gated off and `ClassGated<B>`
land first; both halves are done. The plan's Stage 11 section carries
the ruling, because that is where the construction order lives.
**A gating change closes the connection, in whichever direction it
moves.** A handler is chosen once at establishment and libp2p never
rebuilds it, so a connection whose peer crosses the data-plane
boundary is carrying the wrong protocol set from that moment. Losing
the trust is decided by `connections_to_close`, so the closure lands
in `set_trust`'s ADR-0012 count; gaining it is decided by
`ClassGated::poll`, since a promotion is not a revocation and is not
part of that count. Both are ADR-0036's own instruction: close and
re-establish "rather than allowing a transient privilege mix" — and
the gaining direction is not merely under-privileged, because a peer
holding one `Denied` and one `Allowed` handler is a pair
`NotifyHandler::Any` can route a `kad` query into, where it is
silently dropped.

**The comparison is against the class the connection was ADMITTED
under, not the class the peer held before the change.** That is what
keeps ADR-0036's origin/class separation real: a connection admitted
while the peer was infrastructure-only has carried a denying handler
all along, so nothing is stale, and its origin still decides whether
it survives.
`DcutrHolePunch` (D1) and `RelayCircuit` (D2) were both admitted for
an infrastructure-only peer; the admission predicate — renamed
`names_application_destination` in the same commit, because the old
name described traffic while the rule decides ADR-0036's WITH/FOR
question — now names them, so a circuit or hole punch terminating at
such a peer is refused. D3 was `PreAuthAdmission` bucketing a relayed
inbound by the source PeerId the circuit carries, which
`contracts/CONNECTIVITY.md` §10 forbids by name; `source_label` now
reads the LOCAL address FIRST and charges a `/p2p-circuit` inbound to
the relay — by the relay's PeerId from that address, which the old
signature discarded, else by the relay's IP collapsed to its /64, else
by that address truncated at the marker. **The circuit component
decides, and it is consulted before the remote's IP**: an interim
shape asked the remote for an IP first, which made the rule "no IP
means relayed" and pinned the fix to `libp2p-relay 0.21.1` putting no
address in a circuit's `send_back_addr` (0.22.0 since the 0.57 bump;
`preauth_gate.rs` re-read the lines there and the fact held). **The third case is terminal
on purpose**: while it fell through, a circuit carrying neither still
bucketed on the source, which is D3 in one address shape. **The
`relay:` bucket prefix is a namespace that fix introduced**, so a
relay's circuits cannot collide with a direct inbound from the relay's
own IP. **D2 was a document conflict first**:
`transport/libp2p/CONNECTIVITY.md` §4's matrix and §11 arguably
permitted what ADR-0036's enforcement clause forbids. ADR-0036's
Amendment 2026-09-03 settled it — a circuit whose far end IS the
infrastructure-only peer has a row of its own — and both sections
inherited it, so the code fix that followed had an
unambiguous rule to implement. Read the verdict in
`architecture/roadmap/SPIKES.md` before extending the stage.

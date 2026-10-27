# ADR-0052 — amendment history

### Amendment 2026-09-20 — Peer-supplied names and the paths that enter the book

Raised by p2p-network-dev on PR #111, which builds the DNS transport
(Stage 11's fifth obligation): before it, a `/dns4` address a peer
asserted in Identify's `listen_addrs` was learned into the address book
and failed `MultiaddrNotSupported` on dial — classified structural and
dropped, by accident of a TCP-only Swarm. After it the same address is
resolved and dialled, so a classified peer could make this node query a
name of its choosing and connect to whatever it resolved to. The
record's revisit clause said to reopen it when a name the runtime must
resolve arrived; it was reopened the same day.

Prior wording, rule 2's first clause: "the candidate is a literal IP
multiaddr — no DNS name (the resolver is an oracle)". Prior wording,
rule 8: "A new peer-supplied dial reads this record first. A protocol
that would dial an address a peer named … states its instance of rules
3 and 4 in its transport document before its hook is written".

What changed and why. Rule 1 was read as written — every address dialled
because a peer supplied it — and the Identify path was found inside it
with no hook: `dialing.rs`'s Identify arm learns every `listen_addr`
through `learn_route` into `ConnectionManager::learn_address`, which
checks only that the peer is classified and caps the count. That gap
predates the DNS transport (a loopback or private listen_addr from a
classified peer was learnable and dialable before #111); the transport
widened it from addresses to names. Rule 2's refusal of a name became a
resolver policy rather than a bare refusal, because the design has always
resolved names — ADR-0010's configured bootstrap entries — and what
distinguishes those from a peer's name is who supplied them, not whether
a transport can resolve them: the operator's names resolve, a peer's are
refused at the boundary, and the transport stays. Rule 8 widened from
"a new dial" to "every path that enters the book or a dial", naming the
learn site as the hook where a path originates no dial (mDNS's shape,
built the same day), so that the next path is an instance by rule
rather than by someone noticing. Kademlia routing records and PeerCache
re-entry were named as paths to be read and stated in the change that
reads them, not asserted covered.

The Identify hook lands on PR #111 beside the transport — rule and hook
together, the #102 precedent — as a sibling predicate under rule 6's
subset test, with the instance stated in ADR-0011 §Implementation
implications. The alternative of taking the DNS transport back out
until a resolver policy existed was declined: the policy is one
sentence and the hook one predicate call, and holding a built transport
hostage to them would have re-created the accident (names unresolvable
because of what was built, not because of what was decided).

### Amendment 2026-09-25 — Dials are enforced once, at the root of the outbound path

Raised by p2p-network-dev on PR #111 after the blind re-review of
`6a40a37..8a1d98a` (comment 5827859142) found five paths by which a
peer-supplied address still reached a socket after the two learn
sites of 2026-09-20 had been hooked — libp2p-identify's own address
cache (default 100 entries, returned from its pending hook to any dial
that extends its addresses through the behaviours: Kademlia's walk,
the relay client's reservation dial), Kademlia's in-query FIND_NODE
addresses, the AutoNAT learned-server list, the relay reservation
list, and query-result candidates leaving the driver unfiltered. It
was the third round on one invariant; p2p-network-dev stopped hooking
sites and asked where the rule is enforced for a dial a behaviour
starts itself.

Prior wording, rule 8's opening: "Every path by which a peer-supplied
address enters the book or a dial is an instance, and reads this
record first … Where the path originates no dial, the hook is the
learn site". Prior wording, rule 2: "the local `LearnAddress`
command". Prior wording, rule 5, unchanged in substance: "the hook can
add addresses but not remove one — so the filter runs there by
deny-and-reissue".

What changed and why. The read-back of the pinned `libp2p-swarm`
0.48.0 (`Swarm::dial`) settled the locus: the Swarm calls the ROOT
behaviour's pending-outbound hook once and extends the dial from what
that one call returns, so a wrapper around the composed behaviour —
not a sibling field, which sees only the explicit list (`outbound_gate.rs`
F9) — is the one place every behaviour-contributed address passes
before a socket, and can prune it. Rule 5 gains that clause for
behaviour-extended dials, deny-and-reissue staying the shape for the
explicit list. Rule 8 is restated in two halves because the two hooks
PR #111 built are STORE hooks — they keep the book and the routing
stash clean, which matters because a stored address is later dialled,
offered or handed on — while the DIAL is enforced at the root and
nowhere else; a per-site dial predicate is a diagnostic refinement.
The instance list now says which two stores are hooked, which paths
are not yet enforced, and the two facts that bound the exposure:
Identify's crate-level cache is disabled, and no production composer
of the runtime exists before Stage 12 — the runtime's own `start`
builds Kademlia and the AutoNAT client whenever its config enables
them, so the paths are live in every test or spike that does; the
plan's §15 precondition holds Stage 12 off them until the funnel has
landed with its measurement. Rule 9 answers the provenance question
under all of it: a single funnel must admit the operator's `/dns4`
seed and refuse a peer's, and the routing door as first built
(66e5663) refused the operator's own static-bootstrap name because
`OfferRoutingPeer` carries peer and addresses only. Provenance is made
a property of the DOOR — one operator set, held as a shared handle,
consulted at every store door and at the root — rather than a field on
the carrier, because a field a peer's path could set is a field a
peer's path will set. Rule 2 names `AddAddress`, the command that
exists; the `LearnAddress` it named did not, and the plan's Stage 11
record is corrected the same day. The owner chose on 2026-09-25 to
hold PR #111 for the full closure, so the funnel, its measurement and
the operator set land there beside the store hooks, in
p2p-network-dev's order: cache disabled, measurement, wrapper,
operator set.

Later the same day. The measurement rule 5's implementation clause
asked for was made on PR #111 (`crates/transport/libp2p/tests/
root_funnel.rs`): the union reaches the outer wrapper, so the
deny-at-root fallback is not needed and the clause now records the
result rather than the contingency. Two readings were settled with
p2p-network-dev on the same exchange and written into the clause: the
funnel denies only when it removed at least one address and nothing
survived — a dial no behaviour fed passes through empty as the Swarm's
own `NoAddresses`, because a refusal count must count refusals the
funnel made — and the denial itself is counted (`dials_denied`), the
Swarm discarding the denial of a behaviour-originated dial. And the
test is the record: the plan and the clause had asked for a log
beside the pin, which is `SPIKES.md`'s rule for a pinned spike; a
crate test re-measures on every CI run, which is stronger, and has no
pin to sit beside.

Later still, the same day. p2p-network-dev read the two call chains
and found the restated rule 8 too strong in one sentence: "a class
predicate at an individual dial site is … never the enforcement, and
its absence is not a socket exposure once the root funnel stands".
True of a behaviour-EXTENDED dial; false for a store whose contents
are dialled EXPLICITLY. `autonat_driver::learn_server` stores a peer's
`listen_addrs` verbatim and `reconcile` dials each through the
runtime's own explicit dial (`DialOpts::peer_id(p).addresses(vec![a])`);
`relay_driver::learn` feeds them to the reservation manager and the
picked address is the explicit part of the relay client's own dial,
the funnel pruning only what other behaviours add around it. For
those two stores the learn-site hook is the enforcement, and the
sentence now says so; the plan's §15 lifts the two Identify-learn
opt-ins on their hooks, not on the funnel. The Identified event's raw
`listen_addresses` stays raw by decision: it is the consumer's
evidence, not a store the runtime dials.

Landed, later on 2026-09-25. Everything the 2026-09-25 amendment
named as owed is on PR #111's head: the root funnel wrapping every
Swarm the runtime builds (eefacd4, 61c89bc) with its measurement
(`tests/root_funnel.rs`); the AutoNAT and relay learn-site hooks
(03e5b11) and the query-candidate hook (20fe4b7); the operator set,
seeded from the profile's static blocks at start (56c4e56, 748ce6a);
Identify's cache disabled (a02c14d); the refusal counts on one handle
(f7baced); the wiring proven on a running node (b11f850). Rule 8's
instance list and its implementation clause now read as landed, and
the plan's §15 precondition reads MET on that head. Rule 9 records how
the set is seeded and the one door the runtime still lacks — a peer-
door learn command for Stage 12's composer, owed by p2p-network-dev
before a profile composes a discovery provider whose candidates must
reach the book; the conformance test's use of `add_address` is named
as a test topology, not a pattern.

### Amendment 2026-09-25 — A peer's own circuit address enters the book through the boundary

Raised by the DNS-focused blind review of PR #111 (issue comment
5829426441, outside its focus): the Identify learn site refused every
`/p2p-circuit` address as `Relayed`, so a NATed peer's advertised relay
address never entered the book and `DialPeer`'s circuit fallback had
nothing to dial for a peer learned through Identify alone.
p2p-network-dev asked whether a circuit a peer advertises about itself
is peer-supplied in rule 1's sense or needs a rule of its own.

Prior wording, rule 2: "no `p2p-circuit` component (a relayed address
tests the relay, not the peer)".

What changed and why. It is peer-supplied and inside rule 1, and it is
an instance statement, not a new rule. The no-circuit clause was
written for the dial-back and the punch, where a relayed address
answers the wrong question — it tests the relay's reachability, not the
peer's. For the address BOOK the question is different: a circuit is
how a NATed peer is reached at all, so refusing it does not protect the
boundary, it deletes the route. The floor still binds what a socket
would be opened TO — the circuit's transport prefix, the relay's
address through its `/p2p` component — as a literal, or as an operator
address under rule 9; a peer-named relay's name is refused as any
peer-supplied name is. Whether this node may dial through that relay
is authorization, the gate's at dial time, which rule 1 keeps apart
from class on purpose; and a circuit on a node without the relay
transport is refused as structural (PR #111 commit 4e989ba). mDNS keeps
the unscoped floor because a LAN peer needs no relay. The instance is
stated in ADR-0011 §Identify's advertised addresses; the hook is
p2p-network-dev's, on PR #111.

Built, 2026-09-27 (#135): the one door the runtime still lacked — the in-boundary learn command — is `SwarmRuntime::learn`, a discovery candidate through the learn-site boundary into the book (rule 8's hook; `add_address` stays the operator door), and `tests/discovery-conformance`'s composition test converts to it on the same PR. Rule 9's implementation-state tail reads so in the body.

### Amendment 2026-09-27 — The discovery door refuses every circuit

Rule 8's instance list gains the discovery → book door,
`SwarmRuntime::learn`, hooked at its learn site under the discovery
predicate — every `/p2p-circuit` refused as `Relayed`, as the Kademlia
stash and mDNS refuse them. The door's first hook on #135 reused
Identify's `admits_own_route` and admitted a circuit named by a
candidate; the blind review of #135 asked which clause governs, and
this ADR did not say. It says now: the Identify circuit clause
(Amendment 2026-09-25) is the peer's assertion about ITSELF over an
authenticated connection, while a discovery candidate — a cache hint, a
query result, a multicast announcement — may carry a third party's
assertion with no such authentication, so the fail-closed reading is
taken. The cost is the one mDNS already accepts: a relay-only peer
known only through discovery is not dialable from the candidate; its
circuit enters the book through its own Identify advertisement once a
connection exists, inbound or through an operator route. The hook is
`admits_discovered` (6d2c1e92); this statement followed the hook's
correction on the same PR rather than preceding it, which rule 8 asks
for and the record says plainly.

### Amendment 2026-10-09 — The profile door admits a circuit route

**Trigger.** Stage 17's relay pair (p2p-network-dev-02, GZCoord
01a1208f): the dialer daemon exited at start because profile-config's
address grammar takes exactly `/<host>/<value>/<transport>/<port>[/p2p/<id>]`
and refused a static-bootstrap entry of the form
`/ip4/…/tcp/…/p2p/<relay>/p2p-circuit/p2p/<peer>`. With discovery, the
Kademlia stash and mDNS refusing every circuit by rule 8, and the
operator's `AddAddress` command reachable by no IPC verb, a relay-only
peer could be dialled only if IT dialled first: two NATed peers sharing
a static relay both reserve and neither can ever open the conversation.

**Decision.** Rule 9 names profile configuration as an operator door for
an address, and rule 8's discovery row sends a relay-only peer's circuit
"through an operator route"; a grammar that refuses the one form such a
route has contradicts both. `multiaddr-with-peer-id` — one grammar, the
definition `start` seeds the operator set from — is either the
four-component literal with `/p2p/<id>` or a circuit route
`<relay route>/p2p/<relay>/p2p-circuit/p2p/<peer>`, the relay route the
same four-component grammar with the same host set, under the two shape
conditions rule 8's Identify clause already imposes: the prefix ends in
the relay's `/p2p/<relay>`, and exactly `/p2p/<peer>` follows
`/p2p-circuit`; a foreign destination or a second `/p2p-circuit` is
refused by the grammar as a shape error that names it. Whether this
node may dial THROUGH the relay stays the gate's question at dial time
(`RelayCircuit`, ADR-0036's infrastructure class), never the grammar's.
The runtime needs nothing for the static bootstrap case: the operator
set admits an operator circuit and `Dial` / `DialPeer` classify it
(relayed_paths.rs pins that path through the command). The two
infrastructure lists are the exception, found by p2p-network-dev-01's
review of the grammar: `static_relays` and `static_servers` take the
direct form only and refuse a circuit route at validation by role —
`RelayClientSettings::from_profile` refuses a relay reached through a
circuit ("not a relay") and `AutonatClientSettings::from_profile` keys
the server on the first `/p2p/`, so an admitted circuit would validate
and then fail at start, or probe the relay as if it were the server
with the refusal surfacing nowhere (SPIKE-004). One grammar, two roles
restricting it; the role rule is tested beside the shape rules.

**Alternatives rejected.** An IPC verb reaching `AddAddress`: a
convenience for a running daemon, not the door a deployment configures;
it may come later and does not change this rule. Leaving the grammar and
seeding circuits in tests only: ships a relay infrastructure no two
deployed peers behind NATs can use.

**Consequences.** Code: profile-config's `validate_address_grammar` and
its tests (the circuit accepted; a foreign destination, a double circuit
and a circuit without the relay id refused by name; a `dns` relay host
admitted as an operator name) — p2p-network-dev-01's crate, landing with
the relay-pair batch that first consumes it. `config.schema.yaml` states
the two forms at the type's first use. Until it lands, a daemon cannot
be configured to reach a relay-only peer: the recorded gap.

### Amendment 2026-10-09 — Rule 3's own listener is one on an IP the host is known to hold

Stage 17 step 5 gave the network-change detector a second source, the platform's view (CONNECTIVITY.md §14, A 2026-10-09), and with it a state the listener set alone never had: a listener still BOUND on an IP the view has said departed, until the listener poll catches up. The offer sites stopped offering such a listener (`NetworkSet::holds`, de611722), and the punch's own-listener test (`HolePunchScope::within_boundary`) reads the offered set, so at that commit the punch already counted bound ∩ known — except the `NewListenAddr` offer, filtered since e70747ce. The other instances of the same test — the discovery and advertised doors' `own_listeners` (the root funnel's, Kademlia's, the relay state's, the Identify boundary, mDNS, the Learn and OfferRoutingPeer commands) — read the bound set until 7c1eaf7b, so a departed LAN's listener kept admitting private candidates there for the poll's lag. Ruled on p2p-network-dev's question: rule 3's "holds" means bound and on an IP the host is known to hold, at every instance of the test; nothing else in the rule changes (family and range, the per-instance decision, the dial-back's refusal). Built before this note landed: a crate-level held-listener set the detector publishes after every observation, read by every runtime own-listener site through `NetworkSet::own_listeners` and taken by the root funnel beside the operator set (7c1eaf7b; wire test at the Learn site, the funnel's unit test, the detector's publication test). The AutoNAT offer filters have no test that fails on their removal — this host has no probeable listener — and the predicate is pinned through DCUtR; a public-IP row is where it would show.

### Amendment 2026-10-21 — A person's add is the one peer-chosen address admitted through the operator door

ADR-0062 lets a person add a peer's address in the human client, typed
or scanned from a code the other party showed, and persists it in the
bootstrap-peers overlay, which the static-bootstrap provider emits into
the operator set. Rule 8's last sentence held operator inputs outside
the boundary "because no peer chose them" and its implementation text
called feeding a peer's address back through the operator command
laundering. A scanned code's addresses are chosen by the other party,
so #340's review (F16) asked on what ground they pass. Ruled: the
person's choice is the ground, and the floor is the safeguard — the add
method applies rule 2's special-use floor to every component before
anything is stored, and the client refuses a DNS name that arrived by
scan — the one named exception to the sentence; no other path feeds a
peer-chosen address through the operator door.
The same amendment reads rule 2's "whoever supplies them" as
"whichever peer supplies them": rule 9 admits an operator-set address
whatever its class, and the two sentences had read against each other
(#340's review, rounds 4 and 5); the floor is the peer-supplied path's,
and the person's add brings its own.

### Amendment 2026-10-26 — A private-range address of this host on no local network is not this node's address toward anyone

rust-ui-dev-01 observed on the Pixel 9a (2026-10-26) that ADR-0062's
Show listed three rows: the Wi-Fi address a LAN peer could reach and
two carrier-assigned `10.x/32` addresses on the phone's mobile-data
interfaces, which a LAN peer could not; a second phone scanning the
code added all three. p2p-network-dev-02 read the code and found it
doing what decision 4 said — a private range is admitted — and that
the same addresses went further than Show: Identify advertised every
listen address to every connected peer, and a punch candidate set and
rule 3's own-listener test read the same listener set. The runtime
could not tell: its listener set and the platform's view carried IP
addresses and nothing about the network each came from.

Ruled: the reason a private range is admitted anywhere in this record
is the LAN, and a carrier's private address on a cellular network is on
none, so it is no address of this node toward anyone — not shown, not
advertised, not offered, not counted — while the listener stays bound
for what it can reach outward. Rule 10 states it; rule 3's own-listener
clause gains the exception, because the former reading let a phone on
mobile data alone admit a far end's private candidate and dial it into
the carrier's network. The classification crosses the platform bridge
as a derived fact about each address, read off the network's transport
in the platform layer (cellular is no local network; a VPN is, since
its peers reach its private address; a prefix length decides nothing),
so `CONNECTIVITY.md` §14's rule that no transport, network id or
carrier fact crosses stands. Rejected: Show alone (the leak to every
connected peer and their dead dials stay); no change (the same, plus a
scan spending its lines on dead rows). Measured once, on one carrier;
whether a carrier routes between its own subscribers' private addresses
was not measured and the rule does not depend on it.

Propagation: rules 3 and 10; this note; the log row; the digest entry
and keyword row; the README row; ADR-0062 decision 4 and its
conformance cases (its own note); `CONNECTIVITY.md` §14 (the view's
classification, the Identify and offer sites); `DCUTR.md` §6;
`human-client-android.md` §Android network lifecycle. The code is
p2p-network-dev-02's (the view's field, `own_addresses::select`, the
Identify listen set, the offer sites, rule 3's test) with rust-ui-dev's
bridge half (the Kotlin callback reading each network's transport, the
`android-platform` seam), one PR.

### Amendment 2026-10-27 — An address the platform's view does not hold is on no local network

rust-ui-dev-01 re-ran ADR-0062's gate (e) on the Pixel 9a at main
bdc7ac89, the first build carrying rule 10, with Wi-Fi and mobile
data on (OBSERVATION 01a17bb8): Show still listed both carrier
`10.x/32` addresses beside the Wi-Fi one. The platform's connectivity
service had reported neither carrier network to the bridge's listen —
one was the cellular network held in the background behind validated
Wi-Fi, the other the carrier's IMS network, which an app cannot
request — so the view held the Wi-Fi network alone and classified
nothing, while the wildcard listener bound both addresses.
p2p-network-dev-02 read the four surfaces on `origin/main`
(OBSERVATION 01a17bbb): the known set is the view's addresses plus the
listeners' bound set (`CONNECTIVITY.md` §14), the off-local set holds
only what the view classifies, so an address the view never named was
shown, advertised, offered and counted as an own private listener —
rule 10's unclassified-counts-as-local default, applied to a network
the platform never described. On cellular alone the IMS address is
still bound, so rule 3 still admitted a far end's private candidate:
the probe rule 10 was written to refuse.

Ruled: where a view is present, a bound private-range address the
view does not hold lies on no local network for rule 10's four
consequences, and is lifted when a view names it. The reason is what
the view is: every network the platform lets the app use. A network
it withholds is one the app cannot use and no peer reaches it on,
whichever reason the platform has, so the measurement of whether a
different listen would see the background network was not needed.
Written as a rule of the runtime with no platform in it; the desktop,
with no view, is unchanged. The fail direction is the opposite of the
unclassified-transport default and chosen on the weights: wrongly
hiding costs a LAN address the view missed, with the other phone's
code still connecting the pair one way; wrongly showing is the rule 3
probe into the carrier's network and the carrier address handed to
every Identify peer. Two things follow for the bridge. Its listen
admits every network the platform reports to the app, not only
Internet-capable ones, since a Wi-Fi with no uplink or a local-only
hotspot is a LAN the person is on and would otherwise fall outside
the view. And the one LAN a platform may report to no app, a
tethering downstream — the phone's own hotspot access point, a Wi-Fi
Direct group owner — enters the view as a local network where the
platform lets an app read it; where it does not, the hotspot host's
unshown address is the recorded limit, unmeasured at the time of this
note (rust-ui-dev-01's row). Rejected: the bridge reading the host's
interfaces and classifying by absence (a second address source in the
platform layer, classifying from the interface names §14 keeps out,
for a fact the runtime already holds); keeping the rule and widening
the listen alone (the IMS network is withheld whatever the listen
asks, so the rule 3 probe stays).

Propagation: rule 10 and rule 3's own-listener clause, which names
the second case beside the first; this note; the log row; the digest
entry; ADR-0062 decision 4 and its note; `CONNECTIVITY.md` §14;
`DCUTR.md` §6; `human-client-android.md` §Android network lifecycle.
The code is p2p-network-dev-02's (the off-local set in
`network_change.rs`, read
by the four sites as before; j82) with rust-ui-dev-01's bridge half
(the listen, the tethering read if one exists, the Pixel rows), one
PR; the stage record says gate (e) passes only on the re-run row.

### Amendment 2026-10-27 (ii) — What the widened listen admits, measured; the hotspot limit's cause

The first amendment of the day said the bridge's listen admits every
network the platform reports so that a Wi-Fi with no uplink stays
shown, and that a tethering downstream enters the view "where the
platform lets an app read it". #474's review (F1) read the second as
built, which it was not, and rust-ui-dev-01 then measured both on the
Pixel 9a (OBSERVATIONS 01a17c25 and 01a17c33, the combined build of
the rule and the widened listen): a Wi-Fi with no uplink — the A40's
hotspot with mobile data off, the Pixel joined — carries the Internet
capability unvalidated and was heard by the old listen too, so the
no-uplink case never needed the widening; what the widening admits is
a network carrying the local-network capability and no Internet one,
the phone's own hotspot access point, and the platform reports such a
network to an app only when the app targets Android SDK 37 — the
`ENABLE_MATCH_NON_THREAD_LOCAL_NETWORKS` compatibility gate, enabled
from that target, which forced on for the debuggable build made Show
list the access-point address beside the Wi-Fi one; a network held in
the background is never heard, the platform adding the foreground
requirement to an app's listen. The app targets SDK 36. Ruled: the
hotspot host's own address stays the recorded limit, now with its
cause, its cost one direction (the client's shown code connects the
pair); raising the target is the owner's release decision, not this
rule's, since it needs a compile-SDK pin (devex-tooling) and brings
Android 17's runtime local-network permission before any LAN traffic,
the in-room scan included (ux-designer and the owner); the
recommendation given to the owner was not now. Propagation: rule 10;
this note; the log row; the digest entry; `CONNECTIVITY.md` §14;
`human-client-android.md` §Android network lifecycle; the bridge's
own comment (rust-ui-dev-01's cc12f4a8).

### Amendment 2026-10-27 (iii) — The shared address space is on no local network, with or without a view

#474's review noted that rule 10 withheld private-range addresses
only (RFC 1918, ULA), so a carrier that numbers a phone's cellular or
IMS interface from the shared address space, `100.64.0.0/10`
(RFC 6598), would still have that address advertised by Identify;
the book, Show and the punch offer sites already refused it as
special-use, and rule 3 counts private listeners only, so Identify
was the one surface it reached (p2p-network-dev-02's OBSERVATION
01a17c11). Ruled: the range is on no local network unconditionally,
with or without a platform view, the desktop with a cellular modem
included, there being no classification to wait for. The first
reason given — that the space is provider-numbered and no person's
LAN — was wrong as a premise and p2p-network-dev-02 said so while
building it (OBSERVATION 01a17c69): mesh overlays number a person's
own peers from the same space, and ADR-0055 decision 5b's two-host
case runs on it. The ruling stands on the record's own rules instead:
rule 2's floor refuses the range as a peer-supplied candidate at
every learn site, so no peer may learn and dial such an address of
this node, advertising it serves nobody and withholding it costs
nothing, whoever numbered it; a peer's address in that space enters
by the configuration half of rule 9's operator door, as a static
entry (ADR-0055 decision 5b, the run-time add keeping ADR-0062's
floor); and should the floor ever admit the range for a peer-learned
address, this treatment moves with it. Built by p2p-network-dev-02 on
#476 (j84) as one predicate, with the owed case (a listener on a
shared-space address, no view, not advertised, beside a private
listener as the control). Propagation: rule 10; this note; the log
row; the digest entry; the README row; `CONNECTIVITY.md` §14;
`DCUTR.md` §6.

### Amendment 2026-10-27 (iv) — The app stays on target SDK 36 for now

Amendment (ii) of the same day named raising the Android target to 37
as the owner's release decision, with its costs: a compile-SDK pin
(devex-tooling's) and Android 17's runtime local-network permission
asked of the person before any LAN traffic, the in-room scan included
(ux-designer's and the owner's). The recommendation put to the owner
was not now. The owner's word, 2026-10-27, in architect-cto's session,
verbatim: "stay on 36 for now". So the hotspot host's own address
stays the recorded limit with its cause, rule 10's text unchanged
beyond this sentence, and the target is revisited when a release
targets Android 17 for its own reasons or when the hotspot host case
matters to a user. Propagation: rule 10's sentence; this note; the
log row; the digest entry.


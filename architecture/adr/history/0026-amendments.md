# ADR-0026 — amendment history

### Amendment 2026-08-27 — Broadcast ingress consumes its own token buckets

The Decision gave direct inbound two token buckets with stated defaults and
then said only that "Broadcast retains bounded best-effort local drop
behavior". Read against the rest of the paragraph that sentence names a
*delivery* bound — the local queue may drop under overload — and no
*ingress* bound at all.

The gap it left: a trusted peer publishing on a joined channel could spend
unbounded decode, signature-verification and dedup work on every node in
the mesh, because the only limits above that path were the transmit
ceiling and the bounded delivery queue. Both bound how much is *held*;
neither bounds how often the work is *done*. Direct's identical exposure
is exactly what the per-PeerId bucket one line above exists to close, and
the Security implications section already reasons about "a
malicious-but-trusted peer" in those terms.

Broadcast now consumes its own per-trusted-PeerId and global buckets with
direct's defaults.

Two things this deliberately does.

The buckets are **separate instances**, not shared with direct. Sharing
would let a broadcast flood exhaust a peer's direct allowance and vice
versa, which converts a bound on one mode into a denial of the other —
and the two modes are required to stay independently functional.

A message over the rate is **dropped before local delivery admission, and
is still reported to the mesh** under the ordinary ADR-0029 mapping. A
validation verdict answers whether a message was structurally valid and
its publisher authorized; a local rate limit answers neither, so
suppressing the report would tell the mesh something untrue about the
message and, under `Reject`, would penalise a relay for this node's own
congestion. GossipSub has no per-message refusal to return to a publisher
in any case.

The delivery-side rule is unchanged: broadcast local delivery may still
drop under overload where direct must refuse before accepting, because
broadcast makes no per-recipient acceptance promise to anyone.

**What the bucket does not bound**, recorded because the first draft of
this note claimed more than it delivers. It said the gap was "unbounded
decode, signature-verification and dedup work". Fingerprinting, dedup and
fan-out are bounded. Signature verification is not: the GossipSub backend
performs it before the runtime is handed the message, so no runtime-layer
limit can precede it. Envelope decoding is not either, and that one is
structural rather than incidental — the mesh is owed an
Accept/Ignore/Reject verdict, the verdict depends on whether the envelope
decodes, so a limit that skipped decoding would have to answer without
knowing whether the bytes were valid. Both are bounded per message by the
transmit ceiling, which is why the remaining exposure is the repeated
cost of hashing a 48 KiB payload and copying it once per joined session —
which is what the bucket closes.

### Amendment 2026-10-21 — Configured endpoints have a maximum of 64 and no per-profile default

ADR-0061 (a user identity is an EndpointId of the node) capped a node's
identities by the profile's configured-endpoint bound, and its review
(#335, rounds 17 and 18) found that the "default 16 / max 64" this
record and ENDPOINTS.md's resource table carried for configured
endpoints was half true: no configuration key and no code holds a
default of 16; `config.schema.yaml` validates `entries` to `max=64`
and `profile-config`'s `MAX_ENDPOINTS` is 64. The record now says max
64 with no per-profile default, ENDPOINTS.md's table row reads the
same, and ADR-0061's cap cites the bound that exists. The advertised
endpoints' 16 / 32 is a real key (`directory.max_advertised`) and
stands.

### Amendment 2026-10-24 — A session reads its own broadcast losses

This record bounded what a slow consumer costs the node — the oldest
copy dropped, the over-rate broadcast refused before fan-out — and
counted both only in the operator's view (`admin.status`), so a data
client could not say whether it had missed anything. The Claude Channel
bridge's status needed exactly that sentence (`CHANNEL-EVENT.md`
§Delivery), and a
human client that shows a broadcast log is owed it as much. The record
now says a session reads what it lost: `session.losses` on the data
socket under `commands`, IPC 2.11, and `DataSessionPort::losses` at every
binding — two counts, the copies its own queue dropped or could not
take, and the copies the ingress gate refused on a channel it held
at the time.

Four shapes were rejected, and the reasons are what a later reader
needs. A pushed notice was rejected because a loss is a count and not an
event: a push would compete for the queue whose overflow it reports, and
in the reserved lane it would displace the states that lane exists for.
An exact figure was rejected because it cannot be measured: the GossipSub
backend drops before the runtime sees a message (validation, the mesh,
its own queue), so an exact count would claim what nothing observed and a
reader would act on a zero; so neither count claims to be the whole
loss — zero is never read as none lost — and every text that carries
them says which guarantee each gives (the paragraph below). A per-peer or per-channel
breakdown was rejected because the data socket is a non-admin surface:
naming the publisher whose broadcasts were refused is an oracle on who
publishes and when, and a per-channel table grows with the channels a
client joins; two counters per session are bounded by construction. And
a reset on leave was rejected because a leave and a rejoin are the
ordinary life of a channel for a bridge that reconnects, and a count that
fell would read as recovery.

The two counts promise different things, and the first draft of this
note said "permanent" of both; the branch's review (2026-10-24) showed
it false for one. An overflowed broadcast was admitted: the runtime
records its dedup entry after the fan-out whatever was dropped, so
within ADR-0019's window (10,000 entries, 5 min, oldest evicted) a
republish of the same id is a duplicate, and the overflow count is a
lower bound on broadcasts lost rather than delayed. A refused broadcast
was never admitted: the ingress gate returns before any dedup entry
exists, so a republish of the same id reaches the runtime as fresh and
is delivered, and while the publisher stays over its rate every
republish is refused and counted again. The refusal count therefore
counts refused copies, a signal that broadcasts on the session's
channels were refused, never a count of distinct broadcasts lost. Two
ways to make it one were rejected: recording the refused id in the
dedup map, which spends at the gate the per-message work the bucket
exists to bound and lets a flooder evict other publishers' entries; and
a separate bounded set of refused ids, which would drop the republish
that repairs the loss in order to keep the number honest. The smallest
true statement was chosen, and every text that carries the counts says
which guarantee each gives.

One limit is accepted and named beside the method rather than removed: a
session that declines route notices over IPC asks for the minor below
2.4 — negotiation is the only lever the client has (#245 N5) — so it
cannot read a 2.11 method and is answered `ProtocolUnsupported` without
asking. No production client declines route notices (the bridge and the
human clients take them), so the limit costs nothing today; a client
that one day needs both makes declining a `hello` feature of its own,
decided then, never a second cap on the minor.

### Amendment 2026-11-10 — The operator reads what the broadcast ingress gate refused

The amendment of 2026-10-24 gave a session its own two counts and left
the operator with one: `admin.status` carried
`ipc.events_dropped_total`, the session queues' drops, and nothing for
what the ingress gate refuses. SPIKE-001's S30 measured the
consequence (fact 79, `spikes/spike-001/README.md`): of 200 broadcasts
published in 2.18 s, 47 were notified and the gate refused 153, and
`admin.status` read `events_dropped_total` 0 on both daemons before and
after. The only count of the 153 was the receiving bridge's own
`ingress_refused`, which exists for a session that joined and ends with
it; an operator reading the status saw a healthy daemon while three
quarters of a burst was refused.

The record now says the operator reads the refusals for the profile:
`ipc.events_refused_total` in `admin.status`, moved by the branch that
moves a session's `ingress_refused`. Its unit is p2p-network-dev's
reading of that branch (REPLY 01a1c4e6-83ec, 2026-11-10): the gate runs
before dedup and before the fan-out, so one refused inbound message is
one increment whatever the number of sessions on its channel, and the
branch is reached whether or not any session holds the channel. It
carries the guarantee the session's count carries and no stronger one:
a refused message leaves no dedup entry, so a republished copy that is
refused counts again.

Two shapes were rejected. Widening `events_dropped_total` to include
refusals was rejected because that counter has a stated meaning since
2026-11-04, the profile total of every session's `queue_overflow`, a
lower bound on broadcasts lost; adding a signal that counts republished
copies to it would leave a figure that is neither. A per-channel or
per-publisher table was rejected for the reason the session's counts
name no peer and no channel, and because it grows with what remote
peers publish; one counter is bounded by construction.

The first ruling said the member was "optional on the wire as its
sibling is" and needed no minor. p2p-network-dev showed that false for
this member (QUESTION 01a1c4e9-5cc0): `events_dropped_total` had been an
optional property of `admin-status` 1.1.0 since 2026-10-01, so every
reader accepted it, while this one is new to a closed block and a
reader built before it refuses the whole status. It is therefore served
behind IPC 2.13 under ADR-0017's amendment of 2026-10-07, absent below
it; from 2.13, absent says the server keeps no count and `0` says none
refused.

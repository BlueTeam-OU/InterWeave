---
role: "p2p-network-dev"
class: solution
topic: "broadcast-ingress-refusal-unit"
description: what one broadcast ingress refusal counts (per message, per channel, never per session), and where it is counted
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c3fb481079ac713e
---

## what one broadcast ingress refusal counts (per message, per channel, never per session), and where it is counted

The broadcast ingress gate (`admit_prefix`: publisher over its rate, or the
node draining) is counted in exactly one place, `admit_broadcast` in
`broadcast_inbound.rs` (origin/main e5266bb3d, 2026-11-10): one
`LossLedger::refused(channel)` per refused inbound message, before dedup and
fan-out, touching no session. A session's `ingress_refused` is that channel
count read from its join mark, so one refusal moves every holder's count by
one. The per-channel count is kept only for a channel some session holds; the
profile-wide `refused_total` (j159) is moved by the same `refused` call BEFORE
that held-channel check, so an unheld channel counts
(`a_refusal_nobody_joined_for_counts_in_the_profile_total` fails if the
increment moves inside it). Only remote broadcasts
reach the gate; a local publish refused while draining is the publish call's
error. A refusal records no dedup entry: a republished refused copy counts
again.

**Why:** architect-cto asked for this unit (REQUEST 01a1c4e5, j159) to write
`ipc.events_refused_total`'s schema description.
**How to apply:** any counter or test of broadcast refusals.

*Anchors: `interweave:crates/transport/runtime/src/broadcast_inbound.rs::admit_broadcast`, `interweave:crates/transport/runtime/src/session_losses.rs::LossLedger`*

*Observed 2026-11-10 (p2p-network-dev)*

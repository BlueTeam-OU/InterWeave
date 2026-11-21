---
role: "architect-cto"
class: rationale
topic: "j714-redial-within-30-s-2026-11-14"
description: "owner's word 2026-11-14 on the long-outage redial wait — 30 s maximum; ADR-0011 amendment owed, code is p2p-network-dev-01's"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - eac110864b37937d
---

## owner's word 2026-11-14 on the long-outage redial wait — 30 s maximum; ADR-0011 amendment owed, code is p2p-network-dev-01's

The owner, asked whether a returned far daemon may wait up to 300 s to be
redialled after a long outage (272 s measured, #672 / #673): "no, 30
seconds maximum" (2026-11-14).

Decision as I worded it (REQUEST 01a1db4b-6be0-7bcb-97c4-aa6b48428b4a to
p2p-network-dev-01, job j714): a data-plane peer the profile holds a dial
candidate for is redialled with no more than 30 s between a failed
attempt's settlement and the next attempt's start, tick included; the
AutoNAT re-test keeps 30 s rising to 5 min on its own constants
(it shared RETRY_BASE_MS / RETRY_CEILING_MS in
crates/transport/runtime/src/connection_manager.rs); relay ladder, DCUtR
cooldown, quarantine, lift floors unchanged.

**Why:** the far end of a one-sided 5b deployment holds no entry and
dials nothing, so only the dialling side's cadence bounds the reconnect.

**How to apply:** my part is the ADR-0011 amendment triple + digest entry
+ CONNECTIVITY.md s11/s12/s14 + AUTONAT.md s4, as a for/ supply onto
p2p-network-dev-01's branch once it names it and answers three open
questions (does address_backoff_max default 300 s also hold the redial;
does the 31 s first step break "tick included"; other waits). Carry by
name: phone cost of a 30 s redial of every absent contact is not
measured. #673 was armed on the owner's word at 7d0239fb the same day.

Ruled 2026-11-14 (REPLY 01a1db4f-3e84 to p2p-network-dev-01, its job j169,
branch develop-qzapp/p2p-network-dev-01/feat/j169-owner-s-word-2026-11-14):
wording gains two bounded exceptions (per-tick budget of 4, pending-dial
ceiling of 32: dialled at the first moment either has room) and "a circuit
route's hold is its own"; address_backoff_max default 300 s -> 30 s with
the constant (one delay serves both); scheduler wakes at the earliest due
retry so 30 s stays the written number. Supply also covers
architecture/config/config.schema.yaml, config/examples/internet-reachability.yaml
and the plan. Base the supply on main after #673 (a5c4d1905).

DELIVERED 2026-11-14 ~18:50Z: supply on
develop-qzapp/architect-cto-01/for/p2p-network-dev-01/j169-redial-30s,
0876be471 (ADR-0011 triple + digest) and 2ff6c8a6c (CONNECTIVITY.md s5/11/12/14,
AUTONAT.md s4 A 2026-11-14, config schema + example, plan note), REPLY
01a1db63-113d to p2p-network-dev-01. Closes when 2ff6c8a6c is on main. Open:
the relay-hop route hold's duration on its branch (I amend the "ordinary
failure cadence" sentence only if it is not the 30 s address delay); its PR
will need my word if below minimum or boundary.

~18:50Z: folded into BlueTeam-OU/InterWeave#674 (merge 41f94bcd6). The route
hold on #674 keeps its OWN ladder (route_hold_ms, 30 s doubling to 5 min,
8d6c5450a); I RULED keep as built. The text fix (hold names its ladder) is my
job j715: a patch in the j714 session scratchpad, to apply, commit, push on the
for/ branch and announce in reply to 01a1db63-e6fb. The commit guard refuses a
commit once the session's job has left active: queue a job and switch first.
The session scratch is swept when the session ends: the j715 patch is COPIED to
~/.local/state/j715/route-hold-ladder.patch (apply it on the for/ branch
develop-qzapp/architect-cto-01/for/p2p-network-dev-01/j169-redial-30s at
2ff6c8a6c, not on j715's new branch; delete the copy after the push).

*Observed 2026-11-14 (architect-cto)*

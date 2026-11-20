---
role: "p2p-network-dev"
class: threads
topic: "j169-thirty-second-redial-ruling"
description: "j169 (owner 2026-11-14, \"30 seconds maximum\"): peer redial ≤30 s after a failed attempt settles; AutoNAT keeps 30 s→5 min; wake at earliest due; ruled design"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 22bb3d4c83e45e04
---

## j169 (owner 2026-11-14, "30 seconds maximum"): peer redial ≤30 s after a failed attempt settles; AutoNAT keeps 30 s→5 min; wake at earliest due; ruled design

j169 is architect-cto's REQUEST 01a1db4b, from the owner's word on #672's figure: "no, 30 seconds maximum". The branch is `develop-qzapp/p2p-network-dev-01/feat/j169-owner-s-word-2026-11-14`. The PR is mine. Architect-cto supplies a for/ branch onto it, to fold before review: ADR-0011 amendment, CONNECTIVITY.md §5/11/12/14, AUTONAT.md, digest, config.schema.yaml, the internet-reachability example, plan text.

**Ruled text** (REPLY 01a1db4f-3e84): a data-plane peer this profile holds a dial candidate for is redialled so that no more than 30 s pass between the settlement of one failed attempt and the start of the next. Two exceptions:
- when the scheduler's per-tick budget or the gate's pending-dial ceiling is full, the peer is dialled at the first moment either has room;
- a circuit route's hold is its own (ADR-0011 A 2026-10-21).

**What to build:**
1. **Peer retry delay: 30 s flat.** `retry_backoff_ms` is the peer delay. The attempt count stays, for diagnostics only.
2. **AutoNAT re-test gets its own constants**, 30 s rising to 5 min, unchanged. It shared `RETRY_BASE_MS`/`RETRY_CEILING_MS` through `retry_backoff_ms`.
3. **One delay drives both the scheduler and the peer backoff.** `record_failure` passes it to `record_address_failure`, which sets the peer's `until_ms`.
4. **Profile defaults follow the constant.** `address_backoff_max` default goes 300 s → 30 s, range unchanged. `the_accepted_defaults_are_what_the_runtime_runs` (translate.rs) pins those defaults to the constants.
5. **The scheduler wakes at the EARLIEST due retry as well as on its 1 s tick:** one timer for the table, not one per peer. Without it the tick adds up to 1 s (measured +30.99 s).

**Tests and runs:**
- A non-ignored test: the bound holds at the lowest layer, with neither the per-tick budget nor the pending-dial ceiling full.
- The long-outage case re-run on the new code, with a run record beside `2026-11-14-long-outage.md`. Acceptance: the pair reconnects within 30 s of the return, plus the dial itself.

**Stale words to remove:**
- "bounded by 5 min" for the peer redial;
- the 300 s `CEILING` constant in the `within_its_backoff` case;
- the `RETRY_BASE_MS` comment saying the two schedules share numbers.

**Also:**
- The per-tick budget is `max_retries_per_tick` (4); the pending-dial ceiling is `max_pending_dials` (32).
- The phone: the rule applies there unqualified. Its cost is carried by name as not measured.

Related: [[j168-hold-not-proved-items-landed]].

*References: j168-hold-not-proved-items-landed*

*Observed 2026-11-14 (p2p-network-dev)*

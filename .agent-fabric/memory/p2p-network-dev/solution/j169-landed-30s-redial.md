---
role: "p2p-network-dev"
class: solution
topic: "j169-landed-30s-redial"
description: "#674 landed the owner's 30 s redial bound (ADR-0011 A 2026-11-14); what was built, measured and carried"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d1c07852bab37049
---

## #674 landed the owner's 30 s redial bound (ADR-0011 A 2026-11-14); what was built, measured and carried

BlueTeam-OU/InterWeave#674 merged 322c5ce83 on 2026-11-15 (9 work, 10 fix, 5 review rounds).
Built: RETRY_CEILING_MS = RETRY_BASE_MS = 30 s; the scheduler wakes at the earliest retry not yet offered (per-entry `offered` mark, cleared only by `Retry::set_due`) beside its tick; one `max_retries_per_tick` budget per tick period shared by wakes and tick; AutoNAT re-test on its own RETEST_* (30 s..5 min); relay-hop route hold keeps its own ladder (`route_hold_ms`, private); a peer whose every candidate is refused by a hold, judged per relay through `ConnectionPolicy::route_hold_end` (the gate's one rule), is retried at max(30 s, first re-admission); address_backoff_max default 30 s.
Evidence: `reconnect.rs` bound test (paused time, 7 s tick) and budget test; long-outage run `tests/automated-client/runs/2026-11-14-long-outage-30s.md` (16 x 30 s, reconnect 0.96 s after return, at 41f94bcd6 before the wake fixes).
Carried, not findings: ADR-0011 l.76 wording "only route is a held circuit ... at the hold's end" vs the per-relay/max(30 s) rule (sent to architect-cto); no direct unit test of route_hold_ms's ladder; a wake's slot can go to a left-over retry (oldest-due sort); phone radio cost of 30 s dials unmeasured; human-desktop `window_faults` limit-row test timed out once locally under concurrent load (rust-ui-dev's).

*Observed 2026-11-15 (p2p-network-dev)*

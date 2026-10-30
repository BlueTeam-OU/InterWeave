---
role: "architect-cto"
class: threads
topic: "redacted-mayor-deck-claims-2026-10-27"
description: "gzapp product claims confirmed for brand-comms' [redacted] mayor deck (2026-10-27, main 298717e9d) — the rewordings and the strikes, reusable for any sent deck"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dac9b63f482580d4
---

## gzapp product claims confirmed for brand-comms' [redacted] mayor deck (2026-10-27, main 298717e9d) — the rewordings and the strikes, reusable for any sent deck

j414 (brand-comms-01 QUESTION 01a17ca8). Confirmed at gzapp main 298717e9d:
- C1 "arrival estimates" struck → "live vehicle positions, predicted journey times and crowding" (ADR-060 grades freshness; ADR-058 predicted leg durations); status = "pre-production, reference stack with synthetic vehicles/riders, not deployed in any city" (README).
- C2 driver "confirms" → "acknowledges or declines" (ADR-058 strict acknowledged|declined).
- C3 stop-call consolidation OFF the slides: backend merged (#1155 shadow, #1167 active) but driver/passenger UX = completion plan PRs 10/11 unlanded and ADR-079 Proposed (nothing inherits from a Proposed record).
- C4 holds; a restriction blocks stopping, never travel (ADR-059 §2.4).
- C5 geometry = LineString segments along the route (ADR-067); legend "no-stop stretches", never "zones".
- C6 municipal editing = a stop-restriction-editor role grant + city scope (ADR-067 §2.2); no two-party approval workflow exists → never "approves".
- C7, C8 hold (ADR-035/053/058; ADR-043/056/064, "their own fleet's live map").
- C9 vision only. C10/C11: ADR-054 Regime 1 (crowding/tracking) is the ONLY "fully anonymous"; demand from journeys is Regime 2 personal data → "aggregated demand information, derived without identifying any rider"; riders no account, driver tracking consent-based still true.

**C12 (buses vs minibuses, 2026-10-27):** vehicle-agnostic — ADR-002 phone GPS any vehicle; ADR-008 §2.2 mixed fleets; ADR-016 rejected per-vehicle capacity (crowding comparative, never seat counts); ADR-059 §2.1 stopping mode per route (fixed_only|dynamic_only|hybrid_prefer_fixed) at global/route_class (GTFS route_type)/route scope, city-narrowed; no vehicle class field in the registry; no bus line ever run. Slide "One platform for the whole fleet: buses on fixed stops..., minibuses with passenger-chosen stops..." confirmed.

**C13 (equipment, integration, roads, 2026-10-27):** beacon NOT optional — crowding worker reads beacon_sightings only, boarding detection = beacon contact (ADR-016); ADR-008 "mixed fleets" = beacon|sensor families, not beacon-less; sensor optional (env data). No ticketing/payment; self-hosted OTP/Nominatim/tiles (ADR-035); GTFS bundle hand-authored v1 = input file; FCM (ADR-077) is a phone-platform dependency → say "city's or operators' existing systems", not "third-party". Nothing on the roads. Line reworded with "plus a small Bluetooth beacon".

*Observed 2026-10-27 (architect-cto)*

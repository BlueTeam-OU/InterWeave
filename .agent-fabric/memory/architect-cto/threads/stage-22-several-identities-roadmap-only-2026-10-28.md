---
role: "architect-cto"
class: threads
topic: "stage-22-several-identities-roadmap-only-2026-10-28"
description: "2026-10-28 the owner saw Android holds one \"Your card\" and wants one card per EndpointId; that is ADR-0061 / plan §30 Stage 22, unbuilt; the owner's word is roadmap only, do not open or implement; the note is on my docs batch"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 21f94af2a3fd410d
---

## 2026-10-28 the owner saw Android holds one "Your card" and wants one card per EndpointId; that is ADR-0061 / plan §30 Stage 22, unbuilt; the owner's word is roadmap only, do not open or implement; the note is on my docs batch

Owner, 2026-10-28: "an endpoint should exist without a card" (so no card-at-creation rule; a route may be cardless); the problem is the Android app configures only ONE card ("Your card"); an app should hold several EndpointIds, each with ONE card different from the others. Read against main: that is ADR-0061 decisions 1/2/9 and plan §30 Stage 22, recorded 2026-10-21, opening on the owner's word only; both clients hold one identity (`human`) and one own card; no admin.endpoints.add/remove/set_advertise in LOCAL-IPC.md (list, revoke, set_enabled, set_default exist), so no second EndpointId can be made. Owner: "don't implement still, we should put in the road map" → a dated "Where the clients stand" paragraph in §30 on docs/j421 (after folding main, conflict in the §29 bullet resolved by keeping main's text + my bullet).

**How to apply:** when the owner opens Stage 22, the order is p2p-network-dev's admin.endpoints.* methods first (ADR-0017 minor), then rust-ui-dev steps 1–3, ux-designer-01 identity screens; until then no REQUEST goes out. Related: [[owner-identity-model-endpointid-is-user-identity-2026-10-21]].

*References: owner-identity-model-endpointid-is-user-identity-2026-10-21*

*Observed 2026-10-28 (architect-cto)*

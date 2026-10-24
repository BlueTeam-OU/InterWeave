---
role: "rust-ui-dev"
class: solution
topic: "endpoint-walk-effective-state"
description: "How the human client picks its endpoint since #261's persisted endpoint overlay — facade walks config order past EndpointDisabled; no admin.endpoints read (j70)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - be1827db621edaf2
---

## How the human client picks its endpoint since #261's persisted endpoint overlay — facade walks config order past EndpointDisabled; no admin.endpoints read (j70)

Since #261 (ADR-0028 A 2026-10-11) `admin.endpoints.set_enabled/set_default` persist in `<state>/endpoint-overlay.json`, so `config.yaml`'s `enabled` is no longer the daemon's effective state. j70, PR #270 -- review F1 made test ids reverse-alphabetical so a sort shows (branch develop-qzapp/rust-ui-dev-01/fix/endpoint-from-effective-state, 2026-10-11):
- `ClientConfig.endpoints: Vec<EndpointId>` (preference order); `TransportClient::open` claims each in turn, moving on ONLY for `EndpointDisabled` (EndpointInUse stops the walk — never split a person across endpoints); all disabled = `EndpointDisabled` → Refused(NotAvailableToThisClient).
- `TransportClient::endpoint()` = last leased else first configured; `FacadeSide::new(client, wall)` signs `from_endpoint` with it.
- desktop `resolve()` offers every human-client entry regardless of `enabled`.
- Rejected: reading `admin.endpoints.list` (my REPLY 01a129db proposed it) — needs the `admin.endpoints` mutation authority; the claim answer is the effective state with no list/claim race.
- Android unchanged: profile-config's AndroidEndpointNotEnabled validates config.yaml; nothing on the phone disables over admin.
- e2e: tests/desktop-e2e/tests/human_app/kept_disable.rs (disable, daemon restart, app leases human-spare).

Related: [[transport-client-facade-contract]], [[desktop-e2e-stale-binary]]

*References: desktop-e2e-stale-binary, transport-client-facade-contract*

*Observed 2026-10-11 (rust-ui-dev)*

---
role: "architect-cto"
class: workflow
topic: "reflection-read-the-locks-scope-before-ruling-one-client-at-a-time"
description: "W44 — before ruling that two clients may coexist on one profile, read what the \"one at a time\" exclusion actually locks (a store-wide instance lock vs a per-endpoint lease); the contract's lease rule and the client's lock are different…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 59f30d1c995fe58e
---

## W44 — before ruling that two clients may coexist on one profile, read what the "one at a time" exclusion actually locks (a store-wide instance lock vs a per-endpoint lease); the contract's lease rule and the client's lock are different scopes

2026-10-27, X2 on the control plane: I ruled from ENDPOINTS.md that a human lease on `fabric-control` beside the carrier's on `gzcoord` was "two endpoints, two sessions, no conflict", reading the CLI README's "one client at a time ... one store and one endpoint lease (ADR-0041)" as a per-endpoint rule. The blind design review read the code: exit 4 is `HumanClientLock`, the human store's profile-wide single-instance lock taken before any session (startup.rs, main.rs), so while agentd's child holds `listen`, no other `interweave-human` run and no desktop client works on that profile at all. The daemon-side answer was right; the operational consequence was missed.

**Why:** a README sentence that names two things ("one store and one lease") was read for the one I was asked about.

**How to apply:** when a ruling says two things may coexist, find the exclusion mechanism in code (the lock, the lease, the socket) and state its scope in the ruling; a contract rule about leases says nothing about a client's store lock. Related: [[a-ruling-restates-the-current-behaviour-read-at-origin-main]].

*References: a-ruling-restates-the-current-behaviour-read-at-origin-main*

*Observed 2026-10-27 (architect-cto)*

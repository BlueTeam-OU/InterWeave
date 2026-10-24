---
role: "p2p-network-dev"
class: threads
topic: "pr310-j92-landed"
description: "#310 merged 0e2137b6 2026-10-19 (j92 kademlia full-bucket flake + ADR-0055 decision 4 citations); one pre-existing P3 carried to the next kademlia_adversarial change"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4bb6e2ce69754aa2
---

## #310 merged 0e2137b6 2026-10-19 (j92 kademlia full-bucket flake + ADR-0055 decision 4 citations); one pre-existing P3 carried to the next kademlia_adversarial change

PR #310 merged as 0e2137b6 on 2026-10-19. It was armed at 2 work commits on the owner's word ("merge 310").

What it fixed:
- The full-bucket test in `tests/security/tests/kademlia_adversarial.rs` flaked because libp2p-kad seats peers automatically, 500 ms after a bootstrap (see [[libp2p-kad-automatic-bootstrap-on-seat]]).
- Six transportctl comments cited ADR-0055 decision 7 where decision 4 is meant (architect-cto's observation).

**Carried (P3, pre-existing, from round 1 of the review).** The test's comment says the observed peers are "known to the subject only through what the newcomers answer its walks". That is not guaranteed. The test routers built by `hostile_router_as` use kad's defaults: `OnConnected` inserts and automatic bootstrap. So an honest router can learn the observed peers itself and name them directly. Fix candidates:
- give `hostile_router_as` `set_automatic_bootstrap_throttle(None)`;
- or check that no honest router's table holds an observed peer.

That helper is shared by the other tests in the file, so rerun all four. Also carried: `Order::Know` has no acknowledgement. The reviewer suggested a oneshot reply.

Also open for architect-cto: decision 7 says the fabric's roster is not cited by name, yet transportctl's README and identity.rs say "its roster".

*References: libp2p-kad-automatic-bootstrap-on-seat*

*Observed 2026-10-19 (p2p-network-dev)*

---
role: "p2p-network-dev"
class: threads
topic: "j110-withheld-lease-race"
description: "j110 diagnosis: tests/automated-client a_withheld_lease_is_retried... races the daemon's lease release after Client::disconnect(); fix by waiting on the admin lease list"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 680770247d3fc8a8
---

## j110 diagnosis: tests/automated-client a_withheld_lease_is_retried... races the daemon's lease release after Client::disconnect(); fix by waiting on the admin lease list

This is the j110 diagnosis, read on origin/main cf369bc0 (2026-10-23).

**The test:** `tests/automated-client/tests/automated_client.rs:294` calls `to.disconnect()`, then expects `send_once` to be refused `RemoteEndpointUnavailable`. `Client::disconnect` (`tests/automated-client/src/lib.rs:215`) only sets `session = None`.

**The race:** B's daemon frees the endpoint lease when it reads the IPC close, asynchronously. A's next send can reach B before that happens, and is then answered Ok. It failed once under `xtask ci` at 405b24cf and passed at 71762eae.

**The fix:** after `disconnect()`, poll B's daemon until the endpoint's lease is `None`, then send. Read the lease through `home.binding().admin([AdminCapability::Endpoints].into()).leases()`. The same pattern exists as `until_lease` in `tests/desktop-e2e/tests/human_app/harness.rs:211-235`. Put the wait in the test file as an oracle, not in `Client`. Leave the refusal assert as the control. `disconnect()` has only this one caller.

Related: [[stage6-ingress-burst-test-is-load-timed]].

**Built (2026-10-23):** commit 753a39ef, PR #386 (unarmed: 1 work, 0 fix, minimum 8). The wait is `Profile::until_unleased`, and `leased()` has a positive control before the disconnect; forcing `leased()` false makes the control fail. j111 (the release_notices P3s) is the next commits on this branch. Review the head only once j111's commits are on it.

*References: stage6-ingress-burst-test-is-load-timed*

*Observed 2026-10-22 (p2p-network-dev)*

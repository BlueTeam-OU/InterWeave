---
role: "p2p-network-dev"
class: solution
topic: "pr692-j172-landed-two-hosts-flakes"
description: "#692 (j172) landed; two pre-existing two_hosts cases flake under load on a 30 s backoff equal to PATIENCE; gate row backoff_until jitters ~1 ms per read"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5cbb5d3d412d86e1
---

## #692 (j172) landed; two pre-existing two_hosts cases flake under load on a 30 s backoff equal to PATIENCE; gate row backoff_until jitters ~1 ms per read

#692 merged 094c15405 on 2026-11-16 (owner's word, 2 work + 1 fix): four
far-end-returns cases + 200 s idle (ignored, runs/2026-11-16-idle-200s.md)
in tests/automated-client/tests/two_hosts.rs. Review F2 (CONNECTIVITY.md
s14 prose) deferred to architect-cto's j742.

Carried, unfixed (my lane, tests/): under host load ~70, two pre-existing
cases failed once and passed twice alone on 2eb2ab7b:
- the_side_that_learned_an_address_dials_it_on_a_send_and_retries: A's
  first dial hit B before B's TCP listener was up although b.start()
  had returned (serving = IPC socket, not the listener), so B sat under
  the 30 s peer backoff = PATIENCE (30 s).
- two_hosts_talk_over_the_overlay_and_redial_when_its_link_returns: the
  whole case ran in ~1.5 s, link down/up inside the 30 s network-addition
  lift floor counted from the daemon's first fill (my reading, not
  confirmed), so the redial waited the 30 s backoff past PATIENCE.
Class: a wait equal to the backoff it waits through. Fix shape: wait
for the listener (tcp_reaches) before the second start, and a patience
above 30 s where a backoff can stand.

Fact: admin.peers.list backoff_until is a monotonic deadline converted
to wall time per read; two reads of one backoff differed by 1 ms.
Compare with a tolerance (two_hosts.rs JITTER_MS = 50).

*Observed 2026-11-16 (p2p-network-dev)*

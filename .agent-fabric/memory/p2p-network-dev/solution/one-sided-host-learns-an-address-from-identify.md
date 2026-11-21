---
role: "p2p-network-dev"
class: solution
topic: "one-sided-host-learns-an-address-from-identify"
description: "The host with no static entry learns the far host's address from Identify: a close starts no redial, a send dials the book and starts retries (measured, #666 landed)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - fca2068db7132046
---

## The host with no static entry learns the far host's address from Identify: a close starts no redial, a send dials the book and starts retries (measured, #666 landed)

Closed thread (j167, landed as #666, merge 5608d8ea7, 2026-11-14; from architect-cto-01's OBSERVATION 01a1d6e6, 2026-11-13; my REPLY 01a1d6e7). CONNECTIVITY.md §14 A 2026-11-13 says the side with no address "never redials, and a send from it with no connection is answered at once". On origin/main 05d38e9d5:

- **It learns an address.** The Identify arm (`runtime/dialing.rs` ~1351, `learn_advertised`) writes the far host's admitted `listen_addrs` into the substrate book. A private address passes when this host has a private listener of that family.
- **A send then dials.** With a book entry, `runtime/commands.rs` (~940) dials once and answers PeerUnreachable on failure. PeerUnknown comes only from an empty book. So "answered at once" holds only before the first connection or after a restart.
- **Redial door 1, the substrate.** The retry scheduler (`crates/transport/runtime/src/connection_manager.rs`, `schedule_retry`) schedules only after a failed dial, never after a close.
- **Redial door 2, composition.** Composition's `reconnect_targets` (`composition/src/discovery.rs`) dials allowed peers that discovery holds as candidates. Unread: whether an Identify-learned address reaches discovery's candidates (the cache provider).

**Measured 2026-11-13** (commit 382eab99c, `tests/automated-client/tests/two_hosts.rs` `the_side_*` cases, pushed on the j167 branch): with no learnable address B dials nothing and answers PeerUnknown in milliseconds; with A's private address learned (fixture `netns::add_unrouted_address`, an address on `lo` no other namespace routes to) and no send, B dials nothing after the close or on the link's return; a send dials the learned address, answers PeerUnreachable, and that failure schedules a retry (30 s doubling), so B redials from then on. Door 2: the cache learns only `RouteConfirmed` (B's own successful dial), so it opens only after B itself reached A. The overlay's 100.64 is never learned (Identify withholds, rule 2 refuses). The inner netns run hides stderr on success: write to `target/` to read a measurement. Reported to architect-cto-01 (REPLY 01a1d706, addendum after it); they supply §14's wording onto my branch, then I open the PR.

**PR #666** (2026-11-13): the measurement, architect-cto's §14 wording (a07c83ed9), three review rounds (F1/F2/F3 all P3, fixed; R1 race settled by 20/20 runs, not under load). Armed below the minimum on architect-cto's AUTHORIZES (REPLY 01a1d74f) at head 16f966e2; merged 5608d8ea7. The first tree-checks run hit the job's 15-min timeout on a slow runner (main runs ~4 min); `gh run rerun --failed` after the run ended passed, no push, arming kept. Their j686 (ADR-0068) adds a paragraph to §14 without touching this one.

Related: [[j165-data-plane-hold-handover]], [[hold-choice-must-agree-at-both-ends]].

*References: hold-choice-must-agree-at-both-ends, j165-data-plane-hold-handover*

*Anchors: `tests/automated-client/tests/two_hosts.rs::after_a_close`, `crates/transport/libp2p/src/runtime/commands.rs::known_addresses`*

*Observed 2026-11-13 (p2p-network-dev)*

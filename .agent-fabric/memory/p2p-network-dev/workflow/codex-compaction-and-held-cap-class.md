---
role: "p2p-network-dev"
class: workflow
topic: "codex-compaction-and-held-cap-class"
description: "PR #468 (j133) review lessons: Codex fails on compaction over a large range (drop the lenses to fit), a mutation's exit 101 can be a compile error, and a peer-written list's cap must count the book's distinct keys"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d8cef515a71d5467
---

## PR #468 (j133) review lessons: Codex fails on compaction over a large range (drop the lenses to fit), a mutation's exit 101 can be a compile error, and a peer-written list's cap must count the book's distinct keys

Measured on BlueTeam-OU/InterWeave#468, 2026-10-27.

- `fabric-review --engine codex` on a ~12-file range failed twice running ("Long threads and multiple compactions ... answer is not trusted", nothing posted). The same request with the `lenses:` list removed (a copy of the yaml cut at `lenses:`) posted on the next try.
- A mutation that fails with cargo exit 101 may have failed to COMPILE (here: removing the only `insert` left a `HashSet` without a type). Grep the log for `panicked` with the expected message, never trust the exit code alone. Also re-read the line as rustfmt formatted it before anchoring a mutation; an anchor that misses leaves the test run unmutated and green.
- The second engine found the same cap twice in two rounds: first a per-peer cap crowding out the other address family, then duplicates crowding out a distinct route. At the second finding, the class adjudication enumerated every peer-chosen shape (repeats, /p2p-suffix spellings, a circuit with and without its tail, order, family, v4-mapped, count), and one fix closed them all: dedupe by the book's own key (`canonical_dial_address`) before a per-family cap.

**Why:** each round costs a full blind review + a Codex review + a 2 h local CI run on a new head.
**How to apply:** when holding or capping anything a peer writes, key it by what the consumer keys and cap per the rule's own partition, from the first commit. On the first finding against a cap, ask the adjudication to enumerate the input shapes. See [[review-loop-same-invariant-escalate]], [[wildcard-listener-loopback-first-refuses-cache-route]].

*References: review-loop-same-invariant-escalate, wildcard-listener-loopback-first-refuses-cache-route*

*Anchors: `crates/transport/libp2p/src/runtime/dialing.rs::HeldBack`, `crates/transport/libp2p/src/runtime/dialing.rs::canonical_dial_address`*

*Observed 2026-10-27 (p2p-network-dev)*

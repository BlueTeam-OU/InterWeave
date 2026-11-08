---
role: "p2p-network-dev"
class: solution
topic: "j88-peer-list-last-seen-state"
description: "j88 peer-list 1.2.0 last_seen_ms (IPC 2.12) merged in #503 2c9a045e 2026-10-28; what it is, its known limit, who builds on it"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 84bd9958988492f3
---

## j88 peer-list 1.2.0 last_seen_ms (IPC 2.12) merged in #503 2c9a045e 2026-10-28; what it is, its known limit, who builds on it

**Done.** j88 (architect-cto-01's REQUEST 01a181d8) merged as #503, 2c9a045e, on 2026-10-28.
- peer-list 1.2.0 serves `last_seen_ms` from IPC 2.12.
- The value is the time of the reading while a retained connection is open, else the wall-clock time stamped when the last one closed. It is null when no retained connection has existed since the daemon started.
- The table is `crates/transport/libp2p/src/runtime/last_seen.rs`, held in memory only.
- From 2.12 a page holds 456 rows: 512 of the largest 1.2 rows came to 138,825 bytes, over the 128 KiB body.
- The neutral type is `PeerLastSeen { Unreported, Never, At }`.

**Review lessons** (three rounds, F1-F6):
- Re-deriving a past time from monotonic plus wall moves it by 1 ms per reading. Stamp it once on the wall clock.
- `MAX` = the sum of the two sets' caps is reached exactly by two full sets. Never write "no pair of sets reaches it".

**Known limit, stated on the PR:** a peer revoked while the table is full, then trusted again before it reconnects, reads null.

**Builds on it:**
- rust-ui-dev-02's j76 builds the CLI's reading of the field, after their B2b.
- p2p-network-dev-01's j136 put its libp2p and composition fixture sites in after the merge.

*Anchors: `crates/transport/libp2p/src/runtime/last_seen.rs::MAX_LAST_SEEN`, `crates/api/ipc-protocol/src/result.rs::LAST_SEEN_SINCE_MINOR`*

*Observed 2026-10-28 (p2p-network-dev)*

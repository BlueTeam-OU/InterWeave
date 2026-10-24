---
role: "p2p-network-dev"
class: solution
topic: "recheck-adr0019-dedup-key-2026-10-20"
description: "InterWeave's ADR-0019 dedup keys on the transport MessageId, not app_message_id; an application re-send under a fresh id escapes it"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 545f700c2f3f5a42
---

## InterWeave's ADR-0019 dedup keys on the transport MessageId, not app_message_id; an application re-send under a fresh id escapes it

In InterWeave (BlueTeam-OU/InterWeave), the receiving daemon's duplicate suppression (ADR-0019)
is keyed by (source_peer, source_endpoint, destination_selector, transport message_id) for
direct traffic, and (source_peer, channel, message_id) for broadcast. The content fingerprint is
only a conflict check on that key. The cache holds 10,000 entries (`DEFAULT_MAX_ENTRIES`) for
5 minutes (`DEFAULT_TTL_MS`), in memory only (crates/transport/runtime/src/dedup.rs). It is lost
when the receiver restarts.

So any client action that re-sends "the same message" (a retry, or an Undo of Cancel) must reuse
the stored transport id AND the byte-identical payload:
- a fresh id is a new key, so the daemon enqueues a second copy;
- a re-encode under the same id is a fingerprint conflict (`Admission::Conflict`) and is rejected
  (ADR-0050 point 7).

Answered for rust-ui-dev-01 on 2026-10-16 (GZCoord REPLY 01a143b2-ecc9).

**Why:** clients tend to assume the remote dedups by app_message_id. Only the human client's
read_pairs does that, and only for messages already read.

**How to apply:** when asked whether a re-queue is a retry, answer from this key.

Re-checked 2026-10-20 against InterWeave origin/main: both anchors hold (dedup.rs:34-36, the ADR's
key lines 16-17). The 2026-10-20 stale-hard flag came from resolving them in agent-fabric, where
this section is filed; the paths are InterWeave's.

*Anchors: `crates/transport/runtime/src/dedup.rs::DEFAULT_TTL_MS`, `architecture/adr/0019-duplicate-suppression.md::message_id`*

*Observed 2026-10-20 (p2p-network-dev)*

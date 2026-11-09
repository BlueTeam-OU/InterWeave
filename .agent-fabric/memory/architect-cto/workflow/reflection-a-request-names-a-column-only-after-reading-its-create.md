---
role: "architect-cto"
class: workflow
topic: "reflection-a-request-names-a-column-only-after-reading-its-create"
description: "A REQUEST that names a store column or index is written after reading the table's CREATE on main, never from the logical table list in STATE.md; and a correction is checked against BOTH conversation shapes: the UNIQUE key covered the…"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 90831a8e7a17a99f
---

## A REQUEST that names a store column or index is written after reading the table's CREATE on main, never from the logical table list in STATE.md; and a correction is checked against BOTH conversation shapes: the UNIQUE key covered the direct probe and not the channel one, so v13 was needed after all (2026-11-03)

2026-11-03, j462: I asked rust-ui-dev-01 (REQUEST 01a1a079-d3c6) for a
schema v13 index on unread_inbound and kept_inbound over
(conversation_id, app_message_id), reading STATE.md's logical table
list, which writes `unread_inbound(local_row_id, conversation_id, …)`.
schema.rs on main 64326f779 has no such column: a conversation is
(source_peer, source_endpoint_key, channel_key), and each table's
UNIQUE key is (source_peer, source_endpoint_key, channel_key,
app_message_id) — the point read §5.2 needs was already indexed. The
correction went out 20 minutes later (REPLY 01a1a07e-8fc2) after the
worktree for the STATE.md supply was already cut; nothing was pushed.

**Why:** STATE.md's "Logical tables" block is conceptual by its own
word ("Conceptually:"); the physical shape is schema.rs and only it
decides whether a read is indexed. A request naming a column that does
not exist costs the builder a round and me a correction.

**How to apply:** before a REQUEST or a DECISION names a column, an
index or a key, `git grep -n "CREATE TABLE <t>\|CREATE .*INDEX .* ON <t>"
origin/main -- crates/human/store/src/schema.rs` and read the latest
CREATE; name what is there. When the ask is a lookup, check first
whether an existing UNIQUE or index already answers it.
Related: [[reflections-index]].

**Second lesson, same day (j489):** my correction "no v13, the UNIQUE key already indexes the point read" held for a direct target only. The key leads with the publisher, and a channel conversation is `channel_key` alone, so a channel target cannot be probed by it and the planner ranged `*_by_channel`, reading payloads (rust-ui-dev-01 measured it, QUESTION 01a1a080). A column or index claim is checked once per shape the caller holds, not once; v13 landed as `*_by_message` covering indexes (cb1b1a674, for/ branch to rust-ui-dev-01).

*References: reflections-index*

*Observed 2026-11-03 (architect-cto)*

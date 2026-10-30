---
role: "architect-cto"
class: workflow
topic: "reflection-a-distinct-in-a-count-is-read-against-the-tables-key"
description: "Before ruling a count DISTINCT over a column, read the table's UNIQUE key: within a group the key already scopes, DISTINCT is a no-op, and outside it DISTINCT merges rows the key deliberately keeps apart"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ece45f8c49a5d87b
---

## Before ruling a count DISTINCT over a column, read the table's UNIQUE key: within a group the key already scopes, DISTINCT is a no-op, and outside it DISTINCT merges rows the key deliberately keeps apart

In InterWeave's start-window ruling A3 (2026-10-27) I wrote the badge as "distinct (conversation, app_message_id)" with a kept-item case as the reason. The table's key is (source_peer, source_endpoint_key, channel_key, app_message_id), so for a direct conversation the distinct was a no-op, for a channel it merged two publishers' rows the key keeps apart, and the kept-item case was unreachable by a count that reads one table. The owner's external review found it the same day; the test pinned the wrong count with the words "one item".

**Why:** a DISTINCT reads as safety and gets a plausible reason; the key is the only thing that says what it actually does.
**How to apply:** when a ruling names a count, state it against the table's UNIQUE key and the item identity of the layer that shows the rows (the model's dedup key), and name the one case where the two forms differ; if no such case exists, write COUNT(*) and say why. Related: [[reflection-read-a-status-set-from-the-schema-before-ruling-on-it]].

*References: reflection-read-a-status-set-from-the-schema-before-ruling-on-it*

*Observed 2026-10-27 (architect-cto)*

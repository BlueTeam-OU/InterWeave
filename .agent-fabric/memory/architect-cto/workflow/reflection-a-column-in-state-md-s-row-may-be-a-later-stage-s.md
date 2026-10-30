---
role: "architect-cto"
class: workflow
topic: "reflection-a-column-in-state-md-s-row-may-be-a-later-stage-s"
description: "W44 — before naming a column in a ruling or an index, read the STATE.md row's trailing note and the store's schema.rs: a column listed in the row can be documented as a later stage's (contacts.local_endpoint_id is Stage 22's, absent at v9)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0d494d9811d9c271
---

## W44 — before naming a column in a ruling or an index, read the STATE.md row's trailing note and the store's schema.rs: a column listed in the row can be documented as a later stage's (contacts.local_endpoint_id is Stage 22's, absent at v9)

2026-10-27: I supplied a v10 index `contacts (local_endpoint_id, name_key)` because STATE.md's contacts row lists `local_endpoint_id`; the row's own trailing note says the column is Stage 22's and schema.rs lacks it at v9. rust-ui-dev-02 refused the fold and I replaced it with `contacts (name_key)`, widened by Stage 22's migration.

**Why:** a STATE.md row is the target shape; the note after the row says which parts are built.

**How to apply:** before naming a column, grep `crates/human/store/src/schema.rs` for it, or read the row's note; cite the stage that adds it when it is not there. Related: [[reflection-read-a-status-set-from-the-schema-before-ruling-on-it]].

*References: reflection-read-a-status-set-from-the-schema-before-ruling-on-it*

*Observed 2026-10-27 (architect-cto)*

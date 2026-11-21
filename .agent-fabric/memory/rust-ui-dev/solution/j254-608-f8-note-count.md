---
role: "rust-ui-dev"
class: solution
topic: "j254-608-f8-note-count"
description: "ui-model add page — every Some add note goes through page_note so the trace count bumps; j254 #608 F8 supplied at b3f3774c1"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 71d7a97c7b836908
---

## ui-model add page — every Some add note goes through page_note so the trace count bumps; j254 #608 F8 supplied at b3f3774c1

In crates/human/ui-model/src/peers.rs, AddPage.note is set to a Some only
through `page_note`, which bumps `notes`; trace.rs reads (note, count) from
`add_note_said()`, so a direct `page.note = Some(..)` makes a repeated note
print one `note=` line. #608 F8 found two such sites (scan_pressed
ScanAfterField, scanned's refusal); fixed in b3f3774c1 on
develop-qzapp/rust-ui-dev-01/for/p2p-network-dev-02/f8-add-note-count, off
a8c6512a4, delivered 2026-11-10 to p2p-network-dev-02 for their fold.
Guard: ui-slint tests/trace.rs a_scan_note_set_twice_is_said_twice (fails
with the bump removed). Peers' own `add_note` (open_add) has no count.

*Observed 2026-11-10 (rust-ui-dev)*

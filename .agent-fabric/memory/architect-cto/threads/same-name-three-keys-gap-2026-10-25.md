---
role: "architect-cto"
class: threads
topic: "same-name-three-keys-gap-2026-10-25"
description: "InterWeave open thread 2026-10-25: the design's same-name rule has three keys (case, shape, compatibility shape; design 2026-10-21-contact-cards s5.4.1 as #438 rewrote it), the store computes the case key alone (store::name_key, #437)…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 417a31286f3d69a3
---

## InterWeave open thread 2026-10-25: the design's same-name rule has three keys (case, shape, compatibility shape; design 2026-10-21-contact-cards s5.4.1 as #438 rewrote it), the store computes the case key alone (store::name_key, #437), the ui-model's titles still compare exactly (rust-ui-dev-02's j60); human-client-ui.md s13 bullets 4 and 9 state the three-key criterion (#442) and the Stage 21 closing record carries the gap by name

Verified on origin/main 14161365 (2026-10-25): `crates/human/store/src/
store/contacts.rs` `name_key` = NFKC, full case fold, white space, TS 39
skeleton (one key); `crates/human/ui-model/src/model/cards.rs`
`name_is_shared` compares exactly. Owed: rust-ui-dev-02's j60 (twin
titles with the short PeerId on one name_key, and the store's comparison
widened to the design's three keys, or ux-designer narrowing the design
to one, their call); my part is done when s13's criterion and the closing
record name it. Closes when a PR puts the ui-model and the store on the
design's keys, or the design is narrowed and s13 follows.

*Observed 2026-10-25 (architect-cto)*

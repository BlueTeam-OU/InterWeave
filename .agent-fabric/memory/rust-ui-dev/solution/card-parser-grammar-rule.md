---
role: "rust-ui-dev"
class: solution
topic: "card-parser-grammar-rule"
description: "chat-protocol's card classify must decide card-ness by the JSON grammar, not by what serde_json can represent; how (#351)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c265301a0bd70e39
---

## chat-protocol's card classify must decide card-ness by the JSON grammar, not by what serde_json can represent; how (#351)

`crates/human/chat-protocol/src/card.rs` `classify` (plan §29 step 1, PR #351, merged 45074096, 2026-10-22) answers
NotCard / Card / MalformedCard(CardFault). The rule a reviewer broke it on twice: whether a text is a card follows the
JSON GRAMMAR, not serde_json's representability. A kind copy of `1e400`, a lone-surrogate escape, nesting past serde's
recursion limit, or a lone-surrogate key are well-formed JSON other readers accept, so they must not make a card
NotCard (the bridge forwards NotCard raw). How it holds:
- keys read via `deserialize_bytes` (serde_json's unvalidated raw-string path; escapes still decoded);
- a `kind` copy taken as `&RawValue` (chat-protocol enables serde_json `raw_value`), decoded only to compare;
- every other value `IgnoredAny` (iterative, no range/surrogate/depth checks);
- plus a whole-text `IgnoredAny` parse, because the raw key path skips the "no unescaped control character" rule —
  without it a raw tab in a key made non-JSON a card. Never check decoded key bytes for controls: an escaped `\u0009`
  is legal.
A strict failure that is not a duplicate is `CardFault::Unreadable`. Tests: `tests/frozen_cards.rs`
(`json_this_parser_cannot_represent_never_hides_a_card`, `a_raw_control_character_is_not_json_in_a_key_or_a_value`).

Gotcha: tool inputs (Edit/Write/heredoc via the tool) decode `\uXXXX` before writing; build escapes in Rust from a
backslash constant (`let bs = '\\';`) so the test text really carries them.

*Anchors: `crates/human/chat-protocol/src/card.rs::any_top_level_kind_is_card`, `crates/human/chat-protocol/src/card.rs::Unreadable`*

*Observed 2026-10-22 (rust-ui-dev)*

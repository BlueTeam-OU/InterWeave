---
role: "rust-ui-dev"
class: solution
topic: "j95-design-choices"
description: "How the long-message cut and reader are built in ui-model/ui-slint: drawn lines, the custom ListView, the reader overlay, copy limits"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 42992cfa6c2fc6b1
---

## How the long-message cut and reader are built in ui-model/ui-slint: drawn lines, the custom ListView, the reader overlay, copy limits

- The cut (`crates/human/ui-model/src/cut.rs`) counts DRAWN lines: one
  per block plus one per hard break (any UAX #14 mandatory break), and
  line breaks are not characters. architect-cto ruled it (DECISION
  01a1ab12): the contract's "paragraph is a line" let a hard-broken
  paragraph pass whole past the ceiling.
- Slint 1.18 virtualizes a `for` only when its parent type is NAMED
  `ListView` and has the content/visible size properties
  (i-slint-compiler object_tree.rs). std-widgets' ScrollView lacked what
  the reader needs, so app.slint has its own `component ListView` over a
  Flickable. Rows not laid out are not in the focus chain, and
  content-height is an estimate, so End needs a hold (held-at-end).
- The reader is an overlay on the conversation pane; the pane stays
  alive underneath so its scroll and focus survive.
- Each reader link control follows the last piece of its own line:
  `body::flatten_placed` keeps the line through the pieces, and
  `Forms.link_rows` gives TAB its rows.
- Copy uses a hidden TextInput. A refused copy cannot be detected through
  it, a known limit; the testing backend keeps the clipboard but has no
  public reader, so copy is tested through invoke_message_source.
- Spoken text: every accessible label goes through the `Drawn.spoken`
  pure callback, wired per window ([[slint-unset-pure-callback-empty]]).

*References: slint-unset-pure-callback-empty*

*Anchors: `crates/human/ui-model/src/cut.rs::formatted`, `crates/human/ui-model/src/body.rs::flatten_placed`, `crates/human/ui-slint/src/reader.rs::link_rows`, `crates/human/ui-slint/ui/app.slint::component ListView`, `crates/human/ui-slint/ui/app.slint::held-at-end`*

*Observed 2026-11-09 (rust-ui-dev)*

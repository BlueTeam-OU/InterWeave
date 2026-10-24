---
role: "rust-ui-dev"
class: workflow
topic: "labels-english-supplier"
description: "who finalises English for crates/human/ui-model/src/labels.rs values, and the conventions a new placeholder follows — strengthened: Keep is reserved for the message action"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0b5ce98089126a2b
  - 1e9b3a1b2c11dbc4
---

## who finalises English for crates/human/ui-model/src/labels.rs values, and the conventions a new placeholder follows — strengthened: Keep is reserved for the message action

language-culture-en does the first read of every person-facing value in
crates/human/ui-model/src/labels.rs (since InterWeave#229). It delivers each value by locator
(key and text, naming its cold read); I paste it verbatim in my own PR with the
`Supplier-Review:` trailer the delivery names, and tell them when it is on main. A new value is a
placeholder I draft, with context for every key: where it sits, the window width, what the
person did, what slots hold. architect-cto reads new values against human-client-ui.md's
vocabulary sections only when asked; where the two disagree, language-culture-en's final text
stands unless the rule it breaks is cited.

Conventions for the next placeholder:
- en-US spelling ("Canceled").
- Sentence case for labels, buttons and headings, with no final period.
- A complete sentence ends with a period. Every ErrorClass value is a full sentence, because it
  fills "Not sent: {reason}" and "Could not open the session: {reason}".
- Terms: "transport daemon" for the local component, "app" for this client (never "client"),
  "list of trusted peers" for the list, PeerId written exactly so.
- **Keep is the product's verb for a message action ("Keep", "Unread, kept"), so no other value
  uses keep/keeps/kept as an ordinary verb** ("where InterWeave stores your messages", not
  "keeps"): a reader takes it as that action. Seen 2026-10-21 in three refusal values on main.
- A value that names a place the screen shows ("the folder below") must be true for every case
  that reaches it: check the mapping in code before accepting the wording.
- Tests look text up through `placeholder_en::…` / `fill(text(UiText::…))`, never a literal.

*Anchors: `crates/human/ui-model/src/labels.rs::ui_texts!`*

*Observed 2026-10-23 (rust-ui-dev)*

## who finalises English for crates/human/ui-model/src/labels.rs values, how it reaches my PR, and the conventions a new placeholder should follow

language-culture-en does the first read of every person-facing value in
crates/human/ui-model/src/labels.rs (since InterWeave#229). It delivers each value by locator
(key and text, naming its cold read); I paste it verbatim in my own PR with the
`Supplier-Review:` trailer the delivery names, and tell them when it is on main. A new value is a
placeholder I draft, with context for every key: where it sits, the window width, what the
person did, what slots hold. architect-cto reads new values against human-client-ui.md's
vocabulary sections only when asked; where the two disagree, language-culture-en's final text
stands unless the rule it breaks is cited.

Conventions for the next placeholder:
- en-US spelling ("Canceled").
- Sentence case for labels, buttons and headings, with no final period.
- A complete sentence ends with a period. Every ErrorClass value is a full sentence, because it
  fills "Not sent: {reason}" and "Could not open the session: {reason}".
- Terms: "transport daemon" for the local component, "app" for this client (never "client"),
  "list of trusted peers" for the list, PeerId written exactly so.
- **Keep is the product's verb for a message action ("Keep", "Unread, kept"), so no other value
  uses keep/keeps/kept as an ordinary verb** ("where InterWeave stores your messages", not
  "keeps"): a reader takes it as that action. Seen 2026-10-21 in three refusal values on main.
- A value that names a place the screen shows ("the folder below") must be true for every case
  that reaches it: check the mapping in code before accepting the wording.
- Tests look text up through `placeholder_en::…` / `fill(text(UiText::…))`, never a literal.

*Anchors: `crates/human/ui-model/src/labels.rs::ui_texts!`*

*Observed 2026-10-23 (rust-ui-dev)*

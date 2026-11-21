---
role: "rust-ui-dev"
class: solution
topic: "j273-namefield-textinput-clips"
description: Slint 1.18.1 TextInput always clips its ink to its own box (unlike elided Text); InterWeave NameField needs no clip box, measured by tests/name_field_clip.rs
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2a557b272d63293c
---

## Slint 1.18.1 TextInput always clips its ink to its own box (unlike elided Text); InterWeave NameField needs no clip box, measured by tests/name_field_clip.rs

j273 (2026-11-13), commit 68e24b776 on develop-qzapp/rust-ui-dev-01/feat/j273-namefield-holds-a-single-line-textinput = PR #662 (opened unarmed, 1 work; j274 then j275 follow on it, each a fresh session).

- i-slint-core 1.18.1 `textlayout/sharedparley.rs` `draw_text_input` calls `combine_clip` with the item's own size unconditionally, so a TextInput's marks never leave its box; only `draw_text` (Text) clips under `overflow: clip` alone (see [[j271-elided-label-clip]]).
- Measured on the desktop window under Xvfb, scale 1: a 32-code-point character set as the contact page prefills it (`set_contact_name_draft(cluster::drawable(..))`) changed rows 236..=242 of an input at 235.5..254.5. Mutant (value also drawn by an unclipped elided Text in the box): marks from row 146, test fails.
- Desktop tests can drive the real window through `i_slint_backend_testing::ElementHandle` (find by accessible label, default action, `absolute_position`/`size`) after `View::new` + `show`.
- Not run on a phone (Skia uses the same draw_text_input).

*References: j271-elided-label-clip*

*Anchors: `crates/human/ui-slint/tests/name_field_clip.rs`, `crates/human/ui-slint/ui/app.slint::component NameField`*

*Observed 2026-11-13 (rust-ui-dev)*

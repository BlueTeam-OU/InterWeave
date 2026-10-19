---
role: "rust-ui-dev"
class: solution
topic: "j93-swipe-focus-rules"
description: "j93 (2026-10-17): #286's later swipe rules built on the j93 branch stacked on j91/#292 — Keep/Unkeep focus, focus when the snackbar closes under its Undo (UndoClosed), late Cancel after leaving"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 92fd8cba586e5083
---

## j93 (2026-10-17): #286's later swipe rules built on the j93 branch stacked on j91/#292 — Keep/Unkeep focus, focus when the snackbar closes under its Undo (UndoClosed), late Cancel after leaving

j93 builds the swipe spec's rules #286 added after a030a44a (spec at #286 head 0af22f43, the job named eac50f9b). Branch develop-qzapp/rust-ui-dev-01/feat/j93-interweave-build-the-swipe-spec-s, reset onto j91's head db12b760 (which is stacked on #292 0057924e), so j91 + j93 = 8 work commits for the PR that opens after #292 lands.
- 8552a0df/856361ab Keep/Unkeep: the model's outcome now names the row (focus and moves_from = Item). The view adds `By::{Button,Swipe}` to `Input::Action` and a `pressed` set: a Slint TapArea tap does NOT focus the FocusScope button (FocusScope::input_event runs only if no child accepted the press, i-slint-core 1.18.1 items/input_items.rs:667), so "focus inside the row" alone misses a touch tap on the button.
- db367230/e4a83534 `UiModel::undo_closed()` -> `UndoClosed {id, conversation, focus}` set at every close with no Undo taken (snackbar_closed, open() replacing, undo() with nothing to undo, row_attached, arrived_in). It is separate from `Outcome` on purpose: a new outcome id springs back a committed swipe's slid-out row (render_swipe's committed check). The view knows the Undo held focus only because `focus-id` stays "undo" after the Undo is destroyed (nothing reports losing focus); `focus-id` is now in-out and cleared once no snackbar is left.
- 0023a6c1 late Cancel after leaving: no code change needed. The take-end close of an off-screen conversation's snackbar already makes it announce-only; the tests pin it and a mutation of that close fails them.
- `cargo xtask ci` was run on 0023a6c1 (see the job's evidence for the result).
Related: [[j91-focus-ring-snackbar-time]] [[j90-swipe-device-fixes]] [[mutate-from-a-scratch-copy]]

*References: j90-swipe-device-fixes, j91-focus-ring-snackbar-time, mutate-from-a-scratch-copy*

*Observed 2026-10-17 (rust-ui-dev)*

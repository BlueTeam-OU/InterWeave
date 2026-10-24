---
role: "rust-ui-dev"
class: solution
topic: "slint-focus-new-element"
description: "A Slint control created by the render that requests focus on it misses the first focus-seq change; the snackbar \"undo\" focus key is cleared by the view"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a72bd71efefe5424
---

## A Slint control created by the render that requests focus on it misses the first focus-seq change; the snackbar "undo" focus key is cleared by the view

Found on #384 (Codex review 5621189966 F3, 2026-10-23), when Peers' snackbar Try again had to take keyboard focus.

1. **A control made by the same render that asks for focus does not take it.** ui-slint's `focus_key` defers, then sets `focus-target` and bumps `focus-seq` once. A new ActionButton's `changed focus-seq` saw no change on that first bump. A second bump one timer turn later did focus it. Neither reading `focus-seq` in `init` nor adding a `changed focus-wanted` handler helped. The fix is `View::focus_key_after_draw` in `crates/human/ui-slint/src/lib.rs`: set and bump, then bump again on the next turn.
2. **Focus key "undo" is the conversation snackbar's.** The render clears `focus-id` when it is "undo" and `model.snackbar()` is None (lib.rs, the stale-undo rule). Any other snackbar action must report its own key; the Snackbar component has an `action-key` property for that.

How to see it in a view test: mock 10 ms twice, then render again before asserting `get_focus_id()`. The root renders again in its own turn, so this is what the app does.

Related: [[slint-changed-handlers-deferred]] [[slint-rendered-window-lessons]]

*References: slint-changed-handlers-deferred, slint-rendered-window-lessons*

*Observed 2026-10-22 (rust-ui-dev)*

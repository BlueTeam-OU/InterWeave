---
role: "rust-ui-dev"
class: threads
topic: "j87-swipe-spec-fixes"
description: "j87 delivered 2026-10-16 on #292 (32081440..c542801e): fling window, waiting-Cancel snackbar order, focus-only-from-inside-the-row, focusable headings; what it rests on"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - aeed4088f5f67082
---

## j87 delivered 2026-10-16 on #292 (32081440..c542801e): fling window, waiting-Cancel snackbar order, focus-only-from-inside-the-row, focusable headings; what it rests on

j87 delivered on BlueTeam-OU/InterWeave#292 (branch develop-qzapp/rust-ui-dev-01/feat/j85-interweave-lost-undo-on-a-pick), four work commits 32081440, cbd51f96, 05d0f8cc, c542801e. That makes 7 work commits; j90's fixes go next on the same PR, then review and arm.
Built from the swipe spec at #286's head a030a44a, not yet on main. Re-read it once #286 merges.
- **Fling:** gesture.rs `first_move`. A release within 100 ms of the first move, `offset >= 48` from Slint's `pressed-position`.
- **Waiting Cancel:** `UiModel::cancel_requested(row)` marks the seq when the Cancel is issued; app-core calls it when the command goes out. `cancelled()` checks it: if a newer snackbar is open (id > mark), it only announces and releases the record. `intent_refused(Cancel)` drops the mark.
- **Focus:** `Outcome::moves_from` names the row that was taken away. The view resolves `focus-id` to a row at the start of `render`, before handles retire. Focus moves only if they match, or if `Shared::hidden_from_header` was set by the header's Hide press. Selection is already cleared by the render, so that press is recorded when it happens. `FocusTarget::Header` maps to "h" and `ConversationsHeading` to "heading"; both are `FocusHeading`s in app.slint.
- **Gap:** every focusable element must report `focus-id`, or a stale row stays named there.
Related: [[j85-snackbar-closing-rule]], [[mutate-from-a-scratch-copy]].

*References: j85-snackbar-closing-rule, mutate-from-a-scratch-copy*

*Observed 2026-10-16 (rust-ui-dev)*

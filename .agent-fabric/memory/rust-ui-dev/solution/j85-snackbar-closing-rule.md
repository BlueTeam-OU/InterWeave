---
role: "rust-ui-dev"
class: solution
topic: "j85-snackbar-closing-rule"
description: "ui-slint: one rule in Shared::push takes the snackbar off screen when an input that will close it is queued (SnackUp::closed_by); InterWeave #292"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0e5ab9f928a2ecaa
---

## ui-slint: one rule in Shared::push takes the snackbar off screen when an input that will close it is queued (SnackUp::closed_by); InterWeave #292

j85 (BlueTeam-OU/InterWeave#292, 2026-10-16) replaced #291's Slint-side "Back clears snack-shown" with one Rust rule. `SnackUp::closed_by(input)` names every input that closes the snackbar: `SnackClosed(id)` (timeout, swipe-away, `close_snackbar`), `Left`, `Select` of another conversation, and `TrustPage(true)`. `Shared::push` hides the snackbar when such an input is queued, but not when the press is refused at the cap. `render_swipe` will not redraw a snackbar while a queued input closes it. Each take also closes a conversation snackbar whenever `on_screen()` is not that conversation, and that now includes the trust settings.
When a new input closes the snackbar, add it to `closed_by` and to the take's resolution together, or an Undo shows that the queue answers after the close.
The PR was opened with 3 work commits, unarmed; j87 (ux-designer's j82 decisions, the swipe spec on #286 at 14cafe23, INFO seq 261139) adds its commits to it. Related: [[slint-changed-handlers-deferred]], [[model-only-take-rerender]].

*References: model-only-take-rerender, slint-changed-handlers-deferred*

*Observed 2026-10-16 (rust-ui-dev)*

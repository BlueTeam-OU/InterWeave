---
role: "ux-designer"
class: workflow
topic: "end-device-run-in-known-state"
description: "A device run is not finished until the app is put back in a stated end state; never leave it mid-flow on the phone"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b6fc98047e39abd7
---

## A device run is not finished until the app is put back in a stated end state; never leave it mid-flow on the phone

A device run ends by putting the app back in a stated end state, and the report says what that state is. Leaving
the app on the screen where the last step stopped (e.g. mid-restore on R2) is not an ending.

**Why:** the owner, 2026-11-12, at the start of j116 on the Samsung A40: "the app is where you left it and this
should let you think that it's not the correct way to end a test". The j109/j110 run had left the A40 mid-flow.
A run that leaves the app mid-flow hands the next run (mine or rust-ui-dev's) an unknown starting point and mixes
leftover state into the next result.

**How to apply:** every run script gets a final step, also run when a step fails: leave the flow by the app's own
way back (Back or Cancel, never force-stop unless the run tests that), get to a named screen (Chats, or the
screen the run started on), take a screenshot, and write that state into the report. Before the first step, take a
screenshot and write down the start state. Related: [[device-run-check-focus]], [[workflow-carried-2026-10-18]].

*References: device-run-check-focus, workflow-carried-2026-10-18*

*Observed 2026-11-12 (ux-designer)*

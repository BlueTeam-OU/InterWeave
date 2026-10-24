---
role: "rust-ui-dev"
class: workflow
topic: "brief-facts-checked-first"
description: "A words brief to language-culture states only what the code was checked to do; describe the person's state from the tree, not from intent"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3d01e9d5e2e1eeca
---

## A words brief to language-culture states only what the code was checked to do; describe the person's state from the tree, not from intent

On 2026-10-19 I briefed language-culture-en (REQUEST 01a1550f) on the user-presence "authentication needed" notice. The brief said messages stay readable and drafts are sent later. Both were false: with no service, the store is not open (the listing comes from the facade) and the hub drops commands (android-platform hub.rs `command`). I corrected it in 01a1550f-f5dc before they wrote. The same day I told ux that "reopening the app recovers", which the launch path also contradicts.

**Why:** LC writes the person-facing promise from the brief. A wrong brief becomes a false promise on screen, the very class ux-designer-01 refuses to ship.

**How to apply:** before sending a words brief or a behaviour claim, trace each sentence about what the person can do to the code path (view_side, hub, model), and leave out any I have not traced. See [[interweave-stage17-close-j111]].

*References: interweave-stage17-close-j111*

*Observed 2026-10-19 (rust-ui-dev)*

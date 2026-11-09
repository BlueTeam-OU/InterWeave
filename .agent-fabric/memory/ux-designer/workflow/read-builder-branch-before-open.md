---
role: "ux-designer"
class: workflow
topic: "read-builder-branch-before-open"
description: "Before a spec says a figure is unmeasured or a question open, read the builder's branch for a record of it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 31391dd488e1038a
---

## Before a spec says a figure is unmeasured or a question open, read the builder's branch for a record of it

Before a specification says a measurement is missing, or a question is open, read the building role's branch for a record that already answers it.

**Why:** on #447 (2026-10-25) I wrote "no photograph has been measured at 4:4:4; rust-ui-dev-02's figures will be added here". rust-ui-dev-02 had already recorded seven photographs in `card_photo.rs` (4df2fa26) fifteen minutes earlier, and review round 3 raised it as a P2.

**How to apply:** when a decision says another role will send figures, `git fetch` their branch and grep the code the figures concern (module comments, tests) before writing the gap into the spec. Related: [[spec-cites-only-recorded-methods]], [[j64-photo-subsampling]].

*References: j64-photo-subsampling, spec-cites-only-recorded-methods*

*Observed 2026-10-25 (ux-designer)*

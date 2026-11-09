---
role: "rust-ui-dev"
class: workflow
topic: "verify-review-risk-before-relaying"
description: "A review's \"risk (unproven)\" section is a claim to check against the tree before passing it to another role"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 60e1d5e2d28b27b7
---

## A review's "risk (unproven)" section is a claim to check against the tree before passing it to another role

On #524 (2026-10-31) I passed on the blind review's risk to architect-cto-01 without checking it. The risk was that at the pin (1a4fa590), `list_all` also built contact views and card proposals. architect-cto-01 corrected it from the tree: at the pin, `list_all` was only the three page walks. Contact views entered in 95674b26 (#359) and left with #520. A one-line `git show <rev>:<file>` would have settled it.

**Why:** a risk the reviewer marked unproven is still unproven when I forward it. Forwarding it costs the other role a check and a correction.

**How to apply:** before a message or PR body repeats a reviewer's risk or pre-existing problem, read the claim at the named revision. Then send it as verified or as refuted.

*Anchors: `spikes/spike-011/README.md::read-walk`*

*Observed 2026-10-31 (rust-ui-dev)*

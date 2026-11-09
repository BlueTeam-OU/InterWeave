---
role: "architect-cto"
class: workflow
topic: "reflection-a-review-risk-dated-at-the-pin-is-read-at-the-pin"
description: "W44: a reviewer's \"at the pin X also did Y\" is checked by git show at that revision before it is worded into a record; the function's later life is not its pinned one"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 443fd7fbb9a1da3b
---

## W44: a reviewer's "at the pin X also did Y" is checked by git show at that revision before it is worded into a record; the function's later life is not its pinned one

A review's pre-existing risk on InterWeave #524 said that at SPIKE-011's
pin `list_all` also built contact views and card proposals, so the
timed `read-walk` row understated the start. Read at the pin
(1a4fa590, `crates/human/app-core/src/listing.rs`), `list_all` was
exactly the three page walks the harness times; the contact views and
card proposals entered it with 95674b26 (#359, 2026-10-22) and left
with #520. The substance was worth a sentence, the date was wrong.

**Why:** a reviewer reads the function in the tree it has, or in the
last tree before a removal, and labels it "at the pin"; a record that
copies the label puts a false fact beside a measured figure.

**How to apply:** before wording a claim about a pinned revision, `git
show <pin>:<path>` the function and `git log -S<symbol>` for when the
extra work arrived and left; word the window those commits bound, and
cite them. Worded in SPIKES.md on #523 (4bb7ba34).

*Observed 2026-10-31 (architect-cto)*

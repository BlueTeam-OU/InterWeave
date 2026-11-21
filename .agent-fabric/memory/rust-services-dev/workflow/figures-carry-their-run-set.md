---
role: "rust-services-dev"
class: workflow
topic: "figures-carry-their-run-set"
description: Drafting facts from a spike campaign — each figure names the runs it counts, and a control covers only the cells it ran
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 6d0f2c745caf5f59
---

## Drafting facts from a spike campaign — each figure names the runs it counts, and a control covers only the cells it ran

Two P2s in #612's blind review (head ca3f529bf, fixed in 8a408fbb7) hit
facts I drafted for SPIKE-001 S32 (b). One figure ("7 of 20") mixed the
runs kept apart in `runs/s32-lenient-stub/` with the counted runs. One
cause was credited to a wording ("reply") in a cell the control did not
cover: promptw ran tool and idle only, and the text cell's work already
said "Write".

**Why:** a figure from a pooled count, or a cause read across cells,
reads as settled and passes the caller's check too: the caller copied
the 7 over.

**How to apply:** compute every figure in a draft fact from one named run
set (a directory glob, counted runs only), and state a cause only for
the cells its control ran. See [[stub-must-refuse-what-the-server-refuses]].

*References: stub-must-refuse-what-the-server-refuses*

*Observed 2026-11-10 (rust-services-dev)*

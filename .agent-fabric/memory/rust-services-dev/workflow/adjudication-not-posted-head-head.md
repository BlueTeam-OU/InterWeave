---
role: "rust-services-dev"
class: workflow
topic: "adjudication-not-posted-head-head"
description: "A finding-adjudication brief has range head..head, so fabric-pr post-review refuses its coverage paths; only a later re-review's \"fixed\" closes a finding at arm"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 01c44dbd3f1c6c0f
---

## A finding-adjudication brief has range head..head, so fabric-pr post-review refuses its coverage paths; only a later re-review's "fixed" closes a finding at arm

In InterWeave #635 (j68, 2026-11-12), the review-class adjudication of the blind review's four P3s rendered with range `22caa5391..22caa5391`. `review_brief.py` refuses any other range for that mode. Its report's coverage block named the changed files, so `fabric-pr post-review` refused it: "the coverage block names 7 path(s) outside the range".

Gate 4c of `fabric-pr arm` closes a `Finding:` only on a later `Judgement: … | fixed`, which a re-review of the fix range gives. A "real" verdict closes nothing. So the unposted adjudication cost nothing: I fixed, re-reviewed `22caa5391..<fix head>`, and posted that.

**Why:** editing the adjudicator's coverage block to make it post would alter a review-class report. That is not mine to do.

**How to apply:** keep the adjudication's verdicts in the session and cite them where you answer the finding. Post the blind review and each re-review; arm when a re-review marks every finding fixed. If two review-class dispatches disagree on a finding (here F1's plan half: "not real" in the adjudication, "not fixed" in the re-review), ask the owner of the line to fix it rather than defer. A one-line supply was cheaper than the argument. Related: [[arm-refuses-open-p3-defer-each]], [[fold-review-from-merge-base]].

*References: arm-refuses-open-p3-defer-each, fold-review-from-merge-base*

*Observed 2026-11-12 (rust-services-dev)*

---
role: "rust-services-dev"
class: workflow
topic: "arm-refuses-open-p3-defer-each"
description: "fabric-pr arm refuses on any open review finding, P3 included; a supplier's P3s are lifted one by one with --defer \"<head8>:F<n>\" \"<why>\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 32aebea0516f624f
---

## fabric-pr arm refuses on any open review finding, P3 included; a supplier's P3s are lifted one by one with --defer "<head8>:F<n>" "<why>"

On InterWeave #628 (2026-11-11) the blind review of architect-cto's supplied propagation commit raised three P3s only, no P1/P2, and `fabric-pr arm` still REFUSED: "3 finding(s) open in a review chain … a re-review on that chain judging each fixed or not-real, or --defer". Deferring each with `--defer "0bfd804a:F4" "<why>"` (repeatable) armed it and recorded the reasons in the arming comment.

**Why:** the arm gate counts every open finding in the chain, not only P1/P2; "P3s do not block" is not true of the tool.

**How to apply:** before arming, either fix and re-review, or defer each finding by number with its owner named; when the findings are in a supplier's hunks, send them to that supplier in the same step. Related: [[arm-word-by-message]], [[fold-review-from-merge-base]].

*References: arm-word-by-message, fold-review-from-merge-base*

*Anchors: `agent-fabric:bin/fabric-pr::--defer`*

*Observed 2026-11-11 (rust-services-dev)*

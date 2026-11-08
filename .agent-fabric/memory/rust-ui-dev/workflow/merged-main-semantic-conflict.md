---
role: "rust-ui-dev"
class: workflow
topic: "merged-main-semantic-conflict"
description: "after folding origin/main into an open PR, re-review the merge for semantic conflicts with main's new UI (overlays, pages) — clean --cc is not safe"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - da0abd4350a49794
---

## after folding origin/main into an open PR, re-review the merge for semantic conflicts with main's new UI (overlays, pages) — clean --cc is not safe

A merge of origin/main with an empty `git show --cc` still broke #578: main's new message reader (an overlay hiding the conversation pane) arrived in the merge, and my locked-notice focus ask was spent on the hidden pane card (round 4 F5, P2). Only the re-review of the merge range found it; tests on both sides passed.

**Why:** the PR's rule ("hold the ask under an identity or store screen") was written for the overlays that existed when it was built; a new overlay on main is a new case of the same rule.

**How to apply:** after folding main, list main's new screens/overlays/pages in the files the PR also touches and ask whether each PR rule naming "a screen over X" covers them; also run full `cargo xtask ci` on the merged tree — main's commits pushed `View::new` past clippy's 200-line limit (CI red) on the same fold.

Also: a merge-range re-review that marks main-only paths "not read" breaks the re-review chain (`fabric-review check` then demands a full review). After a fold, brief a full `mode: review` from `git merge-base origin/main HEAD` to the head — the diff is then the PR's own change against the new main, and the open findings ride along automatically. See [[rereview-card-base]].

*References: rereview-card-base*

*Observed 2026-11-06 (rust-ui-dev)*

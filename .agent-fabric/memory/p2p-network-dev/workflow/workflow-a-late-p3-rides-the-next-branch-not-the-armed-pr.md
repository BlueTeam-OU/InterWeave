---
role: "p2p-network-dev"
class: workflow
topic: "workflow-a-late-p3-rides-the-next-branch-not-the-armed-pr"
description: "A P3 doc-wording finding that arrives once a PR is armed (or holds only it back): arm or keep the arming, carry the wording on the next branch or the text owner's own PR, and say so on the thread; a push disarms and costs a re-review"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 5079a2383e56efc6
---

## A P3 doc-wording finding that arrives once a PR is armed (or holds only it back): arm or keep the arming, carry the wording on the next branch or the text owner's own PR, and say so on the thread; a push disarms and costs a re-review

Any push to an armed PR takes the arming off (ADR-019 §5 rule 5, disarm-on-push). A fix push also needs a
re-review of its range before the PR can be armed again. When the only open finding is a P3 wording, and
someone waits on the merge, holding the PR costs more than the finding is worth.
- #378 (2026-10-23): F4 (P3, item 13's wording, architect-cto's text) was the only finding, and rust-ui-dev-02's
  #387 waited. I armed without it, and the wording landed later.
- #393 (2026-10-23): F4 (P3, "line 1" read 1-based) came on the last re-review, with #393 armed and #384 waiting.
  I planned the fix as the first commit of my next branch. It then turned out the text owner's own PR (#389)
  carried it, so that commit was dropped.

**How to apply:** if the finding is P3, it is about wording only, and the PR is armed or would otherwise arm,
then (1) keep or take the arming; (2) answer the finding on the PR thread, naming where it will land; (3) tell
the text's owner, who may already be changing that line in a PR of their own. Ask the owner first if a dependent
PR is not waiting, or if the finding is P2 or above.

*Observed 2026-10-25 (p2p-network-dev)*

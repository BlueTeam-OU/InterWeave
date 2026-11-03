---
role: "p2p-network-dev"
class: workflow
topic: "workflow-a-run-file-fix-sweeps-every-restatement"
description: A finding that a run file or doc claims more than was measured is fixed across every restatement of that claim (title, bullets, module doc, fn doc, PR body) and every trigger word in one pass, or it costs a round per instance
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 6da0f2c882c5f59b
---

## A finding that a run file or doc claims more than was measured is fixed across every restatement of that claim (title, bullets, module doc, fn doc, PR body) and every trigger word in one pass, or it costs a round per instance

On #417 (2026-10-24) one P2, "the run file says the phones connect only after both trust, but run 4's row read connected",
took four re-review rounds. Each fix reworded the sentence the reviewer quoted. The same claim was restated in
the run file's title, a step bullet, the test module doc, a fn doc comment and the PR body. And each new sentence
brought in a new trigger word ("only after Noise", "passes only on reading", "Only a device shows").
The shape that closed it: grep every touched file for the claim's words and for the CLAUDE.md §4 trigger words
(never/only/bounded/exactly/fails closed). Then rewrite each hit as what was measured, or cite the code/source,
or drop it.

**Why:** a re-review is scoped to the fix range, so it finds the next restatement only after you push.
Every hit costs a full round.

**How to apply:** before committing an answer to a prose/claim finding, run
`git diff <base> -- <files> | grep -nE '^\+.*\b(never|only|bounded|exactly|fails closed)\b'` plus a grep for the claim's
own words, over the whole PR range rather than the fix range. Also settle a gate's wording with its owner
(architect-cto) in the same pass, rather than letting the run file decide it. Related: [[workflow-cite-the-contracts-classes-not-their-codes]].

*References: workflow-cite-the-contracts-classes-not-their-codes*

*Anchors: `tests/android-e2e/runs/2026-10-24-pixel-9a-a40-wifi-fresh-pair.md::exchange messages after both trust`*

*Observed 2026-10-24 (p2p-network-dev)*

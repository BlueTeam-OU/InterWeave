---
role: "rust-services-dev"
class: workflow
topic: "fold-review-from-merge-base"
description: "After merging main into a PR, review it from the new merge-base; a re-review of the fold range posts partial and breaks the chain"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 48fba38e78b41459
---

## After merging main into a PR, review it from the new merge-base; a re-review of the fold range posts partial and breaks the chain

After folding origin/main into an open PR, do not re-review `<old head>..<new head>`: that range holds every file main brought in, the reviewer marks them "not read", and `fabric-pr post-review` posts it as partial coverage. A partial review is not a checkpoint, so the next re-review based on its head is refused ("base … is neither the merge-base … nor the head of the PR's last checkpoint review"). Seen on InterWeave #532, 2026-11-03.

**Why:** the gate chains reviews by base= the previous one's head; only a full-coverage review is one a later re-review may start from, and `fabric-pr arm` refuses while any chain still has a `Finding:` with no later `Judgement:` (gate 4c).

**How to apply:** after a fold, dispatch a full review of `<merge-base with main>..<head>` — after the fold that range is only the PR's own files. If an earlier chain still holds open findings, brief that review (mode re-review, same merge-base range) with the old report as previous_findings so its `Judgement:` lines close them; `--defer` records them as carried, not fixed. A brief requirement may not say "is fixed" (fabric-review refuses verdict wording): write "gets a judgement at the head". See [[post-review-from-transcript]].

*References: post-review-from-transcript*

*Anchors: `agent-fabric:tools/fabric/github/review_checkpoint.py::checkpoint`*

*Observed 2026-11-03 (rust-services-dev)*

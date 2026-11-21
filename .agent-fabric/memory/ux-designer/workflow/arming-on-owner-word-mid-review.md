---
role: "ux-designer"
class: workflow
topic: "arming-on-owner-word-mid-review"
description: "when the owner says \"arm N\" while a PR still has an unreviewed fix or an unfolded LC pass — the gates fabric-pr arm applies and how to clear each"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f07f81ce5cca0e7f
---

## when the owner says "arm N" while a PR still has an unreviewed fix or an unfolded LC pass — the gates fabric-pr arm applies and how to clear each

The owner's "arm N" in session lifts only the count rule (under 8 work). Every other gate still applies, and on #683 (2026-11-16) each one cost a step:
- A session that has worked a job and has none active cannot commit, push or arm (ADR-037 rule 3 hook). Re-start the job (`fabric-jobs start`) once its blocker clears. A supplier's job (LC's j330) clears only when they mark it delivered.
- An adjudication (`fabric-review next --adjudicate`) needs a fix commit to judge at, and it is not head coverage. A re-review refuses to chain from a review whose coverage left paths "not read" (the merge of main). Render a full `mode: review` with range `<main merged in>..head`, which is the PR's true diff, and scope both files.
- Arm refuses on any open finding in the chain, P3s included: `--defer "<head8>:F<n>" "<why>"` each, and carry them in a PR comment and a follow-up job.
- Arm refuses while a BLOCKED job names the PR's branch: `--defer-job j<n> "<why>"`. After the merge, `fabric-jobs done` takes only an active or delivered job. A blocked one is closed with `drop`.
- `fabric-jobs switch` picks the queued arm job itself if it was added as a job: drop it once armed.

**Why:** each refusal came only after a try, and the arm took four extra rounds of tool calls.
**How to apply:** on "arm N", fold the pending LC pass, run one full review on the final head, then arm in one call with every --defer and --defer-job. Related: [[no-scope-widening-in-review-fixes]], [[lc-pass-before-arm]], [[review-rounds-mechanics]].

*References: lc-pass-before-arm, no-scope-widening-in-review-fixes, review-rounds-mechanics*

*Anchors: `agent-fabric:tools/fabric/github/arm.py::defer-job`*

*Observed 2026-11-16 (ux-designer)*

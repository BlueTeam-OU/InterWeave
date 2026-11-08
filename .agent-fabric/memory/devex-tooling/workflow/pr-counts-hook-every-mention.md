---
role: "devex-tooling"
class: workflow
topic: "pr-counts-hook-every-mention"
description: "the pr-counts stop hook wants \"(W work, F fix)\" on EVERY \"#N\" mention in a reply, not only the first"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f55db6c408d7ff57
---

## the pr-counts stop hook wants "(W work, F fix)" on EVERY "#N" mention in a reply, not only the first

agent-fabric's runtime/claude-code/hooks/pr-counts.py blocked two
replies on 2026-11-03 (j274, InterWeave #534) that carried the counts
on the first "#534" but repeated "#534" bare later ("goes onto #534").

**Why:** the hook checks each mention beside status prose, not the PR
once per reply. See [[pr-counts-hook-reads-alert-numbers]].
**How to apply:** write "#N (W work, F fix)" at every mention, or name
the PR once and refer to it as "it" afterwards.

*References: pr-counts-hook-reads-alert-numbers*

*Observed 2026-11-03 (devex-tooling)*

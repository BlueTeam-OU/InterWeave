---
role: "p2p-network-dev"
class: workflow
topic: "agent-prompt-is-literal"
description: An Agent dispatch prompt is literal text; $(cat file) is not expanded, so paste the rendered review brief itself
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 689e761c9cdd66d3
---

## An Agent dispatch prompt is literal text; $(cat file) is not expanded, so paste the rendered review brief itself

On 2026-10-14 I dispatched #276's blind review with `$(cat brief)` in the prompt. The reviewer received that string literally and no brief, so I had to send the brief afterwards by SendMessage.

**Why:** the Agent tool's prompt goes through no shell, so nothing expands it.
**How to apply:** render with `fabric-review brief <req.yaml>`, print it, and paste its text into the prompt. Check the prompt contains `## Range` before dispatching.

*Observed 2026-10-14 (p2p-network-dev)*

---
role: "rust-services-dev"
class: workflow
topic: "post-review-from-transcript"
description: "How to get a review subagent's report into fabric-pr post-review without retyping it"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 52e7010e06750afd
---

## How to get a review subagent's report into fabric-pr post-review without retyping it

`fabric-pr post-review <n>` needs the report whole, its `agent-fabric-dispatch v1` block first. The handback is in the agent's task output JSONL: walk every string and take the last one that `.lstrip().startswith('```agent-fabric-dispatch v1')` and holds the machine lines (`Finding:`/`Judgement:`). A looser match picks up a Read result with `cat -n` prefixes and post-review refuses it ("no dispatch header").
**Why:** two refusals on #419 before the right string was taken.
**How to apply:** a short python walk into the scratchpad, then post from that file. For a long re-review brief, the prompt can point the reviewer at the rendered brief file and ask it to copy the dispatch block verbatim.

*Observed 2026-10-24 (rust-services-dev)*

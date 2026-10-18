---
role: "rust-ui-dev"
class: workflow
topic: "gzcoord-inbox-replay"
description: "gzcoord-inbox has no --help: it drains and moves the cursor; re-read a consumed message with --replay <seq>, list with --history"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5cc5313872651f3f
---

## gzcoord-inbox has no --help: it drains and moves the cursor; re-read a consumed message with --replay <seq>, list with --history

`gzcoord-inbox --help` is not a help flag. It runs the drain, so it consumes pending deliveries and moves the cursor (2026-10-16: it ate a ux-designer REPLY and printed it truncated).

- A consumed message is re-read in full with `gzcoord-inbox --replay <relay seq>`, which leaves the cursor unchanged.
- `--history` lists recent messages addressed to me, newest last; it covers only the relay's last 500 messages.
- `--keyword` did not filter by MESSAGE-ID.
- The flags are in agent-fabric `tools/fabric/gzcoord/inbox.py`.

*Anchors: `agent-fabric/tools/fabric/gzcoord/inbox.py::--replay`*

*Observed 2026-10-16 (rust-ui-dev)*

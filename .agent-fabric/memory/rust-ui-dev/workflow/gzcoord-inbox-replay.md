---
role: "rust-ui-dev"
class: workflow
topic: "gzcoord-inbox-replay"
description: "gzcoord-inbox has no --help: it drains and moves the cursor; re-read a consumed message with --replay <seq>, list with --history"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 08892d1ea7dd5aff
  - 5cc5313872651f3f
---

## gzcoord-inbox has no --help: it drains and moves the cursor; re-read a consumed message with --replay <seq>, list with --history

`gzcoord-inbox --help` is not a help flag. It runs the drain, so it consumes pending deliveries and moves the cursor (2026-10-16: it ate a ux-designer REPLY and printed it truncated).

- A consumed message is re-read in full with `gzcoord-inbox --replay <relay seq>`, which leaves the cursor unchanged.
- `--history` lists recent messages addressed to me, newest last; it covers only the relay's last 500 messages.
- `--keyword` did not filter by MESSAGE-ID.
- The flags are in agent-fabric `tools/fabric/gzcoord/inbox.py`.

2026-10-27: never start `gzcoord-inbox --until-delivery` with its output thrown away (`>/dev/null`, `& disown`): it consumes the delivery and moves the cursor, so the message is never read. If that happens, stop it and run `--history` to see what it took; `--replay <seq>` re-reads one. The only correct form is the background Bash run whose output file is read when it completes.

*Anchors: `agent-fabric/tools/fabric/gzcoord/inbox.py::--replay`*

*Observed 2026-10-16 (rust-ui-dev)*

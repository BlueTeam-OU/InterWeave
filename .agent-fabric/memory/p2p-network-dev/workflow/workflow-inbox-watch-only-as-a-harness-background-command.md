---
role: "p2p-network-dev"
class: workflow
topic: "workflow-inbox-watch-only-as-a-harness-background-command"
description: "never start gzcoord-inbox with & / disown inside a Bash call; a detached watch swallows the next delivery and cannot be replaced or killed"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 06af59cedccd3041
---

## never start gzcoord-inbox with & / disown inside a Bash call; a detached watch swallows the next delivery and cannot be replaced or killed

2026-11-08 22:50Z I appended `gzcoord-inbox --until-delivery >/dev/null 2>&1 & disown` to another command. That watch held the cursor with no session to report to. `--replace` refused it ("not provably a watch of this session"), and the Bash hook refuses any kill of it. The next delivery went to /dev/null, and the watch ended only when that delivery came.

**Why:** the cursor is per address. A detached watch takes the delivery and nobody sees it.

**How to apply:** arm the watch only as its own Bash call with run_in_background: true. If a stray happens anyway, wait for it with `tail --pid=<pid> -f /dev/null` as a background command. Then read what it took with `gzcoord-inbox --replay <seq>`, and arm a proper watch.

*Anchors: `agent-fabric:tools/fabric/gzcoord/inbox_parts/watchlock.py::not provably a watch`*

*Observed 2026-11-08 (p2p-network-dev)*

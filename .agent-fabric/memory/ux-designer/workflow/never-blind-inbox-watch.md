---
role: "ux-designer"
class: workflow
topic: "never-blind-inbox-watch"
description: "Never start gzcoord-inbox with output to /dev/null or with & disown; it swallows the next delivery"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ccbd288ec7b87d7e
---

## Never start gzcoord-inbox with output to /dev/null or with & disown; it swallows the next delivery

Start the inbox watch only as a background Bash call with run_in_background and read its output file.
On 2026-10-23 and twice on 2026-10-24 I started `gzcoord-inbox --until-delivery >/dev/null & disown`
by reflex. A delivery it consumed would have been lost, so I had to find and kill it each time
(pgrep -f "bin/gzcoord-inbox"), and a `pkill -f` on the same pattern killed my own shell (exit 144).

**Why:** the cursor is per address, and a watch that exits after a delivery advances it.

**How to apply:** re-arm with the tool call only: Bash(command: 'gzcoord-inbox --until-delivery',
run_in_background: true, timeout: 7200000). Never pkill -f with the pattern in my own command line.

*Anchors: `agent-fabric:bin/gzcoord-inbox::--until-delivery`*

*Observed 2026-10-24 (ux-designer)*

---
role: "rust-ui-dev"
class: workflow
topic: "inbox-watch-never-inline"
description: "Never append gzcoord-inbox to another command with & or >/dev/null; it is only ever its own background Bash call"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e5519d3d82cedb4f
---

## Never append gzcoord-inbox to another command with & or >/dev/null; it is only ever its own background Bash call

`gzcoord-inbox --until-delivery` is started only as its own Bash call with `run_in_background: true`, so its output reaches the session. Twice on 2026-10-25 I appended it to the end of an unrelated command (`... & sleep 0;` and `... >/dev/null 2>&1 & disown`). Each time this started a hidden watcher whose output was discarded. A delivery it took would have been lost, because the cursor is per address and a consumed message is not delivered again.

**Why:** the watch is the session's only inbox. A second, silent consumer swallows deliveries, and nothing shows that it happened.

**How to apply:**
- Re-arm only with the dedicated background call, never inside a compound command.
- If a stray one was started, find it with `pgrep -u "$(id -u)" -fa 'fabric-python.*gzcoord-inbox'`.
- Kill it by PID, never by `pkill -f` (see [[pgrep-f-matches-itself]]).
- Then start the proper watch.

*References: pgrep-f-matches-itself*

*Anchors: `agent-fabric:bin/gzcoord-inbox::--until-delivery`*

*Observed 2026-10-25 (rust-ui-dev)*

---
role: "rust-ui-dev"
class: workflow
topic: "inbox-watch-never-inline"
description: "Never append gzcoord-inbox to another command with & or >/dev/null; it is only ever its own background Bash call"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e5519d3d82cedb4f
  - f929c1d92fca5181
---

## Never append gzcoord-inbox to another command with & or >/dev/null; it is only ever its own background Bash call

`gzcoord-inbox --until-delivery` is started only as its own Bash call with `run_in_background: true`, so its output reaches the session. On 2026-10-25 (twice) and again on 2026-11-05 I appended it to the end of an unrelated command (`... & sleep 0;` and `... >/dev/null 2>&1 & disown`). Each time this started a hidden watcher whose output was discarded. A delivery it took would have been lost, because the cursor is per address and a consumed message is not delivered again.

**Why:** the watch is the session's only inbox. A second, silent consumer swallows deliveries, and nothing shows that it happened.

**How to apply:**
- Re-arm only with the dedicated background call, never inside a compound command, and never as a "placeholder" call.
- If a stray one was started: it cannot be killed (a PreToolUse hook refuses), and `--replace` refuses a detached one
  (2026-11-05: "not provably a watch of this session"). Wait in the background for its PID to leave /proc. Then run
  `gzcoord-inbox --history <last seen seq + 1>`, `--replay` whatever it swallowed, and re-arm.
- 2026-11-05 17:02Z, a fourth time: I appended `gzcoord-inbox --until-delivery >/dev/null 2>&1 & disown` to a
  `git log` check (pid 966219). The trigger each time is wanting to re-arm in the same call as the next step. Re-arm
  as a separate call FIRST, before any other command in that turn.

*Anchors: `agent-fabric:bin/gzcoord-inbox::--until-delivery`*

*Observed 2026-10-25 (rust-ui-dev)*

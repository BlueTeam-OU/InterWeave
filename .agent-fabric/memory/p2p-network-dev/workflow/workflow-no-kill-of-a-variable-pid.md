---
role: "p2p-network-dev"
class: workflow
topic: "workflow-no-kill-of-a-variable-pid"
description: "never `kill $VAR` / `kill \"$(cat pidfile)\"` in a Bash call — the PreToolUse hook cannot see the value and asks the owner every time"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 59a7390664bd0c7f
---

## never `kill $VAR` / `kill "$(cat pidfile)"` in a Bash call — the PreToolUse hook cannot see the value and asks the owner every time

The PreToolUse Bash hook asks for confirmation on any `kill` whose pid is a variable or a command
substitution (it cannot tell it is not this session's own tree). On 2026-11-14 (j110) the owner
had to approve such a prompt repeatedly and complained ("every time you do this ...").

**Why:** each one interrupts the owner for a decision they cannot judge from the screen.
**How to apply:** bound background helpers with their own deadline (`timeout N cmd`, or a loop
on `$SECONDS`) so nothing needs killing; stop a daemon through its own verb
(`transportctl --profile P shutdown`); for a stuck harness task use TaskStop. Never `kill $X`,
never `pkill -f` (see [[pkill-f-kills-own-shell]]).

*References: pkill-f-kills-own-shell*

*Anchors: `agent-fabric:runtime/claude-code/hooks/::kill`*

*Observed 2026-11-14 (p2p-network-dev)*

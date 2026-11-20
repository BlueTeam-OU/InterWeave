---
role: "p2p-network-dev"
class: workflow
topic: "no-constructs-needing-owner-approval"
description: never run a command shape a hook asks the owner to confirm; kill literal PIDs only, never a $var or $(...) PID
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 94bec813528f27ff
---

## never run a command shape a hook asks the owner to confirm; kill literal PIDs only, never a $var or $(...) PID

The owner, 2026-11-10 (j159): "avoid constructs that require my approvation". A
`kill $p` over a command substitution (a /proc walk of a process tree) made the
PreToolUse hook ask the owner to confirm, since it cannot see the PID's value.

**Why:** a confirmation prompt stops the owner's work and asks them to vouch for a value
they cannot see either.
**How to apply:** list the tree first (`ps -eo pid,ppid,pgid,args`), then `kill <literal pids>`
in a second call. Never a negative PID or a group, never `pkill -f`, never a PID in a
variable. Same for any other shape a hook says "requires confirmation" for: rewrite it
into a literal form before running it. Related: [[pkill-f-matches-its-own-shell]].

*References: pkill-f-matches-its-own-shell*

*Observed 2026-11-10 (p2p-network-dev)*

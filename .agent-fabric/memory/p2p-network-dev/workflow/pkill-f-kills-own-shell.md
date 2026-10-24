---
role: "p2p-network-dev"
class: workflow
topic: "pkill-f-kills-own-shell"
description: "pkill -f \"<pattern>\" from a Bash tool call matches the tool's own shell command line and kills it (exit 144)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 61c4f47a13865362
---

## pkill -f "<pattern>" from a Bash tool call matches the tool's own shell command line and kills it (exit 144)

On 2026-10-14 `pkill -u $(id -u) -f "wait-merged 276"` killed the Bash tool's own shell (exit 144), because the pattern also appeared in that shell's command line. Twice.

**Why:** pkill -f matches against full command lines, including the wrapper shell that runs the command.
**How to apply:** stop a tracked background task with TaskStop, or kill by the PID that pgrep prints, after excluding your own shell. Never start a watcher with `& disown`; use run_in_background.

*Observed 2026-10-14 (p2p-network-dev)*

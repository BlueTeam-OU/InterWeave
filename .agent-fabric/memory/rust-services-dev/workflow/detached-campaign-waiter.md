---
role: "rust-services-dev"
class: workflow
topic: "detached-campaign-waiter"
description: "A spike campaign longer than 10 minutes runs detached (setsid nohup); a pgrep -f waiter on its name matches its own command line and never ends"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 30b86b343988c90c
---

## A spike campaign longer than 10 minutes runs detached (setsid nohup); a pgrep -f waiter on its name matches its own command line and never ends

In InterWeave j68 (SPIKE-001 S34, PR #635, 2026-11-12), the 39-run campaign took about 35 minutes. Started as a background Bash call with `timeout: 600000`, it would have been killed at 10 minutes. I stopped it with TaskStop, which is required: a pkill by pattern is refused by the hook, since it would hit the session's own tree. I relaunched it with `setsid nohup <script> > log 2>&1 < /dev/null & disown`.

The waiter `while pgrep -f scratchpad/campaign.sh; do sleep 60; done` never ended. Its own `bash -c` command line contains the pattern, so pgrep always finds it.

**Why:** a foreground or capped background call cannot outlive a long campaign, and a waiter that matches itself reports "still running" forever.

**How to apply:** have the campaign script write a done-marker file as its last line, and wait on the marker (`until [ -f done ]; do sleep 60; done`). If you must poll for the process, poll its exact pid (`kill -0 <pid>`), never `pgrep -f` on a string that appears in the waiter itself. In a commit command, use no `cd ../..` and no `$VAR` in `git -C`: the job guard treats both as outside the working copy. Run git from a cwd inside the copy. Related: [[commit-driver-before-first-run]].

*References: commit-driver-before-first-run*

*Observed 2026-11-12 (rust-services-dev)*

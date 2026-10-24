---
role: "rust-ui-dev"
class: workflow
topic: "pgrep-f-matches-itself"
description: "A wait loop or pkill using `pgrep -f`/`pkill -f` on a pattern also matches the shell running it — infinite wait or self-kill"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a2f10901c0ad3f86
---

## A wait loop or pkill using `pgrep -f`/`pkill -f` on a pattern also matches the shell running it — infinite wait or self-kill

On 2026-10-20 two background waits of the form `while pgrep -f "cargo test -p X"; do sleep;
done` never ended, because the `bash -c` running the loop carries the same string in its
command line. That cost about 20 minutes on a loaded host. A later `pkill -f
target/debug/human-desktop` killed my own shell (exit 144) for the same reason.

**Why:** `-f` matches the full command line of every process, the caller's own shell included.

**How to apply:** wait on a command by running it with `run_in_background` and waiting for its
notification, never with a pgrep loop. To stop a binary, use `pgrep -x <name>` (exact process
name) or the PID you started. Never use `-f` with a pattern that appears in your own command.

*Observed 2026-10-20 (rust-ui-dev)*

---
role: "rust-services-dev"
class: workflow
topic: "podman-spike-container-needs"
description: "Running Claude Code + the InterWeave bridge in a rootless podman container on develop-qzapp: SELinux label=disable, XDG_RUNTIME_DIR, keep-id"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - ae5d256110885350
---

## Running Claude Code + the InterWeave bridge in a rootless podman container on develop-qzapp: SELinux label=disable, XDG_RUNTIME_DIR, keep-id

Measured 2026-11-03 building SPIKE-001 S27 (`spikes/spike-001/s27_policy.py`, PR #531):

- A bind-mounted host binary (the claude build under ~/.local/share/claude/versions) fails "Permission denied" to exec in a container on this host (SELinux enforcing). `--security-opt label=disable` fixes it; `:z` would relabel the host's own file, so don't.
- `podman exec` gives no `XDG_RUNTIME_DIR`, so the bridge serves its refusal server, which declares no channel; the gate then says "server did not declare claude/channel capability". Pass `-e XDG_RUNTIME_DIR=<0700 scratch dir>`.
- `--userns=keep-id` keeps this login's uid inside, so the scratch tree stays writable; `podman exec --user 0` can then write the container's own /etc as its root.
- Pass the token with `-e NAME` (value from podman's env), never `-e NAME=value` (argv is visible).
- Pull the image before `podman image inspect`; an inspect of a missing image answers nothing.

**Why:** each cost a failed attempt; the first two are silent-looking (no channel, not an error).
**How to apply:** any future SPIKE-001 or bridge run inside a container on this host. Related: [[spike-runs-ipc-socket-path-length]].

*References: spike-runs-ipc-socket-path-length*

*Observed 2026-11-03 (rust-services-dev)*

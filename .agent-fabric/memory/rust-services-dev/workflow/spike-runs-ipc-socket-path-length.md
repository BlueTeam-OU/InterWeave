---
role: "rust-services-dev"
class: workflow
topic: "spike-runs-ipc-socket-path-length"
description: "SPIKE-001 runs with daemons fail \"the daemon never served\" when TMPDIR is the long session scratchpad — SUN_LEN"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 6406e3f7dd259984
---

## SPIKE-001 runs with daemons fail "the daemon never served" when TMPDIR is the long session scratchpad — SUN_LEN

SPIKE-001's two-daemon drivers (`s16_run.py`, `s22_org_channels.py`, `s24_print.py`) put each daemon's `XDG_RUNTIME_DIR` under the run's mkdtemp scratch. With `TMPDIR` set to the Claude Code session scratchpad (a path over 100 characters), the daemon refuses its IPC socket: "path must be shorter than SUN_LEN", and the driver reports "the daemon never served" (seen 2026-10-24, j63).

**Why:** a Unix socket path is limited to about 108 bytes.

**How to apply:** run those drivers with `TMPDIR=$XDG_RUNTIME_DIR/<short>` (the driver removes its scratch tree; remove the directory after, Claude Code leaves an empty `claude-<uid>` there) and keep `SPIKE_RAW_DIR` in the scratchpad. Not a target dir, so the no-tmpfs rule does not apply.

*Anchors: `spikes/spike-001/s24_print.py::free_port`, `spikes/spike-001/s16_run.py::class Home`*

*Observed 2026-10-24 (rust-services-dev)*

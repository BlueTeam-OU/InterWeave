---
role: "rust-services-dev"
class: workflow
topic: "spike-builds-scratch-home-install"
description: "SPIKE-001 runs on a named Claude Code build — install it into a scratch HOME and pass SPIKE_CLAUDE; the host's own claude follows the fleet pin"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - f8c3d03217427227
---

## SPIKE-001 runs on a named Claude Code build — install it into a scratch HOME and pass SPIKE_CLAUDE; the host's own claude follows the fleet pin

The login's own `claude` follows the fleet's pin (agent-fabric `runtime/claude-code/harness.json`): it was 2.1.286 for S24–S27 and 2.1.311 by 2026-11-04, so a request naming "the host build" can be stale. S29 (PR #555) installed each build with `HOME=<dir> DISABLE_AUTOUPDATER=1 claude install <version>` under `~/agent-fabric-tmp/claude-builds/<version>/` and named it with `SPIKE_CLAUDE` (`s29_print.py` refuses to run without it).

**Why:** the S29 comparison of fact 75 needed every build on one install location; S28 had compared builds in different HOMEs, which left the cause of fact 69 open.

**How to apply:** check `claude --version` against the request's build before a campaign; install the named build into a scratch HOME; say in the README which build the host now runs. Remove the builds when the job ends (see [[disk-sweep-what-holds-the-bytes]]).

*References: disk-sweep-what-holds-the-bytes*

*Anchors: `spikes/spike-001/s29_print.py::set SPIKE_CLAUDE`, `agent-fabric:runtime/claude-code/harness.json::"claude"`*

*Observed 2026-11-04 (rust-services-dev)*

---
role: "rust-services-dev"
class: workflow
topic: "spike-tmpdir-not-named-tmp"
description: "SPIKE-001 runs: never give TMPDIR a path segment named /tmp/ — extract.py's redaction eats part of the scratch path and the scrub then leaves the home path in evidence"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 2b89a74160d387ff
---

## SPIKE-001 runs: never give TMPDIR a path segment named /tmp/ — extract.py's redaction eats part of the scratch path and the scrub then leaves the home path in evidence

On S32 (2026-11-10), the TMPDIR was `<scratchpad>/tmp`. `extract.py`'s `redact` turns any `/tmp/...` it finds into `$SCRATCH`, so it rewrote only the tail of the scratch path. The driver's `scrub` then no longer matched the whole scratch path. The result was `$HOME/agent-fabric-tmp/.../scratchpad$SCRATCH` in evidence.md, which names the login's directories. `s32_compact.py`'s strict check refused it, and the first probe, made before that check existed, carried the leak in a compaction summary's transcript path.

Claude Code also shortens a long project-directory name (the dashed spelling of the working directory) with a suffix of its own, so a summary's "read the full transcript at" path does not match `dashed(scratch)`. `s32_compact.py`'s `unpath` replaces every dashed spelling that starts with the dashed home directory.

**Why:** the evidence is published, and it must name no local path.

**How to apply:** use a TMPDIR whose segments avoid `tmp`, for example `<scratchpad>/t`. Before committing, grep the evidence for the login name and the dashed home directory. Related: [[spike-facts-cite-committed-runs]].

*References: spike-facts-cite-committed-runs*

*Anchors: `spikes/spike-001/extract.py::def redact`, `spikes/spike-001/s32_compact.py::def unpath`*

*Observed 2026-11-10 (rust-services-dev)*

---
role: "p2p-network-dev"
class: workflow
topic: "device-run-artefacts-outlive-the-scratchpad"
description: "Before a device-run session ends, copy its APK, build script and build log out of the scratchpad, because a review question afterwards needs them"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 12f06c7296a5558c
---

## Before a device-run session ends, copy its APK, build script and build log out of the scratchpad, because a review question afterwards needs them

j152's test-signed APK, `build-release.sh` and `build-release.log` all lived in that session's scratchpad, and it was gone by j156 (#592 review, 2026-11-08). F2 asked which Skia check ran, and the only source left was the old session's transcript jsonl under ~/.claude/projects/<copy dir>/. The v2 verify at min-sdk 24 survived only because the session in between had run it before the scratch went. That session also left a handover memory naming a sibling worktree with the edit, and I read it only after pushing a weaker commit: read a job's handover memory FIRST.

**Why:** a record's claims are reviewed after the session that made them has ended, and its scratch goes with it.
**How to apply:** for a device run, keep the APK, its signing output and the build log in a persistent per-job directory until the record's PR lands. Write the record from the commands that actually ran, not from the plan (j152 wrote "as release.yml steps 2-4 build it" while its script called the class check without arguments). To recover what ran, grep the old transcript's Bash tool_use inputs and results.

*Anchors: `tools/ci/skia_prebuilt.sh::check <target-dir> [<profile>]`*

*Observed 2026-11-08 (p2p-network-dev)*

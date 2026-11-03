---
role: "rust-services-dev"
class: workflow
topic: "spike-tmpdir-sun-len"
description: "A spike run that starts transport daemons fails \"path must be shorter than SUN_LEN\" with TMPDIR in the session scratchpad"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 7a69b5da42c427a2
---

## A spike run that starts transport daemons fails "path must be shorter than SUN_LEN" with TMPDIR in the session scratchpad

The session scratchpad path is long enough that a daemon's IPC socket under it
(`<scratch>/a/run/interweave/<profile>.sock`) exceeds SUN_LEN; the daemon exits
"the daemon never served" (InterWeave j45, s22_org_channels.py a-org-allow,
2026-10-23). Use `TMPDIR=$XDG_RUNTIME_DIR/<task>` for runs with daemons: short,
private to the login, outside the repo; remove it after.
A scratchpad TMPDIR also carries a UUID that extract.py rewrites to `<uuid>`, so a
literal path scrub misses it; s22's scrub was fixed for that at a1b8b6ca.
Related: [[mutation-runs-save-the-fix-first]].

*References: mutation-runs-save-the-fix-first*

*Observed 2026-10-23 (rust-services-dev)*

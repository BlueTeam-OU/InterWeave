---
role: "rust-services-dev"
class: workflow
topic: "spike-tmpdir-sun-len"
description: TMPDIR for daemon runs — the scratchpad is too long for SUN_LEN, but $XDG_RUNTIME_DIR breaks the netns e2e test; set it per run, not for xtask ci
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 7a69b5da42c427a2
  - eaf438ac5935442c
---

## TMPDIR for daemon runs — the scratchpad is too long for SUN_LEN, but $XDG_RUNTIME_DIR breaks the netns e2e test; set it per run, not for xtask ci

The session scratchpad path is long enough that a daemon's IPC socket under it
(`<scratch>/a/run/interweave/<profile>.sock`) exceeds SUN_LEN; the daemon exits
"the daemon never served" (InterWeave j45, s22_org_channels.py a-org-allow,
2026-10-23). Use `TMPDIR=$XDG_RUNTIME_DIR/<task>` for runs with daemons: short,
private to the login, outside the repo; remove it after.
A scratchpad TMPDIR also carries a UUID that extract.py rewrites to `<uuid>`, so a
literal path scrub misses it; s22's scrub was fixed for that at a1b8b6ca.
Related: [[mutation-runs-save-the-fix-first]].

But not for the whole workspace suite: with `TMPDIR=$XDG_RUNTIME_DIR`,
desktop-e2e's `the_bridge_behind_its_own_nat_exchanges_with_the_facade_through_the_relay`
fails ("mount: /run/user/<uid>/iw-second-network-…: mount point does not exist",
exit 32), because its second network is a new mount namespace where that path is
absent. It passes with the default TMPDIR (j49, PR #431, 2026-10-25). Set the short
TMPDIR per run that needs it, never for `cargo xtask ci`.

*References: mutation-runs-save-the-fix-first*

*Anchors: `interweave:spikes/spike-001/s22_org_channels.py`, `interweave:tests/desktop-e2e/tests/claude_channel.rs::the_bridge_behind_its_own_nat_exchanges_with_the_facade_through_the_relay`*

*Observed 2026-11-03 (rust-services-dev)*

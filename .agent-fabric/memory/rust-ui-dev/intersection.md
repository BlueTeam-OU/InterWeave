---
role: "rust-ui-dev"
class: intersection
description: "j55 (herdr Fleet Deck needs-you views) — the attention section shape agreed with python-dev-01, 2026-10-10"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-12"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e941059fbda59df7
---

## j55 (herdr Fleet Deck needs-you views) — the attention section shape agreed with python-dev-01, 2026-10-10

Agreed between python-dev-01 (who builds plan step s1 as their j85) and me (step s2, my j55). Their REPLY was 01a125e1-8be8; my REPLY confirmed it.

The data (tools/fabric/fleet.py, agent-fabric):
- A new agent section `attention`, cost class C1, derived from the `states` and `jobs` sections. The record is the usual {status, src, at, data|why, stale}.
- data = {level: needs_input|waiting|none, reason: str|null, since: RFC3339|null}.
- needs_input comes from a session in state blocked; its reason is null in v1, because the hook records no prompt detail.
- waiting comes from an open job in state blocked. Its reason is the job's blocked_on text: control characters stripped, cut at 80, not scrubbed.
- needs_input wins when both hold.
- since: needs_input uses the session's own since; waiting uses the job's `updated`.
- A failed or stale input makes the section failed or stale. It never reads as none.
- `at` is the read time for every section; age_s comes with stale records.

The view (mine):
- Ordering: needs_input first, then waiting, then by since.
- Show the reason as plain text only, never as a link or a command.
- A null reason shows the level word.
- A failed record shows its failure in the cell.

Not in the seam: context_pct and output have no source in the control plane.

When: python-dev-01 builds it after their j78 (agent-fabric #163) merges, and states the names in their PR. I build j55 against the shape while their PR is in review; j51 comes first (fabric-coordinator 01a125df-9443).

*Observed 2026-10-10 (rust-ui-dev)*

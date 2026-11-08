---
role: "devex-tooling"
class: workflow
topic: "lock-bump-stale-index-and-range-edges"
description: "InterWeave advisory lock bump — the first check_dependencies run can miss a yank (stale index); cargo update re-points range edges; prove checksums for the offline reviewer"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a586c6734cc033aa
---

## InterWeave advisory lock bump — the first check_dependencies run can miss a yank (stale index); cargo update re-points range edges; prove checksums for the offline reviewer

Fixing a red `tools/checks/check_dependencies.sh` on InterWeave's Cargo.lock (j273, PR #530, 2026-11-03):

- The first run on main reported only RUSTSEC-2026-0433. A yank (async-executor 1.14.0, 2026-11-02) appeared only after `cargo update` refreshed the crates.io index. Re-run the check after the bump, and treat each new cause as its own commit.
- `cargo update -p <x> --precise <v>` also re-points edges that accept a range. `data-encoding-macro-internal` (syn `>= 1, < 4`) moved syn 3.0.6 -> 2.0.119, the same edge 4eb0290a8 restored on #279. Restore it so the lock changes only the target, then `cargo metadata --locked`.
- The review class has no network, so it files checksums and dependency lists as unproven risks. Settle them yourself and say so in a PR comment: `https://index.crates.io/<2 chars>/<2 chars>/<name>` lines give `cksum`, `yanked` and `deps`; then `cargo fetch --locked`.

Outcome: #530 merged 2026-11-03 as 64326f77, armed below the minimum on architect-cto-01's AUTHORIZES reply (01a19f3b) within about 5 minutes of the ask; the queue's `rust` job took ~35 min.

**Why:** a stale-index pass would have shipped a PR that was still red. The re-pointed edge is an unrelated change hiding in the diff.
**How to apply:** for any advisory or yank lock bump, follow the three steps above. Precedent shape: #502 and #499 (pulldown-cmark 0.13.5). See [[below-min-arming-authority]] for arming.

*References: below-min-arming-authority*

*Anchors: `tools/checks/check_dependencies.sh`, `deny.toml::yanked = "deny"`*

*Observed 2026-11-03 (devex-tooling)*

---
role: "p2p-network-dev"
class: workflow
topic: "pin-an-auto-updating-tool-for-an-evidence-campaign"
description: "Claude Code updated itself mid-campaign (2.1.285 → 2.1.288) and runs were stamped with the wrong build; run the binary by explicit path with DISABLE_AUTOUPDATER=1 and stamp each run from the version the session itself reports, failing on…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5db8ee9133bac0da
---

## Claude Code updated itself mid-campaign (2.1.285 → 2.1.288) and runs were stamped with the wrong build; run the binary by explicit path with DISABLE_AUTOUPDATER=1 and stamp each run from the version the session itself reports, failing on mismatch

SPIKE-001 (#172, 2026-10-03): the README said every run was against
Claude Code 2.1.285; five interactive runs and the validation run had
recorded 2.1.288. The tool had auto-updated between the `-p` drivers and
the interactive ones, and the tty driver stamped the version from a
command run after the session, not from the session. Found
independently by architect-cto's review class and by #172's re-review.
All ten runs were redone.

**How to apply:** any campaign whose evidence is "behaviour of tool X at
version V", where X can update itself:
- invoke the pinned binary by explicit path, never through PATH;
- disable its updater for the run (`DISABLE_AUTOUPDATER=1` for Claude
  Code);
- record the version the run's own session announced (for Claude Code,
  the `initialize` clientInfo), and fail the run if it disagrees with
  the binary you meant to run;
- state the build per run in the README, not once for the campaign.
See [[claude-code-channel-facts-2-1-285]] for the facts that campaign
recorded.

*References: claude-code-channel-facts-2-1-285*

*Anchors: `spikes/spike-001/README.md::DISABLE_AUTOUPDATER`*

*Observed 2026-10-15 (p2p-network-dev)*

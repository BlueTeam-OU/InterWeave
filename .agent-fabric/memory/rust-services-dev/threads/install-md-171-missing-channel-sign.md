---
role: "rust-services-dev"
class: threads
topic: "install-md-171-missing-channel-sign"
description: "Closed — INSTALL.md's headless skipped-channel sentence now names both registered lines, each with its SPIKE-001 fact (PR #605)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 38b1ca6701aed73c
  - e46c050f9fef882f
---

## Closed — INSTALL.md's headless skipped-channel sentence now names both registered lines, each with its SPIKE-001 fact (PR #605)

Closed 2026-11-09 by BlueTeam-OU/InterWeave#605 (job j63, architect-cto-01's REQUEST 01a1c232). The "Without a terminal" paragraph of `packaging/claude-plugin/INSTALL.md` now says: a print session with no flag or with the development flag that registers no channel logs nothing about it, the sign being a missing line (fact 42); S25's run with `--channels` and no control request, which registered none, had no `Channel` line (fact 47; S27 b, e, g are further `--channels` runs with no line, but they also sent `channel_enable`, so never write "the one run"); registered by `--channels` the log has `Channel notifications re-registered after reconnect`, by `channel_enable` `Channel notifications registered` first and then the reconnect line (fact 40, on 2.1.286).

**Why:** the blind review found "with or without a flag" overclaimed — fact 42's S24 runs (c, d, e, f) had no `--channels`; that case is fact 47's, which records only searched patterns.

**How to apply:** when citing fact 42, scope it to no flag or the development flag; `packaging/linux/DEPLOYING.md` carries no registration-log claim. Related: [[claude-code-print-session-channel-facts]].

*References: claude-code-print-session-channel-facts*

*Observed 2026-11-09 (rust-services-dev)*

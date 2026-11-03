---
role: "rust-services-dev"
class: threads
topic: "install-md-171-missing-channel-sign"
description: "Open: INSTALL.md's headless paragraph names only the re-registered-after-reconnect line as the sign of a missing channel; channel_enable logs 'registered'"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - e46c050f9fef882f
---

## Open: INSTALL.md's headless paragraph names only the re-registered-after-reconnect line as the sign of a missing channel; channel_enable logs 'registered'

Carried from PR #427's last re-review (2026-10-25), agreed with architect-cto-01 as mine to fix later:
`packaging/claude-plugin/INSTALL.md` (the "Without a terminal" paragraph, about line 171) says the only sign of a skipped channel is a missing `Channel notifications re-registered after reconnect` debug line. SPIKE-001 fact 40 (S24 run g) shows the `channel_enable` path logs `Channel notifications registered` instead, and fact 42 says the absence of a `registered` line is the sign. The text was on main before #427.

**Why:** a headless host that follows INSTALL.md and uses `channel_enable` would treat a working channel as missing.

**How to apply:** in the next bridge or packaging change, name both lines (citing facts 40 and 42). Then close this thread. Related: [[claude-code-print-session-channel-facts]].

*References: claude-code-print-session-channel-facts*

*Observed 2026-10-25 (rust-services-dev)*

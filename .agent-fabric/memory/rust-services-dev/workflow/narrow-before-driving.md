---
role: "rust-services-dev"
class: workflow
topic: "narrow-before-driving"
description: "Before building a run driver or asking for an allow rule, check what a CLI-only path can measure"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 843249b690bd2c61
---

## Before building a run driver or asking for an allow rule, check what a CLI-only path can measure

On InterWeave j41 (2026-10-22) I spent a request cycle asking fabric-coordinator for an auto-mode allow entry to
drive nested claude sessions. Most of what the gaps needed was then measured without any session:
`claude mcp get`, a logging wrapper in the plugin's bin, a scratch CLAUDE_CONFIG_DIR, and the MCP log files.
The owner's clarification then dropped the session run altogether.

**Why:** a session driver needs an allow rule, a fleet upgrade and a relaunch, which takes days. CLI-only
measurement needs none of that, and it is reproducible by anyone.

**How to apply:** before asking for an allow rule for a session driver, list what each claim needs. Measure every
one a CLI path can reach first, and ask only for what remains. Related: [[classifier-session-driver]],
[[claude-plugin-runtime-facts]].

*References: classifier-session-driver, claude-plugin-runtime-facts*

*Observed 2026-10-22 (rust-services-dev)*

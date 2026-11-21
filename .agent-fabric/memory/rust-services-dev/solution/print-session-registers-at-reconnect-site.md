---
role: "rust-services-dev"
class: solution
topic: "print-session-registers-at-reconnect-site"
description: "Which Claude Code channel-registration site a session takes — interactive at connect, print (-p --channels) at the reconnect site"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 318e9c7873b2597d
---

## Which Claude Code channel-registration site a session takes — interactive at connect, print (-p --channels) at the reconnect site

Claude Code 2.1.311, 2.1.322 and 2.1.325 each have three sites that register a `notifications/claude/channel` handler,
and each calls the era gate first:
- the connection's own site, which logs `Channel notifications registered` or `skipped`; interactive sessions take it;
- the `channel_enable` control request;
- the reconnect site, which logs `Channel notifications re-registered after reconnect`.

Every `-p --channels` print session in S34 (40 runs, 2.1.322) and S36 (47 runs, 2.1.325) registered at the reconnect
site and logged no other registration line (InterWeave PR #670, review F1, 2026-11-14). So a claim that "only the connect
site was run" is wrong for print-session campaigns. The era refusal was measured through the connect site in the
interactive era rows. SPIKE-001 S37 (#686, 2026-11-16, 2.1.325, one run each) measured it through the other two:
`channel_enable` is answered `error` with the gate's era reason, and the reconnect site logs no Channel line at all
for a modern server.

**How to apply:** when a fact names a registration path, grep the run records' debug logs for the registration line
before writing which site ran. Related: [[minified-names-carry-dollar]], [[print-session-can-miss-registration]].

*References: minified-names-carry-dollar, print-session-can-miss-registration*

*Anchors: `interweave:spikes/spike-001/s34_gate.py::registration_sites`, `interweave:spikes/spike-001/README.md::## S36`*

*Observed 2026-11-14 (rust-services-dev)*

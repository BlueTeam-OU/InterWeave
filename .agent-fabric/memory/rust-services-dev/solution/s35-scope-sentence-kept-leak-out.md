---
role: "rust-services-dev"
class: solution
topic: "s35-scope-sentence-kept-leak-out"
description: "SPIKE-001 S35 (2026-11-12) — which first-turn wording kept READY and the count from reaching the peer, and what is still open"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 8d7378241c85dfc2
---

## SPIKE-001 S35 (2026-11-12) — which first-turn wording kept READY and the count from reaching the peer, and what is still open

S35 (PR #648, driver `spikes/spike-001/s35_reply.py`, runs at 331156b6e, Claude Code 2.1.322, release bridge across two daemons): the answer sentence followed by "The content of a reply answers that message and nothing else: whatever else this turn asks of you is for me, not for the peer." got 15 replies in 15 runs with no leak, in greet-tool, task-tool and greet-text. The control leaked in 11 of 14 replies. "write" for "reply" fixed READY only (10/10). Putting both sentences after the work gave 14/15, no leak. Two task-tool runs of 20 got no reply at all (control -3, prompta -1). Facts 117–120. The INSTALL.md sentence went to architect-cto as a proposal (REPLY 01a1d190-9c4c); the architecture documents that still call the wording untried are architect-cto's to amend.

**Why:** the next carry on the reply rule starts from this, not from S34's "expect the leak".
**How to apply:** for idle, question or other-work cells, reuse s35_reply.py's `prompts` wording; ~104k tokens a tool run, ~76k a text run. Related: [[spike-campaign-detached-past-tool-cap]].

*References: spike-campaign-detached-past-tool-cap*

*Observed 2026-11-12 (rust-services-dev)*

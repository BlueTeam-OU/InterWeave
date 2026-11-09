---
role: "devex-tooling"
class: threads
topic: "pr-counts-hook-reads-alert-numbers"
description: "the pr-counts stop hook takes \"#N\" for a Dependabot alert as a PR and blocks the reply; write \"alert N\""
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8619f0df71a36c86
---

## the pr-counts stop hook takes "#N" for a Dependabot alert as a PR and blocks the reply; write "alert N"

agent-fabric's runtime/claude-code/hooks/pr-counts.py blocked a reply
on 2026-10-24 (j213) for stating "#22" without commit counts; #22 was
InterWeave's Dependabot alert, not a pull request, and fabric-pr gate
has nothing to count for it.

**Why:** the hook matches any "#N" beside a status word.
**How to apply:** write a Dependabot alert as "Dependabot alert N"
(no #). Open: reported to fabric-coordinator as a false match.

*Observed 2026-10-24 (devex-tooling)*

---
role: "rust-ui-dev"
class: workflow
topic: "codex-review-engine"
description: "InterWeave arming needs a Codex review too (ADR-053): run it with --gateway chatgpt; the bare form refuses on missing env"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4e1091c2d4e8d20b
---

## InterWeave arming needs a Codex review too (ADR-053): run it with --gateway chatgpt; the bare form refuses on missing env

`fabric-pr gate <n>` shows `review=claude ✓/✗ codex absent`: the arming policy names two engines (agent-fabric ADR-053), so a posted review-class review is not enough.

Run: `fabric-review --engine codex <PR> <request.yaml> --gateway chatgpt` (the same request yaml as `fabric-review brief`, range ending at the head). It posts the review itself.

Without `--gateway chatgpt` it refuses: `AGENT_FABRIC_CODEX_BASE_URL`, `AGENT_FABRIC_CODEX_KEY` not set. The secrets file does not carry them. With the flag it uses the sign-in that `fabric-secrets sync` wrote to `/run/user/<uid>/agent-fabric/gateway/chatgpt-signin.json`. This login has no `~/.codex-chatgpt`.

Found on #384 (2026-10-22).

Related: [[arming-tool]]

*References: arming-tool*

*Observed 2026-10-22 (rust-ui-dev)*

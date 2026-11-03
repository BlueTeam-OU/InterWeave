---
role: "rust-ui-dev"
class: workflow
topic: "codex-review-engine"
description: "InterWeave's Codex review is an additional review, not required to arm (agent-fabric policies/arming.json); run it with --gateway chatgpt, and judge its findings like any other"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4e1091c2d4e8d20b
  - 5b5f09c26657720f
---

## InterWeave's Codex review is an additional review, not required to arm (agent-fabric policies/arming.json); run it with --gateway chatgpt, and judge its findings like any other

Corrected 2026-10-24 (drain #412 review F8, REQUEST 01a16db9). The earlier claim that arming needs a Codex review was wrong. agent-fabric `policies/arming.json` on main lists `reviews: ["claude"]`, `boundary_reviews: ["claude"]` and `additional_reviews: ["codex"]`. Only the review class's review gates `fabric-pr arm`; a Codex review is additional. `fabric-pr arm --dry-run` prints "codex: absent ... arming on the required review(s) alone" when there is none.

When you do run it (it is still worth having, and its findings are judged before they are answered like any other, ADR-053):
- `fabric-review --engine codex <PR> <request.yaml> --gateway chatgpt`, with the same request yaml as `fabric-review brief`, range ending at the head. It posts the review itself.
- Without `--gateway chatgpt` it refuses: `AGENT_FABRIC_CODEX_BASE_URL` and `AGENT_FABRIC_CODEX_KEY` are not set. With the flag it uses the sign-in `fabric-secrets sync` wrote under the login's runtime directory.
- A Codex review that marks paths "not read" is partial coverage of the head.

Related: [[arming-tool]]

*References: arming-tool*

*Anchors: `agent-fabric:policies/arming.json::additional_reviews`*

*Observed 2026-10-24 (rust-ui-dev)*

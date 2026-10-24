---
role: "architect-cto"
class: rationale
topic: "reflection-why-a-root-test-suite-partitions-open-prs-by-file"
description: "The rule that two open PRs never change one crate exists so hunks never meet; in a root test suite each tests/*.rs is its own binary, so the suite partitions by file while the harness and its Cargo.toml are untouched, and library and…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0d85ba12dab23030
---

## The rule that two open PRs never change one crate exists so hunks never meet; in a root test suite each tests/*.rs is its own binary, so the suite partitions by file while the harness and its Cargo.toml are untouched, and library and application crates keep the per-crate rule

InterWeave DECISION 01a1631f-68fa (2026-10-22): "the file partition is
the rule… not for a library or application crate"; INFO 01a1631f-7241
asked for the remit sentence. .agent-fabric/roles/rust-services-dev.md
still says "never change the same crate" (2026-10-23); drop this entry
once the remit carries the file rule.

*Observed 2026-10-23 (architect-cto)*

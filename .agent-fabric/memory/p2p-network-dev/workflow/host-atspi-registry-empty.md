---
role: "p2p-network-dev"
class: workflow
topic: "host-atspi-registry-empty"
description: "on develop-qzapp, 5 desktop-e2e human_app cases fail because the AT-SPI registry lists nothing; they fail on main too, so a red there is the host, not the change"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 99235b7b175eebf7
---

## on develop-qzapp, 5 desktop-e2e human_app cases fail because the AT-SPI registry lists nothing; they fail on main too, so a red there is the host, not the change

- **What fails.** Five cases in `tests/desktop-e2e` `human_app` fail on develop-qzapp under p2p-network-dev-02: `accessibility::the_tree_read_over_atspi_...`, the three `reading::` restart cases and `trust::a_trust_removal_...`. Each panics at `human_app/a11y.rs:91`: "the client's window never reached the AT-SPI registry, which lists []".
- **Not a regression.** Measured 2026-10-15 for j34. The same five fail on main's crates (d49c4b5f) with the same message. Every other workspace test binary passed (243).
- **How to run the suite here.** Use `cargo test --workspace --no-fail-fast`. Without it, `cargo test` stops at human_app and the binaries after it never run.
- **When reporting.** Name the five and the main control. Never filter them out silently.

*Anchors: `tests/desktop-e2e/tests/human_app/a11y.rs::never reached the AT-SPI registry`*

*Observed 2026-10-15 (p2p-network-dev)*

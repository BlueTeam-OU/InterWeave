---
role: "p2p-network-dev"
class: workflow
topic: "host-atspi-registry-empty"
description: "on develop-qzapp, desktop-e2e human_app cases that read the a11y tree fail because the AT-SPI registry lists nothing (5 on 2026-10-15, 12 on 2026-10-26, 14 on 2026-10-28); a red there is the host, not the change"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 286345af97e121f1
  - 99235b7b175eebf7
---

## on develop-qzapp, desktop-e2e human_app cases that read the a11y tree fail because the AT-SPI registry lists nothing (5 on 2026-10-15, 12 on 2026-10-26, 14 on 2026-10-28); a red there is the host, not the change

- **What fails.** Cases in `tests/desktop-e2e` `human_app` that read the window over AT-SPI fail on develop-qzapp under p2p-network-dev-02. Each panics at `human_app/a11y.rs` (line 92 on 2026-10-26) with "the client's window never reached the AT-SPI registry, which lists []".
- **The count grows with the suite.** On 2026-10-15 (j34), 5 cases failed: `accessibility::`, three `reading::` and one `trust::`. They failed on main d49c4b5f too. On 2026-10-26 (j75, base 2025672d), 12 failed: the same five plus `codes::the_shown_code_...`, three `refusal::` and three `release::`. All carried the same message. On 2026-10-28 (j88, base 3fe09e2b), 14 failed: the twelve plus `codes::a_code_image_pasted_...` and `codes::a_hostile_clipboard_owner_...`, all with the same message. So count by the message, not by a remembered list.
- **How to run the suite here.** Use `--no-fail-fast`. Without it, `cargo test` stops at human_app and the binaries after it never run.
- **When reporting.** Say that every failure carries the AT-SPI message, and say whether the change touches human_app's files. Never filter them out silently.

*Anchors: `tests/desktop-e2e/tests/human_app/a11y.rs::never reached the AT-SPI registry`*

*Observed 2026-10-26 (p2p-network-dev)*

---
role: "rust-ui-dev"
class: workflow
topic: "view-change-runs-desktop-e2e"
description: "A ui-slint change that moves, renames or hides a control also runs tests/desktop-e2e's human_app suite before the commit, not only ui-slint's own tests"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - fa135086eeda45a8
---

## A ui-slint change that moves, renames or hides a control also runs tests/desktop-e2e's human_app suite before the commit, not only ui-slint's own tests

On 2026-10-19 the row menu (#304 F2, 80fe2281) moved Show source and
Keep into a menu. ui-slint's tests passed, and so did clippy. But
tests/desktop-e2e's human_app suite reads the same tree over AT-SPI in
the real desktop binary: accessibility.rs expected Show source at
rest, and reading.rs pressed the row's Keep. Both broke, and I found it
only later, while changing the titles (F6).

**How to apply:** after any change to what a view exposes (a control's
label, role or presence, a title, a focus stop), before committing:
1. `cargo build -p interweave-human-desktop -p interweave-transport-daemon -p interweave-transportctl`.
   The suite runs those binaries and stops with "... is missing" otherwise.
2. `cargo test -p interweave-desktop-e2e-tests --test human_app -- --test-threads 1`.
   It takes about 10 minutes.
`cargo xtask ci` covers it too, but it is longer. The same holds for
tests/android-e2e's human_chat cases on a device run.

*Anchors: `tests/desktop-e2e/tests/human_app/accessibility.rs::the_tree_read_over_atspi_labels_message_route_and_connectivity_controls`*

*Observed 2026-10-19 (rust-ui-dev)*

---
role: "rust-ui-dev"
class: workflow
topic: "desktop-e2e-needs-with-display"
description: "desktop-e2e human_app fails \"never reached the AT-SPI registry, which lists []\" outside tools/ci/with_display.sh"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 49f65016b26dfac9
---

## desktop-e2e human_app fails "never reached the AT-SPI registry, which lists []" outside tools/ci/with_display.sh

`cargo test --workspace` run in a plain session fails every AT-SPI case of tests/desktop-e2e/tests/human_app
(a11y.rs:92, "never reached the AT-SPI registry, which lists []"), although at-spi2-registryd runs for the login:
accessibility is not switched on there. Under `tools/ci/with_display.sh cargo test -p interweave-desktop-e2e-tests
--test human_app` all 33 pass (2026-10-28, j68 B2b). **Why:** CI runs that binary inside the wrapper's private display
and bus. **How to apply:** run the workspace suite, then that binary under the wrapper; never report those 14 as a
change's failures.

*Observed 2026-10-28 (rust-ui-dev)*

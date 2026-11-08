---
role: "devex-tooling"
class: workflow
topic: "xtask-ci-needs-cargo-machete"
description: "cargo xtask ci exits red on check_unused_dependencies + its self-test when cargo-machete is absent (exit 2 by design); fabric-tools --install cargo-machete"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f39e4a5dbf130721
---

## cargo xtask ci exits red on check_unused_dependencies + its self-test when cargo-machete is absent (exit 2 by design); fabric-tools --install cargo-machete

On an account without cargo-machete, `cargo xtask ci` reports two failed tasks,
`check_unused_dependencies.sh` and `test_check_unused_dependencies.sh`, both exit 2
("cargo-machete is not installed"), which is the guard refusing to pass unevaluated, not a
defect in the change. `fabric-tools --install cargo-machete` installs CI's pin (0.9.2, the
registry's optional row for devex-tooling) into ~/.local/bin; then run just those two
instead of the whole suite again. Seen on j214 / InterWeave #422, 2026-10-24.

Also: a full `cargo xtask ci` takes more than 30 minutes here, so run it in the background
with the 2-hour timeout, not the default 30.

**Why:** a 30-minute run was lost to the background limit, and a second one went red only on
the missing tool.

**How to apply:** before a full suite run on a fresh account, check `command -v cargo-machete`.

*Observed 2026-10-24 (devex-tooling)*

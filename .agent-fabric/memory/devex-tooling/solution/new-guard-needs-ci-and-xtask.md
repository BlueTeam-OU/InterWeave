---
role: "devex-tooling"
class: solution
topic: "new-guard-needs-ci-and-xtask"
description: wiring a new tools/checks guard? it needs a ci.yml step AND a Task in xtask tree_checks(), or the rust context goes red
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cc396c567374b34b
---

## wiring a new tools/checks guard? it needs a ci.yml step AND a Task in xtask tree_checks(), or the rust context goes red

A new `tools/checks/check_*.{sh,py}` in InterWeave is wired in two places,
and each place has its own enforcement:
- a ci.yml step that names it, enforced by `check_guards_are_wired.sh` in
  tree checks;
- a `Task::new` in `xtask/src/main.rs` `tree_checks()`, enforced by xtask's
  unit test `every_tree_check_is_run`, which runs in the `rust` job's
  `--workspace` nextest run, NOT in tree checks.
A supplied ci.yml step alone leaves `rust` red. Found by the supplier review on
for/ branch develop-qzapp/devex-tooling/for/rust-ui-dev-02/android-contacts-backup-step
(a7dd8d11, 2026-10-22).

**Why:** the guards-wired check passing reads as "wired", but it
says nothing about the xtask mirror.
**How to apply:** when asked to wire a guard, run
`cargo test --manifest-path xtask/Cargo.toml every_tree_check_is_run`
as well as the wiring check, and supply both. See [[ci-command-line-is-read-by-guards]].

*References: ci-command-line-is-read-by-guards*

*Observed 2026-10-22 (devex-tooling)*

---
role: "devex-tooling"
class: workflow
topic: "scripted-edit-assert-match"
description: "a python str.replace edit that matches nothing changes nothing, silently; assert the old text first, and re-match after rustfmt"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - deef2fb3d7fffeff
---

## a python str.replace edit that matches nothing changes nothing, silently; assert the old text first, and re-match after rustfmt

A `python3` heredoc edit that does `s.replace(old, new)` without
`assert old in s` is a silent no-op when `old` is absent. On #461
(2026-10-26) `cargo fmt` had already rewrapped an `assert!(...)` onto
several lines, so the replacement that added `.expect()`s to an xtask
test matched nothing. The commit shipped without the fix, and the blind
review found it (round 1 F1). My mutation check missed it too: it removed
the OTHER step, not the one whose absence the broken comparison let
through.

**Why:** the edit reports nothing, so the session believes the change
landed.
**How to apply:** every scripted replacement asserts its old text is
present, or use the Edit tool. After `cargo fmt`, re-read the region
before editing it again. Mutate each input the test claims to guard, not
one of them. Related: [[mutation-runner-verify-backup]].

*References: mutation-runner-verify-backup*

*Anchors: `xtask/src/main.rs::own_features_clippy_is_ci_s_step_after_clippy`*

*Observed 2026-10-26 (devex-tooling)*

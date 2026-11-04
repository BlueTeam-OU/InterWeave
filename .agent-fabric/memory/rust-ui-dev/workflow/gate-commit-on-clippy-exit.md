---
role: "rust-ui-dev"
class: workflow
topic: "gate-commit-on-clippy-exit"
description: "never chain `cargo clippy ... | grep ...; git commit` -- the pipe hides clippy's exit and the commit lands red"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - de9b85ec7a349821
---

## never chain `cargo clippy ... | grep ...; git commit` -- the pipe hides clippy's exit and the commit lands red

2026-10-27 (j68 start window, 8a78f59e): `cargo clippy ... | grep -E "^(error|warning)"; cargo test ...; git commit
...; git push` committed AND pushed a commit clippy refused (similar_names). The grep pipeline's status is grep's,
and `;` runs the next step regardless; the follow-up fix c0cec9c3 then had to count as a work commit.

**Why:** a red commit on a pushed branch costs a fix commit that inflates the PR's work count and a re-read for
the reviewer.

**How to apply:** run clippy and the tests as their own call, read the result, and only then commit in a
separate call -- or chain with `&&` on commands whose exit status is the tool's own (no pipe before the `&&`).

*Observed 2026-10-27 (rust-ui-dev)*

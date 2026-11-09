---
role: "rust-ui-dev"
class: workflow
topic: "conflict-ends-and-chain"
description: "A git revert/merge that conflicts exits non-zero and silently skips every later step of a && chain; diff the staged commit against its message before committing"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c6b9ec894307177e
---

## A git revert/merge that conflicts exits non-zero and silently skips every later step of a && chain; diff the staged commit against its message before committing

On #492 (2026-10-27) `git revert --no-commit bcc38aed && ... && python3 <edit>` hit a conflict, so the edit never ran.
6717c2fd's message and a PR comment then claimed the widened test guard it would have made; the re-review found it (F5).
The mutation I ran looked caught only because a different assertion failed.

**Why:** a conflict is an exit code, and `&&` turns it into silent omission. A test that fails for another reason makes
the gap look covered.

**How to apply:** never chain an edit after a revert, merge or rebase. Run it as its own call once the conflict is
resolved. Before committing, read `git diff --cached` against each claim in the message. A mutation is caught only if it
fails on the assertion the claim names: read the panic text.

*Anchors: `crates/human/store/src/store/window.rs::the_counts_read_a_covering_index_alone`*

*Observed 2026-10-27 (rust-ui-dev)*

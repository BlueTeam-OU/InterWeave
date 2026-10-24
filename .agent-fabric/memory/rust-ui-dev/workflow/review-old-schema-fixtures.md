---
role: "rust-ui-dev"
class: workflow
topic: "review-old-schema-fixtures"
description: "A store rewrite/migration path must be tested from an OLDER-schema store (test-data/upgrade tree), not only one HumanStore::open made at the current version"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 984766f716916607
---

## A store rewrite/migration path must be tested from an OLDER-schema store (test-data/upgrade tree), not only one HumanStore::open made at the current version

On #377 (2026-10-22) every crash-at-each-step test seeded its store with HumanStore::open, so always at the
current schema; the blind review found a P1 the tests could not: a pre-v9 store's stalled copy failed
verify_shape, stranding the store and letting the discard delete the complete copy.

**Why:** a check written against "this build's shape" silently assumes every input was written by this build.
**How to apply:** for anything that reads a store another build wrote (rewrite, copy check, migration, discard),
seed one test from test-data/upgrade/<commit>/.../human.sqlite and one from an artefact an earlier build of the
new code would leave; judge a copy by "opens and is migratable", not by the current shape.

*Observed 2026-10-22 (rust-ui-dev)*

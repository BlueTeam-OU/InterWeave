---
role: "rust-ui-dev"
class: workflow
topic: "cli-verb-routing-in-main"
description: "interweave-human verbs routed in main.rs before verbs::run need a binary-level test of each arm, including the endpoint that falls through"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f1b5557c623fca49
---

## interweave-human verbs routed in main.rs before verbs::run need a binary-level test of each arm, including the endpoint that falls through

In apps/human-cli, a verb that main.rs routes before the session (admin verbs, file-only verbs,
`identity show <endpoint>`) can have an argument value that falls through to `verbs::run`, which
answers "this verb does not run through the session". PR #429 shipped `identity show human` (the
identity in front, named) that way, caught only by the review's judge. A unit test of the inner
function does not reach main's guard.

**Why:** the routing lives in main, which no unit test runs; only the binary does.

**How to apply:** for every main-level arm, add a test that runs the binary (desktop-e2e can write
a profile with no daemon, `Home::new` + `write_config`) with each value class the guard splits on,
and mutate the guard's comparison. Also read a verb's input (stdin, file) before opening any
socket, so this client's own refusals are exit 1 with or without a daemon.

*Anchors: `apps/human-cli/src/main.rs::IdentityShow(Some(endpoint))`*

*Observed 2026-10-24 (rust-ui-dev)*

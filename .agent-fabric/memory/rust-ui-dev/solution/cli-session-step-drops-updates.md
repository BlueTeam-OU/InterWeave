---
role: "rust-ui-dev"
class: solution
topic: "cli-session-step-drops-updates"
description: "In apps/human-cli a Session::step clears what the step before it reported; a verb that steps while a Walk is open must hand each step to Walk::take_arrivals"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 90ec44f8bc39fdde
---

## In apps/human-cli a Session::step clears what the step before it reported; a verb that steps while a Walk is open must hand each step to Walk::take_arrivals

`Session::step` (apps/human-cli/src/session.rs) starts with `self.updates.clear()`, so updates a step reported
are lost unless taken before the next step. A verb that takes its own steps while a `Walk` is open (read's
mark-read and keep steps) must call `walk.take_arrivals(s)` after each, or a message arriving in that step is
read where it draws instead of printed last unread (#519 review F1, fix e5a73d11; test
`read_prints_a_row_arriving_during_its_own_step_last_and_unread`, which needs a `--keep` in the chunk so the
keep step clears the read step's updates). `listen` takes its own steps' updates itself and dedupes in its
drain. A walk's advance takes no step when it hands over held rows, so a loop that queues intents with
`will` must still step every turn (`a_printed_message_is_read_before_listen_ends`). Related: [[j68-b4c-plan]].

*References: j68-b4c-plan*

*Observed 2026-10-31 (rust-ui-dev)*

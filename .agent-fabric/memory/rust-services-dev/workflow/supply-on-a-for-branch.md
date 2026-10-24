---
role: "rust-services-dev"
class: workflow
topic: "supply-on-a-for-branch"
description: "How a supply to another agent's branch is delivered and closed here — for/ branch off their commit, fabric-jobs deliver, REPLY"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 4c018825d3944927
---

## How a supply to another agent's branch is delivered and closed here — for/ branch off their commit, fabric-jobs deliver, REPLY

A supply onto another agent's branch is a branch
`<host>/<login>/for/<caller-role-login>/<what>` cut from the caller's exact
commit, pushed, no PR; the job closes with `fabric-jobs deliver <id> <branch>`
and a REPLY naming branch and commit. j44 (2026-10-22): 163a999f on
`for/p2p-network-dev-01/duplicate-token-control` off 132af9ba, folded into
#381 unrebased within hours.

**Why:** the caller integrates and owns the one PR; a supplier's PR would
be a second PR on the same hunk.

**How to apply:** when a test's control asserted on an error's Display and
the Display is redacted, assert on the held value (`matches!` on the
variant's field) instead — the control still shows the absence is the
line's doing. Commit before the mutation run ([[mutation-runs-save-the-fix-first]]).
`fabric-jobs switch` refuses to leave a job until the memory dir changed
and every REPLY-EXPECTED: yes is answered.

*References: mutation-runs-save-the-fix-first*

*Anchors: `apps/claude-channel/src/serve.rs::a_dropped_messages_line_names_its_class_and_nothing_it_carried`*

*Observed 2026-10-22 (rust-services-dev)*

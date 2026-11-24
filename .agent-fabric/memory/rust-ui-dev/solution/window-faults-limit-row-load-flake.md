---
role: "rust-ui-dev"
class: solution
topic: "window-faults-limit-row-load-flake"
description: "window_faults' limit-row test times out \"waiting for B open\" under the workspace suite's load; passes alone and as its binary"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 676cb921361a0058
---

## window_faults' limit-row test times out "waiting for B open" under the workspace suite's load; passes alone and as its binary

`apps/human-desktop/tests/window_faults.rs`
`at_the_session_bound_the_limit_row_asks_and_asks_again_after_a_reopen`
failed twice on 2026-11-12 (j95, #643 head b48aa057a's fold), each time
panicking at the `until(.., "B open", ..)` 10 s deadline: once in
`cargo test --workspace`, once in a binary run right after the human_app
e2e under `with_display.sh`. Then it passed 3/3 alone and 2/2 as the
whole binary, on the head and on origin/main c0fe3f474. So it is a
load-sensitive wait, not a regression. Before you report it as a change
of yours, rerun the binary once the host is quiet. It is the same class
as the j215 note in that file's `turn` doc. Logs are in
~/agent-fabric-tmp/j95-review/suite/.

*Anchors: `apps/human-desktop/tests/window_faults.rs::at_the_session_bound_the_limit_row_asks_and_asks_again_after_a_reopen`*

*Observed 2026-11-12 (rust-ui-dev)*

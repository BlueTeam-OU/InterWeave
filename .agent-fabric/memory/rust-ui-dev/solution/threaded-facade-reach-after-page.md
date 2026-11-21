---
role: "rust-ui-dev"
class: solution
topic: "threaded-facade-reach-after-page"
description: "window_faults / threaded-facade tests — reach an edge only after the opening's page is held, or the reach is lost and the edge goes Spent"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e7366450ff0cc6d4
---

## window_faults / threaded-facade tests — reach an edge only after the opening's page is held, or the reach is lost and the edge goes Spent

In tests that run the facade on its own thread (apps/human-desktop/tests/window_faults.rs), a reach of the top while the opening's page is still on its way asks nothing (`WindowInput::Reached` acts only on Armed/AtLimit), and the opening page (asked with `unread_room: None`, so it ends `More`) then lands with `side.at` set, which makes the top `Spent` (`page_arrived`, crates/human/ui-model/src/model/window.rs). No limit row or next page comes until focus leaves and returns. A test that turns once after `select` and then reaches the line passes on an idle host and times out under suite load (j233: merge_group run 42493938826, fixed in #594 2f509fe15 by `until(held > 0)` before `focus_line`; armed 2026-11-08 on architect-cto-01 AUTHORIZES 01a1bbfd, 1 work 0 fix).

**Why:** the in-process ui-slint tests never see this; only the threaded facade can.
**How to apply:** after every `select` in a threaded test, wait for `held(key) > 0` before reaching an edge; reproduce a suspected latency race with `faults.set_pages(Fault::Hold)` across the step, released from another thread after a delay.

*Anchors: `apps/human-desktop/tests/window_faults.rs::at_the_session_bound_the_limit_row_asks_and_asks_again_after_a_reopen`, `crates/human/ui-model/src/model/window.rs::EdgeState::Spent`*

*Observed 2026-11-08 (rust-ui-dev)*

---
role: "rust-ui-dev"
class: workflow
topic: "j170-addclose-pr484"
description: "j170 / #484 — add page and Show back arrow read AddClose (\"Back\"); waits on architect-cto AUTHORIZES; RTL copies need their own test"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9560d12b818948e0
---

## j170 / #484 — add page and Show back arrow read AddClose ("Back"); waits on architect-cto AUTHORIZES; RTL copies need their own test

#484 (branch develop-qzapp/rust-ui-dev-01/feat/j170-add-page-back-arrow-reads-back, head b1494d32): the Peers bar's back arrow reads `UiText::AddClose` ("Back") whenever the Peers list is covered (add page or Show), and BackToConversations on Peers itself. The spec is design #480 §5.3/§5.4/acceptance 7/§9 (ux-designer-01; review-clean at e6f9af49, unmerged on 2026-10-27). Two review rounds: r1 found F1 (P3), fixed; r2 clean. Unarmed: 4 work, 1 fix, minimum 8. REQUEST 01a17d72 asked architect-cto-01 for `AUTHORIZES: BlueTeam-OU/InterWeave#484@b1494d32`; pass that message's id with `fabric-pr arm 484 --authorization <id> --basis ...`.

Lessons:
- app.slint draws each top-bar arrow twice, as `if !root.rtl:` and `if root.rtl:`. A view test must walk `set_rtl(false)` and then `set_rtl(true)` **in one view**: the testing backend initialises once per thread, so a second `view()` call panics. Render does not reset rtl; only `set_touch_settings` writes it. Review r1 F1 found the RTL copy was untested.
- `fabric-review brief <req.yaml>` takes no `render` verb. A re-review takes no `previous_findings`, because it reads the last posted review. `fabric-pr post-review` needs the report whole, starting with its dispatch block.
- The desktop-e2e package is `interweave-desktop-e2e-tests`. Build `interweave-transportctl` before running `human_app`, or its release:: and refusal:: cases fail on a missing binary.

*Anchors: `crates/human/ui-slint/ui/app.slint::add-close-label`, `crates/human/ui-slint/tests/peers.rs::back_arrow_names_no_conversations`*

*Observed 2026-10-27 (rust-ui-dev)*

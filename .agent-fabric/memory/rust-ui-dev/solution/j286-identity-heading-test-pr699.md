---
role: "rust-ui-dev"
class: solution
topic: "j286-identity-heading-test-pr699"
description: "j286 / PR #699 — views test that focus on K/K1/R3/R4's title goes with the identity page (on_identity_page); state and what is owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 121553ebb48902df
---

## j286 / PR #699 — views test that focus on K/K1/R3/R4's title goes with the identity page (on_identity_page); state and what is owed

j286 answers #698 r1 risk 2 (architect-cto REPLY 01a1e76c-f318-7aa3-bf25-d76a9b0c90e1).

PR #699, branch develop-qzapp/rust-ui-dev-01/feat/j286-interweave-views-test-that-focus-on, base f8d460174:
- 6834880b0 work: views test raises K (store_page KeyLost), K1, R3, R4 (identity_page of a real flow via `checked_flow`), closes the page, expects `conversations-heading`. Mutation: removing R3/R4/K1_HEADING from on_identity_page fails it.
- 76a760d47 fix F1: `checked_flow` asserts R2 Primary gives `[Effect::Check(_)]` before `flow.checked` (checked() moves R2 on regardless of entry).
- 73293f508 fix F2: comment names onboard.rs's Effect::Check arm instead of an untested "only".
- Re-review of 73293f508 clean, posted. arm --dry-run refuses only on the floor (1 work).
- ARMED 2026-11-17 at 73293f50 on architect-cto-01 AUTHORIZES (REPLY 01a1e7ab-08a2-7f0e-98b3-7f6208df3db1); wait-merged running; deliver j286 after merge. A red check is mine to answer before any new arm.

ux half of j286 answered by ux-designer-01 REPLY 01a1e76e-e7f0 (R3 Yes -> R4 title focus; ux writes section 8).

Gotchas: j277's record commit 5fbb74962 was already on main as 4084d5b9d (#682) — check `git log origin/main -S` before folding an old "rides the next PR" branch. fabric-review brief: re-review reads previous findings from the last posted review (no previous_findings field); the post-review hook needs the bare `fabric-pr post-review --repo O/R <n> < file` form. A posted review makes no threads: answer findings in a PR comment.
DONE: #699 merged 773b2c685 2026-11-17; j286 delivered.
Follow-up j287 (blocked on #693 merge): ux REPLY 01a1e7fd says #693 258417be1 writes R3 Yes -> R4 title rule + acceptance 15 step 'Tab from R4's title reaches Continue' (trace test lacks it); #693 section 8 puts R3's stop on IdR3Question (focusable), #698 built R3's title (IdR1Title text) -- whichever #693 decides, build to it. REPLY 01a1e7fe sent.
ux REPLY 01a1e809 (#693 r6 dd38a8df4): acceptance 15 now has TWO Tabs on R3 and R4 -- R3 IdR3Question -> CopyPeerId -> IdR3Yes; R4 title -> CopyPeerId -> Continue. Section 5 copy button under every PeerId outside a banner is not yet on R3/R4 (section 9); j287 must build the button with the steps, a single-Tab test would break when it lands.

*Anchors: `crates/human/ui-slint/tests/views.rs::focus_on_any_screens_title_goes_with_the_page`, `crates/human/ui-slint/src/lib.rs::on_identity_page`*

*Observed 2026-11-17 (rust-ui-dev)*

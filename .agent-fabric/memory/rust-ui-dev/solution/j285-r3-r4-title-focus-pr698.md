---
role: "rust-ui-dev"
class: solution
topic: "j285-r3-r4-title-focus-pr698"
description: "j285 / PR #698 — R3/R4 open on a focusable title at a valid check; a repeated notice takes focus via IdentityFlow::notices_raised; armed state and what is owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 21a9be2f91131b5f
---

## j285 / PR #698 — R3/R4 open on a focusable title at a valid check; a repeated notice takes focus via IdentityFlow::notices_raised; armed state and what is owed

j285 answers #697 review pre-existing P-1/P-2 (design/2026-10-19-android-identity.md section 8).

Built on PR #698, branch develop-qzapp/rust-ui-dev-01/feat/j285-interweave-r3-r4-focus-on-title, base 2e8f7ba8a:
- 4761f7ccc ui-model: `IdentityFlow::notices_raised()` counts every result that sets a notice on C4 (sealed Err), R2 (ChecksumFailed, OtherIdentity, restored Err), R3 (restored Err), S1 (offline Err); not start_notice (lock refusals at launch would steal focus), never on cancelled_step.
- 76c41ee9f ui-slint: R3/R4 set heading_key `r3-heading`/`r4-heading` (identity.rs), listed in lib.rs `on_identity_page`; app.slint `changed page` screen branch: forward to r3 or r4 -> typing=false, go(0, heading-idx()); `notice-new()` also true when page.notices-raised moved. R3 Yes -> R4 also lands on R4's title (my choice, not in design; named for ux-designer in the PR body).
- Review r1 (3 P3: stale heading comments, "only" comment without test, 4 untested increments) fixed in 91ea4140a, c3bc14310, 8e22d1dd3; re-review judged all fixed, posted.
- Unfixed risks from r1: on_identity_page has no test for the new keys; counts compared across flow objects (no path found).

2026-11-17: #698 (2 work, 3 fix) ARMED at 8e22d1dd on architect-cto-01's AUTHORIZES (REPLY 01a1e76c-f318); wait-merged running; deliver j285 after merge. Follow-ups queued as j286: on_identity_page views test for r3/r4-heading, and ux-designer-01's answer (QUESTION 01a1e76e-6425) on R3 Yes -> R4 title focus, which architect-cto asked be ux's sentence in section 8.

Gotcha met: the trace-test helper `text()` returns &'static str (no .as_str()); clippy doc_markdown wants `PeerId` backticked in doc comments, test docs included.
ux-designer-01 REPLY 01a1e76e-e7f0 (2026-11-17): yes, R3's Yes opening R4 puts focus on R4's title, navigating (provisional); ux writes it into section 8 and acceptance 15 on #693 or the PR after, and will name the commit. j286's ux half is answered; the on_identity_page test remains.
DONE: #698 merged f8d460174 2026-11-17; j285 delivered.

*Anchors: `crates/human/ui-model/src/identity_flow.rs::notices_raised`, `crates/human/ui-slint/src/identity.rs::R3_HEADING`, `crates/human/ui-slint/ui/app.slint::notice-new`*

*Observed 2026-11-17 (rust-ui-dev)*

---
role: "rust-ui-dev"
class: solution
topic: "j204-identity-app-bar-arrow"
description: "j204 identity app bar back arrow — back_arrow() screen list, segment 0 = arrow then banner (banner-idx), pushed d0ff179d on feat/j204 stacked on j203"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 49afd9a4ff50373d
---

## j204 identity app bar back arrow — back_arrow() screen list, segment 0 = arrow then banner (banner-idx), pushed d0ff179d on feat/j204 stacked on j203

j204 (design 2026-10-19-android-identity.md §3/§8) pushed 2026-11-04 on
develop-qzapp/rust-ui-dev-01/feat/j204-identity-design-s3-the-app-bar,
498cabbc (ui-model) + d0ff179d (ui-slint), stacked on j203 11dc8a51 (not
j202 2fbf0388 as the job said: j203 already holds j202 + j206's AddClose).

- `IdentityFlow::back_arrow()`: C1 C2 C3 R1 R2 R3 S1 S2. W and K1 (Back
  leaves the app) and C4/R4 (no Back) have none. Inactive (`back-active`)
  while `waiting()`. The store-recovery page shares IdentityPage and
  leaves `back-arrow` false.
- Focus: segment 0 is the arrow (idx 0) then the banner at
  `banner-idx()`; the full list's arrow keeps seg 0 too, and the page's
  arrow gates `focus-wanted` on `!page.review` so the two never both
  take focus.
- Trace-test traps: R1's first Tab now lands on the arrow (two Tabs to
  the primary); one `Identity::new` per test (the testing backend
  inits once), switch screens by replacing `page.flow`.
- Mutants: 2 ui-model + 2 view, all caught. No device run; desktop does
  not show identity screens, so no desktop e2e. Rides j199's PR with
  the rest of the stack.

Related: [[j203-acceptance-15-focus]], [[j202-show-all-my-words]], [[pipe-masks-exit-status]].

*References: j202-show-all-my-words, j203-acceptance-15-focus, pipe-masks-exit-status*

*Anchors: `crates/human/ui-model/src/identity_flow.rs::back_arrow`, `crates/human/ui-slint/ui/app.slint::banner-idx`*

*Observed 2026-11-03 (rust-ui-dev)*

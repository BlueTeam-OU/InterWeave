---
role: "rust-ui-dev"
class: solution
topic: "j203-acceptance-15-focus"
description: "j203 identity acceptance-15 focus rules pushed 11dc8a51 on feat/j203 (stacked on j202 393bb85e); IdScreen on the page, arrow probe, typing gated on picker; IdBack dropped for AddClose (j206)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 717b05b92f1bfee1
---

## j203 identity acceptance-15 focus rules pushed 11dc8a51 on feat/j203 (stacked on j202 393bb85e); IdScreen on the page, arrow probe, typing gated on picker; IdBack dropped for AddClose (j206)

j203 (2026-11-03/04): branch develop-qzapp/rust-ui-dev-01/feat/j203-identity-design-acceptance-15-dialog-close at 11dc8a51. It fast-forwarded onto j202, plus j206's two commits (57e5192c, 393bb85e, also pushed to j202): IdBack removed, the arrow uses AddClose (spec #543 51519bc3; ux OBSERVATION 01a1a383, LC REPLY 01a1a385-cb9b gave AddClose's doc). It rides j199's one PR (waits on e87e1462 reaching main).

- IdentityPage.screen (Slint enum IdScreen). `changed page` acts on a screen change first. A picker screen opens typing on the first enabled key (init too). C3→C2 and R2→R1 focus the primary (seg 6).
- Defect found by the test: typing carried onto R1 swallowed Enter. capture-key-pressed now needs page.picker.
- Dialog: saved-typing at open, restored at close.
- Arrows: no loops in Slint, so a probe. arrow-seq → the focused stop's `source` → probe-seq → every stop's `offer` → `changed offered` resolves. Each round is one change-handler round inside run_change_handlers. Per-instance `property <int> arrow: root.arrow-seq; changed arrow => …` on every stop. Weighted distance 13·major²+minor². The beam rule and the tie-break were removed: nothing on these pages turned on them, so their mutants survived. Pixel rounding makes w→Down = s.
- Two `Identity` test pages in one test panic (the testing platform is set twice). Use one test per page.
- 9 mutants of the final code caught. The commit message says 14, which is wrong.
- Not built: C2's PeerId page (IdPeerIdWritten) and the landscape layout. Told ux in INFO 01a1a3b4.

Related: [[j199-identity-keyboard-reach]] [[j202-show-all-my-words]] [[slint-rebuilt-page-focus]]

*References: j199-identity-keyboard-reach, j202-show-all-my-words, slint-rebuilt-page-focus*

*Anchors: `crates/human/ui-slint/ui/app.slint::IdentityScreen`, `crates/human/ui-slint/tests/trace.rs::arrows_move_focus_to_the_nearest_stop_their_way`*

*Observed 2026-11-03 (rust-ui-dev)*

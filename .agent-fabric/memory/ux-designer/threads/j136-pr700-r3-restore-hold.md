---
role: "ux-designer"
class: threads
topic: "j136-pr700-r3-restore-hold"
description: "j136 — #700 open (4 work): R3 held while restore runs, R3 notice placement/order, F11 runs, mode scope, A5; waits LC 01a1e8ef"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - db69e7e505dfe0ac
---

## j136 — #700 open (4 work): R3 held while restore runs, R3 notice placement/order, F11 runs, mode scope, A5; waits LC 01a1e8ef

2026-11-17. PR #700 on develop-qzapp/ux-designer-01/feat/j136-interweave-identity-693-full-review-carried, base e33d518b6:
- 0431e0896 long-message A5: TM-57 unrun device cases in "Not run yet"; load 4.8-6.0 (48-60 on 10 CPUs).
- ffc313671 identity §8: typing/navigating modes are R2's and C3's only; "R2 and C3 open as typing".
- e142a14a8 identity §8: R2 held-control hypothesis names the inactive back arrow.
- ee7f091e9 identity: §6 Restore running row, §7 Back while R3's restore runs, §8 R3 focus order + "While R3's restore runs" (Yes/No shown disabled, CopyPeerId still copies), R3 notice under app bar above question; acceptance 15 runs 4 (held), 5 (IdStorageFailed), 6 (cancelled prompt); revision note.
Not carried: #693 full review pre-existing P3 2 (acceptance for older running-check rules) — not in j136's list, said in PR body.
Sent: LC REQUEST 01a1e8ef-1b96 (language-culture-en), INFO 01a1e8ef-5b99 to rust-ui-dev-01 (build's R3 back/recheck unguarded).
Next: fold LC, review class on e33d518b6..head, post; 4 work < 8 → owner's word to arm.
Related: [[j134-pr693-carried]], [[arming-on-owner-word-mid-review]].
2026-11-17 ~10:15Z: LC 15fe6c082 folded (merge 0d9ab8a81). LC Qs answered in 0733a88ec (notice action shown disabled during restore; 6th run start; A5 "first turn"). Asked LC pass of 0733a88ec: REQUEST 01a1e8f9. Restarting a job blocked on an answered LC request: `fabric-jobs start j136` plain works once LC's job clears; a hook judges the whole compound command, so start the job in its own call.

*References: arming-on-owner-word-mid-review, j134-pr693-carried*

*Observed 2026-11-17 (ux-designer)*

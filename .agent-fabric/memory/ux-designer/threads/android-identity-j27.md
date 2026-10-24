---
role: "ux-designer"
class: threads
topic: "android-identity-j27"
description: "j27: Android identity screens design note (create, show phrase, recover) for plan §20 steps 7-8 — where it is and what it waits on"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 938238fbff0c55d4
  - b6034c89df3db4a5
---

## j27: Android identity screens design note (create, show phrase, recover) for plan §20 steps 7-8 — where it is and what it waits on

2026-10-19: rust-ui-dev-01 asked (REQUEST 01a1527b-bd4f) for §20 steps 7-8's screens; rules fixed by android-key-custody.md at ecf20942 (FLAG_SECURE, no clipboard, IME-free per-position BIP-39 picker, words never persist, prompt before on-demand display, fresh-install restore confirms derived PeerId by eye).
- Note: design/2026-10-19-android-identity.md on develop-qzapp/ux-designer-01/docs/android-identity (d0307136). Sent to rust-ui-dev (REPLY 01a15280-67c0).
- Waiting: language-culture-en's English + §9 strings (REQUEST 01a15280-7c5e, branch for/ux-designer-01/android-identity); architect-cto Q1-Q4 (QUESTION 01a15280-7390: stopped Activity = leaving?; screen reader may read a focused word?; commit at C4?; replace-with-new path on Q1?).
- Then open my InterWeave PR (fold language branch unrebased, blind review). The j26 review PR comes after (one open PR per agent).
Related: [[stage17-android-review-j26]]
- 2026-10-19: architect-cto answered Q1-Q4 (REPLY 01a15282-ac06; custody doc 425ec5d0): words kept while stopped, focused label may be spoken, seal at C4, no new-identity button (IdQLost text). Note at ef8af262 with void-paper marker (87c7bd88). Still waiting: language-culture-en strings.
- 2026-10-19: folded language-culture b85d0bae (merge 4a6de629); Q-a..Q-c answered at 367b3ae6 (screen Q1 renamed K1; C1/R1 say words restore identity only; R3 no-PeerId line). Waiting on their 4 pending strings (REPLY 01a15288-b836).
- 2026-10-19: PR #303 opened (identity note). r1 review: 1 P1 (picker could not enter act/can/car/cat/man/win), 5 P2, 4 P3; all judged by review class; answered 00c2ee4e..ac0c932f. Waiting: language-culture strings (01a1529b-fe9c), architect Q5/Q6 (01a1529b-e259). Then fold, re-review, arm (owner word if under minimum).
- 05:30Z: folded lc 41ca6ef0, wired 4 keys (5e869afe); architect Q5 (Android has no settings export; §9 amended 2f3b0587: state fact, ask nothing) and Q6 (none) applied at c0cad1ae; r2 re-review running on 5e869afe; asked lc for IdC1Point5/IdR1Settings reword (01a152a0-6e61), architect re "transport" in label (QUESTION sent).
- 05:40Z: r2 answered (d92a0517, 14d65c62); lc IdFormatLine folded; read-back rows rekeyed d723f721 (IdStorageUnknown pending lc, REPLY 01a152aa-4c68); architect Q8 (public PeerId-field reader) + Q9 (lock handover) ruled 252a6f45, applied 488e8e82. Next: fold lc, round-3 re-review.
- 2026-10-19 ~06:40Z: #303 MERGED (cfb0379d), 8 review rounds; j27 done, REPLY 01a152da-0d25 to rust-ui-dev. Q10 (lock hand-back on refusal) still open with architect-cto.

*References: stage17-android-review-j26*

*Observed 2026-10-19 (ux-designer)*

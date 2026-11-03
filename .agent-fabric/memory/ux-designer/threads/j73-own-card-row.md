---
role: "ux-designer"
class: threads
topic: "j73-own-card-row"
description: "j73 own card row (contact-cards 5.5) delivered as #469 (merged f2295901); carried risks listed in a PR comment"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 475e7e05add05e29
---

## j73 own card row (contact-cards 5.5) delivered as #469 (merged f2295901); carried risks listed in a PR comment

j73 (QUESTION 01a17b74 from rust-ui-dev-02, 2026-10-27): four gaps in contact-cards §5.5 plus #467 F17.
Answered in spec first: 0b2ccf58 (5.5 menu Retry/Cancel no Show source; Accepted row no menu, not activatable; spoken label adds CardSentPhoto "with your photo", proposed for language-culture; 3 focus rows in §9 item 1) and bc9b57fa (identity-key 5.2: opening the card page leaves the conversation, closing the Cancel snackbar, so no old-key Undo; acceptance 13 case E).
Provisional REPLY sent 01a17b77-dbfe. PR #469 opened (2 work, under the 8 minimum: architect-cto's AUTHORIZES needed).
2026-10-27 session 2: review dispatched on fd456bee..bc9b57fa; REQUEST 01a17b7a sent to language-culture-en (English pass + CardSentPhoto). Next: post review, fix, then ask architect-cto to authorize the arm on the final head.
j71/j72: #467 merged as fd456bee, both c9d361a5 and 1cf914da on main.

Round 1 review posted (F1-F4, all judged real; F2 lowered to P3). LC pass 3ae73eab folded (4ee4a98e; CardSentPhoto = "Your photo was sent with the card."). Fixes: 4f? F1 (identity-key 5.2: the step itself closes a card's Cancel snackbar, since the share sheet's Open your card keeps the conversation open; late Cancel success after the step only announced; acceptance 13 E runs both routes) and 37b0e61f (F2 split focus rows, F3 new contact-cards 9 item 27, F4 Keep only). Re-review bc9b57fa..37b0e61f dispatched. Owed after: tell rust-ui-dev-02 the F1 rule is new behaviour (final REPLY to 01a17b74), ask architect-cto to authorize arming.
Re-review of 37b0e61f posted: F1-F4 fixed, new F5 (desktop has no long snackbar timeout: seam) and F6 (CardSentPhoto read on pending rows, must claim no delivery), both judged real, fixed in 758b480c. REQUEST 01a17b8a to language-culture-en to reword CardSentPhoto (placeholder "The card includes your photo."). Next: fold their words, re-review 37b0e61f..head, then architect-cto authorization, then final REPLY to rust-ui-dev-02 (F1 rule is new behaviour).
Folded 727cfaee (a5c454f6). Said yes to LC rewording CardSentLabel/CardPreviewSent to claim no act (REPLY 01a17b8c); waiting their commit, then re-review 37b0e61f..head.
Folded 79e2e9b4 (51bf3014): CardSentLabel/CardPreviewSent = 'Your contact card'. Re-review 37b0e61f..51bf3014 dispatched. Tell rust-ui-dev-02: labels.rs strings change (CardSentLabel, CardPreviewSent, CardSentPhoto).
Re-review 2 (51bf3014) clean, posted. Then c206054e (Android tree check blocked, timed fallback) and eca2b907 (in-flight Cancel withdrawn, no return on refusal; late announcement after KeyMade; acceptance 13 4th run). Re-review 51bf3014..eca2b907 dispatched.
Round 3 re-review (eca2b907) posted: my in-flight Cancel rule wrong (withdrew taken rows; refusal hid live row), refusal feedback/focus missing, order check weak. Fix drafted uncommitted; adjudication running.
Round-3 fixes committed 9af66bf9 (judged real); re-review eca2b907..9af66bf9 dispatched.
Round 4 F7-F9 fixed 7a1b9e15. rust-ui-dev-02 built row on feat/j54-own-card-row 72e58dbe; can't drop activatable role (Slint 1.18): chose (a), item 27 blocked by toolkit (768d1f12, 47b493c6, 9ca43d28); REPLY 01a17b9a sent with 3 string changes + identity-key 5.2 rules. Re-review 9af66bf9..9ca43d28 dispatched. Then architect-cto authorization.
Round 5 F10 fixed c97d9fd5; re-review 9ca43d28..c97d9fd5 dispatched; 8 work commits now (minimum met).
Round 7 clean, posted; #469 ARMED (8 work, 8 fix) on owner's word; carried risks posted as PR comment. Next: on merge, verify on origin/main, fabric-jobs deliver j73, switch.
DELIVERED: #469 merged 2026-10-27 as f2295901. Owed follow-ups: the five carried risks in #469's comment; item 27's role check re-checked when Slint offers a conditional action.

*Observed 2026-10-27 (ux-designer)*

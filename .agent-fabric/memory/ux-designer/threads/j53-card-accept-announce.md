---
role: "ux-designer"
class: threads
topic: "j53-card-accept-announce"
description: "j53 state — contact-cards CardPhotoNotUsed final text and Accept announced once, PR #416"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - bfa874f241662b05
---

## j53 state — contact-cards CardPhotoNotUsed final text and Accept announced once, PR #416

j53 (language-culture-en OBSERVATION 01a16df9): #416 b9fcbe8b, branch develop-qzapp/ux-designer-01/docs/card-accept-announce (worktree interweave-j52-add-a-peer-5-5-a).
- s10 CardPhotoNotUsed = LC final; s8 Accept announces nothing (focus to confirmation row reads it), snackbar unannounced; s9 item 1 focus row TalkBack check.
- Told LC (prose read offered) and rust-ui-dev-02 (#414, provisional). Review round 1 running.
Next: post review, AUTHORIZES (1 work), tell rust-ui-dev-02 binding, deliver.
Lesson: when focus moves to a control that states the result, the live announcement of the same result is a duplicate; check every focus-moving action against s8's announcement list.
- LC wording (REPLY 01a16dfd-4a4b) applied verbatim as d3390b7c; round 2 must cover b9fcbe8b..head. rust-ui-dev-02 QUESTION 01a16dfc-c984 answered (b) 01a16dfd-1a8e.
- round 1 posted (F1-F3 P2, F4 P3), fixed e2ba532e (CardUndoHint, snackbar timeout waits while focus in row). LC asked CardUndoHint (lc3). Next: fold LC, round 2 b9fcbe8b..head. rust-ui-dev-02 must hear of CardUndoHint+timeout hold once final. Carried: case U, X's existing photo when card photo discarded.
- LC 40b970e4 folded (CardUndoHint 'just above the box where you type'); clause 92924de3. Gap: no composer when network withheld -> LC asked (lc4) for one text or CardUndoHintNoComposer. Then round 2 b9fcbe8b..head.
- LC 94724e02 folded (CardUndoHint 'Undo comes after all the messages in this conversation', true without composer). Round 2 running b9fcbe8b..94724e02.
- round 2 posted (F2 partly, F5 P2 no cap, F6 P3), fixed c8bf5f38 (wait spans row..snackbar, 2-min cap). LC told of changed clause. Round 3 next.
- round 3 running 94724e02..c8bf5f38. LC REPLY 01a16e07: apply verbatim after round 3: 'two things hold'->'these hold'; bullet 2 clause -> 'so the snackbar does not close while the person moves focus from the row to the Undo that the hint names'.
- round 3 CLEAN on c8bf5f38 (posted). 6855f7e6: LC wording + re-entry clause; round 4 running c8bf5f38..6855f7e6. Clean -> AUTHORIZES, tell rust-ui-dev-02 (CardUndoHint, wait span, 2-min cap).
- round 4 posted (F7 P3 no re-entry check), fixed d04c411d (+'does not time out ... within the cap', changed LC's 'does not close'). Round 5 running 6855f7e6..d04c411d.
- round 5 CLEAN on d04c411d (posted). AUTHORIZES asked (arch2). Carried: checks timed at 6 s though platform may be longer; popups inside span; snackbar tree position (desktop SR pass).
- #416 ARMED on AUTHORIZES 01a16e0d; rust-ui-dev-02 told final (ui6). wait-merged running. After merge: deliver j53.
- 2026-10-24: #416 merged 5f983d9f; j53 delivered. Owed: desktop SR pass on #414 (snackbar after composer; popups in span).

*Observed 2026-10-24 (ux-designer)*

---
role: "ux-designer"
class: threads
topic: "j62-case-e-no-photo"
description: "j62/j63 contact cards: CardKeepsNoPhoto after Remove photo, case S without CardSame; the editor photo step's focus and speech; PR #443"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c25424637998173b
---

## j62/j63 contact cards: CardKeepsNoPhoto after Remove photo, case S without CardSame; the editor photo step's focus and speech; PR #443

j62 (rust-ui-dev-02 OBSERVATION 01a17304) and j63 (their QUESTION 01a17307) are both on BlueTeam-OU/InterWeave#443,
branch develop-qzapp/ux-designer-01/feat/j62-contact-cards-case-e-after-remove (2026-10-25).
- 03d61dcd: a no-photo local choice gets `CardKeepsNoPhoto`; `CardIncludesPhoto` is hidden when the card photo will not be used;
  case S shows `CardSame` only when the name and photo are equal, otherwise the kept lines. The build (main 996a9ad0) shows CardSame plus the kept lines.
- dfe3d009: §5.2 photo step. Focus goes back to the button that opened the picker; `CardPhotoPreparing` while preparing; `PhotoAdded` and
  `PhotoUnusable` announced once; the editor's Remove photo is an unsaved change, written on Save (main removes it at once with a snackbar).
- Round 1 review dispatched on 996a9ad0..dfe3d009. LC REQUEST 01a1730e-1a0a to language-culture-en. REPLY 01a1730e-936c to rust-ui-dev-02 (provisional).
- 2 work commits < 8: arming needs the owner's word or architect-cto's authorisation (as for #441, [[j61-card-updates-photo-removed]]).
Related: [[j60-same-name-keys]].
- 12:40Z round 1 posted (F1 P2: a card with no photo vs a person-chosen photo had no defined line; F2-F5 P3). The judge said F1, F3, F4, F5 real, F2 partly.
  LC a69441e6 folded fast-forward (PhotoAdded text "Photo ready"). Fixes ad63f377 + 3c3c402f; judgement comment posted.
  Round 2 re-review dispatched (brief2.md, dfe3d009..3c3c402f). LC pass 2 REQUEST 01a17314-acb9.
  Rule now: CardKeepsPhoto whenever the photo is person-chosen (any card); CardSame only with the same name and no photo on either side.
  After the merge: tell rust-ui-dev-02 the build items 2a-2e (CardKeepsNoPhoto, CardSame vs kept lines, CardIncludesPhoto, editor Remove at Save).
- 12:47Z round 2 posted (F1-F5 fixed; F6 P3: Remove photo over an unsaved photo during preparation; 2a: E row's CardKeepsNoPhoto needs "card carries a photo").
  Fix 2e7672be; round 3 dispatched on 3c3c402f..2e7672be (brief3.md). LC pass 2 f416926d merged as a26697ce (conflict in §5.2, resolved to "saved or not").
  LC pass 3 REQUEST 01a17318-12b9. Next: post round 3, then round 4 on 2e7672be..a26697ce (the merge), then arm on the word.
- 12:52Z round 3 CLEAN, posted on 2e7672be. LC pass 3 5df7c3c7 folded. Fix c87340f2 (the round 3 risk and the fixture). Round 4 dispatched on 2e7672be..c87340f2 (brief4.md). LC pass 4 asked.
- 12:55Z round 4 CLEAN, posted on c87340f2. LC pass 4 ed27eb51 folded and pushed; answered their "only" question (no change, line 512 already says it).
  Round 5 dispatched on c87340f2..ed27eb51 (brief5.md). Then: post round 5, PR body, arm on the owner's word (6 work, 4 fix < 8).
- 12:58Z round 5 CLEAN posted on ed27eb51; PR body updated. Arm REQUEST to architect-cto-01 (AUTHORIZES #443@ed27eb51). After the merge: deliver j62+j63, INFO to rust-ui-dev-02 with the build items.
- 12:57Z architect-cto-01 AUTHORIZES 01a17322-a3e6 for #443@ed27eb51 (no later push). Waiting on the rust check, then: fabric-pr arm 443 --authorization 01a17322-a3e6-700a-a8d1-8b56e623b942.
- 13:0xZ ARMED #443 at ed27eb51 (6 work, 4 fix, 1 merge) on AUTHORIZES 01a17322; queued; wait-merged running.
- MERGED 14161365. INFO 01a1733b-95aa to rust-ui-dev-02 with the build items. j62+j63 delivered. Carried: CardKeepsPhoto reachable only in store; item 7 'same name' not 'character for character'; second choice during preparation reading.

*References: j60-same-name-keys, j61-card-updates-photo-removed*

*Observed 2026-10-25 (ux-designer)*

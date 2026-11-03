---
role: "ux-designer"
class: threads
topic: "j60-same-name-keys"
description: "contact-cards 5.4.1 same-name rule became two keys (case key OR shape key); PR #438 with #435's carried items"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 553403e6abc6f029
---

## contact-cards 5.4.1 same-name rule became two keys (case key OR shape key); PR #438 with #435's carried items

j60 (rust-ui-dev-02 OBSERVATION 01a171f9): 5.4.1 folded case before the TS #39 skeleton, so capital lookalikes ("ꓮda", "Ꭰave") were missed.
The previous session's provisional REPLY 01a171fa proposed fold, skeleton, fold. A sweep of confusables.txt 16.0.0 (scratch script, 2026-10-25) showed that still misses "ꓟax"/"Max" and "Βob"/"Bob".
Rule written on PR #438 (head f84f77ea): prepare (NFKC, whitespace); case key = fold, skeleton; shape key = skeleton, fold, skeleton; same when either key is equal.
INFO 01a17205-0ad6 told rust-ui-dev-02 not to build fold-skeleton-fold.
#438 also carries the three #435 items from [[j58-proposal-menu-carried]]: the menu anchored to the card, as app.slint does; the case E slide-and-lift check; a photo is "not a local choice", not "from a card".
LC REQUEST 01a17205-9619 to language-culture-en. Round 1 review dispatched on 829dbccc..f84f77ea.
3 work commits < 8, so arming needs the owner's word or the below-minimum authority.
- 07:55Z round 1 posted (F1 P3: "Ian"/"Lan" is a false warning, not a lookalike; F2 P3: the menu's third place). LC 761f9e70 folded fast-forward.
  Fix 9d340c51: the shape key has no NFKC (catches "Alſie"; residual misses 26 -> 17), F1, F2, the evidence method and hash. Sweep script posted on #438 (issuecomment-6395097730).
  Round 2 re-review dispatched (rv438-rr2.md). LC pass 2 REQUEST 01a1720e-d960; INFO 01a1720e-ed6a to rust-ui-dev-02.
  Carried to the next contact-cards change: "a photo the person chose" can be reached only in the store, not through the UI (round 1, section 2).
- 08:05Z round 2 posted (F1 and F2 fixed; F3 P3: the over-the-card example). LC 9253e7d2 folded; my edits were merged into it by hand.
  Fix f2fdf85b: a THIRD key, the compatibility shape key (NFKC, then the shape key's steps), because dropping NFKC lost 45 compatibility joins (circled I, fullwidth 0).
  Misses stay 17. The visible area is the window inside the system bars. The evidence comment was refreshed with the 3-key script.
  Round 3 dispatched (rv438-rr3.md, 9d340c51..f2fdf85b). INFO 01a17213-67a9 to rust-ui-dev-02; LC pass 3 REQUEST 01a17213-7178.
- 08:12Z round 3 posted (F3 partly fixed; F4 P2: no fixture needs the third key; F5 P3: the Ⓘ example and the 45 split). LC f7e0da74 folded.
  Fix 24a2d59c: "J０hn"/"John" is joined only by the compatibility shape key, and the 45 split into 15 alike + 30 joined after the fold. Round 4 dispatched (rv438-rr4.md). LC pass 4 asked.
- 08:20Z round 4 CLEAN, posted on 24a2d59c; it reproduced 45/15/30 with ICU 77. LC pass 4 a5955a9a folded and pushed (words only). Round 5 dispatched on 24a2d59c..a5955a9a (rv438-rr5.md).
  Then: post round 5. Arming needs the owner's word: 7 work < 8.
- 08:25Z round 5 CLEAN, posted on a5955a9a. PR title and body updated to three keys. INFO to rust-ui-dev-02: the rule stands.
  Waiting on the owner's word to arm: 7 work, 3 fix, minimum 8. After the merge: deliver j60, tell rust-ui-dev-02, switch.
- 08:30Z owner's word 'arm 438': ARMED at a5955a9a (7 work, 3 fix); wait-merged running.
- #438 MERGED e891e9bc; INFO to rust-ui-dev-02; j60 delivered. Carried: 'a photo the person chose' can be reached only in the store (round 1, section 2); the build still compares names exactly until rust-ui-dev builds 5.4.1.
- 10:35Z LC OBSERVATION 01a1729f: CardUpdatesPhotoRemoved final English 'Your contact's photo will be removed.' Taken as j61 (fresh session).

*References: j58-proposal-menu-carried*

*Observed 2026-10-25 (ux-designer)*

---
role: "ux-designer"
class: threads
topic: "j58-proposal-menu-carried"
description: "contact-cards proposal menu and photo removal (#435, merged 829dbccc); three items carried to the next contact-cards spec PR"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7b1048ad21335b69
---

## contact-cards proposal menu and photo removal (#435, merged 829dbccc); three items carried to the next contact-cards spec PR

#435 merged 2026-10-25 as 829dbccc (design/2026-10-21-contact-cards.md). It answered rust-ui-dev-02's QUESTIONs 01a1718c (j58) and 01a171a0 (j59).
- Menu keys: long press, desktop secondary click, Menu/Shift+F10 on any row control; never Enter (no card focus stop).
- Warned row (§12 O3, the owner's answer "Keep menu + TalkBack"): only the swipe is withheld. The press-drag-release onto Accept chooses nothing; that rule is the designer's reading of O3, not an owner answer.
- Photo removal: CardUpdatesPhotoRemoved ("Current photo removed") and CardPhotoNotUsedRemoved, in cases U and E. The store rule (card's photo or its absence) is kept.

Carried to the next contact-cards spec PR (from round 3's risks):
1. s5.4.4: say where the proposal menu is anchored, or drop "near where the press began".
2. s9 item 7: add the slide-and-lift check on case E's warned "Anna Rossi" row, not only case D.
3. s10/item 7: "came from a card" should be "not a local choice". The store overwrites a photo with no recorded source, while the contacts.rs module comment (l.16-18) says it stays. rust-ui-dev-02 was told in INFO 01a171de.

Related: [[j55-proposal-actions]], [[wording-pass-keeps-conditions]].

*References: j55-proposal-actions, wording-pass-keeps-conditions*

*Observed 2026-10-25 (ux-designer)*

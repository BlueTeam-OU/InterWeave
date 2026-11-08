---
role: "language-culture"
class: solution
topic: "interweave-copy-state-and-failure-texts"
description: "InterWeave labels.rs: wording of state, refusal and failure texts — hedges, \"still\", \"this attempt\", direction-free and session-bound rows"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6d0d2a02cce99b61
---

## InterWeave labels.rs: wording of state, refusal and failure texts — hedges, "still", "this attempt", direction-free and session-bound rows

InterWeave human-client copy (crates/human/ui-model/src/labels.rs): texts that state a state, a refusal or a failure.
- A hedged fact keeps its hedge in the advice too: "may be able to use it" followed by "To keep this key, update" promises what the "may" denied; write "To try to keep" (KeyNewerVersion, #411, 72a314b9, 2026-10-24).
- A case split is written with "if", never "where" ("Where it has not been sent"): a second-language reader can take "where" as a place (KeyNewPending, #411, 8dcd31a3, 2026-10-24).
- "could not be shown" is wrong for a value the app never displayed (CardPhotoNotUsed: the photo is decoded only at Accept); "could not be used" is true for a decode failure and for a size refusal alike (#414, REPLY 01a16df9-200d, 2026-10-24).
- Another admin client is "another program or window on this device", never "another app": transportctl and the command-line client are not apps to a reader. A refusal that does not prove the other change is hedged with "might have changed ... at about the same time", never "may" (read as permission) and never "almost" (there may have been no other write) (the superseded texts, #425, 65295171, 2026-10-24).
- A state line after a change by someone else drops "still": "{peer} is still not trusted" is false when the other change was a revoke of a peer trusted in between (TrustStepSuperseded, #425, 2026-10-24).
- A guard beside a button that stays puts the condition first: "If you still want to trust {peer}, choose Trust again." With "only if ..." after the verb, the cold reader dropped the condition. Repeat {peer} instead of "this peer" next to "this device" (#425, 65295171, 2026-10-24).
- A line that must be about this attempt only, beside an earlier attempt whose result is not known, makes the attempt the subject ("This attempt to add the address saved nothing."), never "The address was not saved" (a stored-state claim); a trailing "this time" is skipped by a second-language reader (AddSuperseded, #425, ad05a91e, 2026-10-24).
- The superseded texts were judged true in the eviction-floor case (desktop, another program writing over 4,096 keys during one call): "might have changed ... at about the same time" claims no cause, and s5.7 counts the floor. A rewording that states the cause ("another program's changes kept this one from being made") was declined (j172, REPLY 01a1713e).
- Session-bound drops (start window, #472, j221, 8338ce6e, 2026-10-27): rows the bound drops are "no longer shown" and "cannot be shown again", never "hidden" (Hide is an action) and never "removed" alone (reads as deleted). A button that shows rows while its effect drops others needs the reason in the text ("makes room for"): three cold reads read Show + "stops showing" as a contradiction. Don't use "earlier" and "earliest" for two different groups of rows in one text. A count line that covers messages and card proposals: "... messages and contact cards: {count}".
- A string placed at a spot a person can reach from either direction (a gap in a conversation, reached by scrolling up or down) names no direction: "more", "not shown yet", never "earlier" or "older". The start-window Earlier… keys keep their names while their texts are direction-free (#488, j225, 6ab24474, 2026-10-27).
Related: [[solution-carried-2026-10-10]]

*References: solution-carried-2026-10-10*

*Observed 2026-11-03 (language-culture)*

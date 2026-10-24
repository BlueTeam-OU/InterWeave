---
role: "ux-designer"
class: threads
topic: "stage21-design-backlog"
description: "flows owed to design/ when Stage 21 opens at Stage 17's close — peer by address (ADR-0062) and key-change proposal (ADR-0063)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d7288d5e631d9362
---

## flows owed to design/ when Stage 21 opens at Stage 17's close — peer by address (ADR-0062) and key-change proposal (ADR-0063)

architect-cto-01 INFO 01a15fc3 (2026-10-21, #340 merged e3a05d18): when Stage 21 opens, design/ owes:
- ADR-0062: onboarding step 3 = add a peer by address (typed or scanned multiaddress, refusals by name) or show your own (text + QR of at most 16 lines, never on a lock screen), trust as a second act on the same screen; a client with no address says it can reach no one yet (human-client-ui.md §2, §3).
- ADR-0063: each identity has a signing key; contact page and identity page show a 16-hex fingerprint beside the PeerId; a card on a known route with a different key is a key-change proposal in plain words with both fingerprints, accepted only by the person (§3; criteria in §13).
Mine to place: join design/2026-10-21-contact-cards.md or notes of their own. Nothing before Stage 21 opens.
Related: [[j30-contact-cards-identities]].

*References: j30-contact-cards-identities*

*Observed 2026-10-21 (ux-designer)*

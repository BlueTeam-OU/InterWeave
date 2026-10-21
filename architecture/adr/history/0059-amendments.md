# ADR-0059 — amendment notes

### Amendment 2026-10-21 — The photo carries pixels and nothing else, a proposal has no Keep, and the store records where a contact's name and photo came from

ux-designer-01's specification of the card flows (#334,
`design/2026-10-21-contact-cards.md`, questions A1–A3 of its §12, the
same day the record was accepted) asked three things the record implied
but did not say. Ruled: (A3) a photo that leaves the device carries
pixels only — the sending client re-encodes it and strips every
metadata block (EXIF, XMP, GPS, thumbnails, the file name) before it
enters the card, and a receiver treats any metadata that arrives as
absent; decision 5 says so, HUMAN-CHAT.md §Contact card holds the rule
and gains the fixture case of a photo with a GPS block that the sender
strips. (A2) A proposal has no Keep: Accept is the only way a card
outlives the session, and an unaccepted proposal cannot be kept as
content; decision 3's "like any item the person did not keep" meant
that, and `human-client-ui.md` §3 now says it in those words. (A1)
Decision 3's "a local edit is never overwritten by a card" needs the
store to know where a value came from: STATE.md's `contacts` gains, for
the name and for the photo, the source (card or person) and when,
content-free columns in the §Logical tables review, added by the
migration plan §29 step 2 lands and listed in its exit gate's (d). None of the three changes what a card is, who may
send one or what a receiver does with it before acceptance.

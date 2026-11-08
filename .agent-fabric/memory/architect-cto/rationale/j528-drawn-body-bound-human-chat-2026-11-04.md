---
role: "architect-cto"
class: rationale
topic: "j528-drawn-body-bound-human-chat-2026-11-04"
description: "j528 (2026-11-04): HUMAN-CHAT.md Bounds now caps what a human client draws of one body unasked (cut past 2,000 grapheme clusters or 40 model lines, preview at most 1,600/32, fallback and Show source cut the same, whole body in a reader on…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 263ae9d8942e14b5
---

## j528 (2026-11-04): HUMAN-CHAT.md Bounds now caps what a human client draws of one body unasked (cut past 2,000 grapheme clusters or 40 model lines, preview at most 1,600/32, fallback and Show source cut the same, whole body in a reader on the press); ADR-0050 rule 2 A 2026-11-04; on #562 at ca50e895d

2026-11-04, j528. rust-ui-dev-02's OBSERVATION 01a1a896-4c46 (their j92):
the desktop renderer keeps item geometry in i16 and panics once one
drawn item passes 32,767 px, measured at 1,350–1,500 one-letter list
items at scale 1 (16 KB of source); the plain-text fallback is as tall.
ux-designer-01's design note `design/2026-11-04-long-message.md` (#561)
chose cut-in-conversation + reader over drawing in pieces, and derived
the figures; their INFO 01a1a8a6-fc67 lowered them from 2,400/48 to
2,000/40 after review F9 (48 headings at 200 % on a Pixel 9a estimated
~33,000 px, over the ceiling; 2,000/40 gives ~27,500).

Ruled (REPLY 01a1a89e-9df5, DECISIONs 01a1a899-33db then 01a1a89e-ab37
to rust-ui-dev-02): the bound is the CONTRACT's, in HUMAN-CHAT.md's
Bounds bullet beside nesting and tables, for the reason the inline
nesting bound is ("a peer's message must never do that"); the earlier
line-box cap with paged source is withdrawn. Copy of the whole source
on every text message is allowed, on the press only, source and nothing
else, one write per press. Condition other figures must keep: tallest
whole body at the bound, narrowest layout, highest scale, under the
renderer's ceiling; a failed measurement lowers the figures, never the
text size. The cut rejects no envelope, changes no read/Keep/retention
state; the bridge forwards the whole text.

Landed on InterWeave #562 at ca50e895d (3 work, 1 fix): HUMAN-CHAT.md
Bounds sentence, ADR-0050 rule 2 + history note + table row, digest
entry + keyword row. TM-57 and its tests land in rust-ui-dev-02's j92
PR. Told rust-ui-dev-02 (INFO 01a1a8c5-48bd) and ux-designer-01
(01a1a8c5-498b). The look spec's "A very long message. The bubble
wraps." line (`design/2026-10-16-android-look.md` §8) is ux-designer's
to revise, which #561 does.

Round 4 (posted on ca50e895d): F5 P2 — the clusters' counting domain
was unstated and link controls (one drawn per link, full destination
as label) fell outside both figures: 2,000 empty-label links = one
model line, zero drawn clusters, 2,000 controls. Fixed at 30e5731ef:
clusters counted over `text` as sent, each link control counts as a
line, the preview carries only the controls of links it shows whole;
F6 lead "one body, received or sent"; F7 fit stated as the design's
estimate; F8 the 200 KB paragraph named a probe past the ceiling; plan
ruling (15) points its show-source line at the amendment. Risks left
for rust-ui-dev-02's measurement: one giant grapheme cluster
(combining marks) under both figures; Android's ceiling assumed the
same as the desktop's. Told both roles (INFO 01a1a8cc-df60/-dff8).
Lesson: `fabric-jobs deliver` before the review is posted blocks
post-review (job left active); retract + start reopens it.

Round 5 (on 30e5731ef): F5 partly — preview keyed on "text shown whole"
let a short-label link with a 150 KB destination carry its control.
Fixed at 31d0a32f3: a preview carries a link's control only when the
link's whole source, destination included, lies inside the shown text.
Round 6 clean, F5 fixed; posted. Third risk for rust-ui-dev-02: a
whole body of 40 link-control lines wrapping long destinations at 200 %
on the narrowest layout has no estimate (INFO 01a1a8d0-61d4). #562 final:
3 work, 3 fix at 31d0a32f3, six rounds posted; arms on the owner's word.

*Observed 2026-11-04 (architect-cto)*

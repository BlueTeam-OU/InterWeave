---
role: "architect-cto"
class: workflow
topic: "reflection-a-ruling-is-final-only-after-its-amendment-passed-review"
description: A ruling a lane will build on is final only after the amendment carrying it has passed the review class; until then the message says pending review and the reviewed ADR text binds, and when review changes it a numbered correction lists…
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0ba824ebb7120f30
---

## A ruling a lane will build on is final only after the amendment carrying it has passed the review class; until then the message says pending review and the reviewed ADR text binds, and when review changes it a numbered correction lists what differs

2026-09-27..10-01, both projects: "Withdraw my 'not intended'… the
blind review of the amendment I wrote from it found the answer wrong
twice" (01a0e351); "The blind review of PR #1009 corrected three points
of my census reply" (01a0ee33); "The supplier review of the ADR-0011
amendment found two P2s in my message's shape; build to this"
(01a0e655); DECISION 01a0e25e, tile rules revised after review; "the
ruling I gave you in 01a0e887 is not recorded yet and may change; I
should have said so then" (01a0e986). The architect's side of
[[reflection-cite-an-amendment-only-once-its-pr-is-on-main]]; a line
in the gzcoord-send skill for DECISIONs would carry it
(fabric-coordinator's).
Three more cases 2026-10-07..08: a P1 in my pull-queue ruling (01a1182e), two P2s on where the guard goes (01a119dc), the dead-letter livelock (01a11b46).

*References: reflection-cite-an-amendment-only-once-its-pr-is-on-main*

*Observed 2026-10-23 (architect-cto)*

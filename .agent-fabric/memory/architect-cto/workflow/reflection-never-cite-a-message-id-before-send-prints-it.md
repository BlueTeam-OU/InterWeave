---
role: "architect-cto"
class: workflow
topic: "reflection-never-cite-a-message-id-before-send-prints-it"
description: "a message id is cited only after gzcoord-send printed it; two replies on 2026-10-30 cited a guessed prefix (01a18b43, 01a18b49) that no message carries"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 946bc6f08ec8bfda
---

## a message id is cited only after gzcoord-send printed it; two replies on 2026-10-30 cited a guessed prefix (01a18b43, 01a18b49) that no message carries

2026-10-30: in two replies I cited the id of a message I was about to send or had just composed ("OBSERVATION 01a18b43", "REQUEST 01a18b49") by guessing the next prefix; the ids gzcoord-send minted were 01a18b42-5ba7 and 01a18b4f-884b, so the recipient cannot resolve the citation. A UUIDv7 prefix is time-based and not predictable at the second.

**Why:** the reader resolves ids against the relay; a wrong one is a broken reference in the record.

**How to apply:** send the message whose id is to be cited first, read the id from gzcoord-send's `sent seq N TYPE <id>` line, and only then compose the message that cites it; when two go out together, cite "my REQUEST to <login> of today, subject <...>" instead of an id. Related: [[reflection-the-repository-name-in-a-message-is-copied-never-composed]].

*References: reflection-the-repository-name-in-a-message-is-copied-never-composed*

*Observed 2026-10-30 (architect-cto)*

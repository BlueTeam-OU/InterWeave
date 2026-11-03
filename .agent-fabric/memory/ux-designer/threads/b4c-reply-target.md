---
role: "ux-designer"
class: threads
topic: "b4c-reply-target"
description: B4c reply line for a target not held — ReplyPresent if reachable (held/released/stored), ReplyUnavailable only if neither held nor stored
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b1a606c99a57e4dd
---

## B4c reply line for a target not held — ReplyPresent if reachable (held/released/stored), ReplyUnavailable only if neither held nor stored

rust-ui-dev-02 QUESTION 01a18ecc (2026-10-30): windowed mode shows a reply's target as unavailable once the ceiling releases it.
My provisional REPLY 01a18ecd: decide by reachability. ReplyPresent ("In reply to an earlier message") when held, released, or in store;
ReplyUnavailable only when neither held nor stored. Store has no by-id lookup on main; asked whether B4c can afford one, else interim ReplyUnavailable as a carried gap.
CLI JSON shape is architect-cto's (same QUESTION). Cost answer REPLY 01a18ece: not cheap in B4c (index+window read+model cache), interim ReplyUnavailable. Written as start-window s5.2 + item 17 in PR #518 (j85), worktree ~/projects/interweave-j85; Rounds 1-5 posted, round 5 clean at 4cb32263 (F1-F7 fixed). Carried risks: PR comment 6534562572. English 782daf3d folded (ff); round 6 clean at 782daf3d. Armed on REPLY 01a18ee2; MERGED 7631498a 2026-10-31. Delivered. Lookup is architect-cto j462; item 17 (b) waits on it.

*Observed 2026-10-30 (ux-designer)*

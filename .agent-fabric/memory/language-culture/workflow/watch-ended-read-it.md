---
role: "language-culture"
class: workflow
topic: "watch-ended-read-it"
description: a completed inbox watch is a delivery to read at once; never report the watch as running without checking
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 61cba232ed70764f
---

## a completed inbox watch is a delivery to read at once; never report the watch as running without checking

On 2026-11-16 (j334), I told the person "the inbox watch is still running". It had already exited 0 on a delivery: ux-designer-01's REQUEST, queued as j335. I read it only after the stop hook said that no watch was running.

**Why:** `gzcoord-inbox --until-delivery` exits when a message arrives. Its completion notice can come in while other work is going on, and an unread delivery holds the sender's fold.
**How to apply:** when a watch completion notice arrives, read its output and run the watch again before anything else. Before saying a watch is running, check that its task has not completed. Related: [[next-job-fresh-session]].

*References: next-job-fresh-session*

*Observed 2026-11-17 (language-culture)*

---
role: "ux-designer"
class: threads
topic: "j118-reader-opening-back"
description: "j118 — long-message note §5.4 \"While the reader opens\" (Back/Escape during the 0.4 s first go + 50 ms turns); written 050122b25 on PR #642, LC pass pending"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7812f67c0dd93751
---

## j118 — long-message note §5.4 "While the reader opens" (Back/Escape during the 0.4 s first go + 50 ms turns); written 050122b25 on PR #642, LC pass pending

j118 (2026-11-12): design/2026-11-04-long-message.md, commit 050122b25 on
develop-qzapp/ux-designer-01/feat/j115-add-a-peer-5-1-specify (PR #642, pushed). Written from
rust-ui-dev-02's A40 A/B (REPLY 01a1cfc2; my yes 01a1cfc3; my Back condition 01a1cc80 item 2).
Decided: a Back/Escape before the first screen is never lost, honoured at the end of the first go or
the next turn; closes to the conversation, focus on Open the whole message (§9 row extended), opening
stops; inside the first go no bar (first screen may flash, 0.26 s measured on list, accepted); past it
the bar goes with the reader; a second Back acts on the conversation (platform convention); A5 gains
the check (close within 1 s of Back, build reports Back-to-close and first-screen time).
Carried, not run: Back on the stress shapes (heading autolinks, reference link) and desktop Escape.
Note: the job text says "Read the whole message"; the label is OpenWholeMessage "Open the whole message".
Asked: LC English pass REQUEST 01a1d005-fe88 (for/ branch off 050122b25); INFO to rust-ui-dev-02 01a1d006-0617.
Next: fold LC's for/ branch as a word diff (conditions a-f in the request), then it rides #642's review.
Related: [[j90-long-message]], [[j115-connect-hold]], [[wording-pass-keeps-conditions]].

*References: j115-connect-hold, j90-long-message, wording-pass-keeps-conditions*

*Observed 2026-11-12 (ux-designer)*

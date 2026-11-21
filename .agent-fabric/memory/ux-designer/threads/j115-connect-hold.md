---
role: "ux-designer"
class: threads
topic: "j115-connect-hold"
description: "j115 — add-a-peer §5.1 holds C4/R4 after Continue until Connect is decided (1 s progress, 10 s bound): delivered as #642 (merged 8f4c4c042); carried risks in PR comments"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6915a32746b4c102
---

## j115 — add-a-peer §5.1 holds C4/R4 after Continue until Connect is decided (1 s progress, 10 s bound): delivered as #642 (merged 8f4c4c042); carried risks in PR comments

**State, 2026-11-13: delivered and closed.** #642 merged as 8f4c4c042 on 2026-11-12, after five review rounds. Rounds 3 to 5 came back clean. language-culture passed every sentence: j292, j294-j298.

What #642 decided, in design/2026-10-22-add-a-peer.md §5.1 ("Between Continue and the decision", "A launch with Connect still owed"), §7, §8 item 12 and §9 `ConnectStarting`:
- No flash of Chats. Continue is disabled at once.
- "Starting InterWeave…" shows after 1 s.
- The seal comes first. Its outcomes stay as identity note §6 has them, and the hold starts at the seal's success.
- A 10 s bound counts from the commit. A prompt pauses it.
- At the bound, Chats opens with the start counted as deferred, and the start goes on.
- FLAG_SECURE is neither set nor cleared at the press.
- Back on C4/R4 does nothing.
- A launch opens Connect over Chats only if Chats is untouched within 10 s.
- The evidence is heuristic: Nielsen's visibility of system status, and the response-time limits. It was not tested with people.

Carried:
- Risks are in #642's PR comments 6826387397 and 6829493256:
  - the grant is not a trace point;
  - the order of the start hook against a refusal;
  - the "busy profile (IdGoingOffline)" leftover in §5.1's outcome list, which cannot be reached because the seal holds the lock;
  - the prompt-result line also times the 10 s restart.
- Identity note item 15 step 3 was revised on main (2026-11-13): Connect or Chats first after R4, never Chats then Connect.
- Follow-on: #651/#658's launch count is job j124. See [[connect-owed-since-launch]] and [[j121-launch-rule-pr655]].

Lessons are in their own memories: [[lc-pass-before-arm]], [[review-rounds-mechanics]] (post before the job leaves active), [[wording-pass-keeps-conditions]].
Related: [[j117-marker-log-text]], [[j114-item15-revision]].

*References: connect-owed-since-launch, j114-item15-revision, j117-marker-log-text, j121-launch-rule-pr655, lc-pass-before-arm, review-rounds-mechanics, wording-pass-keeps-conditions*

*Observed 2026-11-13 (ux-designer)*

---
role: "rust-ui-dev"
class: threads
topic: "j216-locked-check-status-pr578"
description: "j216 delivered on #578, merged 20ce43c8 2026-11-06 with j217, which closed the cached-ready -10 restart gap"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b773e27e95c49bf7
---

## j216 delivered on #578, merged 20ce43c8 2026-11-06 with j217, which closed the cached-ready -10 restart gap

2026-11-05: #577 merged as 570fb5829 while already in the merge queue, so a push onto its branch was refused ("Branches that are queued for merging cannot be updated"). The j216 fix therefore became the first commits of #578 instead:
340aaf088 (android-platform `ServiceHost::locked_before_start`) and 2928ffa7a (human-android: `identity_state`'s two AUTHENTICATION_NEEDED arms call it). Merged with main as cf55ad5a6. #578 is unarmed (2 work, 0 fix). Later human-crates jobs add commits to it until it reaches the minimum, then it gets a blind review and is armed.
INFO 01a1ada3-240f to ux-designer-01 gave them the commit IDs.

Open gap (ux QUESTION 01a1ada3-a70b, answered in REPLY 01a1ada5): `MainActivity.identityReady` is process-wide and never cleared.
- A later service start refused -10 within the same process sets the unlock status but not `authentication_needed`, and asks for no prompt. The store answer is `OtherRefusal`, so the chats stay on screen.
- Result: no Unlock and no prompt until the process ends.
- Routing it to the locked notice is mine to build once ux-designer-01 specifies it. If they ask, queue it as a job and put it on #578.

2026-11-05 later: j215's test fix 3f3e1d2cf joined #578 (3 work, 0 fix, unarmed).
ux-designer-01 specified the gap as #579 section 11, cases L1-L4, at head e3a8937f8. Not merged yet: it waits for an English pass and the owner's word. That message was REPLY 01a1adb3.
My REPLY 01a1adb4 answered their question: the service never retries a refused locked start, on a timer or at a network change. A sticky recreate after process death is a platform restart (L4).
Building L1-L4 is owed once #579 lands. It is not queued as a job yet.

Lesson: when #N is armed, check `mergeQueueEntry` (GraphQL) before planning a push onto it. Once it is queued, follow-up work goes on the next branch.

*Observed 2026-11-05 (rust-ui-dev)*

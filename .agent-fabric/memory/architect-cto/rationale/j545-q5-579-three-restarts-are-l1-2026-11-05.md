---
role: "architect-cto"
class: rationale
topic: "j545-q5-579-three-restarts-are-l1-2026-11-05"
description: "Q5 on #579 (ux-designer-01, 01a1adac-7f85): the store recovery Try again, a discard's restart and the restart after a network grant are L1 (the prompt rises); ruled 2026-11-05 (REPLY 01a1adae-487d); custody doc names the three at…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a4eea842d86fddc1
---

## Q5 on #579 (ux-designer-01, 01a1adac-7f85): the store recovery Try again, a discard's restart and the restart after a network grant are L1 (the prompt rises); ruled 2026-11-05 (REPLY 01a1adae-487d); custody doc names the three at 84b711958 on #575

2026-11-05, ux-designer-01's QUESTION 01a1adac-7f85 (#579, design/2026-11-05-unlock-policy-pending.md §11, Q5): in a process that keeps running, which refused `UserNotAuthenticated` restarts does the prompt answer. Ruled (REPLY 01a1adae-487d, seq 864562): all three are L1. ADR-0042 A 2026-10-20 takes exactly one refusal out of "the prompt answers it": a live Activity's CONTINUATION after a cancel (return from Home/Recents, configuration recreation), because a person who declined is not asked again each time they switch back in. What sorts a case is what started the restart, not the lifecycle callback: a network grant is the person's answer to a dialog over the Activity, landing in onResume, not a switch back in. No ADR change; naming the three in `android-key-custody.md`'s failure list (user-presence bullet, the "every other refusal" examples) is propagation.

DELIVERED 2026-11-05: commit 84b711958 (`Kind: work`) on #575, F1 of round 6 (§11 not at this head) answered by f3398371c (cited "on ux-designer's #579 at 17b8bf95e at the time of the ruling"), now 4 work, 5 fix, pushed; the sha sent to ux-designer-01 as REPLY 01a1adb2-0ddd (seq 864614). #577 edited line 25 of the same file (j544), a different line; no conflict. Round 6 posted (review 5792696973, one P3, its 19 merged-in paths unread so not coverage); round 7 (review 5792771359) is the full clean review of 570fb5829..f3398371c and is the coverage; round 8 (F1 judged fixed) could not be posted as a review object (post-review refuses a base that is neither the merge-base nor the last checkpoint's head) and is PR comment 6670754939, so the arm takes `--defer "84b71195:F1"`; the PR awaits the owner's word. Lessons: (1) a range whose base predates a fold of main carries main's paths and does not count as coverage; range from the merge-base. (2) Post the fix-range re-review BEFORE a fresh full review of the head: once a checkpoint at the new head exists, the narrower re-review is refused and the finding chain can only be closed by --defer.

**Why:** the 10-20 exception's reason is the person's declining, so only an un-acted continuation qualifies; everything the person starts gets the prompt.
**How to apply:** sort a refused restart by its cause, never by the Android callback it arrives in. See [[j544-577-f3-android-key-custody-line-25-wording-2026-11-05]].


**CLOSED 2026-11-08:** #575 (5 work, 6 fix) merged 4c4cf3587 at 2026-11-08T06:16Z on the owner's word 'arm 575' at head 2d6ca2bf3; F1 P3 lifted by --defer (cited §11 L1 on main via #579, 4w/3f, 62ba5cf01).

*References: j544-577-f3-android-key-custody-line-25-wording-2026-11-05*

*Observed 2026-11-08 (architect-cto)*

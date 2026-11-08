---
role: "ux-designer"
class: threads
topic: "j96-locked-restart-in-process"
description: "j96 — a restart refused for the screen lock in a running process; delivered as unlock note §11, #579 merged 62ba5cf01"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 810aadbf68eb0bb9
---

## j96 — a restart refused for the screen lock in a running process; delivered as unlock note §11, #579 merged 62ba5cf01

j96 (2026-11-05) answered rust-ui-dev's REPLY 01a1ada5. In a process that had once found the identity ready,
a start refused UserNotAuthenticated (-10) got no prompt and no locked notice.

Delivered: §11 of design/2026-11-05-unlock-policy-pending.md, in #579, merged 62ba5cf01. It went through five
blind review rounds (findings F1-F9, all fixed). language-culture-en's English pass (j247) was folded, and
#579 was armed on architect-cto's AUTHORIZES (01a1adc0). rust-ui-dev was told; the build is theirs to queue.

The rule (ADR-0042 A 2026-10-20, Q5 ruled by architect-cto 01a1adae; custody line 84b711958 on #575):
- The prompt answers every refusal. The ONE exception is a live Activity continuing (a return, or a
  config recreation) after a cancelled or denied prompt, with no start run since: the notice and Unlock.
- A refused start is never "a start that ran".
- Cases are sorted by what started the restart, never by the lifecycle callback:
  - L1: the person's action (R's Try again, a discard restart, S2's Done, a network grant).
  - L2: the exception above.
  - L3: every other start in front of the person (a launch, a return with no cancel, a foreground
    Stay-reachable restart).
  - L4: a background restart. No prompt. The next launch is L3; the next return is L3, or L2 after a cancel.
- While locked, the list, an open conversation and the composer with its draft all stay (look spec §6).
- rust-ui-dev (01a1adb4): the service never retries a locked start itself.

Mistakes the review caught, worth not repeating:
- I first called every return the exception, but the ADR's exception needs the qualifier "after a cancel".
- I hid the composer against a recorded rule.
- A wording answer ("L2 cannot follow L4") rested on a false reason.

Carried: no acceptance item for a launch after an L4 refusal. Add that step to the Pixel run, j95.

Build: rust-ui-dev's #578 part 1 (fa9dfde0b) routes L1-L4; part 2 (list/draft kept) in progress.
In j95, check that focus on the notice's Unlock reads the notice text too (REPLY 01a1ae33): as built,
the button says only "Unlock", and the live region does not fire at an L2 return.
Related: [[j93-unlock-policy-pending]], [[no-scope-widening-in-review-fixes]].

*References: j93-unlock-policy-pending, no-scope-widening-in-review-fixes*

*Observed 2026-11-06 (ux-designer)*

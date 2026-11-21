---
role: "ux-designer"
class: threads
topic: "j121-launch-rule-pr655"
description: "j121 delivered on #655 (add-a-peer §5.1 launch count, item 12 checks; identity §8 SR activate, step 3 by identity=closed); 4 review rounds, LC j302+j303 folded; awaits owner's arm word"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b7505388190e828a
---

## j121 delivered on #655 (add-a-peer §5.1 launch count, item 12 checks; identity §8 SR activate, step 3 by identity=closed); 4 review rounds, LC j302+j303 folded; awaits owner's arm word

2026-11-13, PR #655, head 5ff01009f, round 4 clean (review 5885405579); 4 work, 5 fix; under the floor, so arming needs the owner's word.
- add-a-peer §5.1: the count starts when Chats shows at this launch. Openings before the start succeeded count, a menu closed again counts, a launch into a conversation counts (none exists on Android; notifications open Chats). A later start in the same process counts from its ask.
- #651 differs in two ways (count from the service start; a menu only while open): REQUEST 01a1d2dd to rust-ui-dev-01, for their j268.
- Item 12 uses Peers, menu and Settings openings, since with Connect owed Chats lists no conversation. The queued case is blocked until a host test of the root's order exists.
- Identity acceptance 15 step 3 reads R4's going by `identity=closed`. Chats = no `page=connect` after it. Fail = `connect=owed` last before it, then `connect=open`. Step 2's new window writes `page=conversations` after closing, because the Tracer starts with page "".
- SR-activate check blocked on both clients (desktop has no identity screens); host test owed by rust-ui-dev.
Lesson: an acceptance case must be set up on the state it needs (no conversation exists with both lists empty).
Related: [[connect-owed-since-launch]], [[sr-activate-is-press]], [[lc-pass-before-arm]].
rust-ui-dev-01 REPLY 01a1d2dd-f4e8: 3b (SR-activate host test a_screen_reader_activate_on_p_after_a_presses_p_again) on #654; 1 and 3a in j268 after #655 lands. My REPLY 01a1d2de corrected their 'count from a successful start' to 'from Chats showing'. Owed: check j268 against that anchor.
2026-11-13: #655 merged as e781aa60d on the owner's 'arm 655'. One queue eviction came from a flaky android-e2e admin_deputy timeout; re-armed on the same head. j121 delivered.

*References: connect-owed-since-launch, lc-pass-before-arm, sr-activate-is-press*

*Observed 2026-11-13 (ux-designer)*

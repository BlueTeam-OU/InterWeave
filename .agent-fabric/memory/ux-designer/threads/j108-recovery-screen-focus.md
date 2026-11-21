---
role: "ux-designer"
class: threads
topic: "j108-recovery-screen-focus"
description: "j108 — focus when a refused start raises R, K or K1 (identity note §8, §10 item 15); PR #609"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cd51120113db4386
---

## j108 — focus when a refused start raises R, K or K1 (identity note §8, §10 item 15); PR #609

j108 (2026-11-10): the j107 Pixel run traced focus=none when R rose (REFUSE_STORE+RESTART).
Decided, commit 3413c4907 on PR #609 (branch develop-qzapp/ux-designer-01/feat/j108-identity-note-say-where-focus-goes):
R, K and K1 raised by a refused start, at a launch or over another screen, put focus on their HEADING (made focusable),
any IME closes, and Tab follows the reading order to the primary action. Why the heading and not the primary: the person
expected another screen, so it says what happened before what to do. The desktop R (a dialog window) keeps Try again as its initial focus.
R staying after a refused Try again keeps focus on Try again (unlock F11/A15).
Checks: R by the ui-trace focus= line with REFUSE_STORE+RESTART over Chats, over a conversation with the IME up, and at a launch;
K1 with identity §10 item 9's hook; K waits on a key-invalidated store hook (rust-ui-dev-01, not yet asked).
The contact-cards focus table has a pointer row. 2026-11-10 ~05:10Z: review r1 (3 findings, all judged real) -> fix 97069d33a (heading key named by rust-ui-dev, launch setup, K1-over-screen waits on hook);
r2 F4 (Home is a return, not a launch) -> fix 997a83c87; r3 clean. All posted. Carried risks: PR comment 6759516448.
LC English pass taken as language-culture-en j261 (REPLY 01a1c3d7). Build REQUEST 01a1c3d9 to rust-ui-dev-01 (heading keys + hooks a/b), reply expected.
Next: fold LC's commits, re-review their range, then arm on the owner's word (1 work, below minimum).
1 work commit: arming needs the owner's word (or the dry-run's authority).
Related: [[j98-unlock-s11-pixel-review]], [[focus-table-in-spec]], [[define-once-point-elsewhere]].
2026-11-10 ~05:20Z: rust-ui-dev-01 took build + hooks as j246 (REPLY 01a1c3da), keys r-heading/k-heading/k1-heading, build starts once #609 is on main.
r4 F5: lib.rs:1611 moves focus to Chats only for focus ids starting "identity-"; told rust-ui-dev-01 (01a1c3dd); item 15 checks R leaving with focus on r-heading -> conversations-heading.
r5 F6 (Tab moved focus off the heading) fixed with Shift+Tab; r6 clean. LC pass folded (merge d2158d099); Q1/Q3 answered ee8910715;
LC asked for a pass on ee8910715 (01a1c3e2). Next: fold it, then ONE review (rr7 json in session scratchpad) of 5d22d2984..head, then the owner's word to arm.
Carried risks: user-presence 60 s window for the hook runs, narrow-conversation half of F15 unchecked.
LESSON (r9 F9): key_unlock_policy has only background-compatible and user-presence; "policy none" in my notes meant background-compatible. Grep config.schema.yaml before naming a config value in a check.
r7 F7 (platform restart refused again: check + table row), r8 F8 (Try again's own refused start: step, key identity-primary), r9 F9 fixed; r10 running.
2026-11-10: LC's j262 pass folded (ff e0cdd0a73); r11 re-review CLEAN, posted. #609 (6 work, 7 fix) unarmed:
under 8. Under the owner's rule of 2026-11-09 (no agent waits on the owner to arm), the arm was asked of
architect-cto-01 by REQUEST 01a1c449-6777 (AUTHORIZES BlueTeam-OU/InterWeave#609@e0cdd0a7); arm with
--authorization <their id> when it comes. j108 delivered; the Pixel run of item 15 is j109, blocked on rust-ui-dev-01/j246.
architect-cto-01 AUTHORIZED #609@e0cdd0a7 (REPLY 01a1c44a-eaed-7680-9f37-105979421643, AUTHORIZES in header; this head only).
NOT YET ARMED: the job hook refused publishing from the j108 session after the switch. Next session: check head is still
e0cdd0a7, then `fabric-pr arm 609 --authorization 01a1c44a-eaed-7680-9f37-105979421643 --basis "architect-cto's word ..."`.
LESSON: arm before `fabric-jobs deliver`/switch; after the switch the session can no longer publish.
2026-11-10: ARMED #609 at e0cdd0a7 on 01a1c44a (queued at 1). A local fold of origin/main (2306cafd5, pure automatic
merge, tree checked by merge-tree) was dropped unpushed: pushing it would have disarmed the PR and voided the head-bound authorization.
2026-11-10 07:34Z: #609 MERGED 3300e5b5 (6 work, 7 fix). INFO 01a1c462 told rust-ui-dev-01 j246 can start; j109 waits on that build.

*References: define-once-point-elsewhere, focus-table-in-spec, j98-unlock-s11-pixel-review*

*Observed 2026-11-10 (ux-designer)*

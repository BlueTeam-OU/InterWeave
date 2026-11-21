---
role: "ux-designer"
class: threads
topic: "j114-item15-revision"
description: j114 — identity note §10 item 15 revised after the A40 run (launch focus=none, Tab round, Connect after restore); waits on the English pass, then PR
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 26782350b3f5c1a9
---

## j114 — identity note §10 item 15 revised after the A40 run (launch focus=none, Tab round, Connect after restore); waits on the English pass, then PR

j114 (2026-11-12): commit d4bbf2456 on develop-qzapp/ux-designer-01/feat/j114-identity-note-item-15-kept-k1 (pushed),
design/2026-10-19-android-identity.md §10 item 15, three changes from the A40 (Android 11) run of j109/j110:
1. focus=none in the same millisecond just before the heading key at a launch is the new window's empty start, not a fail
   (rust-ui-dev-01 REPLY 01a1ce5b; my answer 01a1ce5b-2abf).
2. Where adb cannot send Shift+Tab (A40, Android 11), Tab round the cycle to R's heading.
3. Kept-K1 step 3: Connect to someone after Chats when no peer is known is correct (add-a-peer §5.1).
English pass asked of language-culture-en: REQUEST 01a1ce62-2a76 (job blocked on it). Next: fold their pass unrebased,
open the PR (1 work commit, under 8 -> owner's word to arm), one blind review via fabric-review brief.
Related: [[j110-kept-k1-checks]], [[j108-recovery-screen-focus]].
Side thread (2026-11-12): R2 Enter after "art" doing nothing on the A40 is a defect, taken by rust-ui-dev-01 (REPLY
01a1ce62-4a13): host test + count-only picker trace, then a branch and APK for me to re-run without the owner.
LC pass: aa80b729b on develop-qzapp/language-culture-en/for/ux-designer-01/j114-item-15-english (fold unrebased).
Step 3 meaning sent to LC (REPLY 01a1ce66): cite §5.1, "no address and no peer", Connect may follow a brief Chats
(build opens Connect only after the peer-list and trust reads answer, ui-model connect_turn). The unspecified
pending state is a separate low job, not j114. Waiting on LC's reworded step 3 before folding + PR.
PR #633 opened (LC's aa80b729b+f2d7418ae folded ff). r1 review posted on f2d7418a: F1 P2 (step 3 never failed a wrong
Connect; fix reads the connect= trace line, deferred = void) and F2 P3 (evidence sentence named only Chats); both fixed
in def7feafd. LC pass on the fix asked: REQUEST 01a1ce6e. Next: fold it, re-review (fabric-review next from r1 req
with --pr 633), post, owner's word to arm (3 work, under 8). Requests/briefs in the session scratchpad r1-*.
OWNER'S WORD in session 2026-11-12: "arm when review end" (#633, under 8). Plan: r2 re-review on 042728186 running; fold LC's
pass on the Q4 clause (REPLY 01a1ce80-a7e0), short re-review of it, then fabric-pr arm 633 --basis "the owner's word ...".
R2 Enter side thread: rust-ui-dev-01 found the cause (keys dispatched before the zero-delay input timer) and fixed it on
PR #634 (006259e90); APK to follow (REPLY 01a1ce8b). When it arrives, add a job: device re-run of "art"+Enter on R2.
ARMED #633 at 3860f1db (6 work, 7 fix) on the owner's word, 2026-11-12; r3 clean (F1-F3 fixed). Carried: r3 risk "how to place
a log line vs R4's continue" (identity= lines); §10 line 909 Shift+Tab reverse-order check has no Tab fallback (pre-existing).
j116 APK: A host scratch path sha256 47ceafb0... (same debug key as j246); tell rust-ui-dev-01 once
installed so they remove both /var/tmp drops.
j117 note: the marker-text commit is 4fa1b3b9f (#634 head), not a3cf5c1b8 (rust-ui-dev-01's correction 01a1ce9f-a79f).
DELIVERED: #633 merged 20e48edc4 (2026-11-12). Next jobs: j116 A40 run (needs owner address), j117, j115.

*References: j108-recovery-screen-focus, j110-kept-k1-checks*

*Observed 2026-11-12 (ux-designer)*

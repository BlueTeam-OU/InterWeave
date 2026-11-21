---
role: "rust-ui-dev"
class: threads
topic: "j246-heading-focus"
description: "j246 R/K/K1 heading focus — #611 a346f9edb clean, AUTHORIZED by architect-cto 01a1c523 (2026-11-10); arm with --authorization; ux check of SECURE_ON at K1 owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e5fb9456e8faa057
---

## j246 R/K/K1 heading focus — #611 a346f9edb clean, AUTHORIZED by architect-cto 01a1c523 (2026-11-10); arm with --authorization; ux check of SECURE_ON at K1 owed

j246 (REQUEST 01a1c3d9 from ux-designer-01): PR #611 opened 2026-11-10, head 46f91361e
(364abef11 ui-slint, 8cb5d2a89 android-platform, 46f91361e human-android), 3 work commits,
under the 8 minimum, so arming needs the owner's word or the arming authority. APK
a host scratch path (sha aa19513c…). REPLY 01a1c50b sent to ux with keys + hooks.

Design: IdentityPage.heading-key makes the title a FocusNote stop in segment 0 (after the
arrow, before the banner); window in-out identity-heading-wanted, which IdentityScreen takes in
init / changed page / changed heading-wanted. `heading-held` keeps the rebuilt page's own rule
(again(at-seg)) from overriding the heading when K replaces R with the overlay up: Slint ran
`changed heading-wanted` BEFORE `changed page`, so focus went back to Try again (caught by a
test). lib.rs on_identity_page() counts the three keys (F15 gate).

Found: before #611, a Service start answering RecoveryRequired in a running process raised
nothing (only identityState at a launch made K1). Built: jni start → onboard::recovery_refused
→ activity::raise_recovery → onboard::raise_recovery (Over::Running → Ask.RESTART, new
Ask.SECURE_ON=16). Onboarding's bools hit clippy's 3-bool limit only on the Android target:
run check_android_target.sh with the clippy wrapper before pushing app changes.

Related: [[android-target-clippy]] [[slint-changed-handlers-deferred]] [[slint-rebuilt-page-focus]]

**State 2026-11-10 ~11:10Z.** Round 1: F1 (P1) a K1 kept by recovery_refused past its window
reopened as a launch flow (ready_state said READY, so no FLAG_SECURE; BIND no-op on a kept
binding) and F2 (P3) hook's once untested; both judged real (review 5840622306). Fixes
5e06eb4bd/383a72258 (ready_check.rs: kept → RECOVERY; start() uses Over::Running + SECURE_ON),
then fa2c10f11 (test gated on the crate's own dev-stand-ins never ran in CI: no workspace member
enables it → use any(test, feature)) and a346f9edb (start_answered(ran) drops a kept K1).
Second re-review 5840791522: both fixed, no finding. APK a host scratch path (sha 2f1fcc5e).
REPLY 01a1c520-c1d9 to ux. Arm: `fabric-pr arm 611 --authorization <id>` once architect-cto
answers REQUEST 01a1c520-7868 and CI is green. Lesson: a test gated on an app crate's own
feature is compiled out of `nextest --workspace`.

**Authorized 2026-11-10 11:06Z**: architect-cto REPLY 01a1c523-3ac6 AUTHORIZES #611@a346f9edb.
Arm: `fabric-pr arm 611 --authorization 01a1c523-3ac6-71fc-b3ae-962c46f5ca85`. Do not push
first. Also owed: ask ux-designer-01 to check the window flag (FLAG_SECURE) at K1 in the item
15 run. The flag is set by timing (relayIdentity, no acknowledgement). If it is ever seen off
at R2, gate R2 on an acknowledgement as showSecured does; that is a new job, not #611.
Second AUTHORIZES REPLY 01a1c54b-8911 (11:50Z), same head. The job blocker still waits on
architect-cto-01/j594 (active, not closed). Carried items, owed after the arm: ask ux-designer-01
to add to item 15 on the Pixel: REFUSE_IDENTITY; the Activity finished before K1 ends; relaunch.
Check FLAG_SECURE is on before K1 draws, and that the end of the restore starts the client. The
kept-K1 paths in onboard.rs compile for Android only, so no host test covers them. If R2 is
reachable before the flag goes on, that is a P1 follow-up. render() dropping a kept K1 when
Show my phrase ends predates #611; it is mine to schedule as a new job.

**Armed 2026-11-10 ~12:00Z** with `--authorization 01a1c54b... --defer-job j246` (the arm refused while j246 was blocked on its own PR). ux item-15 checks sent as REQUEST 01a1c551.

**ux item-15 checks done 2026-11-12** (REPLY 01a1ce56 from ux-designer-01, on the Samsung A40 at the owner's choice, not the Pixel; APK a346f9edb): all three pass. FLAG_SECURE is on at K1 (first reading 50 ms after identity=k1), at R2 and at K1 after a relaunch. No reading showed it off. Not covered: the ~50 ms before the first dumpsys poll, so whether K1's or R2's first frame drew unflagged is not ruled out. K1 is kept across the Activity (same pid, relaunch reopens K1). The end of the restore starts the client (NetworkService up, then the onboarding's Connect step). So the acknowledgement-gated R2 follow-up is not triggered by evidence; it would only be needed if a frame-exact check ever shows an unflagged frame. ux-designer-01 is checking "art" not being reachable by keyboard from adb against item 15's Enter rules (theirs to raise).

*References: android-target-clippy, slint-changed-handlers-deferred, slint-rebuilt-page-focus*

*Observed 2026-11-10 (rust-ui-dev)*

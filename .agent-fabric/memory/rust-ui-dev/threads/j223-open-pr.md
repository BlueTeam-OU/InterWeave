---
role: "rust-ui-dev"
class: threads
topic: "j223-open-pr"
description: "j223 / PR #582 (j218+j220+j222): ARMED at a69dcabb on the owner's word 2026-11-07 (5 work, 1 fix); device checks owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ffdf107f02f6b15f
---

## j223 / PR #582 (j218+j220+j222): ARMED at a69dcabb on the owner's word 2026-11-07 (5 work, 1 fix); device checks owed

2026-11-07: the owner asked in session to open the PR for j218, j220 and j222.
- `fabric-jobs switch` made branch feat/j223-open-the-pr-for-j218-j220 off origin/main, in worktree interweave-j223-open-the-pr-for-j218-j220.
- Its first step is `git merge --ff-only bbdfad4a8`. That is the j222 head on origin, carrying c01eb6a70 (j218), 0e1d4d175 (j220), f8278f236 and bbdfad4a8 (j222): 4 work commits. Alternatively, open the PR from the j222 branch itself.
- Then: fold origin/main, run the full suite (cargo xtask ci, plus the Gradle debug+release build for verifyReleaseNative), open the PR, dispatch the blind review, and post it.
- Arming: 4 < 8, so it needs the owner's word. Persistence (the store open) is touched, so treat it as a security boundary: the review is required before the arm.
- Already verified on the j222 head: the hook test is mutation-checked; clippy passes with and without dev-stand-ins; check_android_target passes; debug and release APKs build; the release .so has no DebugHooks entry.
- Device evidence: ux-designer-01 runs j98 on bbdfad4a8 (both hooks). No device run of mine.

Related: [[j222-discard-hook]] [[j220-restart-hook]] [[j218-one-unlock-prompt]]

2026-11-07 progress: PR #582 opened (branch feat/j223-open-the-pr-for-j218-j220 ff'd to bbdfad4a8, already on main).
- xtask ci: all green except desktop-e2e daemon.rs lock-wait marker case (load-race, p2p-network-dev-02's j92; passes 5/5 alone; OBSERVATION 01a1b630, verbatim output REPLY 01a1b632-a3fa).
- Review r1 at bbdfad4a: F1 P2 real (starts count read when start answers; hook's runtime stopped by the replaced start's waiter). Fixed 7acf6e8d7 (snapshot at startClient queue time; availability restart under synchronized(binder)). r2: F1 fixed, Finding none. Both posted.
- Arm dry-run: only the count blocks (4 work, 1 fix). Needs the owner's word or architect-cto AUTHORIZES #582@7acf6e8d.
- Open risks from review (not findings): R1 prompting stuck if a prompt never calls back; R4 onboard.rs asked()/unasked() wiring untested; R6 stale late waiter. Device check owed: two back-to-back RESTART broadcasts.
- Later 2026-11-07: CI's android apk job went red: lintDebug UnspecifiedRegisterReceiverFlag on DebugHooks' pre-33 registerReceiver (j222 verification built APKs but never ran lintDebug -- run `./gradlew :app:lintDebug :app:lintRelease` before pushing Android Kotlin). Fixed a69dcabb7 (@SuppressLint on register; R7: annotation covers the 33+ call too). r3 clean, posted; armed on the owner's word "arm 582".

*References: j218-one-unlock-prompt, j220-restart-hook, j222-discard-hook*

*Observed 2026-11-07 (rust-ui-dev)*

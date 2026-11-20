---
role: "rust-ui-dev"
class: threads
topic: "j229-phone-order-step2"
description: "j229 phone order 2026-11-08: CLOSED 2026-11-09 (step 2 done, step 6 run: 1 pass, 1 test defect queued as j240); read packages inside the lease before a run"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a3898afffecf7c7a
---

## j229 phone order 2026-11-08: CLOSED 2026-11-09 (step 2 done, step 6 run: 1 pass, 1 test defect queued as j240); read packages inside the lease before a run

- Step 1 finished 13:15Z. p2p-network-dev-01's INFO 01a1bb4c says both phones now hold the test-signed release of 4c4cf358 with a TEST identity. Addresses: Pixel a private-network address:38897, A40 a private-network address:39716.
- The debug APK of main 3e3c6b902 is at a scratch path, sha256 f5144dfe. To rebuild: `gradlew assembleDebug` in apps/human-android/android (see [[android-shell-build]]).
- The scripts are a scratch path (args: serial, apk, outdir) and a40.sh (own adb port +1, run under `fabric-lease samsung-a40`).
- 14:30Z: the auto-mode classifier refused the install script that contained `adb uninstall` ("Irreversible Deletion"). The owner's word does not lift that check. Nothing was run on either phone.
- OBSERVATION 01a1bb59 went to fabric-coordinator, asking for an auto-mode.json change or for step 2 to be dropped. INFOs 01a1bb59-86fc and -8ed3 told rust-ui-dev-02 and ux-designer-01 that step 3 goes ahead on the release build.
- If the change is cleared: run the Pixel through phone-run, then the A40 through a40.sh. Then tell rust-ui-dev-02 and ux-designer-01.
Related: [[j228-host-node-for-a17]] [[a40-own-adb-server]]
- 13:54Z REPLY 01a1bb70 from fabric-coordinator: option (a). The auto-mode entry is 654eb3b8 on agent-fabric #510 (adb uninstall / pm clear for org.interweave.*, Pixel and A40, only under the lease). It takes effect after #510 merges, `fabric-ctl all upgrade fabric` runs and I relaunch. The coordinator will say when. Then rerun step 2 unchanged. The A40 now carries rust-ui-dev-02's debug build of #585 with no identity (INFO 01a1bb5c); the Pixel still has step 1's release.
- 15:27Z INFO 01a1bbc5 from fabric-coordinator: #510 merged f8f91855 and has been distributed. Relaunch, then rerun step 2 with each uninstall inside the lease wrapper. The Pixel goes first, because ux-designer-01's 3b and 4 wait on it. The A40 still needs the uninstall of #585's debug build.
- 2026-11-08 16:05Z: STEP 2 DONE after the relaunch. Pixel (API 37) and A40 (API 30): each had its org.interweave.* packages uninstalled under its lease, then got the debug APK of 3e3c6b902 (same app as ec98eaf31). The A40 also lost org.interweave.human.test. Logs are in each phone's run.log under a scratch path. The Pixel's window has FLAG_SECURE, so screencap is black; a black screencap alone is no fault. INFOs went out: 01a1bbe9-2d09 to ux, -8d13 to rust-ui-dev-02, -8e17 to the coordinator. Left for me: j228 support during 3b, and step 6's optional app_lifecycle cases on the A40, last.
- 2026-11-08 22:33Z: only step 6 is left: j224's optional app_lifecycle cases on the A40, last. Both leases were free, but no message said step 5 (p2p-network-dev-02, j69 row 2) had run. QUESTION 01a1bd49 asked them; j229 is blocked on its answer. The A40 carries rust-ui-dev-02's j95 build 73006e66b (signer 26d1eb18, INFO 01a1bc1f). It showed a secure keyguard at 17:05Z.
- 22:35Z: p2p-network-dev-02 started step 5 at 22:32Z, holding both leases for ~45 min (REPLY 01a1bd4a). They will say when they release them. My REPLY 01a1bd4a-d8f2 listed what is on each phone. j229 is blocked until 23:20Z; step 6 starts when they release the A40.
- 2026-11-08 22:50Z, j238 (REQUEST 01a1bd4c from p2p-network-dev-02): one debug and androidTest pair at d1be8eb89, built in the detached worktree ~/projects/interweave-j238-d1be8eb89. APKs are in a scratch path; app sha 4f0a2140, test sha 1efb4b15, signer f8b1b43e.
  - A40 DONE: the old packages were uninstalled and the new ones installed (REPLY 01a1bd5a).
  - Pixel: install -r both through phone-run with `a scratch path <addr> <app> <test> replace`. It waits on the address the coordinator gets from the owner (REPLY 01a1bd4e). Then REPLY to 01a1bd4c with the sha256s.
  - Step 6 (my app_lifecycle cases on the A40) uses this same build, after step 5.
  - Remove the j238 worktree when done.
- 2026-11-09 08:10Z: j238 DELIVERED. Pixel install -r of the d1be8eb89 pair through phone-run (REPLY 01a1bf5b). Both phones hold the pair, signer f8b1b43e. p2p-network-dev-02 now runs step 5 under both leases. Step 6 comes after their release: app_lifecycle optional cases on the A40, run from ~/projects/interweave-j238-d1be8eb89 (keep that worktree until then). The A40 can lock on timeout: keep the screen on with svc power stayon true inside the lease, and set it off at the end.
- 2026-11-09 08:23Z: STEP 6 DONE, so j229 is closed. rust-ui-dev-02's j95 run had replaced the d1be8eb89 pair on the A40 at 08:13Z; I reinstalled it inside my lease before the cases.
  - app_lifecycle count-notice case: PASS.
  - Process-death case: FAIL at its last step only, because the host opens the pulled store as plaintext and the store is now Keystore-encrypted. Queued as j240.
  - Log: A scratch path.
  - Messages: INFO 01a1bf67-f9bc to the coordinator; REPLY 01a1bf67-29d5 handed the A40 to p2p-network-dev-02 for step 5.
  - The build worktree (4.6G) is removed. The APKs stay in a scratch path until step 5 has run.
- Lesson: another login's run on a shared phone can replace the build installed for a later step. Read the packages inside my own lease before running, and reinstall in the same lease.

*References: a40-own-adb-server, android-shell-build, j228-host-node-for-a17*

*Observed 2026-11-09 (rust-ui-dev)*

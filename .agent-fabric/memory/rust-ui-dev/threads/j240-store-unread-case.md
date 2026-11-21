---
role: "rust-ui-dev"
class: threads
topic: "j240-store-unread-case"
description: "j240: app_lifecycle process-death case reads the encrypted store in the app's process via a new store_unread case; pushed 47ed7f798, device run on the A40 owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a7ec2cd8c146394b
---

## j240: app_lifecycle process-death case reads the encrypted store in the app's process via a new store_unread case; pushed 47ed7f798, device run on the A40 owed

- 2026-11-09: branch develop-qzapp/rust-ui-dev-01/feat/j240-android-e2e-app-lifecycle-a-process, 3 work commits off main 1799f3aa8, pushed 47ed7f798, no PR.
  - efad300e1: `cases::STORE_UNREAD` (tests/android-e2e-cases/src/store.rs) and `CaseCtx::store_cipher`.
  - 91e530dfe: apps/human-android/src/e2e.rs passes `StoreKeystore`.
  - 47ed7f798: app_lifecycle.rs uses the case instead of pulling the store.
- Why: since Stage 21 the store is SQLCipher under a Keystore-sealed key. A pulled copy fails on the host with NotADatabase.
- Checks: host clippy for e2e-cases, android-e2e and human-android; check_android_target OK; check_human_layering OK. Mutation: selecting kept_inbound fails the unit test.
- Owed: the device run on the A40. The pair is in a scratch path and already installed there (same signer f8b1b43e as d1be8eb89, so install -r). The APK build log is a scratch path. Run `fabric-lease samsung-a40 -- bash a scratch path a scratch path`. After it: blind review, then the PR (3 work commits, under 8, so it needs the owner's word to arm).
- Lesson: a test closure `log` that borrows `phone` blocks `phone.stop()` before a later `log()` call. Use `phone.log()` after the stop.
Related: [[j131-stage17-lane-pr358]] [[j229-phone-order-step2]] [[a40-own-adb-server]]
- 2026-11-09 ~09:30Z: A40 runs of 47ed7f798. Run 1 failed at setup because target/debug/transport-daemon was not built (build it first: `cargo build -p interweave-transport-daemon`). Run 2 had my app installed (sha a0dd4e7a; the test APK is identical to d1be8eb89's, 1efb4b15). The phone was behind a secure keyguard with stay_on_while_plugged_in=0, so both cases waited 300 s for a person and failed with no device step. j240 is BLOCKED on QUESTION 01a1bf90 to fabric-coordinator, asking for the owner's unlock. The owner's rule (coordinator REPLY 01a1bf93): the A40 stays on stay_on_while_plugged_in=7 before and after every run; never set it to 0. An unlock is the owner's step. Unlocked after 13:02Z; run 3 started then (log run3.log). Logs: A scratch path, run2.log.
- 2026-11-09 13:13Z: device run DONE on the A40 (unlocked, stay_on 7): both app_lifecycle cases pass on 47ed7f798 in 32 s (log a scratch path; run3 lost adb before any step). The process-death case asserted store_unread PASS with messages 1 and 2 unread. Coordinator told (REPLY 01a1c073-da67); p2p-network-dev-02 told the lease is free (INFO 01a1c073-cd82). NEXT: fold origin/main (3 commits, plan §20 prose only, ae7c0dc9a), blind review, open the PR (3 work commits: arming needs the owner's word), then deliver.
- 2026-11-09 13:21Z: main folded (e457812b9, plan prose only), pushed; PR #603 opened (3 work, 0 fix; arming needs the owner's word). Round-1 blind review dispatched on 1799f3aa8..e457812b9, request a scratch path.
- 13:29Z: the first review used base 1799f3aa8 (main before the fold), and post-review refused it. The base must be the PR's merge-base with main (ae7c0dc9a). Its F1 (no test for the guard) and F2 (an uncited dated observation) are fixed in d5e9565ff and 0da696e24 (2 fix commits); the F1 test fails when the guard is removed. A full review of ae7c0dc9a..0da696e24 has been dispatched.
- 13:37Z: the round-1 review (ae7c0dc9a..0da696e24) found F1, the unreachable separator guard. It was fixed in 7906c0292, and the re-review judged it fixed. Both reviews are posted, along with a comment answering them. #603 is 3 work, 3 fix, 0 threads, head 7906c0292 reviewed. It waits for CI and the owner's word to arm (`fabric-pr arm 603 --basis "owner's word ..."`). After it merges, deliver j240 and close it.
- 2026-11-10 03:2xZ: CI went red on RUSTSEC-2026-0459 (libsqlite3-sys), fixed on main by #602. Folded main as a18b62c62 (conflict in tests/android-e2e/Cargo.toml). The fold's re-review (5836226308) was clean. ARMED on the owner's word "arm 603" (architect-cto DECISION 01a1c231); REPLY 01a1c373 sent. wait-merged is running. After the merge: fabric-jobs deliver j240, then switch (only j232 is open, blocked on the Pixel).
- DONE: #603 merged 31130c5a0 (2026-11-10); j240 delivered.

*References: a40-own-adb-server, j131-stage17-lane-pr358, j229-phone-order-step2*

*Observed 2026-11-09 (rust-ui-dev)*

---
role: "ux-designer"
class: threads
topic: "j98-unlock-s11-pixel-review"
description: "j98 Pixel review of unlock note §11 (A14-A20, F10-F14) — build 0e1d4d175 with rust-ui-dev's RESTART hook; owner's yes to replace v0.2.0"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2a832f19040985c2
---

## j98 Pixel review of unlock note §11 (A14-A20, F10-F14) — build 0e1d4d175 with rust-ui-dev's RESTART hook; owner's yes to replace v0.2.0

j98 (2026-11-07): Pixel review of design/2026-11-05-unlock-policy-pending.md §11.
- The Pixel had release v0.2.0 with no identity since 2026-11-05 (packaging/android/evidence/2026-11-05-pixel-9a-update-and-downgrade.md:140).
  A release build has no run-as and no hooks, so it cannot run §11.
- The owner's word (2026-11-07, in my session): rust-ui-dev-01 may uninstall v0.2.0 and install the debug build.
  One phone session, after the hook. Pixel at a private-network address:38897.
- The reviewed build is 0e1d4d175, not 20ce43c8. It is j218's fix c01eb6a70 (one prompt per showing) plus j220's
  debug hook: `adb shell am broadcast -a org.interweave.human.debug.RESTART -p org.interweave.human`.
  The hook works only while the Service runs. It covers A19, the launch-after-L4 step and the two-prompt check.
- A16's discard half needs a store-refusal hook, requested in 01a1b5aa. Without it, carry that half as blocked.
- User-presence without a new identity: edit config.yaml runtime.android.key_unlock_policy to `user-presence` (kebab-case; user_presence does not parse) via run-as,
  then make one granted start (the reseal, #577).
- Install request: 01a1b5aa to rust-ui-dev-01. Next: onboarding with the owner, then the run plan
  (scratchpad j98-plan.md: A18, A17, A16 network via GrapheneOS toggle, A19 x3, A20, launch after L4).
Related: [[j96-locked-restart-in-process]], [[workflow-carried-2026-10-18]], [[device-run-check-focus]].
- 2026-11-07 ~11:05Z: the install is not done. The APK sha256 at 0e1d4d175 is 7d6ca799…bbcd. adb connect to .62:38897 fails
  for both rust-ui-dev-01 and me, so the pairing is lost; the owner has to re-pair both logins.
  rust-ui-dev-01's guard refused the uninstall: the owner has to give the word in their session.
  The discard hook is rust-ui-dev j222.
- 2026-11-07 run (the phone's clock is UTC+4 vs the host's). Setup: the owner onboarded. I edited the config
  (user-presence, then availability_mode: stay-reachable); the reseal was granted 16:31. The app logs `ui-trace focus=/notice=/prompt=` lines, which give focus without TalkBack.
  PASS: A18 + A20 (focus to conversations-heading); A19 b1 (L3, cancel -> focus=notice); L2 return (no prompt);
  A19 b2 (no prompt, no notification added; the return raises the prompt).
  NOT REACHABLE: A19 b3. After a cancel no Service runs, so the RESTART hook has nothing to restart, and no in-process L4 follows a cancel.
  Two-prompt: not reached. Under stay-reachable a return past the window starts nothing, so no prompt.
  Design question (mine): the no-reach notice shows ABOVE the locked notice (by note §4's order) while locked.
  Still owed: launch-after-L4, A16 network, A14/A15 + A16 discard (REFUSE_STORE), A17 (needs the A40 peer), TalkBack.
  The owner grants slowly: ask for "holding", then say "grant" per prompt.
- j99 (queued): long-message §6 names/titles, 95c44ffc1 on #581. It needs a blind review + LC English pass before arming.

2026-11-07 evening session (outputs in session scratchpad b3.out, g2.out, r1.out, r2.out, r4.out):
- launch-after-L4 PASS: online, Back, 66 s, RESTART from launcher -> start -10, no notification added (FGS one removed); launch -> prompt=unlock; cancel -> focus=notice. Grant -> one start, focus=conversations-heading (A20 PASS).
- R via REFUSE_STORE+RESTART within the 60 s window -> R directly, no prompt (StoreNeedsRecovery, start -5).
- A15 PASS visually: Try again past window -> prompt; cancel -> R stays, no other change, no prompt in 12 s; Try again again -> one prompt. NO focus= trace after cancel: focus on Try again unverified (TalkBack).
- A14 PASS: Try again -> prompt=unlock first; grant -> one start (answer 1) -> online. NO focus= trace after R->Chats: where focus lands unverified (possible finding).
- A prompt left up times out after 10 min (prompt=cancelled, stop answered false).
- Layout note: the locked notice card has no side margin (edge to edge), the no-reach card has 26 px margins (shots b3.png vs u1.png).
- Left: A16 network grant, A17 (A40 peer), A16 discard (after A17), TalkBack, restore config, record + OBSERVATION to rust-ui-dev-01.
- A16 network: `pm revoke ... INTERNET` kills the process; Stay-reachable restarts with "no network access: one notice, then end". The grant must come through the app's own dialog (note §10 Q5), so the owner taps it. Launch prompt timed out ungranted at 01:31 phone time; INTERNET restored with pm grant (USER_SET). App is locked, offline, notice focused. Paused there.
- A16 network grant FINDING (n4/n5 logs, 01:49:04 phone time): tapping Allow in GrantPermissionsActivity makes ActivityManager kill the app ("permission grant or revoke changed gids"), MainActivity force-removed "app died, no saved state"; person lands on the launcher, no restart, no prompt. The note's L1 network-grant row (§11 table, A16 bullet 2, F11) assumes a live Activity; on GrapheneOS it is unreachable. Next launch is L3. Spec revision is mine (owner/architect: the Q5 answer REPLY 01a1adae sorted it as L1).
- A17 setup (02:05-02:15 phone time): A40 paired to my login (connect a private-network address:39716; harness a40.sh). Show my address is FLAG_SECURE: owner exchanged addresses and trusted both. Pixel config now availability_mode foreground-only (restore later). Chats empty on both; no visible way to start a conversation (Peers row menu: Copy address/Remove; Contacts empty; Your card has no Share). Neither log shows a connection. Blocked: ask rust-ui-dev-01 how a direct conversation is created on this build.
- rust-ui-dev-01 REPLY 01a1b81c: no person-facing way to start a direct conversation (plan §18 carry 11, §29; contact-cards §5.7.4 "not designed here"); Chats rows come only from stored messages. A17 needs a third node (host daemon) to send to the Pixel first. Design gap to raise with architect-cto (mine). Findings sent after the relay drill: OBSERVATION 01a1b81a-4214 (reply expected).

- 2026-11-08: owner said yes; REQUEST 01a1b9b9-d669 to rust-ui-dev-01 for a host node (daemon + interweave-human) to message the Pixel first, for A17. Awaiting reply.
- 2026-11-08 phone order (fabric-coordinator 01a1ba04, job j101): my slots are 3b (A17+A16 discard, Pixel, after rust-ui-dev-01 step 2 debug APK), 4 (TalkBack, both, after rust-ui-dev-02 3a; then restore config, tell p2p-network-dev-02), 6 (j95). Expect a new Pixel identity at 3b: redo identity, trust A40 + j228 node, config by run-as; send rust-ui-dev-01 the new address. Answered yes (01a1ba05).
- j228 node up (01a1ba17): /ip4/a private-network address/tcp/47228/p2p/12D3KooWKqPd9SGtBXZYi5uBJxfmcQSkKhbkAJXvMnSLvaqRnosk (VM NAT, Pixel cannot dial; node dials Pixel). At 3b: send new Pixel address, add+trust node on Pixel, say when in front.

*References: device-run-check-focus, j96-locked-restart-in-process, workflow-carried-2026-10-18*

*Observed 2026-11-07 (ux-designer)*

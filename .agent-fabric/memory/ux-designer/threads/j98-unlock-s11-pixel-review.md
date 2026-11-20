---
role: "ux-designer"
class: threads
topic: "j98-unlock-s11-pixel-review"
description: "j98 Pixel review of the unlock note §11 (A14-A20, F10-F14): closed by the j107 re-check 2026-11-10; one spec revision still owed (network grant kills the process on GrapheneOS)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 926c84a990cf3c21
---

## j98 Pixel review of the unlock note §11 (A14-A20, F10-F14): closed by the j107 re-check 2026-11-10; one spec revision still owed (network grant kills the process on GrapheneOS)

The review covered design/2026-11-05-unlock-policy-pending.md §11. It ran on the Pixel 9a from 2026-11-07 to 2026-11-10, on debug builds with rust-ui-dev-01's RESTART and REFUSE_STORE hooks.

**State, 2026-11-13: done, except one spec revision.**
- Passed on the Pixel:
  - A14, A15 (seen on screen only), A17, A18, A19 b1-b2, A20, and the launch after L4.
  - A16's discard notice. Its focus on Chats was fixed as j108; see [[j108-recovery-screen-focus]].
- Findings went to rust-ui-dev-01:
  - OBSERVATION 01a1bb5a-8c5b (A17). Finding 4 was a build defect and was fixed on j232/j235.
  - OBSERVATION 01a1bc3c (3b).
  - OBSERVATION 01a1c3c6 (the j107 re-check).
  rust-ui-dev-01 built them as j226, j232 and j235. j235 is on main via d1be8eb89.
- A19 b3 cannot be reached: after a cancel no Service runs, so the hook has nothing to restart. It is carried by name.
- The grant cases need a fingerprint, so a person-free run cannot do them. They are carried by name.
- TalkBack is parked as j105 until Android has an accessibility tree.

**The note revision is done as j126 ([[j126-network-grant-row]], #659 armed 2026-11-13). It was owed as follows:**
- The L1 network-grant row in §11's table, A16's second bullet and F11 all assume the Activity is still alive after the grant. On GrapheneOS, Allow in the system dialog kills the process; see [[android-permission-grant-kills-process]]. The person lands on the launcher, and the next launch is L3. Rewrite the row from that.
  - The Q5 answer came from owner and architect-cto (REPLY 01a1adae), which sorted the grant as L1. Tell architect-cto when the row changes.
- An open question: should the note say that a read message which was not kept is gone after the grant's reload? Retention removes it. Seen 2026-11-08.

Lessons from these runs are in their own memories:
- [[android-run-as-config-edit]];
- [[device-run-check-focus]] (adb hygiene);
- [[end-device-run-in-known-state]].
Related: [[j96-locked-restart-in-process]], [[j93-unlock-policy-pending]].

Run record 2026-11-07, kept from the earlier version of this thread (design and architecture notes cite it):
- 2026-11-07 run (the phone's clock is UTC+4 vs the host's). Setup: the owner onboarded. I edited the config
  (user-presence, then availability_mode: stay-reachable); the reseal was granted 16:31. The app logs `ui-trace focus=/notice=/prompt=` lines, which give focus without TalkBack.
- A16 network: `pm revoke ... INTERNET` kills the process; Stay-reachable restarts with "no network access: one notice, then end". The grant must come through the app's own dialog (note §10 Q5), so the owner taps it. Launch prompt timed out ungranted at 01:31 phone time; INTERNET restored with pm grant (USER_SET). App is locked, offline, notice focused. Paused there.
- A16 network grant FINDING (n4/n5 logs, 01:49:04 phone time): tapping Allow in GrantPermissionsActivity makes ActivityManager kill the app ("permission grant or revoke changed gids"), MainActivity force-removed "app died, no saved state"; person lands on the launcher, no restart, no prompt. The note's L1 network-grant row (§11 table, A16 bullet 2, F11) assumes a live Activity; on GrapheneOS it is unreachable. Next launch is L3. Spec revision is mine (owner/architect: the Q5 answer REPLY 01a1adae sorted it as L1).
- A17 setup (02:05-02:15 phone time): A40 paired to my login (connect a private-network address:39716; harness a40.sh). Show my address is FLAG_SECURE: owner exchanged addresses and trusted both. Pixel config now availability_mode foreground-only (restore later). Chats empty on both; no visible way to start a conversation (Peers row menu: Copy address/Remove; Contacts empty; Your card has no Share). Neither log shows a connection. Blocked: ask rust-ui-dev-01 how a direct conversation is created on this build.

*References: android-permission-grant-kills-process, android-run-as-config-edit, device-run-check-focus, end-device-run-in-known-state, j108-recovery-screen-focus, j126-network-grant-row, j93-unlock-policy-pending, j96-locked-restart-in-process*

*Observed 2026-11-13 (ux-designer)*

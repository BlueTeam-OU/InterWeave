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

*References: android-permission-grant-kills-process, android-run-as-config-edit, device-run-check-focus, end-device-run-in-known-state, j108-recovery-screen-focus, j126-network-grant-row, j93-unlock-policy-pending, j96-locked-restart-in-process*

*Observed 2026-11-13 (ux-designer)*

---
role: "ux-designer"
class: threads
topic: "j110-kept-k1-checks"
description: "j110 — #611's three kept-K1 checks for identity note §10 item 15; spec addition now, Pixel run blocked on the owner's keep-v0.2.0 choice"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8b11a024902933f4
---

## j110 — #611's three kept-K1 checks for identity note §10 item 15; spec addition now, Pixel run blocked on the owner's keep-v0.2.0 choice

j110 (2026-11-10, REQUEST 01a1c551 from rust-ui-dev-01; taken by REPLY 01a1c599-4df9): add to identity note
(design/2026-10-19-android-identity.md) §10 item 15, and run on the Pixel with j109:
1. REFUSE_IDENTITY hook, finish the Activity before K1 ends, relaunch: K1 opens again, not Chats.
2. FLAG_SECURE on before K1 draws and before R2 shows any word. rust-ui-dev-01 read the code (REPLY 01a1c599-f9da,
   at e260b4058): ordered when a kept K1 opens in a later window (onStart sets it); NOT ordered when K1 rises over a
   running window (raise_recovery pushes SECURE_ON, Java posts secure(true), Slint draws on its own thread). K1 draws
   no word; R2 only after the person acts, so the gap is timing, accepted by architect-cto for #611. Device check:
   `dumpsys window` FLAG_SECURE with K1 up, before touching it. Off there -> P1, rust-ui-dev-01 gates R2 as a new job.
3. Restore end starts the client: Chats live, no second K1.
Build: A host scratch path sha256 2f1fcc5e...0c1b (#611 head a346f9ed).
BLOCKER (2026-11-10, owner in session): keep release v0.2.0 and its data on the Pixel. Every debug APK so far is
signed CN=Android Debug (cert sha256 18f39ab0...), so install over v0.2.0 = uninstall = data loss. j95, j109 and j110's
run all wait on that. No emulator option: develop-qzapp has no /dev/kvm and no emulator installed (checked 2026-11-10).
SPEC WRITTEN 2026-11-10: #614 (d57131ab7, item 15 bullet + R bullet now uses REFUSE_IDENTITY for K1-over-window;
K still hookless). Review r1 4 findings fixed in 2e8eeb27, r2 clean; English pass (985005) folded; ARMED on the owner's word in session; MERGED 02a825495 (2026-11-10).
Status REPLY 01a1c5a5 sent to rust-ui-dev-01 with the unverified Back-on-K1 lock risk; rust-ui-dev-01 traced it (REPLY 01a1c5a6, code only): the lock drops with the old window thread, so check 2 should show plain K1; a held state on the phone goes to them as a finding. Device REPLY to rust-ui-dev-01 still owed after the run.
Related: [[j108-recovery-screen-focus]], [[workflow-carried-2026-10-18]].

*References: j108-recovery-screen-focus, workflow-carried-2026-10-18*

*Observed 2026-11-10 (ux-designer)*

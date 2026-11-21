---
role: "rust-ui-dev"
class: threads
topic: "j251-kept-k1-secure-case"
description: "j251 (#615 carry F3/F4 + Show-under-K1 risk): secure_off rule widened to an open Show my address, android-e2e kept_recovery.rs case; branch feat/j251-..."
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 76fedc3a3187f7d4
---

## j251 (#615 carry F3/F4 + Show-under-K1 risk): secure_off rule widened to an open Show my address, android-e2e kept_recovery.rs case; branch feat/j251-...

2026-11-10, branch develop-qzapp/rust-ui-dev-01/feat/j251-human-android-615-carry-android-e2e.

- The hooks the case needs already exist (DebugHooks.kt REFUSE_IDENTITY + RESTART). S1 before
  its prompt keeps the service running, so RESTART's start is refused while S1 shows, no person.
  S1 Back before offline -> at_flow_end Raise -> K1 rises; secure_off logs
  "an identity flow shows: FLAG_SECURE stays on".
- Risk read and confirmed by code: show_identity(None) leaves Show my address's page; K1 raised over
  an open Show and ended by a restore asked SECURE_OFF with addresses drawn. Fix: ui-slint
  guard_show returns ShowOpen; activity.rs SHOW_OPEN thread-local; onboard::secure_off asks off only
  when ready_check::secure_may_clear(flow, show) (host-tested). No device path: K1 ends only by a
  restore (needs the phrase) or Back (Finish).
- New log line in raise_recovery's held-back path: "K1 kept: an identity flow shows, ..." (the case
  waits for it).
- Not covered: Keep path / service_asked operand (needs a user-presence profile + a person).
Related: [[j248-kept-k1-show-phrase]] [[j222-discard-hook]]

**PR #619** opened 2026-11-10 (4 work). Review 5846534956: F1 P3 (show_open wiring untested),
F2 P3 (ShowOpen doc). Judged (5846568067): F1 partly, F2 real. Fixes b213ecb05, de2d8036f,
c9324704c (3 fix). Re-review 5846644989 on c9324704: both fixed, Finding: none.
Device run NOT done: A40 install -r of 49bc355d2 OK (sha d2cf3f0e) but secure keyguard; Pixel
off-network (100.62 no route). Script a scratch path <serial> (A40 via
`fabric-lease samsung-a40 -- bash a scratch path a scratch path a private-network address:39716`;
edit its APK names to the head's build). Arming: under 8 work -> owner's word, or add j253.
Lessons: a finding-adjudication range is head..head with card_base (renderer insists, 2026-11-10);
its posted coverage must be {"paths":{}} or post-review refuses.
2026-11-10 ~19:30Z: rust-ui-dev-02 asked (QUESTION 01a1c6eb) about the A40's d2cf3f0e pair = my 49bc355d2 build; answered REPLY 01a1c6ed: replace it. The device run must install the c9324704c build (base 2f852aef) fresh.
2026-11-13 21:45Z: phones order 1 (DECISION 01a1d6db) put j251 at Samsung slot 2 after ux-designer-01 (Pixel slot 1 released unused: uninstall barred by [[pixel-no-destructive]]). Start on ux-designer-01's release INFO; first `fabric-lease samsung-a40 --who` (INFO 01a1d6de), every step under `fabric-lease samsung-a40 -- ...`; Samsung a private-network address:39716. Uninstall org.interweave.* then a fresh install (run.sh does install -r only: change it).
2026-11-13 22:20Z: DONE. kept_recovery passed on the Pixel 9a (fresh install of c9324704c on the owner's yes, notifications dialog answered first); record #619 issuecomment-6866352872; Pixel released (INFO 01a1d700).

*References: j222-discard-hook, j248-kept-k1-show-phrase, pixel-no-destructive*

*Observed 2026-11-10 (rust-ui-dev)*

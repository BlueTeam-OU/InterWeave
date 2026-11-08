---
role: "rust-ui-dev"
class: threads
topic: "j89-a40-second-window"
description: "j89 (A40 second window) outcome - PR #560, the 2026-11-04 A40 record, what each of items 1-5 showed, and what is carried (j90 Android open stall)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 529481a0ee5844c4
---

## j89 (A40 second window) outcome - PR #560, the 2026-11-04 A40 record, what each of items 1-5 showed, and what is carried (j90 Android open stall)

j89 (architect-cto-01's REQUEST 01a1a689) became PR BlueTeam-OU/InterWeave#560, MERGED 2026-11-04 as ed46059f. It was armed at 3c6253dc
with 12 work and 2 fix commits. Its review and its re-review were both posted, and both P3 findings were fixed. The
record is `apps/human-android/device-runs/2026-11-04-samsung-a40.md`. The delivery REPLY to architect-cto-01 is
01a1a835. The matrix cells and plan lines are theirs to move.

- TM-54: a lock change and a fingerprint enrolment did not invalidate any key. No key is bound to authentication
  (`KeyInfo`), so an invalidated key needs a user-presence profile, which was not run. The store key and the
  identity key are inside secure hardware (TEE).
- TM-48: the R1 and K1 cases passed on the device.
- TalkBack: the owner called it "unusuable". This is the plan's section 19 gap, with no AccessKit adapter.
- F4 Trust at 200 % failed and was fixed: unwrapped Texts widened the column (a2aa9802). F6 and F10 are met.
- Acceptance 23 is not reachable: INTERNET cannot be revoked on API 30.
- Opening a conversation stalls `android_main` for 7.7-12.3 s on the debug build, and that caused two ANRs. This is
  queued as my job j90, which starts by measuring a release-profile build.
- `provision_replacing` used to write config.yaml 0666; it is owner-only since 797eef6c.

Related: [[a40-device-run-traps]] [[a40-lock-change-word]] [[desktop-paging-relayout-cost]]

*References: a40-device-run-traps, a40-lock-change-word, desktop-paging-relayout-cost*

*Observed 2026-11-04 (rust-ui-dev)*

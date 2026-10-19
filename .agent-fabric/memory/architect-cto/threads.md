---
role: "architect-cto"
class: threads
description: "Owner's direction 2026-10-19 to close InterWeave Stage 17 (plan §20), with a new gate clause: ux-designer reviews the Android app against Telegram/WhatsApp standard; the per-clause gap read from main and the role sequence (j148)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - af7f7cad4a056c97
---

## Owner's direction 2026-10-19 to close InterWeave Stage 17 (plan §20), with a new gate clause: ux-designer reviews the Android app against Telegram/WhatsApp standard; the per-clause gap read from main and the role sequence (j148)

The owner said on 2026-10-19 "we have to close stage 17" and added
that the Android app's aesthetics are far below standard: ux-designer
must review it to bring it to the level of the mainstream messaging
apps (Telegram, WhatsApp). Job j148.

Gap read from origin/main (e254aaba) on 2026-10-19, per §20 gate clause:
- met or ready: (a) spikes; (d) and (g) tests exist stand-in + device;
  (e) endpoint_overlay.rs in profile-config; (f) TalkBack carried
  (#297); (i) no Proof::Stage(17); steps 1-6 and 9 in the app; OSV
  scan documented.
- missing: steps 7-8 (recovery Activity, BIP-39 picker, no clipboard);
  platform tests FLAG_SECURE, notifications, backup exclusion,
  user-presence diagnostic; API 34+ run recorded (ADR-0041
  remoteMessaging); PeerId copy on device and render parity (f);
  interoperability desktop-Android matrix row (c) or re-carry; device
  run with relayed route-indicator statement; step 10 store metadata
  and Play-policy quote; OSV at closing head (h).

Done 2026-10-19: clause (j) landed as d2d339cc on IW #301 (the
closing PR, 1 work commit, unarmed by design until every clause has
evidence or a named carry). REQUESTs sent, replies expected:
ux-designer-01 01a1526e-81ce (ranked review under design/ after #300
lands, re-review), rust-ui-dev-01 01a1526e-82b6 (steps 7-8, four
platform tests, API 34+ run, PeerId copy + render parity, step 10,
the (j) fixes), p2p-network-dev-02 01a15270-5828 ((c) device run and
route-indicator statement, interoperability items, confirm (d)/(e);
addressed by the in-flight-branch rule, may hand to -01),
devex-tooling 01a1526e-84fe ((h) scan at the closing head; osv-scanner
is not installed for my login). Closing record then on the owner's
word; status moves to Stage 18 (§21). The PR body carries the
agreements. Nothing in the gate is restated in README/IMPLEMENTATION.

Replies in by 2026-10-19 04:40Z (both recorded in the #301 body):
- p2p-network-dev-02 01a15272-7e2a: the (c) device run exists only as
  a report (no committed log on main; DECISION 01a12770 named one);
  route indicator proved at the ui-model only; platform matrix NOT
  BUILT, re-carry by name; (e) landed; their committed log + matrix
  wait on rustup/cargo-ndk for that login and on host commit signing
  (timing out since 2026-10-18 ~21:00Z).
- rust-ui-dev-01 01a15272-9de5: all six items on four j111-* branches,
  opened after #299 merges; device cases run in the instrumentation
  process, so FGS lifecycle, process death and network callbacks had
  only host stand-ins before; allowBackup=false may not stop D2D at
  targetSdk 36 (step 9 measures, dataExtractionRules fix).
- Decided 2026-10-19 (REPLY 01a15277-39b4, custody-doc supply
  ecf20942 on architect-cto-01/for/rust-ui-dev-01/custody-phrase-display):
  step 8 shows the phrase on-device (creation and on demand under a
  device-credential/Class-3 prompt whatever the key policy); a
  fresh-install restore confirms the DISPLAYED derived PeerId
  (IDENTITY-RECOVERY.md phrase-only path), no typing, no scanning;
  step 10 is §20's, the listing itself §22's.
- ADR-0041 A 2026-10-19 (432a354f on #301, after folding main 441361da):
  Play's remoteMessaging use case (continuity across the person's own
  devices, QR-paired web client) is narrower than the platform's
  definition (transfer text messages device to device); category
  stands, use case hand-entered in the Play declaration, step 10 quotes
  both texts + the gap, the Play-policy condition closes only at §22's
  store submission, specialUse is the fallback. Pages verified by moli
  the same day (support answer 13392821; developer fg-service-types,
  updated 2026-10-16). Same amendment, e47f5fcb: Play's FGS criterion
  "runs only for as long as necessary to complete the task" (answer
  13315670) answered by stating the task as person-ended; binds
  specialUse too; the likeliest refusal ground, named in
  apps/human-android/STORE-CHECK.md (rust-ui-dev-01, 26b34071).
- ux-designer-01's four custody questions (01a15280-7390 -> REPLY
  01a15282-ac06): Q1 stopped Activity keeps words in memory, Q2 focused
  word's label is not an announcement (both written into the custody
  doc, 425ec5d0 on the for/ branch, range now ecf20942..425ec5d0), Q3
  seal only on the last create screen after verification, Q4 no in-app
  replace-identity path in v1 -> OPEN PRODUCT ITEM for the owner,
  carried by name in the closing record.
Still awaited: ux-designer-01 01a1526e-81ce (the (j) review itself),
devex-tooling 01a1526e-84fe.

*Observed 2026-10-19 (architect-cto)*

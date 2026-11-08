---
role: "rust-ui-dev"
class: solution
topic: "pixel-debug-key-not-mine"
description: "rust-ui-dev-02 has no Android debug keystore; the Pixel's InterWeave app is signed by another account's key, so install -r from here would be refused or wipe data"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c32bd7544cc993a0
---

## rust-ui-dev-02 has no Android debug keystore; the Pixel's InterWeave app is signed by another account's key, so install -r from here would be refused or wipe data

As of 2026-10-26, this account has no `~/.android/debug.keystore`, and none exists anywhere in
its home. A debug APK built here gets a fresh key. The Pixel 9a's recorded installs (2026-10-20/21
device runs) came from rust-ui-dev-01's branches, so the installed app is most likely signed with
that account's key. The phone's signature was not read to confirm it.

ux-designer-01 asked me to `adb install -r` main 05d3f9cd keeping the data (REQUEST 01a17610). I
declined with REPLY 01a17614: it is rust-ui-dev-01's install to make, or a wipe on the owner's
word.

**Why:** `install -r` with a different key is refused (signature mismatch), and the way around it
uninstalls the app, which deletes the owner's profile on the phone.

**How to apply:**
- Before agreeing to an install that keeps the data, check that the key matches. Read the
  installed app's signer through phone-run (`dumpsys package org.interweave.human`) and compare it
  with the keystore that will sign the build.
- Never generate a new key and uninstall without the owner's word.

*Anchors: `apps/human-android/device-runs/2026-10-21-pixel-9a.md`*

*Observed 2026-10-26 (rust-ui-dev)*

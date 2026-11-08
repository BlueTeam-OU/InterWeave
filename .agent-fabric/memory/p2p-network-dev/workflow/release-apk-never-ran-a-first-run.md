---
role: "p2p-network-dev"
class: workflow
topic: "release-apk-never-ran-a-first-run"
description: "an Android release APK was never taken through onboarding on a device: v0.1.0 and v0.2.0 cannot make an identity on a fresh install (no first-run profile without dev-stand-ins); every device run used a debug build"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c2db1d33638b0c9e
---

## an Android release APK was never taken through onboarding on a device: v0.1.0 and v0.2.0 cannot make an identity on a fresh install (no first-run profile without dev-stand-ins); every device run used a debug build

Found 2026-11-05 (j141, Pixel 9a, API 37). A fresh install of the
PUBLISHED v0.1.0 and v0.2.0 APKs could not create an identity. The app
said "the app could not save your identity". Logcat said: "an identity
write was refused: Storage(... the profile document cannot be read: No
such file or directory)". The cause is `apps/human-android/src/jni.rs`
`fn provision`, which writes a profile only under the debug-only
feature `dev-stand-ins`. profile-config's `provision_embedded` was
meant to replace it, and nothing called it. Every device run up to
then used a debug build, or an update over one. So the release path
through Create had never run once, while release checks passed on
hashes and signatures.

**Why:** a feature-gated stand-in hides a missing production path. The
debug build is the only build anyone runs, so it never fails.

**How to apply:**
- A release check for a platform with a debug/release split installs
  the RELEASE artefact on a clean device and goes through first run.
- A stand-in under a feature names its replacement. Grep for a caller
  of the replacement before believing it landed.
- The package-level half still holds and is cheap: install -r keeps
  firstInstallTime, and versionCode 1 over 2 is refused with
  INSTALL_FAILED_VERSION_DOWNGRADE with or without -r.
- The fix is the app lane's (rust-ui-dev-02 took it). A supply of the
  advertise:false document is j147. architect-cto recommends 0.2.1. See
  [[j134-v0-2-0-release]].
- A40 (API 30), 2026-11-05: the same refusal. "Identity created" (C4)
  shows BEFORE the write; the seal runs on C4's Continue
  (identity_flow.rs, C4's primary is Effect::Seal). When a person reports
  that screen, ask them to tap Continue before you read the log. Records:
  packaging/android/evidence/ on PR #570. The owner said 2026-11-05: no
  compatibility with 0.1.0 is owed.
- Driving a release build: onboarding is FLAG_SECURE (a screencap is
  black) and the release build has no ui-trace in logcat, so the owner
  taps Create by hand. The app's own `interweave` logcat tag still
  carries refusals. The Pixel 9a was at a private-network address on 2026-11-05
  (pair port 35761, connect 44767), having moved from .10.

*References: j134-v0-2-0-release*

*Anchors: `apps/human-android/src/jni.rs::fn provision`, `crates/config/profile-config/src/provision.rs::pub fn provision_embedded`*

*Observed 2026-11-05 (p2p-network-dev)*

---
role: "ux-designer"
class: solution
topic: "android-run-as-config-edit"
description: "editing the Android human app's profile config on a debug build via run-as — value spelling, file mode, quoting, reseal"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - fcbf367e4ec6d18e
---

## editing the Android human app's profile config on a debug build via run-as — value spelling, file mode, quoting, reseal

On a debug build of the InterWeave Android app, the profile config can be changed without a new identity. The file is `/data/user/0/org.interweave.human/.../interweave/config/profiles/human-android/config.yaml`, edited through `adb shell run-as`. Learned on the Pixel, 2026-11-07/08, in the unlock-note runs.

- The values are kebab-case. `runtime.android.key_unlock_policy: user-presence` parses, and `user_presence` does not. The same holds for `availability_mode: stay-reachable` and `foreground-only`.
- A fresh profile may have no policy lines at all. Add them under `runtime.android`.
- **chmod 600 after every edit.** Writing with awk to a new file and then mv left mode 0666, and the app refused the start (`DirectoryRefused`, other-writable).
- Quote the whole remote command in double quotes: `adb shell "run-as org.interweave.human sh -c '...'"`.
- A changed unlock policy takes effect after one granted start, which reseals the key (#577).
- A release build has no run-as and no debug hooks. A run that needs them needs a debug build, and replacing a release build wipes the app's data, so ask the owner first.

Restore every changed line after the run, and say in the report that it was restored. Related: [[j98-unlock-s11-pixel-review]], [[device-run-check-focus]].

*References: device-run-check-focus, j98-unlock-s11-pixel-review*

*Anchors: `crates/config/profile-config/src/runtime.rs::stay-reachable`, `crates/transport/embedded/src/lib.rs::DirectoryRefused`*

*Observed 2026-11-13 (ux-designer)*

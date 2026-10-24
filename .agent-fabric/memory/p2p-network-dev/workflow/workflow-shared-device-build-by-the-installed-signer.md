---
role: "p2p-network-dev"
class: workflow
topic: "workflow-shared-device-build-by-the-installed-signer"
description: "On the shared Android test phone, the APK (and androidTest APK) for any lane's run is built by the account whose debug key signed the installed app and installed with adb install -r; another account's key forces an uninstall that wipes…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 7c299709d36d1d1b
---

## On the shared Android test phone, the APK (and androidTest APK) for any lane's run is built by the account whose debug key signed the installed app and installed with adb install -r; another account's key forces an uninstall that wipes the app's identity and data

A debug APK is signed with the building account's own debug key. Android installs over an app signed with
another key only after an uninstall, which wipes the app's data: its Keystore-sealed identity, its kept
rows, and any review content seeded on it. ux-designer-01 hit this on 2026-10-16, and the owner chose not to
wipe. I would have hit it on 2026-10-19.

The way the window works, on every device run since:
- The installing account (rust-ui-dev-01) builds the requested head, the debug and the androidTest APKs, with
  its key, and installs both with `adb install -r`, so the data is kept. It reports each APK's sha256.
- The other lane runs its instrumented cases from its own account, under the `pixel-9a` lease, on an adb
  server of its own. It installs and uninstalls nothing.
- One build per window serves every lane. The host side of a case is built at the same commit as the APK.

**How to apply:** when asking for a device window, name the commit for the installer to build, never an APK
of your own, and ask for the built sha so the run record can name it. If you push after naming a commit,
send the new head before they build. Related: [[domain-device-tests-run-in-the-app-process]].

*References: domain-device-tests-run-in-the-app-process*

*Observed 2026-10-22 (p2p-network-dev)*

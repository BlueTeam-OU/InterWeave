---
role: "architect-cto"
class: rationale
topic: "android-targetsdk-37-hotspot-decision-2026-10-27"
description: "open owner decision 2026-10-27 — raise the Android app's targetSdk from 36 to 37? The only way the hotspot host's own address reaches the bridge (ENABLE_MATCH_NON_THREAD_LOCAL_NETWORKS, since target 37); costs: compileSdk pin +…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 746d1f2c06bb5cb8
---

## open owner decision 2026-10-27 — raise the Android app's targetSdk from 36 to 37? The only way the hotspot host's own address reaches the bridge (ENABLE_MATCH_NON_THREAD_LOCAL_NETWORKS, since target 37); costs: compileSdk pin + ACCESS_LOCAL_NETWORK runtime permission before any LAN traffic (scan included)

Measured by rust-ui-dev-01 on the Pixel 9a (API 37) 2026-10-27, combined build 76a6b728+7d082119 (OBSERVATION 01a17c25): the phone's own hotspot (wlan1, WIFI, LOCAL_NETWORK capability) is reported to an app's network listen ONLY when the app targets SDK 37 (platform_compat gate ENABLE_MATCH_NON_THREAD_LOCAL_NETWORKS, 349487600, enableSinceTargetSdk=37; verified by `am compat enable` on the debuggable app: Show then lists the AP address beside the Wi-Fi one). The app targets 36 (build.gradle.kts compileSdk/targetSdk 36). A LOCAL_NETWORK-capability listen does not help at 36.

Ruled: under rule 10's second case the hotspot host's own address stays the recorded limit at target 36, with this cause named in the texts (my j410 batch); the pair connects one way via the client's code. Raising targetSdk to 37 is the OWNER's decision, not rule 10's: it needs compileSdk 37 (devex-tooling toolchain pin) and brings Android 17 behaviour changes, among them ACCESS_LOCAL_NETWORK as a runtime permission the person must grant before any LAN traffic (the in-room scan included) — a UX/product change (ux-designer-01 + owner). My recommendation to the owner: not now; revisit when a release targets Android 17 anyway or when the hotspot host case matters to a user.

Also owed: rust-ui-dev-01 runs the no-uplink Wi-Fi row (A40 hotspot without data, Pixel joined); a no-uplink Wi-Fi still carries INTERNET, so that row says whether the widened listen mattered.

*Observed 2026-10-27 (architect-cto)*

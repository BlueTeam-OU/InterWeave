---
role: "rust-ui-dev"
class: solution
topic: "android17-internet-runtime-permission"
description: "On the Pixel 9a (Android 17, API 37) INTERNET is a runtime permission; ungranted, EmbeddedHost::start fails with EPERM"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 84b64f70a3789177
  - dd0779051a49829f
---

## On the Pixel 9a (Android 17, API 37) INTERNET is a runtime permission; ungranted, EmbeddedHost::start fails with EPERM

Measured 2026-10-10 in j59, with the debug APK from main 7765ce82 on the Pixel 9a. The build is google/tegu/tegu:17/CP3A.261005.005, API 37.

- `dumpsys package permission android.permission.INTERNET` shows prot=dangerous|instant.
- The app's INTERNET permission is listed under "runtime permissions" with granted=false after install.
- Without it, EmbeddedHost::start answers `Internal("substrate: transport: Operation not permitted (os error 1)")`. The JNI start code is -3.
- After `pm grant org.interweave.human android.permission.INTERNET`, start answers 0.
- netpolicy is clean (blocked=NONE), and logcat has no avc denials.
- The phone also has OTHER_SENSORS and ACCESS_LOCAL_NETWORK as runtime permissions.

**Why:** this was the cause of the "substrate: transport: " start failure reported in j56 / OBSERVATION 01a1253d. It is not a transport bug.

**How to apply:**
- In j51 batch 2, apps/human-android must request INTERNET at runtime before start (and check ACCESS_LOCAL_NETWORK for LAN peers).
- The client needs a refused-permission state instead of "will connect when it starts".
- For any device run until then, grant INTERNET with `pm grant` first.
- I offered p2p-network-dev-01 a typed permission-denied variant, and that is optional (REPLY 01a12701).

Related: [[android-shell-build]] [[android-spike-device]]

**j60 agreement (2026-10-10, REPLY 01a12702-a951):** p2p-network-dev-01 adds `EmbeddedRefused::NetworkDenied(String)`.
- start() answers it when a listener's socket fails with PermissionDenied (EPERM or EACCES).
- The detail is for the log only.
- My side, in batch 2:
  - an exhaustive match in android-platform, plus its own JNI code;
  - on NetworkDenied, check the INTERNET permission: granted means the generic failure state, because a bind below port 1024 also gives EACCES.

**architect-cto DECISION 01a12706-6f32 (2026-10-10)** adopts my journey 1-6:
- Q1: on network denied, the Service opens the store and the hub read-only, with no runtime and no lease. There are no send controls and no join. The ui-model state is `network_denied`, and its texts are placeholders for language-culture-en.
- Q2: a stay-reachable restart without the permission posts ONE notification ("network access is off, open the app") and ends. It is never silent and never loops.
- Q3: human-client-android.md gets the section "Runtime permissions and the network-denied state", plus an ADR-0041 amendment (ii), at 82e676a6. architect-cto names it once it is pushed, and I fold it unrebased into batch 2.
- The "will connect when the service starts" text is retired on Android.

**j51 batch 2 build state (2026-10-10).** Branch develop-qzapp/rust-ui-dev-01/feat/android-shell-batch2. architect-cto's section 5fd9f3a8 is folded at d68e0220.
Rust half done:
- ui-model: SessionNotice::NetworkDenied, Intent::AllowNetwork, network_access(), sending_offered(), plus labels.
- app-core: Opener::ask_network_access.
- ui-slint: the notice action; the composer is hidden while sending is not offered.
- android-platform: offline.rs Offline<B> (BackendUnavailable), ServiceHost::start_without_network/is_network_denied, ToView::NetworkAccess.
- jni: startWithoutNetwork -> 2, waitNetworkAsk, text keys 5/6 NetworkOff*.
Kotlin half TO DO (the earlier step-1 edit "bind only after grant" was dropped):
- MainActivity binds always. On the first creation it asks INTERNET first, then POST_NOTIFICATIONS.
- A thread loops on Native.waitNetworkAsk(1000) while the Activity is started. On true, runOnUiThread: if shouldShowRequestPermissionRationale(INTERNET), requestPermissions; else open ACTION_APPLICATION_DETAILS_SETTINGS.
- On a grant, or onStart with access held, call the Service's networkGranted() through a LocalBinder.
- Service.startClient: hasNetwork ? start : startWithoutNetwork. NETWORK_DENIED (2) gets no awaitEnd and no relayNotices.
- networkGranted(): if mode==2, stop + startClient on the lifecycle executor.
- Q2: if a start command came (Stay reachable) and mode==2, post ONE NetworkOff notification (Text 5/6), then leaveForeground. Guard it once, and cover both orderings (onStartCommand vs the answer).
- Native.kt: add NETWORK_DENIED, startWithoutNetwork, waitNetworkAsk, Text.NETWORK_OFF_TITLE=5 and Text.NETWORK_OFF_BODY=6.
- Later: the Stay-reachable switch is offered only while POST_NOTIFICATIONS is held. The language-culture-en placeholders: NetworkDenied, AllowNetwork, NetworkOffTitle, NetworkOffBody.

**Committed and pushed 2026-10-10** on feat/android-shell-batch2: a525ddce (.kotlin ignore), ab56993f (ui-model+app-core state/hook + forced arms), a51e8358 (ui-slint), 81a007b7 (android-platform posture), ed21c70e (JNI). 5 work commits, no PR yet.
- Green: host tests of ui-model, app-core, ui-slint, android-platform and human-desktop; clippy -D warnings; tools/ci/check_android_target.sh (needs JAVA_HOME=/opt/android-sdk/jdk ANDROID_HOME=/opt/android-sdk).
- Not checked: ab56993f building on its own (bisect).
- Next is the Kotlin half (plan above), then the device run.
- Carry (done): the `narrow` binding loop and the 3 viewport-height deprecations were fixed in j95 (7b2c6f95, 53e98925), not batch 2; see [[slint-window-size-binding-loop]].
- Paused for j62 (FleetDeck rename, PRIORITY) on the owner's word.

*References: android-shell-build, android-spike-device, slint-window-size-binding-loop*

*Observed 2026-10-17 (rust-ui-dev)*

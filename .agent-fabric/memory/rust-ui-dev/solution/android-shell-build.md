---
role: "rust-ui-dev"
class: solution
topic: "android-shell-build"
description: "how the j51 Android app shell builds (Gradle+cargo-ndk+Slint NativeActivity), the env it needs, and first on-device findings"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8bac3363b37d6785
---

## how the j51 Android app shell builds (Gradle+cargo-ndk+Slint NativeActivity), the env it needs, and first on-device findings

Branch: develop-qzapp/rust-ui-dev-01/feat/android-app-shell (j51), 2026-10-10.

Build chain, proven on the Pixel 9a:
- `apps/human-android/android/gradlew assembleDebug` runs an Exec task that calls `cargo ndk -t arm64-v8a -P 30 -o app/build/rustJniLibs build -p interweave-human-android`.
- Environment: PATH must start with ~/.cargo/bin; JAVA_HOME=/opt/android-sdk/jdk; local.properties `sdk.dir=/opt/android-sdk` (local.properties is not committed).
- Slint's Android backend build.rs compiles a Java helper and needs ANDROID_JAR set to platform 36. Without it, it takes platform 30 and fails on android.window. The Gradle task sets it.
- AGP 9.4.1 has built-in Kotlin (no kotlin plugin). AGP 9's DSL has no android.ndkDirectory; use androidComponents.sdkComponents.
- Slint 1.18.1 `backend-android-activity-06` is NativeActivity only. MainActivity.kt extends NativeActivity, and the cdylib exports `android_main`.

Device facts:
- Slint keeps its platform per thread. A recreated Activity gets a fresh android_main thread, and init succeeds again in the same process.
- The listener reports MainEvent GainedFocus/LostFocus/Destroy. That is the read gate.
- First render defects: the two-pane desktop layout overflows a 1080 px phone; nothing respects the status/nav bar insets; dark theme gives white text on the light conversation pane; the empty facade shows "Reconnecting".

Related: [[embedded-host-seam]] [[labels-platform-values]] [[android-spike-device]]

**j51 progress (2026-10-10), branch head edc1aacb, pushed, no PR yet.**
Commits so far:
- ui-slint android feature
- human-android cdylib and Gradle project
- android-platform (ServiceHost, Hub, Notices, stand-ins)
- the endpoint from runtime.android
- ui-model HostText/RuntimeHost (j49)
Folded: architect-cto's glossary and landing-zone commits, p2p-network-dev-01's --locked, architect-cto's availability record (9ce103f2).

NEXT:
1. ui-slint: the View carries a RuntimeHost from the app, replacing the hard-coded Daemon.
2. Phone layout: narrow single pane, insets, dark-theme contrast.
3. JNI (jni 0.22, EnvUnowned::with_env) on Kotlin object Native. Calls: start(dataDir), waitEnded, stop, waitNotice(ms), texts. Plus MainActivity (NativeActivity) binding NetworkService. The Service is bound in foreground-only mode, FGS remoteMessaging when stay-reachable. The Activity attaches to the Hub with an Rc<View> surface and a fresh ModelSide per link.
4. Notifications (step 4): channels, POST_NOTIFICATIONS, VISIBILITY_PRIVATE, count-only.
5. Placeholders for language-culture-en with context.
6. Instrumented tests and the ADR-0041 API-37 run.
7. j44 network callback.
8. transport-client per-peer path snapshot.
Waiting on others:
- p2p-network-dev-01 j53/j55: Ended::RuntimeEnded/AvailabilityChanged, provision_embedded, set_availability. Their branch is feat/android-runtime-side.
- devex android-ci contributor branch.
- The owner: the NVD_API_KEY secret.

**2026-10-10, later.** Head 9a816faa:
- The Service, the JNI and the Activity root are in. Folded devex's CI (d700096a).
- On the Pixel, the attach and the Android text work. EmbeddedHost::start fails with Internal("substrate: transport: ") (empty), reported to p2p-network-dev-01 (OBSERVATION 01a1253d).
- UX gap that is mine: a refused start shows "will connect when it starts". A refusal state is needed.
- Still to do: the phone layout, the refusal state, the placeholders' context to language-culture-en, the instrumented tests, the ADR-0041 run, j44, and the path snapshot.
- j48 delivered (a11c5705 for #252's re-review).
- herdr #11 is j47: its review is posted with F1 (P2, unknown->idle never sounds on the API path, headless.rs:3149), F2 and F3 (P3).

**PR #255 opened 2026-10-10** (head 18f78d84; 19 work commits, 1 review fix):
- The blind review was dispatched (brief: scratchpad pr255-review.yaml).
- It arms at the gate once the review is posted and no P1/P2 is open.
- Batch 2, after #255: the refusal state; the device evidence (instrumented tests, the ADR-0041 run), which needs the runtime to bind on the device (f5882e2b diag); the Stay-reachable switch (after p2p's #254); j44; link opening; the path snapshot; the Keystore adapter (j53, after the trait and #255 are on main).
- language-culture-en finalised the 14 Android values (fe758be3).
- #255 was ARMED at head ad583073 (23 work, 12 fix). arm.sh flagged it over 16: next time, smaller batches.
- Carried to batch 2:
  - the P3: UNANSWERED "never" has no test, since jni.rs is android-only and the codes would need to move to a host-compiled place;
  - catching Throwable;
  - the facade outliving its close;
  - path replay;
  - the stray libif_watch .so in the APK;
  - gradlePluginPortal in pluginManagement;
  - verification-metadata.
- **#255 MERGED** (f9df4eab, 2026-10-10).
- j56 in progress: a supply onto p2p-network-dev-01's #254 for Ended's three variants. Worktree interweave-j56, branch for/p2p-network-dev-01/ended-variants, based on their e5653002. It adds ServiceHost::set_availability, availability() reading host.availability(), the test-hooks feature, Kotlin restart, and the JNI codes 0-3.
- j56 was folded into #254 at 234e39ed (p2p-network-dev-01's INFO 01a125e9-266d).
- Carried to batch 2 from #254's re-review:
  - P3-1: wait_ended's doc should name the runtime end and a changed mode.
  - P3-2: awaitEnd's KDoc should name the end and the restart.
  - P3-3: no test ties jni.rs's codes 2/3 to Native.kt's ENDED_*; a swap would turn a dead runtime into a restart loop.
  - R1: set_availability has no JNI entry or switch yet.
  - R2: startClient queued in awaitEnd does not re-check `alive`, so a stop/start/stop could happen near onDestroy.

*References: android-spike-device, embedded-host-seam, labels-platform-values*

*Observed 2026-10-10 (rust-ui-dev)*

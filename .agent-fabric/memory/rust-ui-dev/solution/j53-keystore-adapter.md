---
role: "rust-ui-dev"
class: solution
topic: "j53-keystore-adapter"
description: "j53 (§20 step 6) Keystore SeedCipher built on batch-2 branch; design, what is unverified, open ruling on the user-presence window"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 52323e761eb1555e
  - 807b6756a1132389
---

## j53 (§20 step 6) Keystore SeedCipher built on batch-2 branch; design, what is unverified, open ruling on the user-presence window

Built 2026-10-11 on develop-qzapp/rust-ui-dev-01/feat/android-shell-batch2 (head 4f8453b5, pushed, no PR yet; opens after #260 merges, carrying a4cd49fd).

- crates/human/android-platform/src/identity.rs `start_identity(app_data_dir, profile, &dyn SeedCipher)`: custody::unlock; Unprovisioned -> first run (dev-stand-ins only: ProfileIdentity::generate + custody::provision under config's runtime.android.key_unlock_policy), else Unprovisioned; RecoveryRequired/UserNotAuthenticated/Unavailable map through, record never replaced. Host-tested with a fake cipher (tests/identity.rs), mutation-checked.
- apps/human-android: KeystoreCipher.kt (AES-256-GCM, StrongBox then TEE, status-byte framing 0 OK,1 invalidated,2 missing,3 user-not-auth,4 auth,5 unavailable); src/keystore.rs SeedCipher over it (JavaVM + Global class found inside the native method -- a natively attached thread sees only system classes); jni.rs start provisions the profile first, then identity; refusals -9/-10/-11.
- Compiles and clippies for aarch64-linux-android (tools/ci/check_android_target.sh via bash, CARGO wrapper to run clippy); assembleDebug builds. NO device run: Keystore unexercised on a phone.
- Ruled: user-presence window 60 s, AUTH_DEVICE_CREDENTIAL|AUTH_BIOMETRIC_STRONG (architect-cto DECISION 01a12a32, ADR-0042 A 2026-10-11 ed29ce7b folded). UnrecoverableKeyException -> KEY_INVALIDATED (SPIKE-009 D6a). Step-7 risks they named: keyguard inside the window unknown; after a background restart no Activity is up to show the UserNotAuthenticated prompt.
- #262 = batch 2 + j53 (posture Kotlin half added after review F1; first-run is FirstRun param). Reviews: 03c8e217 (F1-F6), 049a630c (N1,N2), e8531c54 (R1), all fixed.
- Still to do: Keystore instrumented test in androidTest, sharing j66's custom Instrumentation runner (no AndroidX deps).
- Seen: the binding loops in app.slint's narrow layout -- fixed in j95 (branch develop-qzapp/rust-ui-dev-01/feat/j95-interweave-app-slint-s-7-pre, 53e98925); see [[slint-window-size-binding-loop]].

Related: [[j66-android-e2e-seam]] [[embedded-host-seam]] [[android-shell-build]]
- 2026-10-11: #262 ARMED (18 work, 9 fix) after the review of f55f00c8 (no P1/P2). Carried to the next batch (j66 device run): F1 P3 -- ui-model test that text(NetworkOffBody) contains text(AllowNetwork); risk -- keystore2 may rethrow a transient KeyStoreException from getKey as UnrecoverableKeyException: settle on device, narrow on e.cause.isTransientFailure() -> UNAVAILABLE. Over 16 work commits: smaller batches next time.

*References: android-shell-build, embedded-host-seam, j66-android-e2e-seam, slint-window-size-binding-loop*

*Observed 2026-10-11 (rust-ui-dev)*

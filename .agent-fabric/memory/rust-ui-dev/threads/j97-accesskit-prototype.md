---
role: "rust-ui-dev"
class: threads
topic: "j97-accesskit-prototype"
description: "j97 (2026-10-17): app-side AccessKit adapter prototype for Android built, two APKs (unpatched / hover-patched) waiting for the Pixel; what is measured and what is not"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e769fd608efc3889
  - fa448fc93fa33ef9
---

## j97 (2026-10-17): app-side AccessKit adapter prototype for Android built, two APKs (unpatched / hover-patched) waiting for the Pixel; what is measured and what is not

j97 is architect-cto-01's time-boxed measurement (REPLY 01a14718): one working day, a scratch branch, no PR, nothing vendored, the backend hover change local only. Report the five items in a REPLY to 01a14718. Nothing lands before the owner decides.

State on 2026-10-17 ~01:40Z:
- Commit 839e5ac7 on develop-qzapp/rust-ui-dev-01/feat/j97-android-accesskit-go-on-the-time (local, not pushed). It adds `crates/human/ui-slint/src/android_a11y.rs` (405 lines, 1 file) plus 6 lines in `apps/human-android/src/activity.rs` and 3 deps (accesskit =0.24.1, accesskit_android =0.7.5 with embedded-dex, vtable 0.5). It compiled for aarch64 on the first try.
- Shape: `InjectingAdapter` on `android.R.id.content`'s child, found over JNI from Rust using AndroidApp's vm/activity pointers. Kotlin is not changed. The tree is built from `accessible_descendents`. A 50 ms slint Timer rebuilds the tree when the PropertyTracker is dirty and polls `focus_item`. Actions go back through `invoke_from_event_loop`. It logs to logcat under the tag `a11y-proto`.
- APKs: built from 839e5ac7 with `./gradlew assembleDebug` in the j97 worktree, one unpatched and one with the hover patch: HoverMove returns `InputStatus::Unhandled`, a one-line code change, applied with a temporary `[patch.crates-io]` in `.cargo/config.toml`; config and lock were restored afterwards.
- Why hover can work at all: NativePostImeInputStage forwards an event the native queue left unhandled to the view hierarchy, where the adapter's OnHoverListener is. This is read, not measured.
- Not built in: popups, scroll actions (Slint has none; a Flickable is not an accessible item), and text-input roles beyond MultilineTextInput.
- Blocked: the Pixel was unreachable (no adb device, no mDNS). The owner must re-pair it. This is the same blocker as j94.

Related: [[android-accesskit-bridge]] [[device-review-run-method]] [[release-phone-after-testing]] [[j94-install-292-apk]]

**2026-10-18.** architect-cto-01 REQUEST 01a14d9d (queued as j103) asks for the device report now. j141, the owner's decision, waits on nothing else.
- The phone's address answered on TCP, but adb connect was refused: the pairing is lost. My REPLY 01a14d9e asks the owner for the pairing code and port.
- The hover patch is the registry's i-slint-backend-android-activity 1.18.1 with HoverMove → InputStatus::Unhandled at androidwindowadapter.rs:428.
- Items to report (01a14718), each with its evidence level:
  1. TalkBack swipe navigation and double-tap on Chats; what is announced for a message row, the route, trust and connectivity (the §13 set).
  2. Touch exploration: unpatched not working, patched working?
  3. Focus following via polling focus_item: the lag, and whether the focus change is announced.
  4. A chat row off screen reached by swipe: scrolled into view or not.
  5. Adapter size: 405 lines, 1 file, plus 6 lines in activity.rs and 3 deps.
- After the run, install main 400b42b6 for ux-designer-01 (j98).
- 2026-10-18 rebuilt both from 839e5ac7 (unpatched sha 7f7045a8, patched 5756c4dc; their libs differ). The patched build needs `cargo update -p i-slint-backend-android-activity --offline` after adding the config patch, because cargoNdk runs `--locked`; restore config.toml and Cargo.lock afterwards. Waiting only on the owner's pairing code and port.
**2026-10-18 device run, report REPLY 01a14db9 to 01a14d9d.**
- Paired with the owner's code.
- TalkBack is com.android.talkback/com.google.android.marvin.talkback.TalkBackService. Enabling it opens TalkBack's tutorial over the app (re-`am start` the app) and briefly a volume panel (wait ~14 s).
- Injected `input swipe`/`tap` are NOT TalkBack gestures, and mouse hover is not touch exploration, so items 1, 2 and 4 need the owner's finger.
- Measured:
  - the uiautomator tree (rows focusable with title desc; nothing clickable; no row description);
  - Tab focus moves logged 30-60 ms after the key;
  - no TalkBack box follows the focus.
- The wireless link drops mid-run: reconnect before every adb call (`adb get-state || adb connect`).
- The hover-patched build is installed now. Next: the owner's checklist or main 400b42b6 for ux-designer-01, as architect-cto-01 says.
- Order (architect-cto-01 REPLY 01a14dba): the owner finger-tests the hover-patched build first, then the unpatched one, then main 400b42b6 for ux-designer-01. Install NOTHING until architect-cto-01 says. The gaps are noted on the scratch branch in 48cf3672 (local). Main's APK is being rebuilt into a scratch path.

*References: android-accesskit-bridge, device-review-run-method, j94-install-292-apk, release-phone-after-testing*

*Observed 2026-10-17 (rust-ui-dev)*

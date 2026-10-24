---
role: "rust-ui-dev"
class: workflow
topic: "device-review-run-method"
description: "how to run a recorded acceptance pass of the InterWeave Android client on the Pixel by adb: real rows from host peers, evidence, redaction, traps (j88, 2026-10-16)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 42a3dfc7b9e5800f
  - 85571680f6a71963
---

## how to run a recorded acceptance pass of the InterWeave Android client on the Pixel by adb: real rows from host peers, evidence, redaction, traps (j88, 2026-10-16)

Learned running swipe spec §11 on the Pixel 9a for j88 (its findings landed as InterWeave #292; the run's local evidence directory did not survive the 2026-10-17 /var/tmp wipe).

**Real rows of every kind.** I used a scratch, uncommitted `#[ignore]` test in tests/android-e2e. Its parts:
- `AdbDevice::connect` and two `Desktop` daemons.
- `relayed_example_of("human-android.yaml", ...)`, given both static routes joined with `", "` (`concrete` quotes the one route it is given).
- `phone.reach` for each port.
- A loop reading `send <i> <text>`, `stop <i>`, `start <i>` and `quit` from a file.
- The phone keeps its data (install with `adb install -r` run through `.agent-fabric/bin/phone-run`, never raw adb; no `pm clear`). Kind C = stop the daemon, then send from the phone. Kind D lasts only for the session.
- A HumanChatV2 `app_message_id` must be 32 hex characters; anything else is refused as `InvalidEnvelope`.

**Evidence.**
- `dumpsys vibrator_manager` lists each haptic with its time, which proves the tick. Grep it for the app's package only, because the phone is the owner's.
- The store is readable through `adb exec-out run-as org.interweave.human cat interweave/state/profiles/human-android/human/human.sqlite` (and the `-wal`), then Python's sqlite3. That is how the ids of a restored record were checked.
- `screenrecord` MISSES some frames from the Slint window. Use screencap stills for any state claim.

**Traps that cost reruns.**
- An Undo tap sent as a separate tool call arrives after the 6 s snackbar has closed. Put the gesture and the Undo in one script.
- A layout line ("Connected through a relay", "Hide conversation") appears or disappears and moves rows ~60 px. Screenshot before each gesture.
- Recents, the launcher and Settings show the owner's apps and the phone's address. Redact them: ffmpeg `drawbox` with `-fps_mode passthrough` (without it the re-encode drops the later frames), or trim.
- Removing the app from Recents ends the activity, not the process (the service holds it). Use `am force-stop` for a real process death.
- Force RTL set by adb (`settings` plus `setprop`) does not reach a running configuration. Use `cmd locale set-app-locales <pkg> --locales ar`, and `--locales ''` to undo it.
- The default adb server on 127.0.0.1:5037 is shared by all logins, and another account's kill-server there drops the device. Run on your own server: `export ANDROID_ADB_SERVER_PORT=$((15037 + $(id -u) % 1000))` (see [[release-phone-after-testing]]).
- Hold the phone for the whole run with a background `fabric-lease pixel-9a --label <job> -- sleep N`. Kill it to release.
- A `pkill -f` whose pattern also appears in your own command line kills your shell (exit 144).

Related: [[j88-device-review-run]] [[release-phone-after-testing]] [[android-e2e-device-run]] [[slint-swipe-in-flickable]]
- 2026-10-18 (j97): adb-injected touches never drive TalkBack. `input swipe`/`tap` are not TalkBack gestures, and `input mouse` hover is not explore-by-touch. TalkBack's gestures are a person's check. What adb can measure under TalkBack: the uiautomator tree, keyboard focus via keyevent TAB, and TTS timing from MediaFocusControl/TextToSpeechServiceImpl logs.

*References: android-e2e-device-run, j88-device-review-run, release-phone-after-testing, slint-swipe-in-flickable*

*Anchors: `tests/android-e2e/src/adb.rs::AdbDevice`, `tests/support/src/e2e/mod.rs::relayed_example_of`*

*Observed 2026-10-16 (rust-ui-dev)*

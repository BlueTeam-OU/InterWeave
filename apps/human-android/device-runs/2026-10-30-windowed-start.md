# Pixel 9a and Samsung A40 device runs, 2026-10-30: the windowed start

PR #517 (merged 13256d30) moved the Android client onto the windowed
start the desktop uses. The service's facade sends the view summaries
and contacts at each attach and each start, not the whole store, and an
opened conversation reads its rows a page at a time. Review F1 made a
model built at an attach or at a running-state change be told the
conversation the view shows. This record is the device evidence for that
head. The runs were made by rust-ui-dev-01, whose debug key signs the
installed app (REPLY 01a18ecd to REQUEST 01a18e66).

Build: PR #517's head 79d72395.
- `app-debug.apk`: sha256 beginning 78b47292.
- androidTest APK: sha256 beginning 425c686e.
- Signer certificate SHA-256 beginning 18f39ab0 (Android Debug).
- `dumpsys package` read the same signature on both phones, versionCode 2,
  versionName 0.2.0.

Each phone was installed with `adb install -r`, keeping its data: the
first-install time was unchanged on both. The Pixel ran under
`.agent-fabric/bin/phone-run` and the A40 under the `samsung-a40` lease.
No adb process of the login was left afterwards.

| device | API | case | result |
|---|---|---|---|
| Pixel 9a | 37 | `human_chat`: relayed, direct, Wi-Fi | 3 passed, 32.2 s |
| Samsung A40 (SM-A405FN) | 30 | `human_chat`: relayed, direct, Wi-Fi | 3 passed, 28.2 s |

The command was `cargo test -p interweave-android-e2e-tests --test
human_chat -- --ignored --test-threads 1`. The chat case opens its
conversation after the windowed start and before its first draft
(review F2), then exchanges messages with a desktop.

## Re-attach with a conversation open

This exercises the review F1 path through an attach. On each phone a
conversation was opened, the app was sent Home, and was then brought back
(`am start`, a hot launch). Both times the conversation showed its
header and rows. On the A40 the composer then took a new message and
Send added its row. Its peer had stopped, so the row stayed pending in
the store.

## Not shown

- **A service restart with a conversation open.** That is the
  running-state path of review F1. At phone width, Stay reachable is on
  the list's menu, not the conversation's, so turning it off means
  leaving the conversation first. The service's network-availability
  restart cannot be driven over wireless adb without dropping the link.
  The path is covered on the host only, by
  `a_fresh_model_is_told_the_open_conversation_and_avatars_again`
  (ui-slint) and `a_model_side_made_tells_its_surface_once` (app-core).
  The Android glue between them (`AndroidSurface`) was not exercised on
  this path.
- **A message arriving after the re-attach.** No trusted peer was live.
- **TalkBack**, and a radio relay.

## Observed once, cause not found

On the A40, the conversation pane still read "Select a conversation"
3 s after a row was tapped, and its rows were shown by the next
screenshot, about 10 s later. On the Pixel the rows showed within 3 s.
There was one try on each phone. This is carried to job j77, which is
about the cost of the first draw after a page, as a device row to
measure there.

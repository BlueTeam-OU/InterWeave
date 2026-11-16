# Samsung A40 device run, 2026-11-16: A5's Back runs judged on the uptime clock

Device: the **Samsung A40** (SM-A405FN, Android 11, **API 30**), at
192.168.1.9:37606 over wireless debugging, on this login's own adb
server (port 15065) under the `samsung-a40` lease; no adb process of
this login remained after any step. Every time below is UTC. The phone
was unlocked throughout; `stay_on_while_plugged_in` was 7, set 7 for the
run and restored 7.

What was asked (architect-cto-01, REQUEST 01a1e240, for ux-designer-01's
REQUEST 01a1e23f): run #669's device cases once for the two items on
design/2026-11-04-long-message.md A5's carry list that the Back key
covers, the list shape's Back at half its first-screen time and the two
Backs pressed together; and settle the harness's four facts first.

## The phone before and after

p2p-network-dev-02 released the A40 at 16:53Z (INFO 01a1e547). It held
that login's debug pair of 8b6b1d35b (app apk 1396145e, test 88247508,
certificate SHA-256 00a92280). Under architect-cto's word (01a1e546, as
that INFO cites it), the pair was pulled to the host and then uninstalled
with its data at 16:54Z; nothing else on the phone was touched. The phone
was released back to p2p-network-dev-02 at 17:01Z (INFO 01a1e54f-3801)
holding this run's pair.

## The build

main at 3196cd731, `./gradlew assembleDebug assembleDebugAndroidTest`
with the native library at the root `Cargo.toml`'s `[profile.release]`
settings passed as `CARGO_PROFILE_DEV_*` (thin LTO, one codegen unit,
debug 1, overflow checks on, debug assertions off, no incremental), as
in the 2026-11-12 and 2026-11-13 records: a release APK carries no trace
and cannot be judged by the case. App apk c710e51f, test apk 16e618fa;
this login's debug key, certificate SHA-256 0392b790 (`dumpsys package`
prints its signatures as `[26d1eb18]`, the form the earlier A40 records
call the signer). The host side ran `target/debug/transport-daemon` and
the `long_message` test binary of the same commit.

## The harness's four facts (16:55Z)

Each was left unproved by #669 (`tests/android-e2e/src/back_run.rs`);
a failure here would have been a harness fix, not A5 figures. All four
hold on this phone.

- **adb's exit status.** `adb shell 'exit 7'` returned 7 to the host and
  `adb shell false` returned 1, so the case sees the sender's own status.
- **Key injection by app_process.** `KeySender SCROLL_LOCK:0
  SCROLL_LOCK:30`, run three times as the case runs it (`CLASSPATH=<test
  apk> app_process /system/bin ...KeySender`), printed `sent` for every
  press and exited 0; InputDispatcher logged each `Inject key` and
  delivered it to the focused window.
- **One clock.** Every send stamp lies between a `log` mark written just
  before and just after the sender in the same shell, both read from
  `logcat -v threadtime -v monotonic` (for example before 775798.237,
  sent 775799210 and 775799241, after 775799.761); InputDispatcher logged
  the first injection at monotonic 775799.211 with the event time
  775799210 ms, the sender's stamp. The two presses of a run were 30 to
  31 ms apart.
- **logcat's line shape.** `775917.518 23817 30261 I interweave: ui-trace
  reader=drawn ms=183`: the monotonic time with exactly three
  millisecond digits, then pid, tid, level and the ` interweave: ` tag,
  the shape `back_run::monotonic_trace` reads. The cases read every
  trace line of both runs without a harness failure.

## The runs (16:55 to 17:01Z)

`cargo test --locked -p interweave-android-e2e-tests --test long_message --
--ignored --exact <case> --test-threads 1 --nocapture`, each case once.
Verdicts are `back_run::judge`'s, by A5; times are the phone's uptime
clock in ms, Back to close from the first Back's send to the trace's
conversation page.

| Case | Shape | Back after Enter | Second Back after the first | First screen | Back to close | Pages after the reader | Verdict |
|---|---|---|---|---|---|---|---|
| `on_a_device_back_while_the_reader_opens_closes_it_to_its_conversation` | list | 90 | — | drawn 103 ms after the Back, shown 63 ms | 166 (limit 1,000) | conversation, focus on Open the whole message (`read:0`) | **pass** |
| `on_a_device_two_backs_pressed_together_while_the_reader_opens_take_two_steps` | distinct headings | 600 | 30, sent before the close | not drawn | 53 (limit 150) | conversation, then the chat list | **pass** |
| same | distinct links | 600 | 31, sent before the close | not drawn | 33 (limit 150) | conversation, then the chat list | **pass** |

Both cases passed: no round failed and none was void. The list shape's
first screen came 193 ms after the Enter was sent on the uptime clock,
and the app's trace measured it as `reader=drawn ms=183`, so the Back at
90 ms was pressed at about half its first-screen time, as A5 asks. The
first screen was drawn after that Back and before the close, the cost of
the first turn the design accepts (§5.4). In each pressed-together round
both Backs were sent before the close, and they took exactly two steps:
neither was lost and the two were not merged.

## What this does not show

- **No screen recording.** The cases ran without
  `INTERWEAVE_ANDROID_SHOTS`, so no frame was kept. A5's frame conditions
  were not inspected: inside the first turn, that no frame showed the
  bar; past it, that the bar went away with the reader. The trace has no
  line for the bar. The list round closed 256 ms after the Enter, under
  the 0.5 s before which the reader shows nothing (design §8), but that
  is the view's rule, not a frame seen on the phone.
- **One round each**, as asked; the 2026-11-13 record ran three.
- The times are the sender's send and the app's trace line, not the
  frame on the screen.
- **Not run here**: Back past the first go and the two Backs 0.3 s apart
  as judged since #669; the navigation bar's and the app bar's Back; the
  desktop; the Pixel; the release APK.

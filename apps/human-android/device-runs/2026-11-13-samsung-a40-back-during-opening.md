# Samsung A40 device runs, 2026-11-13: Back while a long reader opens

Device: the **Samsung A40** (SM-A405FN, Android 11, **API 30**) of
`2026-11-12-samsung-a40-release-profile.md`, on this login's own adb
server under the `samsung-a40` lease, over wireless debugging; no adb
process of this login remained after any run. Every time below is UTC.
The screen's stay-awake setting was saved before each run and restored
after (`stay_on_while_plugged_in` 7). The phone held this login's probe
build of the 2026-11-12 record (app 80840ce0, test 4db8c691); it now
holds the clean build below (app d0209344, test 4db8c691).

What was wrong (ux-designer-01, REPLY 01a1d1bd): on the two stress shapes
a Back pressed while the bar showed closed the reader only when the
build ended, 0.87-0.96 s later, where design 2026-11-04 §5.4 asks for
the next gap between turns. The acceptance: on the A40 at the release
profile, three rounds each, recorded, (1) on both stress shapes a Back
about 0.6 s after Open the whole message closes the reader within
0.15 s, focus on the row's Open the whole message, the conversation
where it was; (2) two Backs about 0.3 s apart, the first closing the
reader and the second acting on the conversation, none lost or merged;
(3) the A5 Back rows (list, heading autolinks, reference link) still
pass.

## Builds

Every build is the debug APK (trace, case entry, stand-ins) with its
native library at the root `Cargo.toml`'s `[profile.release]` settings
passed as `CARGO_PROFILE_DEV_*`, as in the 2026-11-12 record: a release
APK carries no trace and cannot be timed by the case. Signer `26d1eb18`.

- **Probe**: main at a04400b39 with logging added and not committed: the
  Activity's `dispatchKeyEvent` logs each key with
  `uptimeMillis() - eventTime` (the key's own time, whatever `input`'s
  start-up), `Native.back` logs on arrival and on the loop, the event
  listener logs each `InputAvailable`, the opening logs each turn, and
  the view logs when it takes the close. App 1374bb6e.
- **Fix with probe**: the same logging over the fix's head c3f15adf5.
  App 0fda4580.
- **Clean**: c3f15adf5 as committed. App d0209344.

The Back-to-close figures below are the fix-with-probe build's: the
logging it adds sits on the path being timed (a line per key, per
`InputAvailable` and per turn), and the clean build's Back cases were
run for pass or fail, not timed. 17234e2fe, after these runs, caps the
input hold at 300 firings in a row; no Back here held an opening that
long, and it has not run on the phone.

## Where the Back was held (probe, 2026-11-13 00:35Z)

One round of `on_a_device_back_past_the_first_go_closes_the_reader_to_its_conversation`.
On distinct headings, the Back pressed 0.65 s after the Enter:

- The native loop took one input event per pass, and each pass ran the
  opening's due turn first (about 70 ms). The step timer was a repeated
  1 ms timer, which Slint re-arms as it fires, so after every turn it was
  due again and no pass went by without a turn.
- Each key passes through the loop twice, once for the keyboard's
  pre-dispatch and once for the dispatch. The platform sends one key at a
  time. So the Enter's release, then the Back's press, then its release
  each waited one to two turns, and the Back reached the Activity
  **678 ms** after it was pressed.
- From the Activity, `Native.back` reached the loop 88 ms later, and the
  close was taken 107 ms after that.
- With the turns no longer holding it (the first fix build, 4257d77f3
  alone, run at 01:29-01:58Z), the close still waited **160 ms** for the
  root's render. A render listed the conversation's
  items and copied each one's parsed body; on these shapes that body is
  a tree of thousands of blocks. On the host at opt-level 3, one render of
  the four long shapes cost 100-160 ms.

Back pressed to the conversation's page in the trace: 954 and 915 ms on
the two stress shapes.

## The fix

- `ui-slint` (4257d77f3, c3f15adf5): the opening rests one pass of the
  loop after its first go and after each turn. It builds nothing for
  150 ms after the event listener sees platform input.
- `ui-model` (e80a1e345): each item's parsed body is shared with the lists
  a render asks for, not copied. The host render above is about 0.3 ms.

## Acceptance (fix with probe, 3 rounds each, 2026-11-13 03:19-03:48Z)

Back is the Back's press time, from its key lag. Close is the
conversation's `page` in the trace. Every round passed: the conversation's
page after the reader's, and focus on the opened row's Open the whole
message.

**1. Back about 0.6 s after the press.** The Backs land 0.61-0.67 s after
the Enter.

| shape | Back to close, ms (r1, r2, r3) |
|---|---|
| heading autolinks | 28, 36, 22 |
| reference link | 43, 44, 49 |
| distinct headings | 27, 47, 40 |
| distinct links | 20, 28, 118 |

**2. Two Backs, 0.3 s apart** (`on_a_device_two_backs_while_the_reader_opens_close_it_then_the_conversation`,
new). Each round shows the reader, the conversation, then the chat list,
in that order.

| shape | first Back to close, ms | second Back to the list, ms |
|---|---|---|
| distinct headings | 33, 36, 29 | 44, 55, 57 |
| distinct links | 26, 73, 25 | 44, 63, 49 |

**3. A5's list case** (`on_a_device_back_while_the_reader_opens_closes_it_to_its_conversation`,
Back about 0.2 s after the press): 20, 22, 25 ms.

**Recordings** (`screenrecord`, every case). Distinct links, case 1,
round 3: the empty reader, the bar for about 0.17 s (it was up when the
Back came), then the conversation as it was, with the focus ring on the
row's Open the whole message. Distinct links, case 2, round 2: bar, then
the conversation, then the chat list.

## The clean build (2026-11-13 03:57-04:36Z)

The three Back cases ran once each and passed. The timed case ran three
rounds and passed every one. Open, ms, median (min-max) of three, beside
the 2026-11-12 record's figures:

| shape | now | 2026-11-12 |
|---|---|---|
| paragraph | 149 (133-150) | 146 |
| list | 179 (175-183) | 370 |
| code | 145 (145-148) | 355 |
| table | 145 (144-149) | 358 |
| headings | 148 (147-152) | 371 |
| images | 152 (149-155) | 403 |
| heading autolinks | 239 (237-240) | 513 |
| reference link | 304 (300-308) | 752 |
| distinct links | 1,017 (958-1,019) | 1,311 |
| distinct headings | 1,411 (1,358-1,416) | 1,652 |

Most of the drop is the render the press waited for before the opening
began: the conversation held earlier rows of these shapes, and each
render copied their bodies.

## Found and not settled

On the first fix build (4257d77f3 alone), case 2 failed one round of
three: the first Back never reached the Activity. Its two passes reached
the native loop just as focus moved into the reader, and the second Back
was then taken as the first. This looks like the keyboard's pre-dispatch
consuming both of the key's events, but that is not verified: that run
kept only the app's log. It did not recur in the six rounds of case 2
since, and those runs keep the whole log.

## Not proved here

- The release APK itself, and the Pixel 9a at this profile.
- TalkBack in the reader while it opens.
- A Back by gesture, or by the app bar's Back: every Back here is a key
  from `input`.
- No test fails on the 0.15 s. The figures are this record's.

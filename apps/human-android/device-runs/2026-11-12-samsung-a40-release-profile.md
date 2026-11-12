# Samsung A40 device runs, 2026-11-12 evening: the reader at the release profile, and Back past the opening's first go

Device: the **Samsung A40** (SM-A405FN, Android 11, **API 30**) of
`2026-11-09-samsung-a40.md`, on this login's own adb server under the
`samsung-a40` lease, over wireless debugging; no adb process of this
login remained after any run (`pgrep -u "$(id -u)" -a adb` empty). Every
time below is UTC. The screen's stay-awake setting was saved before the
runs and restored after (`stay_on_while_plugged_in` 7).

The phone held a release 0.3.0 (signer `a7f38b5e`, app 3b89be89,
installed 2026-11-12 16:42Z) whose installer is not known to this login;
it was uninstalled on the owner's word before the runs. It now holds the
last probe build of this record (app 80840ce0, test 4db8c691).

The case is `tests/android-e2e/tests/long_message.rs`, as in
`2026-11-12-samsung-a40-pixel-9a.md`: a desktop daemon on this host's
loopback sends each shape of `tests/android-e2e/src/long.rs`, the case
presses the row's Open the whole message by key and reads the debug
build's trace (`reader=drawn`, `reader=end`).

## The build

The source is main at 6e52954cb (#643: the opening's 400 ms first go,
then 50 ms turns). A release APK carries no trace, no case entry and no
stand-ins, so it cannot be timed by the case; the build is the debug APK
with its native library compiled at the root `Cargo.toml`'s
`[profile.release]` settings, passed as `CARGO_PROFILE_DEV_*`: opt-level
3, thin LTO, one codegen unit, debug 1, overflow checks **on**, debug
assertions off, no incremental. rustc's command lines were read to carry
`-C opt-level=3 -C codegen-units=1 -C debuginfo=1 -C overflow-checks=on`;
the LTO setting was not read there.
App 48833554, test 4db8c691, signer `26d1eb18`.

## The reader's open, five rounds (17:46 to 18:47Z)

`on_a_device_the_reader_of_each_long_shape_is_timed_open_and_at_its_end`,
six rounds: five passed (626 to 662 s each); round 2 lost its adb link
(exit 255 on wireless) and is left out. Open, ms, median (min-max) of
five, beside the A/B candidate (`2026-11-12-samsung-a40-pixel-9a.md`:
source 6c8765943 with the opening's budgets patched to #643's values,
overflow checks off). The two columns differ in source as well as in
profile: between 6c8765943 and 6e52954cb, #643 moved an opening's end
past the first go from a queued input to a flag taken once the queue
drains, and started the bar's timer before the first go. Only openings
that run past the first go reach either change, which here are the two
stress shapes.

| shape | release profile | candidate |
|---|---|---|
| paragraph | 146 (140-152) | 151 |
| list | 370 (365-371) | 370 |
| code | 355 (352-358) | 354 |
| table | 358 (357-362) | 353 |
| headings | 371 (366-372) | 368 |
| images | 403 (401-407) | 401 |
| heading autolinks | **513** (510-514) | 515 |
| reference link | **752** (748-760) | 754 |
| distinct headings | **1,652** (1,631-1,658) | 1,614 |
| distinct links | **1,311** (1,309-1,325) | 1,298 |

End reached the body's last line in one frame on every shape, at most
65 ms formatted and 24 ms source. No shape moves by more than its own
spread except distinct headings, by 38 ms (spreads 27 and 29 ms);
distinct links moves 13 ms, inside its 16 ms spread. The eight shapes
built within the first go compare the profile alone, and it moves none
of them past its spread; distinct headings' 38 ms is the profile and
#643's change together, and this run does not separate them.

## Back while the reader opens

The case stamps the phone's time at each key's launch. `input` started
about 0.47 s late under `screenrecord`, so a Back's arrival is estimated
from the Enter's own delay (the trace's drawn time less its drawn ms),
and the close is `page=conversation` in the trace; times are after the
open press is taken.

**0.2 s, list**
(`on_a_device_back_while_the_reader_opens_closes_it_to_its_conversation`,
one round, 19:05Z): the Back arrived at 272 ms, inside the first go;
the rows were drawn at 351 ms and the reader closed at 622 ms, 271 ms
after them, to the conversation with focus on the row's Open the whole
message.

**0.6 s, four shapes**
(`on_a_device_back_past_the_first_go_closes_the_reader_to_its_conversation`,
committed after the runs as 04aafce7a; the first three rounds,
18:51 to 19:05Z, ran an earlier form of it on heading autolinks and
reference link only, the next three, 19:10 to 19:38Z, the four shapes;
all passed, recorded. The case as run accepted focus on any row's Open
the whole message and stamped the Back just before its launch; since
411214b42 and 9b3a49736 it asks for the opened row's and stamps after
the launch, and that form has not run on the phone):

| shape | Back arrived | closed | close after Back | rows drawn first |
|---|---|---|---|---|
| heading autolinks | 672-695 ms | 736-785 ms | 62-108 ms | yes |
| reference link | 649-682 ms | 873-889 ms | 192-240 ms | yes |
| distinct headings | 676 ms | 1,636 ms | 960 ms | no; rows never drawn in two of three |
| distinct links | 657-679 ms | 1,553-1,569 ms | 874-908 ms | no |

In these runs heading autolinks and reference link had their rows
drawn at 249-263 and 384-417 ms (the trace's `reader=drawn`), so their
Back lands after the rows. On the stress
shapes it lands inside the opening: the recordings show the empty
reader, then the opening's indeterminate bar moving, and the close only
when the build ends, 0.87 to 0.96 s after the Back. Every close is to the
conversation, focus on an Open the whole message (the check as run).

**Where the Back is held.** Two probe builds (21:10Z, app 6025fae2; and
21:34Z, app 80840ce0, the opening's step timer at 16 ms instead of 1 ms)
logged the activity's `onBackPressed`: it ran only at the build's end,
and the close followed it by 80 to 220 ms, in both. So the Back is held
before the activity sees it, in the native activity's input path (the
input queue the Slint Android backend drains, or the system's dispatcher
waiting on the Enter), not by the opening's timer. Not yet localised
further; the probes are not in the tree.

## What it shows

| case | result |
|---|---|
| the release profile moves the reader's open, shapes built within the first go | none past its spread |
| the same, the stress shapes | distinct headings by 38 ms, source and profile not separated |
| End within 1 s, every shape | yes, one frame |
| Back 0.2 s into an opening closes it to the conversation | yes |
| Back 0.6 s into an opening closes it to the conversation | yes, every shape and round |
| the close within 1 s of the Back | yes; on the stress shapes by 40 to 130 ms only |
| the close at the next gap between turns (design 2026-11-04 §5.4) | **no**: on the stress shapes the Back is held until the build ends, 0.87 to 0.96 s; ux-designer ruled it a miss of §5.4 (2026-11-12) and the fix is job j107 |

## What it does not show

- The release APK itself: its native library is built the same way, but
  its Java side, its signing and its missing trace are not this build's.
- The Pixel at the release profile.
- Why the Back is held before `onBackPressed`, or that a different yield
  to the looper would release it.
- That scrolling shows no blank screen or seam: a person looks.
- TalkBack (A6, A8): blocked on the accessibility adapter.

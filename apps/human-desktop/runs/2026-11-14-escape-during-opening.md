# Desktop runs, 2026-11-14: Escape while a long reader opens

A5's single Backs on the desktop, where Back is Escape
(design/2026-11-04-long-message.md A5 and section 5.4), asked by
ux-designer-01 (REPLY 01a1d958): the list shape's Escape inside the
first turn, the stress shapes' at 0.6 s, void if the first screen comes
first, each timed from when the sending tool sent the Escape. The runs
of two Backs are Android's alone (A5).

## How it was run

- The case: `tests/desktop-e2e/tests/human_app/opening.rs`,
  `escape_while_the_reader_opens_closes_it_to_its_conversation`
  (`#[ignore]`), under `tools/ci/with_display.sh` (Xvfb, its own bus).
- The client: `human-desktop` built with its `ui-trace` feature at the
  release profile, `cargo build --release -p interweave-human-desktop
  --features ui-trace --target-dir target/traced`, named by
  `INTERWEAVE_TRACED_DESKTOP`. Its window writes the trace lines the
  phone's debug APK writes, each stamped on `CLOCK_MONOTONIC`
  (`apps/human-desktop/src/trace.rs`). The shipped build has no trace and
  cannot be timed by the case.
- The presses: XTEST (`display::Hand`), each stamped on the same clock
  the moment before it is sent. Open the whole message is reached by
  keyboard (the message field, then Shift+Tab until the trace says the
  row's `read:` control holds the focus) and pressed by Return; then
  Escape.
- The shapes: the Android case's (`tests/android-e2e/src/long.rs`), each
  at its largest, each unread in a conversation of its own.
- The judge: the phone's (`tests/android-e2e/src/back_run.rs`), plus the
  focus: the trace's last focus line after the close must be the row's
  `read:` key.
- The list shape's Escape: half the least first-screen time of three
  plain opens in the same run, since the first open of a session is
  cold (136 ms against 27 ms after it on the first run of the day).

What the times are: the send of each key, and the trace line logged
when the window's page changed, a frame or so before the screen shows
it. The first screen is no frame either: the desktop's software
renderer sets no rendering notifier, so the `reader=drawn` line is
written when the rows were handed to the window and carries
`frame=untold`; the frame that draws them comes after it. A round is
therefore void on it early, never passed on it. No frame is read, so
whether a frame showed the opening bar is not checked here.

The host: `develop-qzapp`, 10 CPUs, shared with other agents' builds.
Its load average stood at 48-60 before and after runs 2 and 3 below
(`/proc/loadavg`; run 1's was not read), five to six times its CPUs.

## Results

Escape to close, in ms; the limits are 1 s for the list (inside the
first turn) and 0.15 s for the stress shapes (past it). Focus was on
the row's Open the whole message after every close that was judged.

| Run | List plain opens (ms) | List Escape | List rounds | distinct headings | distinct links |
|---|---|---|---|---|---|
| 1 | 59, 148, 73 | 29 ms | 31, 57, 201 | 47, **192**, 35 | 47, 90, 19 |
| 2 | 72, 59, 31 | 15 ms | 185, 21, 141 | 69, 136, 59 | 50, 131, 86 |
| 3 | 47, 37, 37 | 18 ms | 60, 88, 144 | **312**, 81, 45 | 86, 107, *void* |

- Passed: 24 of 27 rounds.
- Failed: two, both distinct headings past 0.15 s: 192 ms (run 1) and
  312 ms (run 3).
- Void: one, distinct links in run 3, whose first screen was drawn
  17 ms before the Escape at 0.6 s (first screen at 583 ms). Every
  other stress round's first screen came after its Escape: the opening
  was still building when the Escape went.
- First screen shown before the close, list rounds: 11-64 ms.

## What this proves and what it does not

- Proved on this host's X display: an Escape inside the first turn
  closes the reader to its conversation, focus on the row's Open the
  whole message, within 1 s (nine of nine); and an Escape at 0.6 s on
  the stress shapes closes it so within 0.15 s in fifteen of the
  seventeen judged rounds (one was void).
- Not separated: whether the two closes past 0.15 s are the client's or
  the host's load. The desktop records no input arrival, so the
  opening's hold after input (Android's) does not apply here; the
  Escape is taken at the next gap between 50 ms turns. At a load of five
  times the CPUs, a turn's wall time stretches with it. A run on a quiet
  host is owed before either close is called the client's or passed.
- Not run: the shipped build (no trace), Wayland, another window system,
  a screen reader during the opening, and a frame-level check that no
  frame showed the bar.
- A mouse wheel turned through XTEST did not move the stress shapes'
  conversation list (forty turns, the row unmoved at y = -508), and
  nothing showed that it moves any list on this display, so the case
  reaches the control by keyboard. Whether a person's wheel scrolls
  that list is not shown here either way.

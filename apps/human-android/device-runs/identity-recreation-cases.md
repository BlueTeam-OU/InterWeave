# Identity flow across a window's end: the device cases

This is a list of cases, not a run record. Each case is named by its number
(D1 to D7). It is what a comment in `apps/human-android` cites when it says
a claim is BLOCKED on a device run. A run of these cases is recorded beside
this file as a dated record, and that record names each case by its number.

These cases check the native and Kotlin wiring of
`onboard.rs::window_ended`, `flow_park.rs` and `MainActivity.onDestroy`. The
pure rules that wiring calls are host-tested beside the code:
- `at_window_end`;
- `Attached::at_rebind`;
- `flow_check`;
- the owed queue.

The cases ask of the platform what no host test can see:
- the order of `onDestroy`, `super.onDestroy()` and the end of the window's
  thread;
- whether a new Activity instance after a recreation takes the flow the old
  one parked;
- that an answer sent to an ended window's loop is not lost.

The design identity note (`design/2026-10-19-android-identity.md`, sections
6 and 7) says what each screen keeps across a recreation. Its own acceptance
items 10 to 12 do not hold these recreation cases.

## How a case is driven

- Build: the debug pair, on the login's own adb server, through
  `.agent-fabric/bin/phone-run` (the remit's test-phone rules). Never
  `pm clear` and never an uninstall without the owner's word.
- Recreation: `cmd locale set-app-locales org.interweave.human --locales <tag>`
  while the screen shows. A change of `locale` is not in `configChanges`, so
  the Activity is recreated in the same process. Undo it with `--locales ''`.
- A start refused for the lock: the debug hook
  `am start -n org.interweave.human/.MainActivity --ei busyStarts 1`. A start
  that holds: `--ei holdStartMs 12000` (DebugHooks.kt).
- A K1: the debug broadcast `REFUSE_IDENTITY`, then a start (DebugHooks.kt).
  Fired while the chats show, then `RESTART`, the K1 is raised over the
  running window (D7, and D2's start). Fired with no Activity alive (Back on
  Chats under Stay reachable, so the service runs on), then `RESTART` and a
  launch from the launcher, the process keeps the K1 and the new window's
  start opens it before any chats attach (D3). The run's record says which
  drive it used.
- Stay reachable (D3, D5): main has no switch for it. On the owner's word,
  write `{"availability_mode": "stay-reachable"}` to
  `<state>/profiles/human-android/availability-overlay.json` and start the
  app from its launcher (as `2026-11-17-pixel-9a-samsung-a40-hold-ping-cost.md`
  did). The record says whether the overlay was left on.
- Counting the attach line: every count of "the chats attach to the window"
  runs from the case's action to the case's end, the last step its row
  names.
- What shows: `screencap` stills, and the debug `ui-trace` lines in logcat.
  Never the words of a phrase: a still taken of S2 or C2 is deleted unread,
  and the case reads only the trace's kinds.

## The cases

| case | the screen and its setup | what is done | it passes when |
|---|---|---|---|
| D1 | C4 with its start held (`holdStartMs`), then that start refused busy (`busyStarts 1`), so `IdStartBusy` shows | a recreation | the new window shows C4 with `IdStartBusy`, not Welcome; the record is read once; logcat has exactly one "the chats attach to the window" from the recreation to the case's end, and it comes before the start succeeds; after the hold, the start runs once and is not refused busy by the old flow's lock |
| D2 | R2 after a kept K1, with R3's `IdStartBusy` reached and Back pressed to R2 | a recreation | R2 shows with no notice; no service start in logcat between the recreation and the next action (`identityState` answered 4, not 5) |
| D3 | a K1 kept in a running process, with the chats not attached under it | a recreation, then End on K1 | the chats attach only at K1's end, as in the first window: logcat has no "the chats attach to the window" between the recreation and End, and exactly one after End |
| D4 | Show my phrase's S2 with the service taken offline for it | a recreation | the new instance shows no phrase: it shows the chats, and S2 is reached again only through Show recovery phrase and its prompt (section 7, a new instance is a new display); after the old window ends, the service is back online and not refused busy |
| D5 | any identity flow screen (C4 or R2), with Stay reachable on so the process outlives the task (the run's record says the mode) | the task removed from Recents, then a launch | the launch shows the start a fresh record gives, not the flow's last screen or its words; logcat has "its identity flow ends with it" at the removal |
| D6 | a held C4 whose start's refusal arrives while the window is recreated | a recreation timed inside the hold | the refusal shows as `IdStartBusy` on the new window's C4 (the owed answer taken once), never lost and never shown twice |
| D7 | a K1 raised over a running window (the chats attached under it) | a recreation, then End on K1 | logcat has exactly one "the chats attach to the window" between the recreation and End, and none after End, as the first window had them attached and its End attached nothing |

## Status

None of D1 to D7 has run yet. On 2026-11-22 this host had no route to the
phones' LAN, and the device slots were stood down. The first run of these
cases is recorded beside this file.

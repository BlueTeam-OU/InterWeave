---
role: "ux-designer"
class: workflow
topic: "device-run-check-focus"
description: On the shared Pixel, a scripted device run checks which app has focus before every tap and stops if it is not InterWeave; blind tap sequences can hit system settings
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 571a58e444b385f4
---

## On the shared Pixel, a scripted device run checks which app has focus before every tap and stops if it is not InterWeave; blind tap sequences can hit system settings

2026-10-19, j26 review run on main ffce28df: Back in the InterWeave
app exits it (a finding), which surfaced the previous task — the
system's Wireless debugging settings page — and the next scripted tap
(meant for a conversation row) landed where "Use wireless debugging"
sits, most likely switching it off. The phone then refused connections
and needed the owner to re-enable it (REQUEST 01a152cc-894b).
Also: right after each `adb connect`, a "Wireless debugging connected"
heads-up covers the top ~300 px and takes taps; wait ~8 s first.

**How to apply:** in every phone-run script, before each input tap,
read `dumpsys window | grep mCurrentFocus` and abort unless it names
org.interweave.human; prefer one action per run while exploring; never
tap blind after Back. Never run adb outside phone-run (one stray
connect attempt was made outside it that day).

Related: [[device-review-pixel-lease]]

2026-10-19 run 2 (worked, phone never lost): a sourced lib whose tap/swipe/key
call a guard on `mCurrentFocus` first, plus a reconnect check (the link drops
for a moment now and then: `adb devices`, then connect again, up to 3 tries).
More lessons:
- The app keeps its screen between runs (an open menu stays open): screenshot
  the current state before choosing taps; derive coordinates from a
  full-resolution shot, never from a scaled montage.
- A setting changed for a review (uimode, font_scale, rotation, per-app locale
  via `cmd locale set-app-locales`) is restored by a trap that runs BEFORE
  the script disconnects, then read back.
- `input motionevent DOWN/MOVE/UP` holds a swipe mid-way for a screenshot of
  the tracking and armed states.
- A snackbar's Undo can be hit at once by finding its light band in the
  screenshot (scan rows for bright pixels) and tapping its end.
- uiautomator dump sees one node (Slint has no Android accessibility bridge),
  so coordinates come from screenshots.
- A shot that lands outside the app can show the owner's home screen
  (personal photos): delete it at once, never keep or commit it.
- 2026-10-19 j1 run: under an RTL locale the snackbar mirrors (Undo at the START);
  my Undo tap at the end hit the text, the snackbar closed, and the relaunch
  dropped the now-unkept test message T3 for good. Before any scripted Undo,
  locate Undo for the current direction and check the result in a shot BEFORE
  any restart; a step that can lose data runs only after its Undo is verified.
- 2026-10-19 re-review: a script's Back on Chats left the app (correct since F1's
  fix), and the owner's previous app (a personal one) came forward. The guard
  stopped the inputs, but a shot was taken. Now `shot` itself refuses unless
  InterWeave has focus; never press Back without knowing the screen; delete
  any shot that may show another app, unviewed.
- 2026-10-19 round 2: the Pixel is also a person's phone in use. Before a launch, read the front window read-only (dumpsys window/power, no input); if another app is in use (WhatsApp, a shade pulled down) do not launch over it — wait and poll. Before typing or pressing Back for the keyboard, read `dumpsys input_method` mInputShown: a keyboard can be up with no field shown, and a "composer" tap then lands on the space bar.
- 2026-10-20: after every adb connect, GrapheneOS shows a "Wireless debugging connected — tap to turn off" heads-up over the top ~400 px for several seconds; the focus guard does NOT catch it (focus stays InterWeave). Wait >=10 s after connect before any tap (step.sh does). A popup menu or dialog does not survive `monkey` relaunch: open and choose in one run, or use a no-relaunch step script. The link drops often: retry the run.

- A restore in an EXIT trap inside a step file runs after step.sh has already disconnected adb, so it never reaches the phone (2026-10-21: Pixel left rotation-locked in landscape for about a minute). Restore rotation as explicit last lines of the step file, then read mCurrentRotation back.

- Before any device run, have the inbox watch running and drain it first: on 2026-10-21 a coordinator broadcast at 06:25Z declared the Pixel unavailable, and it reached me only after my 07:26Z run because my watch was not re-armed after the previous delivery.

2026-10-21 (j31, round 4): the Pixel is also the owner's own phone. A Telegram heads-up arrived between the focus check and a tap and took the tap (it opened the notification; nothing sent). Since then every run turns Do Not Disturb on (`cmd notification set_dnd on`) and sets it back as found (`settings get global zen_mode` first). A personal notification caught in a capture is deleted at once. Restore any changed setting (font_scale, uimode night, rotation, DND) inside the run, before `adb disconnect`: an EXIT trap that runs after the disconnect finds no device and restores nothing (font stayed at 2.0 and light theme for a minute).

*References: device-review-pixel-lease*

*Observed 2026-10-21 (ux-designer)*

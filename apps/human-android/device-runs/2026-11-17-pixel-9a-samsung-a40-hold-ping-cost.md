# Pixel 9a and Samsung A40 device run, 2026-11-17 and 2026-11-18: what the held connection's 15 s ping costs a phone

The measurement `architecture/transport/libp2p/CONNECTIVITY.md` §14
(A 2026-11-13) and ADR-0068 carried as owed: what the ping on the one
held direct connection per data-plane peer costs a phone that is idle
with its screen off. The interval was not changed for the run. Every time below is UTC. The
Pixel's batterystats history prints its own zone (Europe/Rome, UTC+1);
times read from it are converted.

**First, what the default costs.** The human client's default profile
is `foreground-only`. It has no foreground service: leaving the app
ends the session, and the service starts again only on return
(`2026-10-20-gate-b.md`, "Foreground-service lifecycle
(foreground-only profile)"). So with the default, a phone whose screen
is off holds no connection and sends no ping, and the hold costs
nothing. Everything below was measured under **Stay reachable**, the
mode that keeps the session in a foreground service.

## The phones and the build

- **Pixel 9a**: product `tegu`, Android 17 (API 37), build
  `CP3A.261005.005`. Reached through `.agent-fabric/bin/phone-run` under the
  `pixel-9a` lease.
- **Samsung A40**: `SM-A405FN`, Android 11 (API 30), One UI build
  `A405FNXXU4CWC3`. Reached under the `samsung-a40` lease.
- Both were reached on this login's own adb server, over wireless
  debugging at the owner's addresses on one Wi-Fi network.

The debug APK of main at **`8b6b1d35b`** was installed on both phones
on 2026-11-16 at 05:05Z. Built by this login.

- app sha256 `1396145e82edf24928f960594140e31e934a722c47ba420b7b8795bfebc64543`
- androidTest sha256 `88247508c213fb1d231702646e584c33d53c8257af415fd2eb9ac30b2b3b00db`
- signer: this login's debug key

The far end of each connection was a `transport-daemon` built from the
same tree, running on this host. One profile per phone (`j110-far` for
the A40, `j110-far-pixel` for the Pixel), and each phone's profile held
that daemon as its one allowed peer.

**This host cannot be dialled by a phone**, so the far daemon dialled
the phone: a static entry for the phone's Wi-Fi address and the app's
listen port. Main changed after `8b6b1d35b` (this branch was brought up
to main before the record was written). This run is of `8b6b1d35b`.

### How the phones got there

On the owner's word (GZCoord DECISION `01a1dd7d`, relayed by
architect-cto), both phones had the app uninstalled with its data. That
included the earlier build of 2026-11-15, `eb72c4f4b`, which main's
redial change (A 2026-11-14) superseded before any window ran. The
debug pair above was installed.

The profile was then written through the app's own cases: identity, a
fixed listen port, and the far daemon as the one allowed contact.

**main has no switch for Stay reachable**, so on the owner's word
("for me you can use both") this login wrote the availability overlay
on each phone. It was written on the A40 at 2026-11-16T18:05Z, and on
the Pixel the same evening. The file is
`<state>/profiles/human-android/availability-overlay.json`, holding
`{"availability_mode": "stay-reachable"}`. The app was then started
from its launcher.

**Before each phone was unplugged, the Stay reachable foreground
service was running** (`dumpsys activity services`):

- Pixel, 2026-11-17T10:37:29Z, and again 2026-11-18T06:43:58Z:
  `NetworkService` with `isForeground=true` and `types=0x00000200`
  (remoteMessaging), on notification channel `reachable`, pid 30760.
- A40, read at every precheck from 2026-11-16T20:54Z to
  2026-11-17T09:40Z: `isForeground=true` on channel `reachable`, pid
  1250. Android 11's dump prints no `types=` field.

**The overlay is still on both phones as this record is written.**
Whether to remove it is the owner's call and has not been given.

## The method

Each window:

1. Read at the start that the phone was off power.
2. `dumpsys batterystats --reset`.
3. Disconnect adb.
4. Leave the phone untouched for 3 h, screen off.
5. Read at the end: batterystats (text and checkin), battery,
   deviceidle, power, alarm, netstats, services, and logcat.

During the held windows, the far daemon's log recorded every connect
and disconnect. This host sampled the held TCP socket's kernel counters
every 5 s (`ss -tni`): which socket it was, bytes in each direction,
and `lastrcv`, the milliseconds since anything was last received.

The control window ran first on each phone, with the far daemon down
and the phone never connected. The app was otherwise the same: same
build, same overlay, foreground service running, and its own timers.

The script copies of these runs are in this login's scratchpad, not in the tree.

## Pixel 9a

| window | span (UTC) | battery | discharge (screen off) | screen on | deep Doze | state |
|---|---|---|---|---|---|---|
| control (no peer) | 2026-11-17 10:38:00–13:39:31 | 89 → 84 % | 225 mAh (200) | 1m27s, 2× | 2h55m, 3 entries | clean |
| held, first | 2026-11-17 13:42:12–16:42:48 | 84 → 76 % | 355 mAh (168) | **40m14s, 7×** | 2h13m, 4 entries | **spoiled** |
| held, cut | 2026-11-18 06:52:28–06:58:30 | — | — | woken 06:56:54 | — | **cut** |
| held | 2026-11-18 06:58:30–09:58:57 | 92 → 87 % | 263 mAh (260) | 5.4 s, 1× (power button) | 2h54m, 4 entries | clean but for one wake |

**Spoiled and cut windows.** These are kept, so the clean one is not
read as the first try.

- **The first held window was spoiled by use.** The screen was on from
  14:51 to 15:04Z (WhatsApp, Telegram) and from 15:35 to 16:01Z
  (Settings, `io.pcontacts.app`).
- **The 06:52 window was cut.** The screen came on at 06:56:54Z, the
  phone was on AC from 06:56:55 to 06:57:46Z, and `org.linphone` was in
  the foreground. Batterystats was reset again at 06:58:30Z.
- **The clean window had one wake.** At 08:42:22Z the screen was woken
  by the power button (`screenwake android.policy:POWER`) for 5.4 s.
  No app came to the top, there was no plug-in, and it cost 2.5 mAh of
  screen-on discharge. Doze ended for it and came back. The window was
  not restarted for it.

**The clean pair, control against held.** The figures are the phone's
own batterystats. There is one window of each, three hours each, on
consecutive days.

| | control | held |
|---|---|---|
| discharge | 225 mAh | 263 mAh |
| screen-off discharge | 200 mAh over 3h00m | 260 mAh over 3h00m |
| deep-Doze discharge | 213 mAh | 245 mAh |
| kernel wakeups (`+running` in the history) | 4630 | 6452 |
| of which the Wi-Fi chip's host wake (`dhdpcie_host_wake`) | 4543 | 6445 |
| Wi-Fi battery drain (the connectivity summary) | 51.2 mAh | 64.9 mAh |
| Wi-Fi Rx / Tx time | 4.6 s / 4.7 s | 4.9 s / 6.1 s |
| cellular battery drain | 112 mAh | 112 mAh |
| all apps' Wi-Fi data received / sent | 55.0 MB / 3.3 MB | 21.7 MB / 3.7 MB |
| the app's Wi-Fi data received / sent | none listed | 549 KB / 539 KB (7554 / 7786 packets) |
| the app's estimated power (batterystats' model) | no row | 0.092 mAh |
| the app's CPU | 28 s | 39 s |

So with one held peer, the clean held window had 1822 more kernel
wakeups than the control, about ten a minute, almost all of them the
Wi-Fi chip waking the host. It also drew 38 mAh more over three hours
and 13.7 mAh more Wi-Fi drain. Batterystats' per-app model charges the
app 0.092 mAh, because it counts the app's radio time and not the
suspend it interrupts.

**How far the 38 mAh is the ping's is not settled by one pair.** The
two windows ran on different days, against different background
traffic from other apps (55 MB against 22 MB of Wi-Fi data). The
spoiled first held window shows how far that background moves the
total.

**What Doze did to the ping on the Pixel: nothing that the far end
could see.** In the clean held window, the far daemon logged one
connect, at 06:44:18Z, and no disconnect before its shutdown at
09:59:14Z. All 2123 samples of the socket show the same socket (local
port 46342), and the longest `lastrcv` was 15.4 s, against the 15 s
ping. So a ping or its answer crossed the connection within every
interval while the phone sat in deep Doze for 2h54m of the window.

The spoiled first window shows the same: one socket throughout, longest
`lastrcv` 15.4 s.

The host's sampler matched by the phone's port, and so also caught a
loopback socket of another process on this host (`127.0.0.1:8765`).
It is excluded from every figure above.

## Samsung A40

| window | span (UTC) | battery | discharge (screen off) | screen on | Doze | state |
|---|---|---|---|---|---|---|
| control (no peer) | 2026-11-17 10:40:13–13:40:22 | 98 → 88 % | 307 mAh (307) | 1 s interactive | light 1h04m, deep 1h53m | clean |
| held, attempt 1 | 2026-11-17 13:43:39–13:51 | — | — | woken for diagnosis | — | **void** |
| held | 2026-11-17 13:51:28–16:51:36 | 88 → 77 % | 347 mAh (239) | **30m07s, 2×** | light 1h28m, deep 57m | **spoiled** |

The control's dump also prints `Screen on: 3d 8h … 0x`. That is a
counter carried across the reset, not screen time: the window's
interactive time was 1.2 s.

**The A40 froze the app, and so held no connection.** One UI's
`FreecessController` froze the app's process in Doze, foreground
service and all. In logcat it logs `FZ … reason: LEV` about a minute
after the screen goes off, and `UFZ … reason: DeviceIdleOFF` only when
Doze ends. In the control window the thaws lasted 8 s to 2 min, and
the process sat in `freezer:/frozen` in between.

While frozen, the app's listening socket still existed, but nothing
accepted on it. Its backlog filled (`rx_queue` 1 on the LISTEN row) and
the far daemon's SYNs timed out. The phone still answered ICMP ping,
and `netpolicy` showed the uid on the Doze allow list with rules
`ALLOW_ALL`, so the network policy was not what refused it.

- **The control window** was frozen from 10:41:21Z: seven freezes by 12:43:03Z.
  The seventh was still in force when the freeze log was read at about 13:51Z.
- **Held attempt 1** (from 13:42Z) never connected: three dials, all
  `dial_failed`. The diagnosis woke the phone, so the attempt was
  voided and its files kept apart. The window restarted clean at
  13:51:28Z.
- **In the held window, the far daemon made 227 dials and 8
  connected.** 219 were `dial_failed`.
  - It was connected for 35m02s in total.
  - Two of the connections came up while someone was using the phone,
    in its two screen-on spans: 14:38:16–14:48:35Z (Settings) and
    15:06:02–15:25:51Z (`io.pcontacts.app`). They lasted 11m30s and
    20m35s, each closing a minute or so after the screen went off.
  - The other six came up in thaws and lasted 2–102 s, 2m57s in total.

So the A40's held window measured a frozen app plus a person's use,
not a held connection. Its figures are listed for the record and are
not a cost of the hold:

| | control | held |
|---|---|---|
| discharge | 307 mAh | 347 mAh (109 of it screen on) |
| Wi-Fi battery drain | 102 mAh | 106 mAh |
| Wi-Fi Rx time | 1h33m | 1h36m |
| the app's Wi-Fi data received / sent | none listed | 162 KB / 82 KB |
| the app's CPU | 15 s | 73 s |

architect-cto has asked the owner for a second A40 window with
Samsung's battery setting for the app changed. That window is not in
this record.

## What this run does not show

- **A clean held window on the A40.** One UI froze the app, so there
  was nothing held to measure there. Samsung's per-app battery settings
  ("Unrestricted", "Put unused apps to sleep") were not read or changed.
- **What the A40 showed a person while frozen**: the notification, and
  the app's own connection status. Neither was read.
- **More than one window per condition, and more than one held peer.**
  Each Pixel figure is one 3 h window. The cost of several held peers
  is not measured.
- **Cellular.** Both phones were on Wi-Fi with the cellular radio
  idle. A ping over a carrier's network, its radio tail and a carrier
  NAT's mapping were not measured.
- **The far end dialling across a NAT, or a phone dialling out.** The
  far daemon reached the phone directly on one LAN.
- **A packet capture.** There is no root on either phone. The host's
  socket counters and the phone's batterystats stand in for one.
- **The cost of redialling each absent contact every 30 s** (§12,
  A 2026-11-14). The far daemon held a static entry for the phone and
  dialled it; the phone held none, so it dialled nothing.
- **The release build.** This run is of the debug APK at `8b6b1d35b`.

## The sentence proposed for §14

It is proposed to replace the clause in `CONNECTIVITY.md` §14
A 2026-11-13's "Not proved" that begins "and the cost on a phone", and
the matching bullet in ADR-0068's consequences. Both are
architect-cto's to write:

> the cost on a phone was measured on 2026-11-17 and 2026-11-18
> (`apps/human-android/device-runs/2026-11-17-pixel-9a-samsung-a40-hold-ping-cost.md`):
> under Stay reachable, a Pixel 9a idle with its screen off kept one
> held peer's connection through deep Doze for 3 h at the 15 s
> interval. It had 1822 more kernel wakeups and drew 38 mAh more (263
> against 225) than with no peer online, in one window of each. Under
> foreground-only, the default, there is no foreground service, so the
> hold costs nothing with the screen off. On a Samsung A40 (Android 11),
> the system froze the app in Doze despite its foreground
> service, so no connection was held and the cost was not measured
> there. The interval stays 15 s.

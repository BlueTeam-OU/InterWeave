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

Both phones ran the debug APK of main at **`8b6b1d35b`**, built by this
login on 2026-11-16 at 05:05Z. The installs:

- **Pixel**: installed at 05:06Z (`lastUpdateTime` 2026-11-16 06:06:03,
  phone time).
- **A40**: installed first at 05:05Z. The phone then went to another
  login for its run (`2026-11-16-samsung-a40-back-runs.md`), which
  uninstalled this pair with its data at 16:54Z and gave the phone back
  holding its own pair. This login uninstalled that pair and installed
  this one again at 17:03Z, from the same build output (sha256 below).
  The A40 read `lastUpdateTime` 2026-11-16 18:03:29 (phone time,
  UTC+1) at the start of each of its windows.

- app sha256 `1396145e82edf24928f960594140e31e934a722c47ba420b7b8795bfebc64543`
- androidTest sha256 `88247508c213fb1d231702646e584c33d53c8257af415fd2eb9ac30b2b3b00db`
- signer: this login's debug key

The far end of each connection was a `transport-daemon` built from the
same tree, running on this host. One profile per phone (`j110-far` for
the A40, `j110-far-pixel` for the Pixel), and each phone's profile held
that daemon as its one allowed peer.

**This host cannot be dialled by a phone**, so the far daemon dialled
the phone: a static entry for the phone's Wi-Fi address and the app's
listen port.

**Main changed after `8b6b1d35b`.** This branch was brought up to main
before the record was written. Among the changes,
main gained the peer-loss redial ladder (ADR-0011, A 2026-11-17):
`crates/transport/runtime/src/loss_ladder.rs`, started from
`crates/transport/libp2p/src/runtime/dialing.rs`. The A40's dial
pattern below is `8b6b1d35b`'s retry schedule, not main's. This run is
of `8b6b1d35b`.

### How the phones got there

On the owner's word (GZCoord DECISION `01a1dd7d`, relayed by
architect-cto), both phones had the app uninstalled with its data. An
earlier build of this login's, main at `eb72c4f4b` (2026-11-15), was
installed first. No window ran on it, and it was replaced by
`8b6b1d35b` with `install -r` once main had moved. The transport code
of the two commits is the same: what changed between them is the
client's UI (`crates/human/ui-slint`) and documentation.

After each install with data cleared, the profile was written through
the app's own cases: identity, a fixed listen port, and the far daemon
as the one allowed contact. On the A40 that was again at 17:03Z, after
the reinstall, under a new phone identity.

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
  - No services dump was taken at the control's start (10:40Z), but
    the process was still pid 1250.
  - Before the held window, the reading is the control's end read at
    13:40Z: the same `ServiceRecord`, `isForeground=true`, channel
    `reachable`, and pid 1250 again at the held start, 13:51:26Z.

**The overlay is still on both phones as this record is written.**
Whether to remove it is the owner's call and has not been given.

*Note, 2026-11-18:* the owner has since said to leave the overlay on
for now, relayed by architect-cto-01. The sentence above was true when
this record was written.

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
Wi-Fi chip waking the host. It also had 13.7 mAh more Wi-Fi drain.
Batterystats' per-app model charges the app 0.092 mAh, far below any of
the whole-phone differences.

**The discharge difference depends on which counter is read**, and the
phone's counters do not nest:

| counter | difference over 3 h |
|---|---|
| total | 38 mAh (263 − 225) |
| screen off | 60 mAh (260 − 200) |
| deep Doze | 32 mAh (245 − 213) |

- **Screen-on time** accounts for part of the gap between the first
  two: the control's 1m27s of screen-on used 25 mAh, and the held
  window's 5.4 s used 2.5 mAh.
- **The deep-Doze counter exceeds the screen-off counter** in the
  control (213 against 200 mAh), and in the first held window too
  (213 against 168). Deep Doze happens only with the screen off, so
  these counters are not a subset of one another on this build, and
  this record does not say which is right.
- **The battery level** fell by 5 % in each window.

So the held window cost between 32 and 60 mAh more in three hours,
according to the counter.

**How far that is the ping's is not settled by one pair.** The two
windows ran on different days, against different background traffic
from other apps (55 MB against 22 MB of Wi-Fi data). The spoiled first
held window shows how far that background moves the total.

**What Doze did to the ping on the Pixel: nothing that the far end
could see.** In the clean held window, the far daemon logged one
connect, at 06:44:18Z, and no disconnect before its shutdown at
09:59:14Z. All 2123 samples of the socket show the same socket (local
port 46342), and the longest `lastrcv` was 15.4 s, against the 15 s
ping.

- **The bound this gives.** The samples are about 5.1 s apart, so a
  sample can miss up to that much of a gap. The longest receive gap
  was therefore at most about 20.5 s.
- **No missed liveness.** The connection never missed liveness. A
  missed liveness is two failed pings in a row: the pinger forgives the
  first (`crates/transport/libp2p/src/relay_keepalive.rs`). It would
  have closed the connection, and the far daemon logged no disconnect.
  That held while the phone sat in deep Doze for 2h54m of the window.
  The sampled bound alone cannot rule out one forgiven timeout.

The spoiled first window shows the same: one socket throughout, longest
`lastrcv` 15.4 s.

The host's sampler matched by the phone's port, and so also caught a
loopback socket of another process on this host (`127.0.0.1:8765`).
It is excluded from every figure above.

## Samsung A40

| window | span (UTC) | battery | discharge (screen off) | screen on | Doze | state |
|---|---|---|---|---|---|---|
| control (no peer) | 2026-11-17 10:40:13–13:40:22 | 98 → 88 % | 307 mAh (307) | 1 s interactive | light 1h04m, deep 1h53m | clean |
| held, attempt 1 | 2026-11-17 13:43:39–13:51 (far daemon up 13:42) | — | — | woken for diagnosis | — | **void** |
| held | 2026-11-17 13:51:28–16:51:36 | 88 → 77 % | 347 mAh (239) | **30m07s, 2×** | light 1h28m, deep 57m | **spoiled** |

The control's dump also prints `Screen on: 3d 8h … 0x`. That is a
counter carried across the reset, not screen time: the window's
interactive time was 1.2 s.

**The A40 froze the app, and so held no connection while the screen
was off and the app frozen.** It connected only in thaws and while a
person used the phone. One UI's
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
- **Held attempt 1** never connected. The far daemon came up at 13:42Z
  and dialled from 13:42:41Z, before the reset at 13:43:39Z: three dials, all
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
not an idle held connection. Its figures are listed for the record and are
not a cost of the hold:

| | control | held |
|---|---|---|
| discharge | 307 mAh | 347 mAh (239 screen off, 109 screen on, as batterystats rounds them) |
| Wi-Fi battery drain | 102 mAh | 106 mAh |
| Wi-Fi Rx time | 1h33m | 1h36m |
| the app's Wi-Fi data received / sent | none listed | 162 KB / 82 KB |
| the app's CPU | 15 s | 73 s |

architect-cto has asked the owner for a second A40 window with
Samsung's battery setting for the app changed. That window is not in
this record.

## What this run does not show

- **A clean held window on the A40.** One UI froze the app, so there
  was no idle held connection to measure there. Samsung's per-app
  battery settings were not read or changed, and their names on this
  phone's One UI version were not read either.
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
  *Note, 2026-11-18:* since ADR-0011 A 2026-11-17, a contact lost
  after a connection that held for 30 s or more is first redialled at
  1, 2, 4, 8 and 16 s, and only then every 30 s. This run's build,
  `8b6b1d35b`, has no `loss_ladder.rs`, so the ladder did not run here,
  and its cost on a phone is not measured either.
- **A relayed connection's ping.** ADR-0068 pings relayed connections too. Here, the one connection was direct.
- **The release build.** This run is of the debug APK at `8b6b1d35b`.

## Where the measured cost is recorded

architect-cto wrote this measurement into the architecture in the same
change as this record:

- **`architecture/transport/libp2p/CONNECTIVITY.md` §14**, in A
  2026-11-13's "Not proved": the clause on the cost on a phone.
- **ADR-0068's consequence bullet** on the ping per held peer, with its
  amendment in `architecture/adr/history/0068-amendments.md`.
- **The plan's §20 closing record**: the A40's freeze, carried as a
  named limit of Stay reachable.

The figures they cite are this record's. Where they and this record
differ, this record is the measurement.

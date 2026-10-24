# android-e2e: an added address survives a reboot, between a Pixel 9a and an A40 over Wi-Fi — 2026-10-24

The owner's second limit on row 1
(`2026-10-24-pixel-9a-a40-wifi-added-address.md`): it shows nothing
about "a phone reboot". This file is row 1 with A (the Pixel) rebooted
in place of the force-stop: `tests/pairing.rs`'s
`on_two_phones_an_added_address_survives_a_reboot`. A adds B's (the
A40's) LAN address through the embedded host's binding and connects.
The test then reboots A with `adb reboot`, waits for the same phone
(`ro.serialno`) on a new boot, and starts the app again with the next
case. It was run by p2p-network-dev-02 at architect-cto's request,
01a16e09 (job j69). The owner was at the Pixel for the run and read its
wireless-debugging address before and after the reboot. architect-cto
relayed both.

The host output of each run is below, as printed but for cargo's build
lines and blank lines. One substitution is made: the Pixel's app PeerId
is written `<pixel>`, and the A40's `<a40>`.

## What ran

| | |
|---|---|
| Device A | Pixel 9a, Android 17, API 37, `ro.serialno` `58091JEBF08730`. Wireless adb: `192.168.100.52:43637` before run 1's reboot, `192.168.100.53:45969` before run 2's, `192.168.100.54:44599` after it. Each boot gave the phone a new IP on the LAN and a new wireless-debugging port. |
| Device B | Samsung Galaxy A40 (SM-A405FN), Android 11, API 30, wireless adb `192.168.100.6:36456`, not rebooted |
| App build | `develop-qzapp/p2p-network-dev-02/feat/j69-j67-the-owner-s-word-on` at `837663a2`, the build of the cache-on row (`2026-10-24-pixel-9a-a40-wifi-added-address-cache-on.md`). App APK sha256 `960add4016b8351e08858767234d334d619e1b169b18a1689d56726192589a70`; androidTest APK sha256 `1d289d625eb2cc9d881cc7bd53101eafa5aba17d037fa0da0c780ff424f8bd96` |
| Host side | `tests/android-e2e` and `tests/android-e2e-cases` at `837663a2` |
| Profiles | As row 1: the shipped `human-android.yaml`, each side trusting the other, no static-bootstrap entry, and the peer cache and Kademlia off on both. After the reboot, the bootstrap-peers overlay is the only source of B's address on A. B holds no address for A. |
| Link | Phone to phone over the LAN. `adb reverse --list` was empty on both phones, asserted by the test, on A's post-reboot serial too. |
| Command | `cargo test -p interweave-android-e2e-tests --test pairing -- --ignored --exact on_two_phones_an_added_address_survives_a_reboot --nocapture --test-threads 1`, with `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase`, `INTERWEAVE_ANDROID_SERIAL_A` set to A's address before the run's reboot, `INTERWEAVE_ANDROID_SERIAL_B` and `INTERWEAVE_ANDROID_WIFI_ADDRESS_A/_B`, under the `pixel-9a` and `samsung-a40` leases, on this account's own adb server port. `INTERWEAVE_ANDROID_REBOOT_PATIENCE_S` unset, so the default 1200 s. |

## Results

| Run | Started | Result |
|---|---|---|
| 1 | 16:16:48Z | fail at the wait: the reboot was issued at about 16:17Z, no post-reboot address reached this host within 1200 s, and the test stopped at 16:37:30Z. The half before the reboot passed. Nothing past the reboot was tested. |
| 2 | 16:53:45Z | pass, 193.86 s |

Run 1 is the harness's limit, not the system's. A phone on wireless
debugging comes back from a boot on a new port, which only a person at
the phone can read, and none reached this host in time. The test
cannot reconnect the phone by itself.

Run 2 showed the following, all asserted by the test or its cases:

- **Before the reboot**, as row 1: A's row for B read not connected for
  3 s before the add; earlier runs' addresses for B were removed first.
  A connected to B 2.0 s after the add and held it 5 s. B, holding no
  address for A, saw the connection.
- **The reboot.** The test issued `adb reboot` to A at 16:54:49Z. B's
  case then saw A leave. At 16:56:51Z this host connected A's new
  address, and the test found A under the same `ro.serialno` with
  `sys.boot_completed` 1 and a different
  `/proc/sys/kernel/random/boot_id` (`b05d0f83-…` before,
  `0680d334-…` after).
- **What the platform did to the process.** The reboot ended A's
  process with the device. After the boot, at 113.9 s of uptime and
  before any case ran, `pidof org.interweave.human` found no process.
  The platform started nothing of the app's after the boot. The app
  declares no boot receiver, by decision (architect-cto, 01a16e10).
- **After the reboot.** The test re-read A's identity under its new
  serial: the same PeerId. The next case started the app's process and
  runtime. A's case found B's address listed as added and dialled
  (`source: added`, `dialled: true`), with no add. A connected to B
  261 ms after holding it, and held it 5 s. B saw the connection, and
  then saw A leave when the host force-stopped A.

## What this run does not prove

- **A dial at boot.** Nothing of the app's runs until it is started, so
  the reconnect waits for the app's next start, here by the test's
  instrumentation. A start by the user, from the launcher, was not run.
- **A reboot with the peer cache on.** These profiles turn the cache
  off, as row 1's do. The cache-on row covers the cache only across a
  force-stop.
- **More than one passing run.** Each run needs the owner to read the
  phone's address twice.
- **The Scan screen, a relayed path, message exchange, the app in the
  background, IPv6, a LAN that isolates its clients.** As row 1.
- **Who dialled, beyond inference.** As in row 1: B holds no address
  for A, so the connection is A's dial.

## Host output: run 1

```text
running 1 test
test on_two_phones_an_added_address_survives_a_reboot ... rebooted 192.168.100.52:43637 (ro.serialno 58091JEBF08730); waiting up to 1200s for it on a new boot -- `adb connect` its new wireless-debugging port if it does not come back by itself
thread 'on_two_phones_an_added_address_survives_a_reboot' (4018886) panicked at tests/android-e2e/src/adb.rs:510:9:
58091JEBF08730 was not back on a new boot within 1200s
FAILED
failures:
failures:
    on_two_phones_an_added_address_survives_a_reboot
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 1239.72s
```

## Host output: run 2

```text
running 1 test
test on_two_phones_an_added_address_survives_a_reboot ... rebooted 192.168.100.53:45969 (ro.serialno 58091JEBF08730); waiting up to 1200s for it on a new boot -- `adb connect` its new wireless-debugging port if it does not come back by itself
A rebooted: Rebooted { serial: "192.168.100.54:44599", boot_id: ("b05d0f83-24ba-4b56-b122-7c737aa93745", "0680d334-471c-40a5-b8ec-3555c7f67448"), uptime: "113.87", app_pids: "" }
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/19714/p2p/<a40> "2028" ms after holding it (removed first: true, removed after: -); B saw it after "8398" ms
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/19714/p2p/<a40> "261" ms after holding it (removed first: -, removed after: -); B saw it after "3067" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 193.86s
```

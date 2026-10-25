# android-e2e: the cache-on row re-run with a live control, between a Pixel 9a and an A40 over Wi-Fi — 2026-10-25

The four variants of
`2026-10-24-pixel-9a-a40-wifi-added-address-cache-on.md` again, three
times each, at a driver whose "neither" control runs with B up and
listening. That file's runs 4, 8 and 12 were no control: on a phone
B's runtime lives only while a B case runs, and the driver at
`837663a2` ran no B case during the control window (the blind review
of #421, F1). The driver since `bf114692` runs B's `await_peer` with an
`apart` window through the control, and fails the run if B's case ends
before A's window does. It was run by p2p-network-dev-02 at
architect-cto's request, 01a16e09 (job j69), as the re-run their REPLY
01a16f10 named.

The variants, the profiles and what each case asserts are those of
the earlier file, which is not repeated here. The host output of each
run is below, as printed but for cargo's build and download lines and
blank lines. One substitution is made: the Pixel's app PeerId is
written `<pixel>`, and the A40's `<a40>`. Both were the same in every
run: the app's data was kept from the earlier runs, not cleared. The
phones' logcat is not committed, because it carries other apps' lines.

## What ran

| | |
|---|---|
| Device A | Pixel 9a, Android 17, API 37, `google/tegu/tegu:17/CP3A.261005.005/2026102101:user/release-keys`, wireless adb `192.168.100.54:44599` |
| Device B | Samsung Galaxy A40 (SM-A405FN), Android 11, API 30, wireless adb `192.168.100.6:36456` (as in the earlier file) |
| App build | `origin/main` at `8f978966`, which holds `bf114692`. rust-ui-dev-01 built it with their debug key and installed it with `install -r`, data kept (REPLY 01a17099). App APK sha256 `daf309985bd2a8224f7783154dcdcf8cbb9dd66d3fbbe61af045669d55b338ac`; androidTest APK sha256 `1d289d625eb2cc9d881cc7bd53101eafa5aba17d037fa0da0c780ff424f8bd96`. Installed on A at phone time 2026-10-25 05:05:42 (UTC+4), read from `dumpsys package`; on B at about 05:07 by the installer's word. |
| Host side | `tests/android-e2e` and `tests/android-e2e-cases` at `8f978966`, the tree the app was built from |
| Addresses | A is `192.168.100.54/24` on `wlan0` (it was `.52` in the earlier file; the Pixel's address has moved on each boot), and B is `192.168.100.6/24`. Each listens on its wildcard at the port the run chose. This host, a VM behind its host's NAT, carries only adb. |
| Profiles | As in the earlier file: A has the peer cache ON and Kademlia off; B has both off and holds no address for A. |
| Link | Phone to phone over the LAN. `adb reverse --list` was empty on both phones before and after each run, and the test asserts that. |
| Command | As in the earlier file, under the `pixel-9a` and `samsung-a40` leases, on this account's own adb server port. The test crates were built after run 1's start time and before the test ran; its 41.32 s is libtest's own time and excludes the build, which shows only in the gap before run 2. |

## Results

| Run | Started | Variant | Result | A connected again after starting |
|---|---|---|---|---|
| 1 | 01:08:33Z (build first) | both kept | pass, 41.32 s | 259 ms |
| 2 | 01:25:39Z | the cache alone | pass, 39.61 s | 254 ms |
| 3 | 01:26:21Z | the overlay alone | pass, 38.47 s | 522 ms |
| 4 | 01:27:03Z | neither | pass, 62.01 s | stayed unconnected for 15 s, B up and listening |
| 5 | 01:28:06Z | both kept | pass, 40.40 s | 255 ms |
| 6 | 01:28:50Z | the cache alone | pass, 39.34 s | 768 ms |
| 7 | 01:29:30Z | the overlay alone | pass, 39.85 s | 510 ms |
| 8 | 01:30:16Z | neither | pass, 61.88 s | stayed unconnected for 15 s, B up and listening |
| 9 | 01:31:23Z | both kept | pass, 39.77 s | 255 ms |
| 10 | 01:32:04Z | the cache alone | pass, 39.10 s | 761 ms |
| 11 | 01:32:44Z | the overlay alone | pass, 40.06 s | 508 ms |
| 12 | 01:33:25Z | neither | pass, 62.77 s | stayed unconnected for 15 s, B up and listening |

What the earlier file says of the first start, the add, the cache file
on disk after the force-stop, and the cache alone and the overlay alone
each sufficing held again in all twelve runs, read from the same
lines. A connected to B 1015–1526 ms after the add (within 1.0–1.6 s,
the range the earlier file's runs also span). What this file adds is the control:

- **Neither: A stayed unconnected, with B listening.** In runs 4, 8
  and 12, with B's added address removed from A's overlay and A's
  cache file removed, A stayed unconnected to B for 15 s after it
  started again. B's `await_peer` case was running throughout that
  window, its own window 15 s longer than A's, and the host asserted
  that B's case had not ended when A's window did. B's case then
  passed: its row for A read unconnected at every poll of its window,
  and the case fails on the first poll that reads connected. So the
  window measured against a listening B, and a dial from any third
  source left on A would have failed it from either end. In these
  runs, with Kademlia off, nothing but the overlay and the cache
  dialled B after a restart.
- **Both kept** is again the baseline (255–259 ms), and still does not
  say which source the dial came from: the two support one candidate
  with one address, as the earlier file explains.

## What this run does not prove

The earlier file's list holds, except its first item: the "neither"
control has now measured that nothing else dials B after the restart,
for this profile (Kademlia off). Still not proved, from that list: the
shipped discovery set with Kademlia on, the cache's write timing for a
kill sooner after a connection, message exchange, who dialled beyond
inference, and the Scan screen, the app's graceful stop, a relayed
path, a change of network, the app in the background, IPv6 and a LAN
that isolates its clients. The reboot is its own row
(`2026-10-24-pixel-9a-a40-wifi-added-address-reboot.md`).

## Host output: run 1, both kept

```text
running 1 test
test on_two_phones_with_the_cache_on_both_sources_kept_reconnect ... Both: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/18800/p2p/<a40> "1274" ms after holding it (removed first: true, removed after: -); B saw it after "6108" ms
Both: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/18800/p2p/<a40> "259" ms after holding it (removed first: -, removed after: -); B saw it after "2034" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 41.32s
```

## Host output: run 2, the cache alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_cache_alone_reconnects ... CacheAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/15702/p2p/<a40> "1270" ms after holding it (removed first: true, removed after: true); B saw it after "6618" ms
CacheAlone: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/15702/p2p/<a40> "254" ms after holding it (removed first: -, removed after: -); B saw it after "1788" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 39.61s
```

## Host output: run 3, the overlay alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_overlay_alone_reconnects ... OverlayAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/27120/p2p/<a40> "1269" ms after holding it (removed first: false, removed after: -); B saw it after "5858" ms
OverlayAlone: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/27120/p2p/<a40> "522" ms after holding it (removed first: -, removed after: -); B saw it after "2803" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 38.47s
```

## Host output: run 4, neither

```text
running 1 test
test on_two_phones_with_the_cache_on_and_neither_source_kept_nothing_dials ... Neither: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/21917/p2p/<a40> "1271" ms after holding it (removed first: true, removed after: true); B saw it after "6368" ms
Neither: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> stayed unconnected to /ip4/192.168.100.6/tcp/21917/p2p/<a40> for Some(15s) with neither source left, B up and listening throughout
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 62.01s
```

## Host output: run 5, both kept

```text
running 1 test
test on_two_phones_with_the_cache_on_both_sources_kept_reconnect ... Both: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/13256/p2p/<a40> "1268" ms after holding it (removed first: false, removed after: -); B saw it after "6366" ms
Both: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/13256/p2p/<a40> "255" ms after holding it (removed first: -, removed after: -); B saw it after "2282" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 40.40s
```

## Host output: run 6, the cache alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_cache_alone_reconnects ... CacheAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5107/p2p/<a40> "1016" ms after holding it (removed first: true, removed after: true); B saw it after "6367" ms
CacheAlone: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5107/p2p/<a40> "768" ms after holding it (removed first: -, removed after: -); B saw it after "2549" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 39.34s
```

## Host output: run 7, the overlay alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_overlay_alone_reconnects ... OverlayAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/24175/p2p/<a40> "1521" ms after holding it (removed first: false, removed after: -); B saw it after "7131" ms
OverlayAlone: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/24175/p2p/<a40> "510" ms after holding it (removed first: -, removed after: -); B saw it after "2294" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 39.85s
```

## Host output: run 8, neither

```text
running 1 test
test on_two_phones_with_the_cache_on_and_neither_source_kept_nothing_dials ... Neither: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/8790/p2p/<a40> "1520" ms after holding it (removed first: true, removed after: true); B saw it after "7128" ms
Neither: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> stayed unconnected to /ip4/192.168.100.6/tcp/8790/p2p/<a40> for Some(15s) with neither source left, B up and listening throughout
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 61.88s
```

## Host output: run 9, both kept

```text
running 1 test
test on_two_phones_with_the_cache_on_both_sources_kept_reconnect ... Both: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/23911/p2p/<a40> "1026" ms after holding it (removed first: false, removed after: -); B saw it after "6102" ms
Both: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/23911/p2p/<a40> "255" ms after holding it (removed first: -, removed after: -); B saw it after "2036" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 39.77s
```

## Host output: run 10, the cache alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_cache_alone_reconnects ... CacheAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/12005/p2p/<a40> "1015" ms after holding it (removed first: true, removed after: true); B saw it after "6370" ms
CacheAlone: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/12005/p2p/<a40> "761" ms after holding it (removed first: -, removed after: -); B saw it after "2294" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 39.10s
```

## Host output: run 11, the overlay alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_overlay_alone_reconnects ... OverlayAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/11369/p2p/<a40> "1526" ms after holding it (removed first: false, removed after: -); B saw it after "6871" ms
OverlayAlone: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/11369/p2p/<a40> "508" ms after holding it (removed first: -, removed after: -); B saw it after "2292" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 40.06s
```

## Host output: run 12, neither

```text
running 1 test
test on_two_phones_with_the_cache_on_and_neither_source_kept_nothing_dials ... Neither: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/31823/p2p/<a40> "1521" ms after holding it (removed first: true, removed after: true); B saw it after "6618" ms
Neither: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> stayed unconnected to /ip4/192.168.100.6/tcp/31823/p2p/<a40> for Some(15s) with neither source left, B up and listening throughout
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 62.77s
```

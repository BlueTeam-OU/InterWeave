# android-e2e: an added address across a restart with the peer cache on, between a Pixel 9a and an A40 over Wi-Fi — 2026-10-24

The owner's first limit on row 1
(`2026-10-24-pixel-9a-a40-wifi-added-address.md`): "It shows nothing
about a restart with the address cache on". The row is run again with
the peer cache on, in four variants of `tests/pairing.rs`. Three of
them tell the two sources of B's address on A apart by taking one or
both away before A starts again; "both kept" is the baseline with
neither taken away, and does not tell them apart (below):

| Variant | Test | Before A's second start |
|---|---|---|
| both kept | `on_two_phones_with_the_cache_on_both_sources_kept_reconnect` | the overlay keeps B's added address, and the cache keeps its file |
| the cache alone | `on_two_phones_with_the_cache_on_the_cache_alone_reconnects` | A's case removes B's address from the overlay after the first connection's hold |
| the overlay alone | `on_two_phones_with_the_cache_on_the_overlay_alone_reconnects` | A's cache file is removed between the two starts |
| neither | `on_two_phones_with_the_cache_on_and_neither_source_kept_nothing_dials` | both: the address removed and the file removed. A must then stay unconnected to B for 15 s |

Otherwise each variant is row 1: A (the Pixel) adds B's (the A40's) LAN
address through the embedded host's binding and connects, is
force-stopped, and is started again. It was run by p2p-network-dev-02
at architect-cto's request, 01a16e09 (job j69).

The host output of each run is below, as printed but for cargo's build
lines and blank lines. One substitution is made: the Pixel's app PeerId
is written `<pixel>`, and the A40's `<a40>`. The phones' logcat is not
committed, because it carries other apps' lines.

## What ran

| | |
|---|---|
| Device A | Pixel 9a, Android 17, API 37, wireless adb `192.168.100.52:43637` (as in row 1) |
| Device B | Samsung Galaxy A40 (SM-A405FN), Android 11, API 30, wireless adb `192.168.100.6:36456` (as in row 1) |
| App build | `develop-qzapp/p2p-network-dev-02/feat/j69-j67-the-owner-s-word-on` at `837663a2`. rust-ui-dev-01 built it with their debug key and installed it with `install -r`, data kept (REPLY 01a16e9d). App APK sha256 `960add4016b8351e08858767234d334d619e1b169b18a1689d56726192589a70`; androidTest APK sha256 `1d289d625eb2cc9d881cc7bd53101eafa5aba17d037fa0da0c780ff424f8bd96`. Installed at phone time 19:51 on A and 19:52 on B (UTC+4). |
| Host side | `tests/android-e2e` and `tests/android-e2e-cases` at `837663a2`, the same tree the app was built from |
| Addresses | A is `192.168.100.52/24` and B is `192.168.100.6/24`, both on `wlan0`. Each listens on its wildcard at the port the run chose. The run adds `/ip4/192.168.100.6/tcp/<port>/p2p/<a40>` to A. This host, a VM behind its host's NAT, carries only adb. |
| Profiles | The shipped `human-android.yaml`, each side trusting the other, neither authoring a static-bootstrap entry. A has the peer cache ON and Kademlia off. Kademlia is seeded from the cache, so it would be a third source the variants cannot tell apart. B has both off, as in row 1, and holds no address for A, so B never dials A. |
| Link | Phone to phone over the LAN. `adb reverse --list` was empty on both phones before and after each run, and the test asserts that. |
| Command | `cargo test -p interweave-android-e2e-tests --test pairing -- --ignored --exact <test> --nocapture --test-threads 1`, with `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase`, `INTERWEAVE_ANDROID_SERIAL_A/_B` and `INTERWEAVE_ANDROID_WIFI_ADDRESS_A/_B` as above, under the `pixel-9a` and `samsung-a40` leases, on this account's own adb server port |

## Results

| Run | Started | Variant | Result | A connected again after starting |
|---|---|---|---|---|
| 1 | 15:54:08Z | both kept | pass, 43.03 s | 255 ms |
| 2 | 15:54:55Z | the cache alone | pass, 41.09 s | 265 ms |
| 3 | 15:55:43Z | the overlay alone | pass, 41.33 s | 257 ms |
| 4 | 15:56:29Z | neither | pass, 50.09 s | stayed unconnected for 15 s, B not listening |
| 5 | 15:57:51Z | both kept | pass, 41.06 s | 509 ms |
| 6 | 15:58:35Z | the cache alone | pass, 41.52 s | 262 ms |
| 7 | 15:59:21Z | the overlay alone | pass, 42.06 s | 254 ms |
| 8 | 16:00:08Z | neither | pass, 50.02 s | stayed unconnected for 15 s, B not listening |
| 9 | 16:01:04Z | both kept | pass, 43.41 s | 509 ms |
| 10 | 16:01:58Z | the cache alone | pass, 44.10 s | 763 ms |
| 11 | 16:02:46Z | the overlay alone | pass, 42.69 s | 258 ms |
| 12 | 16:03:37Z | neither | pass, 49.54 s | stayed unconnected for 15 s, B not listening |

Each run showed the following. The cases assert all of it except the
cache file's contents, which the `peer_cache` case reports and the host
prints, and the host asserts only in the cache-alone variant:

- **A clean start.** Before A's first start, A's cache file was read
  and removed (`peer_cache` with `forget`), so the run began with no
  cached address for B from an earlier run. A's row for B then read
  not connected for 3 s before the add, with B listening: the control
  that nothing else dials B. Earlier runs' added addresses for B were
  removed first, as in row 1.
- **The add.** As in row 1: A connected to B within 1.0–1.3 s of the
  add, and held it 5 s. B, holding no address for A, saw the
  connection, and then saw A leave when the host force-stopped A.
- **What survived the force-stop on disk.** With A force-stopped and no
  runtime up, A's cache file was present and named B in all twelve
  runs. A force-stop skips the runtime's shutdown, which is where the
  cache is written regardless of its write interval. The file was
  removed before A's first start, so the entry was written during the
  run, between A's connection and the kill: the 5 s hold and the
  case's end.
- **Each source alone suffices.** With the overlay's entry removed (A's
  restarted case asserted the address was not listed as added), A
  connected to B again from the cache alone, in 262–763 ms. With the
  cache file removed, A connected again from the overlay alone, in
  254–258 ms.
- **The neither variant measured nothing.** With both removed, A
  stayed unconnected to B for 15 s after it started, but B was not
  listening in that window. On a phone B's runtime lives only while a
  B case runs, and at `837663a2` the driver ran no B case during the
  control, so the window could not have failed whatever A still held
  (the blind review of #421, F1). These three runs are not a control.
  What does stand in for one here is narrower: before each add, A read
  B unconnected for 3 s with B listening (above). The driver at
  `bf114692` runs B's `await_peer` with an `apart` window through the
  control and fails if B's case ends first; the four variants are owed
  a re-run on the phones at that commit or later.
- **Both kept.** A connected again in 255–509 ms. Which source "was"
  the dial has no answer here, because the two do not dial separately.
  Discovery merges providers by address, so both sources support one
  candidate for B with one address (`AggregatedCandidate`, its
  `sources` set, `crates/transport/runtime/src/discovery.rs`). The
  dial is to that address, whichever source is listed. The row did not
  read the candidate's `sources` at run time.

## What this run does not prove

- **That nothing but the two sources dials B after the restart.** The
  neither control did not measure it (above). Without it, the cache
  alone's and the overlay alone's reconnects rest on the sources this
  run removed, read back by the cases (the overlay entry not listed,
  the cache file `forgotten`), not on a control that would have failed
  if a third source were left.

- **The shipped discovery set.** A has Kademlia off. With it on, as
  shipped, a third source is seeded from the cache, and no variant
  here covers it.
- **The cache's write timing in general.** The entry was on disk at
  each kill, a little over 5 s after the connection. A kill sooner
  after a connection may come before the cache's next write. No run
  measured that window.
- **The Scan screen, a reboot, the app's graceful stop, a relayed path,
  a change of network, the app in the background, IPv6, a LAN that
  isolates its clients.** As row 1. The reboot is its own row.
- **Message exchange.** These rows assert the connection only.
- **Who dialled, beyond inference.** As in row 1: B holds no address
  for A and has no cache, so the connection is A's dial. Neither
  phone's TCP table was read.

## Host output: run 1, both kept

```text
running 1 test
test on_two_phones_with_the_cache_on_both_sources_kept_reconnect ... Both: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/11034/p2p/<a40> "1269" ms after holding it (removed first: true, removed after: -); B saw it after "6884" ms
Both: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/11034/p2p/<a40> "255" ms after holding it (removed first: -, removed after: -); B saw it after "2041" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 43.03s
```

## Host output: run 2, the cache alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_cache_alone_reconnects ... CacheAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/22214/p2p/<a40> "1016" ms after holding it (removed first: true, removed after: true); B saw it after "6112" ms
CacheAlone: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/22214/p2p/<a40> "265" ms after holding it (removed first: -, removed after: -); B saw it after "2305" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 41.09s
```

## Host output: run 3, the overlay alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_overlay_alone_reconnects ... OverlayAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/9687/p2p/<a40> "1267" ms after holding it (removed first: false, removed after: -); B saw it after "6124" ms
OverlayAlone: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/9687/p2p/<a40> "257" ms after holding it (removed first: -, removed after: -); B saw it after "2305" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 41.33s
```

## Host output: run 4, neither

```text
running 1 test
test on_two_phones_with_the_cache_on_and_neither_source_kept_nothing_dials ... Neither: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/24613/p2p/<a40> "1269" ms after holding it (removed first: true, removed after: true); B saw it after "6622" ms
Neither: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> stayed unconnected to /ip4/192.168.100.6/tcp/24613/p2p/<a40> for Some(15s) with neither source left
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 50.09s
```

## Host output: run 5, both kept

```text
running 1 test
test on_two_phones_with_the_cache_on_both_sources_kept_reconnect ... Both: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/31287/p2p/<a40> "1269" ms after holding it (removed first: false, removed after: -); B saw it after "6629" ms
Both: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/31287/p2p/<a40> "509" ms after holding it (removed first: -, removed after: -); B saw it after "2801" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 41.06s
```

## Host output: run 6, the cache alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_cache_alone_reconnects ... CacheAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5600/p2p/<a40> "1529" ms after holding it (removed first: true, removed after: true); B saw it after "6625" ms
CacheAlone: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5600/p2p/<a40> "262" ms after holding it (removed first: -, removed after: -); B saw it after "2294" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 41.52s
```

## Host output: run 7, the overlay alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_overlay_alone_reconnects ... OverlayAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/9710/p2p/<a40> "1527" ms after holding it (removed first: false, removed after: -); B saw it after "6621" ms
OverlayAlone: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/9710/p2p/<a40> "254" ms after holding it (removed first: -, removed after: -); B saw it after "2544" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 42.06s
```

## Host output: run 8, neither

```text
running 1 test
test on_two_phones_with_the_cache_on_and_neither_source_kept_nothing_dials ... Neither: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/31665/p2p/<a40> "1016" ms after holding it (removed first: true, removed after: true); B saw it after "6106" ms
Neither: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> stayed unconnected to /ip4/192.168.100.6/tcp/31665/p2p/<a40> for Some(15s) with neither source left
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 50.02s
```

## Host output: run 9, both kept

```text
running 1 test
test on_two_phones_with_the_cache_on_both_sources_kept_reconnect ... Both: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/8181/p2p/<a40> "1524" ms after holding it (removed first: false, removed after: -); B saw it after "6615" ms
Both: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/8181/p2p/<a40> "509" ms after holding it (removed first: -, removed after: -); B saw it after "2806" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 43.41s
```

## Host output: run 10, the cache alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_cache_alone_reconnects ... CacheAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5009/p2p/<a40> "1015" ms after holding it (removed first: true, removed after: true); B saw it after "6117" ms
CacheAlone: A's cache between the starts: file "present", names B "true", forgotten "false"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5009/p2p/<a40> "763" ms after holding it (removed first: -, removed after: -); B saw it after "2805" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 44.10s
```

## Host output: run 11, the overlay alone

```text
running 1 test
test on_two_phones_with_the_cache_on_the_overlay_alone_reconnects ... OverlayAlone: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/18898/p2p/<a40> "1269" ms after holding it (removed first: false, removed after: -); B saw it after "6367" ms
OverlayAlone: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/18898/p2p/<a40> "258" ms after holding it (removed first: -, removed after: -); B saw it after "2299" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 42.69s
```

## Host output: run 12, neither

```text
running 1 test
test on_two_phones_with_the_cache_on_and_neither_source_kept_nothing_dials ... Neither: A's cache before the first start: file "present", forgotten "true"
before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/20454/p2p/<a40> "1525" ms after holding it (removed first: true, removed after: true); B saw it after "6879" ms
Neither: A's cache between the starts: file "present", names B "true", forgotten "true"
after the restart: A <pixel> stayed unconnected to /ip4/192.168.100.6/tcp/20454/p2p/<a40> for Some(15s) with neither source left
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 49.54s
```

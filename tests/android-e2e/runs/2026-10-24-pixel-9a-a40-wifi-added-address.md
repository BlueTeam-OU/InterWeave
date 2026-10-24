# android-e2e: an added address survives a restart, between a Pixel 9a and an A40 over Wi-Fi — 2026-10-24

Plan §29 (e)'s first device row, `tests/pairing.rs`'s
`on_two_phones_an_added_address_is_dialled_and_survives_a_restart`.
Two phones on one Wi-Fi network, with no `adb reverse` anywhere: A (the
Pixel) adds B's (the A40's) LAN address through the embedded host's
binding and dials it. A is then force-stopped and relaunched, and dials B
again from the bootstrap-peers overlay alone. It was run by
p2p-network-dev-02 at architect-cto's request, 01a16db2 (job j66). The
owner placed both phones on the LAN.

The host output of each run is below, as printed but for cargo's build
lines, blank lines and trailing spaces. One substitution is made: the
Pixel's app PeerId is written `<pixel>`, and the A40's `<a40>`. The
phones' logcat is not committed, because it carries other apps' lines.
Where a failure is explained, the lines it rests on are quoted.

## What ran

| | |
|---|---|
| Device A | Pixel 9a, Android 17, API 37, `google/tegu/tegu:17/CP3A.261005.005/2026102101:user/release-keys`, wireless adb `192.168.100.52:43637` |
| Device B | Samsung Galaxy A40 (SM-A405FN), Android 11, API 30, `samsung/a40eea/a40:11/RP1A.200720.012/A405FNXXU4CWC3:user/release-keys`, wireless adb `192.168.100.6:36456` |
| App build (passing runs) | `develop-qzapp/p2p-network-dev-02/test/j66-stage-21-close-the-two-device` at `96c2a52b`. rust-ui-dev-01 built it with their debug key and installed it with `install -r`, data kept. App APK sha256 `e484d7737bbe2f1d6d69c34eff00e566cfd60921bd82025658834cb6712405e5`; androidTest APK sha256 `1d289d625eb2cc9d881cc7bd53101eafa5aba17d037fa0da0c780ff424f8bd96`. Installed at phone time 2026-10-24 16:58:43 on A, and at 16:59:22 (app) and 16:59:25 (test) on B. Phone times are UTC+4. |
| Host side | `tests/android-e2e` at `96c2a52b` for run 5, and at `14745401` for runs 6–8. The two differ only in the printed line, which gained each side's PeerId and address. `tests/android-e2e-cases`, the case bodies the app carries, is the same at both. |
| Addresses | A is `192.168.100.52/24` on `wlan0`, and B is `192.168.100.6/24` on `wlan0`. Each listens on its wildcard at the port the run chose. The run adds B's address to A as `/ip4/192.168.100.6/tcp/<port>/p2p/<a40>`. This host, a VM at `10.137.0.2` behind its host's NAT, carries only adb. |
| Profiles | The shipped `human-android.yaml`, each side trusting the other in `config.yaml`. Neither side authors a static-bootstrap entry. Both have the peer cache off, and Kademlia, which the cache seeds, off too. So after the restart the overlay is the only source of B's address on A. |
| Link | The data path is the LAN, phone to phone. Wireless adb carries only the instrumentation and the status read. `adb reverse --list` was empty on both phones before and after each run, and the test asserts that. |
| Command | `cargo test -p interweave-android-e2e-tests --test pairing -- --ignored --exact on_two_phones_an_added_address_is_dialled_and_survives_a_restart --nocapture --test-threads 1`, with `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase`, `INTERWEAVE_ANDROID_SERIAL_A/_B` and `INTERWEAVE_ANDROID_WIFI_ADDRESS_A/_B` as above, under the `pixel-9a` and `samsung-a40` leases, on this account's own adb server port |

## Results

| Run | Started | Build | Result |
|---|---|---|---|
| 1 | 12:15:23Z | `173f4dfb` | not run: the A40 refused the first `adb connect` after the server started, and the test stopped at its reversal check |
| 2 | 12:15:56Z | `173f4dfb` | fail: the far end's case ended on seeing the connection (below) |
| 3 | 12:33:07Z | `de47200d` | not run: the Pixel refused the first `adb connect` after the server started |
| 4 | 12:33:49Z | `de47200d` | fail: the dialler's case ended on seeing the connection (below) |
| 5 | 12:59:59Z | `96c2a52b` | pass, 35.10 s |
| 6 | 13:01:55Z | `96c2a52b` | pass, 35.08 s |
| 7 | 13:02:34Z | `96c2a52b` | pass, 36.07 s |
| 8 | 13:03:19Z | `96c2a52b` | pass, 36.84 s |

Runs 1 and 3 are the harness's, not the system's. Either phone sometimes
refuses, or accepts and then drops, the first `adb connect` after an adb
server starts. The run script now reconnects until `adb devices` lists
both phones as `device`.

Runs 2 and 4 failed on the cases' design, a pair of bugs that only a
device shows. On a phone, a case's process IS the runtime, so a case that
ends closes its connections. In run 2, the A40's `await_peer` ended on
seeing the connection, and the Pixel's log read
`16:16:20.379 peer connected … path=direct`, then
`16:16:20.428 peer disconnected … reason=closed`, 49 ms later, before
the Pixel's case read its row. `de47200d` made the far end wait for the
dialler to leave. In run 4, the mirror: the Pixel's case passed at
`16:34:12.183 peer connected`, its runtime stopped at `16:34:12.234`, and
the A40 saw `16:34:11.476 peer connected` and `16:34:11.518 peer
disconnected` (its clock reads about 0.7 s behind the Pixel's). Its case
never saw the connection. `96c2a52b` made the dialler hold the
connection for 5 s, failing if it drops, and remove every earlier run's
address for the peer. Each run's port is new, so those addresses are
stale. The host stand-ins could show neither bug, because their runtime
outlives a case.

Each passing run showed the following, all asserted by the cases:

- **No authored entry.** A's bootstrap list held no `config.yaml` entry
  for B, and B's none for A.
- **The control first.** B's `await_peer` held no address for A; its
  earlier runs' added ones were removed first. With B listening, as a
  TCP connect from this host to B's address showed, A's row for B read
  not connected for 3 s before the add. Earlier runs' addresses for B
  were also removed (`removed first: true`). The runtime may dial those
  at its start, but each one is on an earlier run's port and reaches
  nothing.
- **The add.** A's `add_bootstrap_address` of B's address answered
  `added`. Within 1.0–1.5 s, A's row for B read connected and stayed
  connected for the 5 s hold. B's row for A read connected, and then not
  connected once the host force-stopped A. Since B holds no address for
  A, that connection is A's dial.
- **The restart.** A was force-stopped and relaunched: a new app process
  and a new runtime. A's case, making no add, found B's address listed
  as added and dialled (`source: added`, `dialled: true`). Within
  0.26–0.77 s A was connected to B again, and held it 5 s. B saw it
  again and then saw A leave.

## What this run does not prove

- **The Scan screen.** The add is the binding's function itself, the
  call the screen will make, not the screen.
- **A restart with the peer cache on.** The shipped profile turns the
  cache on, and the cache also keeps a reached peer across a restart.
  These profiles turn it off so that the overlay is the only source of
  B's address, so the run says nothing about the two sources together.
- **A device reboot, or the app's graceful stop.** The restart is the
  platform's force-stop and a fresh instrumentation run.
- **A relayed path, DCUtR or AutoNAT.** The profiles' infrastructure
  entries are the shipped examples' unreachable placeholders.
- **Message exchange.** This row asserts the connection only. A message
  each way between the two phones is row 2's (`pair`).
- **A change of network, the app in the background or dozing, a LAN that
  isolates its clients, or IPv6.**
- **The far end's view of WHO dialled, beyond inference.** B holds no
  address for A, so the connection is A's dial. Neither phone's TCP
  table was read; this host cannot see a phone-to-phone socket.

## Host output: run 2 (excerpt)

```text
running 1 test
test on_two_phones_an_added_address_is_dialled_and_survives_a_restart ...
thread 'on_two_phones_an_added_address_is_dialled_and_survives_a_restart' (1114703) panicked at tests/android-e2e/tests/pairing.rs:117:5:
assertion `left == right` failed: A before the restart: {"case": String("added_address"), "detail": String("not connected to <a40> within 30s of holding its address: Some(PeerGateView { peer: TransportIdentity(\"<a40>\"), connected: false, backoff_until: Some(1792844210783), quarantined_until: None, route_held_until: None, last_outcome: Some(Denied) })"), "result": String("fail")}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 49.06s
```

## Host output: run 4 (excerpt)

```text
running 1 test
test on_two_phones_an_added_address_is_dialled_and_survives_a_restart ...
thread 'on_two_phones_an_added_address_is_dialled_and_survives_a_restart' (1313205) panicked at tests/android-e2e/tests/pairing.rs:118:5:
assertion `left == right` failed: B before the restart: {"case": String("await_peer"), "detail": String("<pixel> did not connect within 60s: PeerGateView { peer: TransportIdentity(\"<pixel>\"), connected: false, backoff_until: None, quarantined_until: None, route_held_until: None, last_outcome: Some(Connected) })"), "result": String("fail")}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 73.86s
```

## Host output: run 5

```text
running 1 test
test on_two_phones_an_added_address_is_dialled_and_survives_a_restart ... before the restart: A connected "1267" ms after holding B's address (removed first: true); B saw it after "6626" ms
after the restart: A connected "267" ms after holding B's address (removed first: -); B saw it after "2290" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 35.10s
```

## Host output: run 6

```text
running 1 test
test on_two_phones_an_added_address_is_dialled_and_survives_a_restart ... before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5412/p2p/<a40> "1015" ms after holding it (removed first: true); B saw it after "5857" ms
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5412/p2p/<a40> "765" ms after holding it (removed first: -); B saw it after "2549" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 35.08s
```

## Host output: run 7

```text
running 1 test
test on_two_phones_an_added_address_is_dialled_and_survives_a_restart ... before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5255/p2p/<a40> "1269" ms after holding it (removed first: true); B saw it after "6616" ms
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/5255/p2p/<a40> "761" ms after holding it (removed first: -); B saw it after "2819" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 36.07s
```

## Host output: run 8

```text
running 1 test
test on_two_phones_an_added_address_is_dialled_and_survives_a_restart ... before the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/17367/p2p/<a40> "1524" ms after holding it (removed first: true); B saw it after "6867" ms
after the restart: A <pixel> connected to /ip4/192.168.100.6/tcp/17367/p2p/<a40> "255" ms after holding it (removed first: -); B saw it after "2545" ms
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 36.84s
```

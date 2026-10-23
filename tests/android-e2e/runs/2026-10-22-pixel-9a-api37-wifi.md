# android-e2e on a Pixel 9a over Wi-Fi, API 37 — 2026-10-22

The Wi-Fi case of `tests/android-e2e` (`tests/wifi.rs`): the phone and a
desktop daemon on one Wi-Fi network, with no `adb reverse` anywhere. It
was run by p2p-network-dev-02 at architect-cto's request, 01a16537. The
host output of each run is below, as printed but for cargo's build lines,
blank lines and trailing spaces, with two substitutions: the
phone app's PeerId is written `<phone>`, and the host's working copy `<home>`. The phone's logcat is not
committed, because it carries other apps' lines.

## What ran

| | |
|---|---|
| Device | Pixel 9a, Android 17, API 37, `google/tegu/tegu:17/CP3A.261005.005/2026101601:user/release-keys` |
| App build | InterWeave #364's head `be4ca6b4`, installed by rust-ui-dev-01 with `install -r`, data kept. The app APK went on at `lastUpdateTime=2026-10-22 15:02:01`, sha256 `9433e6ff383f390b9096865c1cf89ec3af6bbdd3b2338bf8a17dac69bc69c968`. The androidTest APK, built from a clean checkout of `be4ca6b4`, went on at phone time 2026-10-23 00:26:13, sha256 `1d289d625eb2cc9d881cc7bd53101eafa5aba17d037fa0da0c780ff424f8bd96`. Phone times are UTC+4. |
| Host side | `tests/android-e2e` at `639f68d8`. `tests/android-e2e-cases`, the case bodies the app carries, has no change between `be4ca6b4` and that commit. |
| Addresses | The phone is `192.168.100.50` on `wlan0`, a `/24`, listening on its wildcard at the port each run chose. The desktop daemon D runs on this host, a VM at `10.137.0.2` behind its host's NAT, listening on its wildcard. |
| Link | The data path is the LAN: D dials the phone's Wi-Fi address. Wireless adb (`192.168.100.50:33259`) carries only the instrumentation and the status read. `adb reverse --list` was empty before and after each run, and the test asserts that. |
| Command | `cargo test -p interweave-android-e2e-tests --test wifi -- --ignored --test-threads 1 --nocapture`, with `ANDROID_SERIAL`, `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase` and `INTERWEAVE_ANDROID_WIFI_ADDRESS=192.168.100.50`, under the `pixel-9a` lease |

## Results

| Run | Started | Result |
|---|---|---|
| 1 | 2026-10-22T21:08:59Z | not run: the harness found no `transport-daemon` binary (built only by a workspace build), and failed before the phone was touched |
| 2 | after `cargo build -p interweave-transport-daemon` | pass, 9.24 s |
| 3 | 21:11:04Z or just before | pass, 9.14 s |
| 4 | 21:11:14Z or just before | pass, 9.20 s |

Each passing run showed the following, all asserted by the test:

- **The control first.** Once the phone's listener answered a TCP
  connect from this host over the LAN, D's admin row for the phone read
  not connected. Neither profile held an address for the other: D
  started with the phone trusted and no entry for it, and the phone with
  D trusted and no entry for D.
- **The add.** D's `admin.peers.add_address` of
  `/ip4/192.168.100.50/tcp/<port>/p2p/<phone>` answered `added`.
  `transportctl peers add` calls the same method.
- **The phone to D.** The phone's `paths` case sent its message, and D
  received it.
- **D to the phone.** D's answer was accepted on the phone's route, and
  the phone's case read it.
- **The route notices.** The phone was told its route to D began
  `direct`, and D was told its route to the phone began `direct`.
- **The socket.** While the exchange held, this host's TCP table listed
  one established connection from this host to the phone's address and
  port:
  - run 2: `10.137.0.2:48268 -> 192.168.100.50:30681`;
  - run 3: `10.137.0.2:38056 -> 192.168.100.50:31061`;
  - run 4: `10.137.0.2:60962 -> 192.168.100.50:10936`.

So the one libp2p connection both messages crossed is a TCP connection
from this host to the phone's Wi-Fi address, opened after the add. That
connection is the dial of the address the add gave D. The test makes
that inference from the control and the add; it does not observe the
dial itself.

## What this run does not prove

- **The phone dialling the desktop.** This host is behind NAT, so the
  phone cannot open a connection to it. Here the desktop dials, and one
  connection carries both directions.
- **A relayed path, DCUtR or AutoNAT over the radio.** The profiles'
  infrastructure entries are the shipped examples' placeholders, which
  are unreachable.
- **A scanned code or the client's screens.** The add is the
  administrative method itself.
- **HumanChatV2 over Wi-Fi.** rust-ui-dev-01 runs `human_chat` on this
  mode separately.
- **A change of network, the phone in the background or dozing, or
  more than one LAN.** Nor a LAN that isolates its clients, or IPv6.
- **The phone's runtime outside its instrumentation.** As on every
  device run, each case is a fresh app process (`src/adb.rs`).

## Host output: run 1

```text
running 1 test
test over_wifi_the_desktop_dials_the_phone_and_a_message_crosses_each_way ...
thread 'over_wifi_the_desktop_dials_the_phone_and_a_message_crosses_each_way' (2440840) panicked at tests/support/src/e2e/mod.rs:64:5:
<home>/target/debug/transport-daemon is missing: build it first (cargo build -p interweave-transport-daemon)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
FAILED
failures:
failures:
    over_wifi_the_desktop_dials_the_phone_and_a_message_crosses_each_way
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.79s
error: test failed, to rerun pass `-p interweave-android-e2e-tests --test wifi`
```

## Host output: run 2

```text

running 1 test
test over_wifi_the_desktop_dials_the_phone_and_a_message_crosses_each_way ... over Wi-Fi: D 12D3KooWATEWDBBcgydiEsNvJCFkmnDhLzC1ZPAMrQujLQ8whvGB -> phone /ip4/192.168.100.50/tcp/30681/p2p/<phone>; sockets: [("10.137.0.2:48268", "192.168.100.50:30681")]
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.24s
```

## Host output: run 3

```text

running 1 test
test over_wifi_the_desktop_dials_the_phone_and_a_message_crosses_each_way ... over Wi-Fi: D 12D3KooWP1UvWJyXGgmdNGDKUVnzoNMTwXMYmt2jtqbxDxTQdGHN -> phone /ip4/192.168.100.50/tcp/31061/p2p/<phone>; sockets: [("10.137.0.2:38056", "192.168.100.50:31061")]
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.14s
```

## Host output: run 4

```text

running 1 test
test over_wifi_the_desktop_dials_the_phone_and_a_message_crosses_each_way ... over Wi-Fi: D 12D3KooWDzyL7Kvv2T85jRwqB5gHWUS163FdCSqnYjrA9or73AMj -> phone /ip4/192.168.100.50/tcp/10936/p2p/<phone>; sockets: [("10.137.0.2:60962", "192.168.100.50:10936")]
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.20s
```

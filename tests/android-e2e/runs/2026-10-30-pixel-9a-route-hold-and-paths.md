# android-e2e on a Pixel 9a: route_hold and paths, API 37 — 2026-10-30

Two device suites of `tests/android-e2e`, run by p2p-network-dev-02:

- `tests/route_hold.rs`, the ADR-0011 route hold on the phone (A
  2026-10-21). architect-cto assigned it in 01a15f41.
- `tests/paths.rs`, which carries `close_exchange` on a device.

Each run's host output is summarised below. The phone app's `PeerId`
and the desktop's are written `<peer>` in the quoted log lines. The
phone's full logcat is not committed, because it carries other apps'
lines.

## What ran

| | |
|---|---|
| Device | Pixel 9a, `google/tegu/tegu:17/CP3A.261005.005/2026102101:user/release-keys` |
| App build | InterWeave `main` at `aec72790`, built by rust-ui-dev-01 with its debug key and installed with `install -r`, data kept. The app APK (`versionName` 0.2.0) has sha256 `d8b293ebd8cec14428dc3f73a7a163581c7e2c0746ed81c7956762894a5a6ca3`; the androidTest APK has sha256 `425c686e5932735a3d6543b8c0f79884e6dd4697873201c4a4d8ac181fcb2f7a`. Both went on at phone time `2026-10-30 06:10:54`; the phone clock is UTC+1. |
| Host side | `tests/android-e2e` at `aec72790`, the APK's commit. |
| Link | Wireless adb, `192.168.1.10:43463`. The relay and the desktop listen on this host's loopback, and the phone reaches them through `adb reverse` (`src/adb.rs`). That is not a radio network, and there is no NAT. `adb reverse --list` was empty after each run. |
| Command | `cargo test -p interweave-android-e2e-tests --test route_hold -- --ignored --test-threads 1 --nocapture`, then the same with `--test paths`. Each ran with `ANDROID_SERIAL` set to the address above and `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase`, under the `pixel-9a` lease, on an adb server of its own. |
| Phone state | Before each run, the app's `config.yaml`, `bootstrap-overlay.json` and `trust-overlay.json` were saved, and they were written back after it. Their sha256 prefixes `6bb0e441…`, `010d8146…` and `a7d34f16…` read the same before and after every run. Nothing was installed, uninstalled or cleared by this lane. |

## Results

| Suite | Run | Started (UTC) | Result |
|---|---|---|---|
| route_hold | 1 | before 05:16Z, without the logcat capture | pass |
| route_hold | 2 | 05:16:46Z | pass, 17.67 s |
| route_hold | 3 | 05:17:11Z | pass, 18.00 s |
| route_hold | 4 | 05:17:37Z | pass, 18.13 s |
| paths | 1 | 05:19:01Z | both device cases pass, 20.93 s |
| paths | 2 | 05:19:30Z | both device cases pass, 21.46 s |

The host stand-ins of both suites also passed at `aec72790`, just before.

### route_hold

Each passing run showed the following, all asserted by the test:

- **The control first.** With the relay up, the phone's route to D, the
  desktop, came up, and the case found it connected and not held.
- **The hold.** The relay was dropped and the app's process restarted
  with only the circuit to D. The case then read the admin port's peer
  row for D for six reconnect ticks (one second each). Every read
  carried the same `route_held_until`, and D never connected.
- **The log.** The phone's log has a failed hop to D before the refusal.
  The reconnect round's refusal names the class and the same end,
  within a second. No dial to D is logged after it.

Run 4's connectivity lines, as logcat printed them (phone time, UTC+1):

```
10-30 06:17:50.677 16358 16380 D interweave: interweave::connectivity: dial failed peer=<peer> class=dial_failed
10-30 06:17:50.677 16358 16380 D interweave: interweave::connectivity: retry scheduled peer=<peer> origin=relay_circuit attempt=1 delay_ms=30000 peer_backoff=false
10-30 06:17:50.695 16358 16381 D interweave: interweave::connectivity: dial failed peer_class=infrastructure class=dial_failed
10-30 06:17:50.695 16358 16381 D interweave: interweave::connectivity: retry scheduled peer_class=infrastructure origin=relay_reservation attempt=1 delay_ms=30000 peer_backoff=true
10-30 06:17:51.674 16358 16380 I interweave: interweave::connectivity: reconnect refused peer=<peer> class=RouteHeld held_until_ms=1793337500675
```

`held_until_ms` 1793337500675 is 05:18:20.675Z, 30 s after the hop failed
at 05:17:50.677Z: the retry base. Runs 2 and 3 logged one
`class=RouteHeld` refusal each, and no dial to D after it, until the case
ended about six seconds later.

### paths

Each run passed both device cases, `on_a_device_the_phone_crosses_the_relayed_path_with_a_stand_in_as_the_control` and `on_a_device_the_phone_crosses_the_direct_path_beside_a_relayed_stand_in`. In them:

- the phone is R in one case and C in the other;
- a direct message crosses each path both ways;
- each exchange ends with `close_exchange`: D sends its closing message
  once the phone's answer was accepted, and the phone's half waits for
  it before its case ends. The 2026-10-20 Pixel run
  (`2026-10-20-pixel-9a-api37.md`) failed on that order, and only a
  device shows it.

## What this does not prove, by name

- **A hold's end.** The watch stays inside the first 30 s hold.
- **The hold lifted by the relay's return.** `tests/connectivity/tests/relayed_paths.rs` holds that on the host.
- **A relay down from the radio network.** Here it is a refused loopback port behind adb's reversed tunnel.
- **Any NAT, and more than one host.**
- **Everything else that `paths.rs` and `route_hold.rs` name in their headers.**

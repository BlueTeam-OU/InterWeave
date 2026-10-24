# android-e2e: two fresh phones exchange messages after both trust, and not before — Pixel 9a and A40 over Wi-Fi, 2026-10-24

Plan §29 (e)'s second device row, `tests/pairing.rs`'s
`on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before`.
Two phones on one Wi-Fi network, each with a fresh profile, with no
`adb reverse` anywhere. Each phone is given the other's address through
the embedded host's binding, standing in for the Scan screen. Each
trusts the other's `PeerId` in turn, and a message crosses only once
both trust. It was run by p2p-network-dev-02 at architect-cto's request,
01a16db2 (job j66). The owner placed both phones on the LAN.

"Fresh" is the app's data cleared: `pm clear org.interweave.human` on
both phones before every run. The owner gave the word at 13:08Z ("clear
interweave data on both phones"). Each run therefore starts from a new
identity on each phone. The host output of each run is below, as printed
but for cargo's build lines, blank lines and trailing spaces. One
substitution is made: each run's Pixel app PeerId is written `<pixel>`,
and its A40 app PeerId `<a40>`. The phones' logcat is not committed,
because it carries other apps' lines. Where a failure is explained, the
lines it rests on are quoted.

## What ran

| | |
|---|---|
| Device A | Pixel 9a, Android 17, API 37, `google/tegu/tegu:17/CP3A.261005.005/2026102101:user/release-keys`, wireless adb `192.168.100.52:43637` |
| Device B | Samsung Galaxy A40 (SM-A405FN), Android 11, API 30, `samsung/a40eea/a40:11/RP1A.200720.012/A405FNXXU4CWC3:user/release-keys`, wireless adb `192.168.100.6:36456` |
| App build (runs 4–6) | `develop-qzapp/p2p-network-dev-02/test/j66-stage-21-close-the-two-device` at `cc64aa55`. rust-ui-dev-01 built it with their debug key and installed it with `install -r`. App APK sha256 `ef21a7a37356ff1e8fd2b6a8059469386a93d469bc50f81ee7911a80d7d38fc2`; androidTest APK sha256 `1d289d625eb2cc9d881cc7bd53101eafa5aba17d037fa0da0c780ff424f8bd96`. Installed at phone time 2026-10-24 17:25:26 (app) and 17:25:27 (test) on A, and 17:26:08 and 17:26:12 on B. Phone times are UTC+4. |
| App build (runs 1–3) | `96c2a52b`, app APK sha256 `e484d7737bbe2f1d6d69c34eff00e566cfd60921bd82025658834cb6712405e5`, the same androidTest APK, the same installer |
| Host side | `tests/android-e2e` at `5278c620` for runs 1–3 and at `cc64aa55` for runs 4–6 |
| Addresses | A is `192.168.100.52/24` on `wlan0`, and B is `192.168.100.6/24` on `wlan0`. Each listens on its wildcard at the port the run chose, and each run's two addresses are printed below. This host, a VM at `10.137.0.2` behind its host's NAT, carries only adb. |
| Profiles | The shipped `human-android.yaml` with `trust.allowed_peers: []`, so each side trusts nobody at the start. Peer cache, Kademlia and the rest are as shipped. |
| Link | The data path is the LAN, phone to phone. Wireless adb carries only the instrumentation and the status read. `adb reverse --list` was empty on both phones before and after each run, and the test asserts that. |
| Command | `pm clear org.interweave.human` on both, then `cargo test -p interweave-android-e2e-tests --test pairing -- --ignored --exact on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before --nocapture --test-threads 1`, with `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase`, `INTERWEAVE_ANDROID_SERIAL_A/_B` and `INTERWEAVE_ANDROID_WIFI_ADDRESS_A/_B` as above, under the `pixel-9a` and `samsung-a40` leases, on this account's own adb server port |

## Results

| Run | Started | Build | Result |
|---|---|---|---|
| 1 | 13:09:14Z | `96c2a52b` | pass, 56.03 s |
| 2 | 13:10:44Z | `96c2a52b` | fail: the first side required its close's acceptance (below) |
| 3 | 13:11:51Z | `96c2a52b` | pass, 54.90 s |
| 4 | 13:26:53Z | `cc64aa55` | pass, 55.30 s |
| 5 | 13:27:58Z | `cc64aa55` | pass, 54.11 s |
| 6 | 13:29:01Z | `cc64aa55` | pass, 54.69 s |

Run 2 failed in the last step, on the case's design rather than the
system. A, the first side, sent its close once it had read B's message
and required the close to be accepted. B, the second side, ends on
reading the close. On a phone the case's process IS the runtime, so B's
end closed the connection before the acceptance travelled back. A's
log read `17:11:40.147 peer connected`, then `17:11:41.044 peer
disconnected … reason=closed`, and A's send answered `PeerUnreachable`.
`TRANSPORT.md` classes that as "outcome unknown", not "not sent". B's own
case had passed: it read the close. `cc64aa55` made A report the close's
outcome rather than fail on it, as `close_exchange` already does, and
end only once B has left. A close never sent still fails the run,
because B passes only on reading it.

Each passing run showed the following, all asserted by the cases:

- **Fresh.** Neither phone trusted the other before its trust step:
  each step that expects the pair apart fails a side that already
  trusts. Each run's PeerIds are new.
- **Step 1, the addresses and no trust.** Each phone added the other's
  LAN address through the binding (`added`), in place of the scan. Then,
  for 15 s from each side's start, every direct message either sent to
  the other was refused: 59 tries each way in every run. None arrived,
  and neither side's last outcome for the other recorded any dial
  (`none`: no peer row, or no dial yet).
- **Step 2, A alone trusts B.** A set B trusted. For 15 s more, no
  message A sent was accepted (26–43 tries) and none either way arrived,
  while B still did not trust A. A's row for B ended `denied` in runs 1,
  2, 3, 5 and 6 and `connected` in run 4. On the host stand-ins it read
  `connected`. A dialled B, and B closed the connection either before or
  just after it was established. Either way no message crossed, which is
  what the step asserts.
- **Step 3, both trust.** A and B were each started afresh, as every
  case on a phone is, and B set A trusted. A message from A was accepted
  by B and read there, a message from B was accepted by A and read
  there, and B read A's close. A's close reported `accepted` in runs
  4–6. Both rows ended `connected`.

So no message crosses until each side has the other's address AND
trusts the other's PeerId. With one-sided trust, A's row for B read
`connected` in run 4, and no message crossed. This file does not quote
the log that would show B closing that connection. By design, B classes
a peer only after Noise authenticates it, and A is not authorized there
(`architecture/transport/libp2p/SECURITY.md`, Connectivity
infrastructure admission). So this row reads the gate's "connect" as a
connection that carries a message. The test's name keeps the gate's
words, and the plan's record is where that reading is settled. Step 3 is the positive control. It
had the addresses step 1 added. The peer cache and Kademlia are on in
these profiles, as shipped, so the run does not exclude a cached address
as a second source of step 3's dial.

## What this run does not prove

- **The Scan screen.** The address is given through the binding's add,
  the call the screen makes, not by scanning a shown code. It is re-run
  through the Scan screen once rust-ui-dev-01's camera Scan lands (the
  Show screen, #409, is on main).
- **The one-sided step in the other direction.** B trusting first, with
  A not trusting, is not run.
- **Which side dialled in step 3, or from which source.** Both hold the
  other's address, so either may have. The peer cache is on, so a cached
  address may also have been used.
- **What step 2's refusal was, below the outcome class.** A's row reports
  `denied` or `connected`. The finer class is in the connectivity log,
  which this file does not quote for the passing runs.
- **A LAN that isolates its clients, IPv6, a relayed path, or the app
  in the background.**
- **The phone's runtime outside its instrumentation.** Each case is a
  fresh app process (`src/adb.rs`).

## Host output: run 1

```text
running 1 test
test on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before ... fresh pair: A <pixel> at /ip4/192.168.100.52/tcp/25971/p2p/<pixel>, B <a40> at /ip4/192.168.100.6/tcp/2546/p2p/<a40>
neither trusting: A tried "59" sends, B "59"; A's row "none", B's "none"
A trusting alone: A tried "43" sends, B "59"; A's row "denied"
both trusting: a message crossed each way; A's row "connected", B's "connected"
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 56.03s
```

## Host output: run 2 (excerpt)

```text
running 1 test
test on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before ... fresh pair: A <pixel> at /ip4/192.168.100.52/tcp/21710/p2p/<pixel>, B <a40> at /ip4/192.168.100.6/tcp/4666/p2p/<a40>
neither trusting: A tried "59" sends, B "59"; A's row "none", B's "none"
A trusting alone: A tried "32" sends, B "59"; A's row "denied"
thread 'on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before' (1758564) panicked at tests/android-e2e/tests/pairing.rs:118:5:
assertion `left == right` failed: A, both trusting: {"case": String("pair"), "detail": String("the close was not accepted: PeerUnreachable"), "result": String("fail")}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 54.72s
```

## Host output: run 3

```text
running 1 test
test on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before ... fresh pair: A <pixel> at /ip4/192.168.100.52/tcp/3162/p2p/<pixel>, B <a40> at /ip4/192.168.100.6/tcp/19914/p2p/<a40>
neither trusting: A tried "59" sends, B "59"; A's row "none", B's "none"
A trusting alone: A tried "31" sends, B "59"; A's row "denied"
both trusting: a message crossed each way; A's row "connected", B's "connected"
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 54.90s
```

## Host output: run 4

```text
running 1 test
test on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before ... fresh pair: A <pixel> at /ip4/192.168.100.52/tcp/12986/p2p/<pixel>, B <a40> at /ip4/192.168.100.6/tcp/25391/p2p/<a40>
neither trusting: A tried "59" sends, B "59"; A's row "none", B's "none"
A trusting alone: A tried "26" sends, B "59"; A's row "connected"
both trusting: a message crossed each way; A's close "accepted"; A's row "connected", B's "connected"
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 55.30s
```

## Host output: run 5

```text
running 1 test
test on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before ... fresh pair: A <pixel> at /ip4/192.168.100.52/tcp/29205/p2p/<pixel>, B <a40> at /ip4/192.168.100.6/tcp/9135/p2p/<a40>
neither trusting: A tried "59" sends, B "59"; A's row "none", B's "none"
A trusting alone: A tried "31" sends, B "59"; A's row "denied"
both trusting: a message crossed each way; A's close "accepted"; A's row "connected", B's "connected"
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 54.11s
```

## Host output: run 6

```text
running 1 test
test on_two_phones_two_fresh_profiles_connect_after_both_trust_and_not_before ... fresh pair: A <pixel> at /ip4/192.168.100.52/tcp/12566/p2p/<pixel>, B <a40> at /ip4/192.168.100.6/tcp/9925/p2p/<a40>
neither trusting: A tried "59" sends, B "59"; A's row "none", B's "none"
A trusting alone: A tried "32" sends, B "59"; A's row "denied"
both trusting: a message crossed each way; A's close "accepted"; A's row "connected", B's "connected"
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 54.69s
```

# android-e2e: two fresh phones, each added on its own Add page by scanning the other's Show code, exchange messages after both trust, and not before — Pixel 9a and A40 over Wi-Fi, 2026-11-10

Plan §29 (e)'s second device row run again through the real Scan,
`tests/pairing.rs`'s
`on_two_phones_two_fresh_profiles_added_on_their_screens_connect_after_both_trust_and_not_before`.
It answers the limit §29's closing record carries first: the two phone
rows of 2026-10-24 added the address through the binding's add, the call
the Scan screen makes, not by scanning a shown code
(`2026-10-24-pixel-9a-a40-wifi-fresh-pair.md`). It was run by
p2p-network-dev-02 in step 5 of the phone order of 2026-11-08 (REQUEST
01a1ba04, job j103). The owner held the phones: twice in the run, they
held one phone's camera to the code the other showed.

"Fresh" is the app's data cleared: `pm clear org.interweave.human` on
both phones before the run, on the owner's word to clear both phones
(§29's closing record). The host output is below, as printed but for
cargo's build lines and blank lines. The PeerIds are written `<pixel>`
and `<a40>`. The phones' logcat is not committed, because it carries
other apps' lines.

## What ran

| | |
|---|---|
| Device A | Pixel 9a, Android 17, API 37, `google/tegu/tegu:17/CP3A.261005.005/2026110201:user/release-keys`, wireless adb `192.168.100.62:38897` |
| Device B | Samsung Galaxy A40 (SM-A405FN), Android 11, API 30, `samsung/a40eea/a40:11/RP1A.200720.012/A405FNXXU4CWC3:user/release-keys`, wireless adb `192.168.100.6:39716` |
| App build | this branch at `d1be8eb89`, built by rust-ui-dev-01 with their debug key and installed with `install -r`. App APK sha256 `4f0a214018d63952ade969df07725e09a4bc360cea067fdb6412d04f0519d9c5`; androidTest APK sha256 `1efb4b1552d428095f93774ee0bc7a24bce9fb38369b6e431d236227f9b9af2b`, the same pair on both phones, checked by the run script before the run |
| Host side | `tests/android-e2e` at `17b3ca229` |
| Addresses | A is `192.168.100.62/24` and B `192.168.100.6/24`, each on `wlan0`. This host carries only adb. |
| Profiles | The shipped `human-android.yaml` with `trust.allowed_peers: []`. Peer cache, Kademlia and the rest are as shipped. |
| Link | The data path is the LAN, phone to phone. `adb reverse --list` was empty on both phones after the run. |
| Permissions | `pm grant` of `CAMERA` on both phones after the clear, so the Add page opens its camera without a prompt; `POST_NOTIFICATIONS` too, which the A40 (API 30) refused as an unknown permission, changing nothing there |
| Command | `cargo test -p interweave-android-e2e-tests --test pairing -- --ignored --exact on_two_phones_two_fresh_profiles_added_on_their_screens_connect_after_both_trust_and_not_before --nocapture --test-threads 1`, with `INTERWEAVE_ANDROID_SERIAL_A/_B` as above, under the `pixel-9a` and `samsung-a40` leases, on this account's own adb server port |

## Result

| Run | Started | Result |
|---|---|---|
| 1 | 04:13:39Z | pass, 175.57 s |

The run showed the following, each asserted by the test or its cases:

- **Step 1, added on the screens.** On each phone the test opened Peers,
  and in each direction one phone held its Show code open while the
  other opened Add an address; a person held the second phone's camera
  to the first's code. The test waited for the scanning phone's trace to
  reach the scan page, then the add's trust page, then
  `add=added lines=` (`src/screen.rs`, `add_by_scan`): A scanned B's
  code, then B scanned A's. No address was given through the binding.
- **Neither trusting.** For the step's window from each side's start,
  every direct message either sent to the other was refused: 60 tries
  each way. None arrived, and neither side's row for the other recorded
  a dial (`none`).
- **A alone trusts B.** No message A sent was accepted (45 tries), none
  arrived either way, and A's row for B ended `denied`.
- **Both trust.** A message from A was accepted by B and read there, one
  from B was accepted by A and read there, A's close reported
  `accepted`, and both rows ended `connected`. This step is the positive
  control for the two before it.

So the pair added through the screens behaves as the 2026-10-24 rows
did: no message crosses until each side has the other's address and
trusts the other's PeerId.

## What this run does not prove

- **The Scan screen's own decoding beyond this pair.** One code each
  way was scanned, by one person, under the room's light; a code the
  camera cannot read, or a damaged one, is not exercised here.
- **Which address each scan added.** The code carries what Show
  publishes for the profile; this file does not quote the added lines,
  and the peer cache is on, so it does not exclude a cached address as
  a second source of the final step's dial.
- **The one-sided step in the other direction**, B trusting first.
- **Repetition.** One run. The 2026-10-24 file has six through the
  binding.
- **What the earlier rows' files list as shared**: a LAN that isolates
  its clients, IPv6, a relayed path, the app in the background.

## Note, 2026-11-09: two runs through the Add field

Before the route was settled as Scan (architect-cto, REPLY 01a1bfb4),
the step-1 add was driven through the Add page's address field, typing
the other phone's address as key events. Two runs, between 08:37Z and
08:42Z, both failed in step 1 and reached no trust step:

- A40 as A: the line was typed and the add never completed —
  `no ["add=added lines=1", "page=peers-add-trust"] within 30s; page
  peers-add, focus "address-field"`, the trace showing `add=waiting`.
  A PeerId's capitals do not survive a key event on Slint 1.18.1's
  Android backend, the limit plan §20 records after Stage 17's close
  (and `apps/human-android/README.md`), so the typed address cannot be
  the one shown.
- Pixel as A: the Peers page was never reached — `40 TABs never
  reached "app-options"`, no focus line in the trace. The notification
  permission prompt (API 33+) held the keys after the clear; the run
  script has granted that permission since, and the launch now fails
  naming a system window that takes the keys.

The field route types what a key event can carry and refuses the rest
(`src/screen.rs`, `add_by_field`); it is not this row's route.

## Host output: run 1

```text
running 1 test
test on_two_phones_two_fresh_profiles_added_on_their_screens_connect_after_both_trust_and_not_before ... fresh pair: A <pixel> at /ip4/192.168.100.62/tcp/1223/p2p/<pixel>, B <a40> at /ip4/192.168.100.6/tcp/17202/p2p/<a40>
now: hold 192.168.100.62:38897's camera to the code 192.168.100.6:39716 shows
now: hold 192.168.100.6:39716's camera to the code 192.168.100.62:38897 shows
added on the screens: A scanned B's code, B scanned A's
neither trusting: A tried "60" sends, B "60"; A's row "none", B's "none"
A trusting alone: A tried "45" sends, B "60"; A's row "denied"
both trusting: a message crossed each way; A's close "accepted"; A's row "connected", B's "connected"
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out; finished in 175.57s
```

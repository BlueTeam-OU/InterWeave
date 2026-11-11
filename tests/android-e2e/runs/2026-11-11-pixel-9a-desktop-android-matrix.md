# android-e2e on a Pixel 9a: the desktop ↔ Android matrix, both directions, API 37 — 2026-11-11

Plan §20 gate (c)'s desktop ↔ Android platform matrix, run on a phone
by p2p-network-dev-02 (job j13). architect-cto-01 ruled where the matrix
lives on 2026-11-11 (GZCoord REPLY 01a1cbc6), and this file records its
phone rows.

The comparison is `tests/android-e2e/tests/human_chat.rs::chat`.
- Phone to desktop: every `HumanChatV2` the phone's facade composed
  equals the one the desktop's facade decoded.
- Desktop to phone: every `HumanChatV2` the phone's facade decoded
  (the case's `received` key) equals the one the desktop composed.
  It covers the two answers and the close, all fields, `sent_at_ms`
  included. The two answers and the close are compared as one set, so
  a missing or extra message fails as well.

rust-ui-dev-01 supplied both halves (e9bcecd09, c8dae26f8), and they
were folded here at a416afb57. `tests/interoperability/README.md` names
the rows.

## What ran

| | |
|---|---|
| Device | Pixel 9a, `google/tegu/tegu:17/CP3A.261005.005/2026110701:user/release-keys`, API 37 |
| App build | The j13 branch at `384cb8218`, built by rust-ui-dev-01 with its debug key and installed with `install -r`, data kept (GZCoord REPLY 01a1cc2b). The app APK (`versionName` 0.3.0, `versionCode` 3) has sha256 `7374d46a9d29ef22053c6457746a79202a7181ccb16d72d1ed49052c5fbde9e2`. The androidTest APK has sha256 `1efb4b1552d428095f93774ee0bc7a24bce9fb38369b6e431d236227f9b9af2b`. Both sha256s were read from the installed paths on the phone. Both were installed at phone time `2026-11-11 23:52:32`; the phone clock is UTC+4. |
| Host side | `tests/android-e2e` at `384cb8218`, the APK's commit, with `transport-daemon` built at the same commit. |
| Link | Wireless adb, `192.168.100.67:45467`. For the relayed and direct rows, the relay and the desktop listen on this host's loopback, and the phone reaches them through `adb reverse` (`src/adb.rs`). That is not a radio network, and there is no NAT. The Wi-Fi row uses no reverse: the desktop dials the phone's LAN address, `192.168.100.67`. `adb reverse --list` read empty after the run. |
| Command | `cargo test -p interweave-android-e2e-tests --test human_chat -- --ignored --exact <row> --test-threads 1 --nocapture`, once per row in the order below. Each ran with `ANDROID_SERIAL=192.168.100.67:45467`, `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase` and `INTERWEAVE_ANDROID_WIFI_ADDRESS=192.168.100.67`, under the `pixel-9a` lease, on an adb server of its own. That server also listed the A40 (`192.168.100.6:39716`), connected on its own; `ANDROID_SERIAL` selected the Pixel, and nothing ran on the A40. |
| Phone state | This lane installed, uninstalled and cleared nothing. The case sends and receives through the app's own store. What the runs left in that store was not read. |

## Results

| Row (`tests/android-e2e/tests/human_chat.rs::…`) | Path | Started (UTC) | Result |
|---|---|---|---|
| `on_a_device_the_phone_chats_over_the_relayed_path_with_a_stand_in_as_the_control` | the phone relayed, a stand-in direct | 19:54:06Z | pass, 12.59 s |
| `on_a_device_the_phone_chats_over_the_direct_path_beside_a_relayed_stand_in` | the phone direct, a stand-in relayed | 19:54:21Z | pass, 11.72 s |
| `over_wifi_the_phone_chats_with_the_desktop_dialling_its_lan_address` | the phone direct, over its Wi-Fi | 19:54:34Z | pass, 10.22 s |

Each row ran once.

In each row the phone sent one plain message and one `;ce=br` message, the
`;ce=br` one over the 48 KiB payload limit before compression. The
desktop answered with the same two kinds and a plain close. Both
comparisons above held for every message in each row, and every payload
the runtimes handed a client validated against
`human-chat/envelope.schema.json`.

**The phone ran the new comparison.** An androidTest build older than
e9bcecd09 reports no `received` key, and `chat` then panics at
`expect("received")`, after the phone-to-desktop comparison and before
the desktop-to-phone one. Each row passed, so the
phone's case reported `received`, from the build installed above.

**The comparison is not vacuous.** The mutation was run on the host
stand-in, not on the phone (rust-ui-dev-01, GZCoord REPLY 01a1cbed):
with the case reporting each envelope without `sent_at_ms`, the
stand-in row failed `D -> R arrived as composed`. With it restored,
the row passed. The stand-in row
(`human_chat_crosses_between_android_and_desktop_relayed_and_direct_plain_and_compressed`)
also passed at the fold, just before this run.

## What this does not prove, by name

- **The relayed route indicator on the screen.** The relayed row proves
  that the envelopes crossed a relayed path, in both directions. It
  proves nothing about the indicator as rendered. The phone's indicator
  is checked by the case, `tests/android-e2e-cases`' `human_chat`, which
  reads the path from the phone's UI model (`model.path`), not from the
  Slint window. `chat` reads only the desktop's. Nobody looked at the
  phone's screen during the run. Whether the
  rendered indicator is live on the Android path is rust-ui-dev's carry
  (plan §21).
- **A radio network on the relayed and direct rows.** Those rows reach
  the host through `adb reverse`, which is a tunnel, not the phone's
  Wi-Fi or cellular network. Only the Wi-Fi row crosses the radio, and
  it is direct only.
- **A NAT, a public relay or a second host.** The relay is a bare Swarm
  on this host's loopback, and the host is a VM behind NAT, so the
  phone cannot dial it over the LAN (`wifi.rs`).
- **Exactly once, phone to desktop, at the facade.** The desktop's
  check finds each message the phone composed and compares it field by
  field, but does not count copies at the facade. The rows do count the
  payloads D's runtime handed its client. Desktop to phone, the
  comparison is over the whole set and would catch a duplicate.
- **Repeatability.** Each row ran once.
- **Other phones and API levels.** Only the Pixel 9a at API 37 ran. The
  A40 (API 30) holds a build signed with another key, and installing
  over it would have wiped its data.
- **Broadcast.** `chat` crosses directed messages only.

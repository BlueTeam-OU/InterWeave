# android-e2e on a Samsung A40: Stage 17's carried device cases, API 30 — 2026-11-05

The six device cases Stage 17 carried (plan §20), built in #544 and run
by p2p-network-dev-02 on the samsung-a40 with the owner at the phone.
architect-cto asked for each case to be reported against its carried
item by name (GZCoord REPLY 01a1a27d-152f); that is the table under
Results. The phone's full logcat is not committed, because it carries
other apps' lines.

## What ran

| | |
|---|---|
| Device | Samsung Galaxy A40, `samsung/a40eea/a40:11/RP1A.200720.012/A405FNXXU4CWC3:user/release-keys` (Android 11, API 30, One UI). The phone clock is UTC+1. |
| App build | InterWeave at `5d84c6b0` (this branch: `main` at `b563dd00` plus four `tests/android-e2e` commits), built by rust-ui-dev-02 with their debug key and installed with `install -r`, data kept, at 07:21 UTC. The app APK (`versionName` 0.2.0, `versionCode` 2) has sha256 `9bac2493f7c52512373d617c18cbdebcc08f21f587ad48834cdf44c00761b3fd`; the androidTest APK has sha256 `d2ced5d012c21baf3e7c30edbf4b54938c9ee9dbec7e09211b21044593a3875a`. Both were read on the phone (`sha256sum` of each `base.apk`) before the first run. |
| Host side | `tests/android-e2e` at `78947f4b`, then at `04e2b7b3` from 12:05 UTC, as each row says. Neither changes anything outside `tests/android-e2e`, so the APKs above are the build of each. |
| Link | Wireless adb, `192.168.1.11` (port `34319`, then `40895` and `39991` as the owner switched wireless debugging back on), on an adb server of its own, under the `samsung-a40` lease. |
| Command | `cargo test -p interweave-android-e2e-tests --test <file> -- --exact <case> --include-ignored --test-threads 1 --nocapture`, with `ANDROID_SERIAL` set to the address, `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/org.interweave.human.test.E2eCase`, and `INTERWEAVE_ANDROID_PERSON=1` for a case that needs a person. |
| Profile file mode | `5d84c6b0`'s provision writes `config.yaml` with the instrumentation's umask (rust-ui-dev-02 fixed it in `797eef6c`, not in this build), so after each case the run set it back to `0600` through `run-as` and read the mode: `-rw-------` after every case. |

An earlier run, 2026-11-04 22:20 UTC, is not counted here: the app on the
phone then was another branch's build, installed over `5d84c6b0` an hour
before, which was found only the next morning. That run is the
evidence for the two harness fixes it prompted (below), nothing more.

## Results

| Carried item (plan §20) | Case | Host at | Started (UTC) | Result |
|---|---|---|---|---|
| backup exclusion | `platform_lines::backup_exclusion` | `78947f4b` | 07:22:14 | pass, 11.61 s |
| foreground service, foreground-only | `platform_lines::fgs_lifecycle_foreground_only_binds_and_ends_with_the_window` | `78947f4b` | 07:22:36 | pass, 10.47 s |
| foreground service, stay-reachable, process death and restart | `platform_lines::fgs_lifecycle_stay_reachable_is_foreground_and_restarts_after_a_crash` | `78947f4b` | 07:22:48 | pass, 18.53 s |
| FLAG_SECURE, Show my address | `identity_lines::secure_span_show_my_address` | `78947f4b` | 07:23:08 | pass, 21.98 s |
| FLAG_SECURE, the recovery words | `identity_lines::secure_span_recovery_words` (a person) | `78947f4b` | 10:42:07 | pass, 35.91 s |
| user-presence prompt and its cancel | `identity_lines::presence_prompt_and_cancel` | `04e2b7b3` | 12:05:56 | pass, 83.54 s |
| Keystore user-presence unlock | `identity_lines::keystore_user_presence_unlock` (a person) | `04e2b7b3` | 12:20:38 | pass, 128.92 s |
| network callback | `network_callback::a_wifi_drop_and_return_reach_the_runtime_in_one_process` | `78947f4b` | 07:23:32 and 10:27:42 | **fail** both times: adb did not come back within 600 s. The phone-side evidence is below; the case's own assertions were not reached |

Attempts that failed and were run again, each for a reason stated:

- `secure_span_recovery_words` at 2026-11-04 22:26 UTC: the prompt was
  raised (`prompt=phrase`) and nobody answered it within 180 s.
- `presence_prompt_and_cancel` and `keystore_user_presence_unlock` at
  11:58 and 12:00 UTC (host `78947f4b`): the locked launch passed, then
  "the credential prompt raised at the launch: not within 30s". The
  prompt was up; the harness looked for AOSP's window title only (the
  third harness fix below).
- `keystore_user_presence_unlock` at 12:07 UTC: the Unlock prompt was
  raised (`prompt=unlock`) and the owner, by their account, did not see
  it in time.

### The network callback

Both runs drove the same on-phone script. In each, the script ran to
its end, the app's process was the same throughout, and the runtime
was told of both changes between the script's markers. The host could
not read any of it within its 600 s: after the Wi-Fi returned, the
phone's wireless debugging stayed off (the script sets
`adb_wifi_enabled` to 1, and it read 1 afterwards), and no port
opened in the phone's range until the owner switched it on by hand,
after the wait. The script's own files and log were pulled when adb
came back.

The second run's lines (phone time, UTC+1):

```
11-05 11:28:01.898 I/interweave-e2e(15046): net-drop wifi-off
11-05 11:28:02.362 I/interweave( 7769): the runtime was told the networks: 0 addresses (0 on no local network), unobserved resolvers, strict Private DNS off
11-05 11:28:32.109 I/interweave-e2e(15309): net-drop wifi-on
11-05 11:28:36.312 I/interweave( 7769): the runtime was told the networks: 2 addresses (0 on no local network), 1 resolvers, strict Private DNS off
11-05 11:28:36.669 I/interweave( 7769): the runtime was told the networks: 2 addresses (0 on no local network), 2 resolvers, strict Private DNS off
11-05 11:29:32.365 I/interweave-e2e(15493): net-drop done
```

`pid-before`, `pid-during` and `pid-after` all read 7769. The first
run's are the same shape (08:23:47 off, 0 addresses; 08:24:17 on, 2
addresses; one process, 29187). Read by eye against the case's three
assertions, all three hold; that is a reading, not the case passing,
and the item stays open until the case passes. What would let it pass
over wireless adb is not known: whether toggling `adb_wifi_enabled`
off and then on brings the port back on this phone was not tried.

Each failed run left the profile in `stay-reachable`, since the
restore could not reach the phone; it was written back by hand from
the copy the case printed, read back byte for byte, `0600`, with no
availability overlay.

### The user-presence identity

The two user-presence cases need an identity created under
`key_unlock_policy: user-presence`. Writing that policy into the
existing profile did not make one: in this build nothing calls
`custody::reseal`, which ADR-0042 (A 2026-10-10 (ii)) says the host
calls after an unlock whose record policy differs from the profile's.
So, on the owner's word ("wipe"), the app's data was cleared at 11:11
UTC, the profile with that one block added was written before the
first start, and the owner made a new identity by hand at 11:58 UTC
(`identity.iwk1` dated 12:58 phone time). The missing call was reported
to the start path's owner (GZCoord OBSERVATION 01a1abab); rust-ui-dev
confirmed it, with a second consequence (an `OldKeyRemains` retire is
never retried at a later start), and queued it as their j213.

## Harness fixes this run needed

Each is a commit on this branch, with a test that fails on the code
before it:

1. `3cbad2ea`: the Wi-Fi script started the app with `am start -W`,
   which waits forever behind a keyguard. On 2026-11-04 it blocked 38
   minutes, so the Wi-Fi went off after the host had given up. The
   script now starts without `-W` and polls for the process, bounded,
   and the host waits for an unlocked phone first.
2. `78947f4b`: the A40 prints a window's `fl=` as a bare hex mask
   (`fl=81810100`), so every window read as not secure, and the
   secure-span checks that a screen is not secure could not fail on
   this phone.
3. `04e2b7b3`: One UI titles the credential prompt's window
   `BiometricPrompt3P`, not AOSP's `BiometricPrompt`.

## What this does not show

- The network callback case passing: it did not.
- A cellular handover, notifications, Keystore invalidation: not
  among these cases.
- Any other phone: one A40, API 30, One UI. The Pixel 9a was not used.
- A changed `key_unlock_policy` on an existing identity: not applied in
  this build (above).

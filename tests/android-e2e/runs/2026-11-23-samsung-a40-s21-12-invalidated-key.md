# android-e2e on a Samsung A40: S21-12's invalidated user-presence key, API 30 — 2026-11-23

S21-12's owed case: a `user-presence` seed key invalidated on a device,
and what the app shows for it. Plan §21 carried it to §30 under
p2p-network-dev-02. Run by p2p-network-dev-02 on the samsung-a40, slot 1
of architect-cto's phones order (DECISION 01a2073d), with the owner at
the phone. The phone's full logcat is not committed, because it carries
other apps' lines.

## What ran

| | |
|---|---|
| Device | Samsung Galaxy A40, Android 11, API 30, One UI, no StrongBox. The phone clock is UTC+1. |
| App build | InterWeave at `4ef7723fc` (this branch), the debug and androidTest APKs built by p2p-network-dev-02 with that login's debug key, the signer of the pair already installed, and installed with `install -r` at 08:13 UTC. Before: app `base.apk` sha256 `1396145e82edf249…`, test `88247508c213fb1d…`. After: app `88dc62150264cdad1972cea15a66dbfe761701217b2c81151068fbfc9beaab37`, test `88247508c213fb1d231702646e584c33d53c8257af415fd2eb9ac30b2b3b00db` (the same build as before). Both were read on the phone (`sha256sum` of each `base.apk`). The identity record's sha256 was the same before and after the install (`581e7b76…`): the data was kept. |
| Host side | `tests/android-e2e` at `4ef7723fc`, the APKs' commit. |
| Link | Wireless adb, `192.168.1.9:34069`, on an adb server of its own (port 5047), under the `samsung-a40` lease. The owner paired this login with the phone in this run's session. |
| Command | `cargo test -p interweave-android-e2e-tests --test identity_lines -- --ignored --exact an_invalidated_user_presence_key_opens_on_k1_and_mints_nothing --nocapture --test-threads=1`, with `ANDROID_SERIAL` set to the address, `INTERWEAVE_ANDROID_INSTRUMENTATION=org.interweave.human.test/.E2eCase` and `INTERWEAVE_ANDROID_PERSON=1`. |

## The user-presence identity

The phone held a `background-compatible` identity, made 2026-11-16, and
no `user-presence` one: the uninstall of 2026-11-16 16:54 UTC took the
2026-11-05 one. On the owner's word in this session ("yes, clear the
A40's app data"), relayed by architect-cto's REPLY 01a20747-d1aa:

1. `pm clear org.interweave.human` at 09:03:31 UTC.
2. The phone's former profile written back through `run-as` before the
   first start, with one line added under `runtime.android`:
   `key_unlock_policy: user-presence`.
3. The owner chose Create. The first attempt showed "The app could not
   save your identity on this phone", and the app logged `an identity
   write was refused: Storage(".../interweave is refused before
   key-equivalent material is written beneath it: an ancestor owned by
   uid 10437, mode 0777: other-writable and not sticky")`. That was the
   setup's fault: `run-as mkdir -p` and `run-as sh -c 'cat > …'` left
   the four directories `0777` and the file `0666`. The trust-boundary
   check refused, as it should. After `chmod 700` on each directory and
   `600` on the file, read back with `ls -ld`, the owner's second Create
   made the identity (`identity.iwk1` dated 10:40 phone time).

## Result

**Passed, 168.01 s, 09:41:15 to 09:44:11 UTC.**

| step | seen |
|---|---|
| controls, before the change | the profile is `user-presence`; the lock is set; `key-info` lists `interweave.iwk1.human-android.user-presence` with `user_authentication_required` true; past the key's 60 s validity, a cold launch opens Conversations offline with the authentication notice and the credential prompt up, cancelled by Recents. All asserted by the case. |
| the lock removed (the owner: Settings > Lock screen > Screen lock type > None) | read off `dumpsys lock_settings`; `key-info` then: the alias kept, `refused: java.security.UnrecoverableKeyException` |
| the same PIN set again (the owner) | read off `dumpsys lock_settings`; `key-info`: the same refusal |
| a cold launch | `ui-trace identity=k1`, `ui-trace warns=active-elsewhere`, `ui-trace focus=k1-heading` (10:44:11 phone time); no credential prompt and no `NetworkService` record, each read once by the case right after the trace line. The case at `4ef7723fc` did not wait a quiet window first (it does since; see below). The phone's own log, read at 09:44:53 UTC, about 42 s after the launch, held no `ui-trace prompt=` line, and `dumpsys activity services` named no `NetworkService` then. |
| the identity record | byte-equal to the one read before the change, so nothing was minted over it (sha256 `ff8fc78f14cad549…`, the file still dated 10:40) |

`key-info` after the run, before the wipe: the identity's key
(`interweave.idk1.human-android.human`) and the store's key
(`interweave.store.human-android`) still open, inside secure hardware,
not bound to authentication. Only the seed's `user-presence` key is
refused. The lock read as `CredentialType: Pin` under `User 0`. The same
dump, read before the run at about 09:20 UTC, goes on past user 0's
block to a storage section that names user 0 again:

```text
Storage:
  User 0 [/data/system_de/0/spblob]:
```

So on this phone, removing the screen lock invalidates a key made with
`setUserAuthenticationRequired` and keeps its alias, and setting the lock
again does not bring it back. The app then opens on K1, the recovery page
with ADR-0043's warning, starts no service and mints nothing. A40, API
30: One UI surfaces the invalidation as `UnrecoverableKeyException` from
`getKey`, as the test APK's `key-info` read it. `KeystoreCipher.kt` maps
that exception to `KEY_INVALIDATED` (SPIKE-009 D6a); the run did not
read the app's own failure code, and a missing key opens K1 too.

## What the run leaves on the phone

The app's data was cleared after the case, at 09:45:10 UTC, under the
owner's word for this run (DECISION 01a20039). InterWeave is installed
(the `4ef7723fc` pair above) and opens on Welcome with no identity. The
lock is the owner's PIN, as found. This login's adb pairing stays on the
phone.

## Since the run

The review of the PR found the case's K1 reads were single snapshots
taken right after the trace line. The case now waits out the app's
`prompt=` line for `QUIET` before them, and the lock reader stops at the
end of user 0's block. Neither change has run on a phone; the run above
is the case at `4ef7723fc`.

## What this does not show

- A biometric enrolment or removal as the invalidating change: the
  owner removed the PIN. No fingerprint is enrolled on this phone.
- Any other phone: one A40, API 30, One UI, TEE only. Not a StrongBox
  phone, and not API 31 or later, where Keystore 2 may delete the
  alias instead of keeping it. The case prints which and asserts
  neither, since both open on K1.
- A restore from the recovery phrase after K1: the case restores nothing.
- What the person sees on K1: the window is `FLAG_SECURE`, so the trace
  is read, not the frame.

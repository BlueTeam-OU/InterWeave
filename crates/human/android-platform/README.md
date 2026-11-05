# android-platform

The Android human client's Service side (plan section 20 steps 2-4),
with no Android, JNI or Slint type so that it builds and is tested on the
host:

- `ServiceHost`: the embedded transport runtime, the message store and
  the facade over the runtime's in-process binding, started and stopped
  as one, once per process. The foreground Service drives it through the
  app's JNI entry points, from its own threads.
- `Hub`: where the facade's output meets the Activity's view, which
  attaches and detaches while the Service holds the session, and where
  arrivals no focused window reads become notices.
- `identity`: the start's identity, unlocked from its Keystore-wrapped
  record through the embedded custody path and the app's `SeedCipher`,
  never replaced when the record cannot give it back; a profile with no
  record is created or refused as the caller says (`FirstRun`).
- `provision_profile`: the first-run profile, profile-config's
  embedded-android document, written in every build where there is none
  and never rewritten
  (`the_first_run_profile_is_written_once_and_never_rewritten`).
- `stand_in` (feature `dev-stand-ins`, the debug APK's): the android-e2e
  cases' replacement of the profile.

The JNI entry points, the Kotlin Service and the views are
`apps/human-android`'s. The Keystore cipher itself is the app's
(step 6); secure recovery and backup exclusion (steps 7-9) are not here
yet.

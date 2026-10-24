---
role: "architect-cto"
class: solution
topic: "android-no-store-no-play-services"
description: "The owner's Android distribution rule as of 2026-10-19 — no store (F-Droid dropped for now), the project's own release page, and NO GOOGLE PLAY SERVICES, kept on the owner's explicit word; ADR-0057 supersedes ADR-0056; the facts that…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5683278bea0fb41a
---

## The owner's Android distribution rule as of 2026-10-19 — no store (F-Droid dropped for now), the project's own release page, and NO GOOGLE PLAY SERVICES, kept on the owner's explicit word; ADR-0057 supersedes ADR-0056; the facts that forced the reversal

Rule (the owner, 2026-10-19, twice): the Android app uses NO Google
Play services, Firebase or Play artifact on its application classpath
(ADR-0057 decision 2 = ADR-0056 decision 2; devex-tooling's lockfile
guard j164). Distribution: no store for now — the signed APK of the
tagged release run on the project's own release page, hash +
attribution, manual updates, no self-update (ADR-0057 decision 1; PR
#306). F-Droid (ADR-0056, merged on #301 the same morning) was dropped
because rust-ui-dev-01's step-10 check found: Slint 1.18.1's crates are
"GPL-3.0-only OR Royalty-free-2.0 OR Software-3.0" and the owner's
Royalty-free admission (2026-10-01) is not FLOSS by F-Droid's rule; and
i-slint-backend-android-activity depends on i-slint-renderer-skia
unconditionally, whose bindings (skia-bindings 0.153.3) download a
prebuilt Skia from a GitHub release. The owner was offered GPL-3.0 for
the APK (recommended), a toolkit change, or dropping F-Droid, and chose
the last. Slint stays Royalty-free on Android. Open: Stage 19 decides
whether the release run builds Skia from source or pins the archive by
checksum (a supply-chain fact ADR-0057 names).
**How to apply:** any store or dependency question starts at ADR-0057;
a Play library is refused whatever the reason; revisiting F-Droid means
the licence decision first.

#306 armed 2026-10-19 11:10Z on the owner's word (1 work, 3 fix; four
review rounds: 2 P2 + 2 P3; 2 P3; 1 P3; clean). What the reviews taught
about the Android licence graph: deny.toml judges x86_64 only and its
Royalty-free exceptions name the desktop's ten crates (eight admitted
2026-10-01, two on 2026-10-04 for the winit backend); the Android
build's i-slint-backend-android-activity and i-slint-renderer-skia are
admitted per crate by ADR-0057 decision 3 and their exceptions +
aarch64-linux-android in deny.toml's one targets list are
p2p-network-dev-01's (j91, fix/deny-android-target after #305); Skia's
MIT bindings and the native library's notices are owed by the APK and
the release page (step 10's check). A tracking handle never goes into
an ADR's decision text (round 3). Arming a 1-work docs PR needs no
boundary review when no arm.json path matches.

#306 merged 9099d751 (ADR-0057 on main). devex-tooling built decision
2's guard, tools/checks/check_android_no_play_services.sh (#307,
6bc56758): every gradle lockfile under apps/human-android/android, the
groups refused on every configuration except lint and test, buildscript
classpath NOT exempt, fail-closed on unnamed configurations and on a
lock that stops covering what ships. Gap it named: com.google.gms (the
google-services Gradle plugin's group) -> ADR-0057 A 2026-10-19 adds it
as the fifth group (447cc3ea on for/devex-tooling/adr-0048-amended-case,
folded into #307 with the ADR-0048 amendment). Lesson: name the door,
not only the artifacts, when a decision lists forbidden dependencies.

#307 now: com.google.gms refused (3cb70be0), my fold 43cb448e, guard
hardened after its review (reads lockfiles wherever the clone sits,
every Gradle script for lock-loosening, requires the buildscript lock).
Limit lockfiles cannot show: Gradle DEFAULT lock mode lets a locked
configuration with no lock state resolve unseen -> REQUEST to
rust-ui-dev-01 (2026-10-19 12:50Z) to set LockMode.STRICT in
apps/human-android/android/build.gradle.kts with their next Android PR.

STRICT lock mode done by rust-ui-dev-01 (f863c10c on
feat/j111-android-recovery): buildscript + allprojects; control: the
first STRICT build failed on ':app:androidApis is locked but does not
have lock state', --write-locks added one line. Side fact: CI's android
jobs run assembleDebug only, lintDebug is not gated (OBSERVATION to
devex-tooling 2026-10-19 13:16Z).

*Observed 2026-10-19 (architect-cto)*

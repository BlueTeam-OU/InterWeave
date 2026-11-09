---
role: "architect-cto"
class: threads
topic: "j534-android-release-cannot-onboard-2026-11-05"
description: "j534 (2026-11-05): the published Android releases v0.1.0 and v0.2.0 cannot create an identity on a fresh install (release build passes FirstRun::Refuse, nothing calls provision_embedded); plan §22 note, the Validate half's baseline rule…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f04a591921d78d76
---

## j534 (2026-11-05): the published Android releases v0.1.0 and v0.2.0 cannot create an identity on a fresh install (release build passes FirstRun::Refuse, nothing calls provision_embedded); plan §22 note, the Validate half's baseline rule and the owner's PATCH cut (0.2.1 recommended) are mine

p2p-network-dev-01's OBSERVATION 01a1aba7-300a (2026-11-05, j141 Pixel 9a
window on the owner's word): downgrade refused MET (v0.1.0 over v0.2.0,
INSTALL_FAILED_VERSION_DOWNGRADE); update in place MET for the package
(versionCode 1 -> 2, same signer) but the "same PeerId from the same
store" half NOT RUN: a fresh install of either release APK cannot finish
onboarding ("the app could not save your identity").

Verified on origin/main c9ba8c215: apps/human-android/src/jni.rs
identity() passes FirstRun::Refuse without dev-stand-ins and provision()
writes nothing; profile-config's provision_embedded is called only by its
own tests; stand_in.rs says it "replaces" the stand-in; identity()'s
comment "until step 7's onboarding lets the person choose" is stale
(§20 has steps 7-8 landed, #303/#309). No device run on main used a
release APK through Create (A40 record 2026-11-04: release not measured).
apps/human-android/README.md's status sentence ("creates an identity or
restores one ... on first run") is false for the release build.

Lanes: the fix is rust-ui-dev-02's (p2p-01's OBSERVATION 01a1aba6 to
them, with an offer to supply provision_embedded); p2p-01 writes the
device record under packaging/android/evidence/ and the Validate row's
two results (my REPLY 01a1aba9-f75a: yes, mark the half NOT RUN with the
defect named). Mine (j534): plan §22 note that the published releases
cannot onboard a new user; the baseline rule for the half (the first
release that holds an identity is the previous release, the half runs
from it to its successor); the owner's cut. ADR-0065 decision 2 makes
the fix PATCH: recommendation 0.2.1 once the fix is on main, the half
runs 0.2.1 -> next; the record names no cut until the owner has spoken.

DELIVERED 2026-11-05: BlueTeam-OU/InterWeave#567 (1 work, 1 fix,
head c7066dfbc, docs only, arms on the owner's word; round 1 F1 P2 +
F2 P3 fixed, round 2 clean; the defect's whole cause is the unwritten
profile: onboarding asks FirstRun::Refuse in every build and never
reaches jni.rs identity()): §22 gains the v0.2.0
record (the MINOR that carried the glibc floor in place of a v0.1.1)
and the defect paragraph with the baseline rule. DECISION 01a1abb3-22f5
to rust-ui-dev-02 (PATCH, no version bump in their PR, README sentence
and jni.rs comment ride with the fix); REPLY 01a1abb3-3577 to p2p-01
(cite the §22 paragraph from the device record). Open: the owner's cut
(0.2.1 recommended) once rust-ui-dev-02's fix is on main.

**Why:** the §20 closing record and the README status read as a working first run; every device run was a debug APK, so the release path was never exercised through Create.
**How to apply:** a Validate row that needs "the previous release" needs a baseline release that can hold the state; say which release is the baseline in the record. See [[stage-19-activation-2026-10-20]], [[adr-0065-semver-2026-10-23]].

Also 2026-11-05 (p2p-01 OBSERVATION 01a1abb6): human-android.yaml's
human endpoint advertise: true vs human-client-ui.md §13 — already
decided, ADR-0061 d5 + plan §30 Activate: the examples change in Stage
22; the provisioning document writes false now (their supply 41c4bf9e5
to rust-ui-dev-02). REPLY 01a1abbc-a1ad.


**CLOSED 2026-11-05:** #567 merged 8859f6366 2026-11-05T16:02:50Z; the §22 record is on main. The A40 at API 30 reproduces the defect (p2p-01 INFO 01a1ac5a-9727, record on #570) — cited in the next batch, job j539. rust-ui-dev-02's fix is #569 (in flight at this writing).

*References: adr-0065-semver-2026-10-23, stage-19-activation-2026-10-20*

*Observed 2026-11-05 (architect-cto)*

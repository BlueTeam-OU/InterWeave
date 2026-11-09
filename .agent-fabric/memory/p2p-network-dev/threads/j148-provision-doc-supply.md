---
role: "p2p-network-dev"
class: threads
topic: "j148-provision-doc-supply"
description: "j148 (rust-ui-dev-02's REQUEST 01a1ac17): #569 review F1/F3 in profile-config provision.rs — where to base the supply and who folds it"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - bbe95b6b2cd789be
---

## j148 (rust-ui-dev-02's REQUEST 01a1ac17): #569 review F1/F3 in profile-config provision.rs — where to base the supply and who folds it

j148, accepted 2026-11-05 (my REPLY 01a1ac17-7fe9). One commit on a for/
branch: provision.rs module doc made true (lines 6-8: the app calls
provision_embedded via android-platform's provision_profile on every start,
returning early when config.yaml exists, AlreadyExists = Ok; lines 19-23 and
tests/provision.rs:76: the stand-in's "always wrote" is false, it wrote
advertise true until e473680b4 and had no android: block or peer-cache
provider), the "never replaces" claim at lines 69-70 naming
a_first_run_writes_the_profile_and_a_second_is_refused, and for F3 a pub const
for embedded_document's first line ("# Written on first run by the Android
client"), which apps/human-android's Gradle verifyReleaseNative requires in
the release library; a test pinning it; send rust-ui-dev-02 the const's name.

rust-ui-dev-02's INFO 01a1ac7a (16:11Z): #569 is armed with F1/F3 deferred to
j148; they fold my range into their NEXT PR, on a branch off main once #569
lands — so base the for/ branch on main after #569 merges (it carries my
2ce1c7b06), not on the old for/rust-ui-dev-02/embedded-advertise-off.
Reply with the range.

The A40 since 15:48Z carries a test-signed (8f75414f) release build of
9aba4c441 with the owner's identity; a published APK will not install over it.

Delivered 2026-11-05 ~17:00Z: 0db4cfa68 (EMBEDDED_DOCUMENT_MARKER const, F1 docs,
marker test, mutation-checked). By mistake I pushed it first onto
for/rust-ui-dev-02/embedded-advertise-off, atop 43b89559c (the process_death fix
#569 had not folded yet, head 862414e4). REPLY 01a1aca5 gave the range; correction
REPLY 01a1aca7 said fold only ..43b89559c into #569, and put F1/F3 also on
for/rust-ui-dev-02/provision-doc-f1-f3 (parent 43b89559c). Lesson: read the job's own
memory before choosing the base; the request text predates the INFO that moved it.

*Observed 2026-11-05 (p2p-network-dev)*

---
role: "rust-ui-dev"
class: threads
topic: "j96-release-first-run"
description: "j96 (blocking) - release APK writes no profile on first run; the fix plan, the supply owed by p2p-network-dev-01 (j147), and the docs to change with it"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ceb39b3afb07f7f7
---

## j96 (blocking) - release APK writes no profile on first run; the fix plan, the supply owed by p2p-network-dev-01 (j147), and the docs to change with it

j96 was opened 2026-11-05 from p2p-network-dev-01's OBSERVATION 01a1aba6. I verified it on origin/main:
- apps/human-android/src/jni.rs `provision` writes a profile only under the `dev-stand-ins` feature.
- build.gradle.kts builds the Release variant with `--release` and no features, so a release APK has no
  config.yaml, and onboarding's identity write fails ("the profile document cannot be read").
- This holds for v0.1.0, v0.2.0 and main.

The fix (my REPLY 01a1aba9-2e9e):
- Every build calls profile-config's `provision_embedded` on first run, and `PersistError::AlreadyExists` counts
  as done.
- The stand-in `provision` is removed.

That document writes the human endpoint with `advertise: true`, while human-client-ui.md section 13 wants it off
(the stand-in's test the_stand_in_profile_advertises_no_endpoint). p2p-network-dev-01 is supplying advertise:false
as their j147, on a branch develop-qzapp/p2p-network-dev-01/for/rust-ui-dev-02/<what>, to be folded unrebased.

These go with the fix too (architect-cto via 01a1abad):
- the stale first-run sentence in apps/human-android/README.md;
- the jni.rs identity() comment "until step 7's onboarding lets the person choose".

architect-cto holds the plan record and the owner's decision on cutting a release. The Pixel now carries release
v0.2.0 with no identity, which is the device for proving the fix on a release build.

Related: [[j95-plan]] [[pixel-handover-p2p-network-dev-02]]

architect-cto DECISION 01a1abb3:
- The fix is a PATCH (ADR-0065 decision 2). Do NOT bump the workspace version; the cut (v0.2.1, they recommend) is
  the owner's act.
- The README status sentence and the jni.rs "until step 7's onboarding" comment ride in the same PR.
- The plan record is #567 (fa77a57aa), theirs.

The supply is ready: develop-qzapp/p2p-network-dev-01/for/rust-ui-dev-02/embedded-advertise-off,
c9ba8c215..41c4bf9e5, one commit. embedded_document now writes advertise:false, with the test
the_document_advertises_no_endpoint. I asked (REPLY 01a1abb9) for `allowed_client_kinds: [human-client]` on the
same branch, to match the stand-in, so expect a second commit. Fold with --no-ff, unrebased. architect-cto has
been told about architecture/config/examples/human-android.yaml:44, which still shows advertise:true.
The supply is COMPLETE: c9ba8c215..2ce1c7b06 (41c4bf9e5 advertise off; 2ce1c7b06 allowed_client_kinds
[human-client], with the test the_human_endpoint_admits_the_human_client_kind_only). The supplier's review is 5/5,
with each line's mutation failing its test. Nothing more is coming.

j96 opens its OWN PR beside #565 under the one-PR rule's exception for "a user-visible fix that must land now".
Say so in the PR body. Branch type: fix. Fold the supply first, then make the change.

PROGRESS 2026-11-05: PR #569 (branch develop-qzapp/rust-ui-dev-02/fix/j96-release-first-run-profile, worked in the j95
worktree's checkout because the job guard binds the job to it). The supply was folded at 9500ab872, then three work
commits: bb1068a43 (android-platform provision_profile + tests), ddc0a14ea (jni.rs unconditional + verifyReleaseNative
requires the marker "# Written on first run by the Android client"), 63f29db40 (stand-in document removed).
v0.2.0's release .so has 0 matches for the marker, which is the control. Gradle verifyReleaseNative passed (54 min).
Review dispatched from ~/agent-fabric-tmp/j96-review/review-569.yaml. The count is 3 work + 2
supplied, below the floor of 8, so the arm needs an authority's word. A device run of a fresh release install is owed.

ROUND 1 (2026-11-05 14:30): the review was posted on 63f29db4 with F1-F4, all P3, and the judge confirmed all of them.
I fixed F2, F4, risk 1 (an existence check before the write) and risk 3 in 052597543 (Answers trailer). F1 and F3 are
in provision.rs, p2p-network-dev-01's: I sent REQUEST 01a1ac17 for a supply. CI is red from RUSTSEC-2026-0441; #566
(devex) carries the ignore and is in the queue. Next: fold the supply, fold main after #566, re-review the range with
card_base c9ba8c215, then ask for the arming word (below the floor).
p2p-network-dev-01 accepted F1/F3 as their j148 (REPLY 01a1ac17-7fe9): one commit on a new for/ branch off 2ce1c7b06,
with a pub const for the marker. They send the const name and the range later today, after their A40 j141. When it
arrives, switch the Gradle check to the const's text if it changes.
DEVICE RUN DONE 2026-11-05 15:48Z: on the A40 (API 30), the release variant of 9aba4c441, test-signed (signer 8f75414f;
the test key is in ~/agent-fabric-tmp/j96-review/), replaced v0.2.0. The owner's Create got
through C4's Continue, and the log shows "start answered 0". Recorded in
apps/human-android/device-runs/2026-11-05-samsung-a40.md (a work commit on #569). The A40 now carries that build with the
owner's identity, so tell p2p-network-dev-01 before anyone uses it. #566 is merged into the branch (9aba4c441). CI
went red there from 504 download flakes, and the push re-ran it. Still owed: the j148 supply, a re-review, and the
arming word.
ARMED 2026-11-05 ~16:00Z on the owner's word ("arm 569") at 862414e4, with 6 work and 2 fix commits. The review chain
ran 3 rounds; F5 (a cited test that did not check the claim) was fixed in 862414e41. F1/F3 were deferred to
p2p-network-dev-01's j148 supply, which goes into my NEXT PR. A wait-merged watch is running. Still owed after the
merge: tell p2p-network-dev-01 (once the relay is back) that the A40 carries the test-signed fixed release with the
owner's identity, and that #569 merged with F1/F3 deferred, so j148 folds into my next PR.
RED AFTER ARMING (2026-11-05 16:13Z): the required "rust" check failed. The cause is the supplied 2ce1c7b06
(allowed_client_kinds [human-client]): transport-embedded's process_death.rs leases as client kind "process-death"
and is refused. I confirmed it by leasing as human-client, which passes. I sent REQUEST 01a1ac83 to
p2p-network-dev-01 for a fix commit (their test). Lesson: I skipped the full workspace suite before the review; a
supplied change to a shared document can break the supplier's own tests elsewhere. A full `cargo test --workspace
--no-fail-fast` log is at ~/agent-fabric-tmp/j96-review/workspace-test.log.
p2p-network-dev-01 accepted the process_death fix as j149, ahead of j148 (REPLY 01a1ac84): one fix commit on a for/ branch off 862414e4, sent with a full workspace suite run. Fold it, run the full suite, re-review the range, re-arm.
FOLDED 2026-11-05 16:50Z: the j149 fix 43b89559c (merge 7d619210f) and main (2df4869f5). The full suite runs under fabric-lease heavy; log workspace-test-2df4869f.log. Not pushed yet. Next: push, then a re-review of 862414e4..2df4869f (main's paths must be read, or do a full review over merge-base..HEAD), then re-arm on the owner's word, deferring F1/F3.
j148 SUPPLY ARRIVED: 43b89559c..0db4cfa68 (F1 docs; F3 pub const EMBEDDED_DOCUMENT_MARKER, test the_document_opens_with_the_marker_the_release_check_reads). Fold AFTER the running suite ends. Then name the const in the Gradle comment, run the profile-config, android-platform and transport-embedded tests, push, and re-review 862414e4..head (main folded: the reviewer must read every path). Defers are no longer needed if the re-review judges F1/F3 fixed.
PUSHED 4d6a95565 (main folded + j148 0db4cfa68 + Gradle comment), full suite green on 2df4869f, the three crates green on the head; round-4 re-review dispatched (rr4); verifyReleaseNative rerunning under heavy. Re-arm on the owner's word once it passes; no defers if F1/F3 are judged fixed.
RE-ARMED 2026-11-05 ~17:30Z at 4d6a9556 (8 work, 3 fix) on the owner's word. 4 review rounds; F6 (P3, other roles' prose from the main fold) is deferred: architect-cto takes plan section 22 after the merge (j539, REPLY 01a1acbf); p2p-network-dev-01 was told about packaging/android/README.md (OBSERVATION 01a1acbe-f3f6). verifyReleaseNative passes with the const. The stray watch is gone and its one message was replayed. wait-merged is running.
DONE 2026-11-05 17:55Z: #569 MERGED as fab74dd40 (8 work, 3 fix). I sent the merge sha to architect-cto (INFO 01a1acde-908a,
for their j539 plan line) and to p2p-network-dev-01 (INFO 01a1acde-a14b). The test-only key and the APKs are deleted
from scratch. Still open, owned by others: the v0.2.1 cut (the owner's), the plan section 22 line (architect-cto
j539), the packaging README clause (p2p-network-dev-01). Not shown on a device: the release key, restore, an
in-place update from v0.2.0, the Pixel.

*References: j95-plan, pixel-handover-p2p-network-dev-02*

*Observed 2026-11-05 (rust-ui-dev)*

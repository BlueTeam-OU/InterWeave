---
role: "p2p-network-dev"
class: solution
topic: "j147-embedded-document-matches-stand-in"
description: "the release Android first run writes profile-config's embedded_document; it matches the debug stand-in on advertise off and allowed_client_kinds [human-client] but is NOT the stand-in's document (no android: block / peer-cache there); and…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dabf736a675e19b3
---

## the release Android first run writes profile-config's embedded_document; it matches the debug stand-in on advertise off and allowed_client_kinds [human-client] but is NOT the stand-in's document (no android: block / peer-cache there); and the j141 baseline rule

j147 (2026-11-05): rust-ui-dev-02 found that a release Android build reached onboarding with no config.yaml. The stand-in `provision` (crates/human/android-platform/src/stand_in.rs) runs only under dev-stand-ins, and nothing called `provision_embedded`. They fixed the app (their j96) so the first run calls `provision_embedded`, with AlreadyExists counting as done. I supplied commits on develop-qzapp/p2p-network-dev-01/for/rust-ui-dev-02/embedded-advertise-off, base c9ba8c215: 41c4bf9e5 puts the human endpoint at `advertise: false` (human-client-ui.md §13, line 205: every identity's endpoint has advertise off). 2ce1c7b06 adds `allowed_client_kinds: [human-client]` (rust-ui-dev-02 asked for it, REPLY 01a1abb9). Range c9ba8c215..2ce1c7b06, folded unrebased into their j96 PR. Each has a test in profile-config tests/provision.rs, and each test was mutation-checked.

Why it was wrong: embedded_document had copied architecture/config/examples/human-android.yaml, which still says `advertise: true`. That is decided and sequenced, not a defect: ADR-0061 decision 5 sets every identity's endpoint to advertise false, "every example profile that declares it, which plan §30 changes". The example profiles change in Stage 22's batch (plan §30 Activate; human-desktop.yaml too). Answered in architect-cto REPLY 01a1abbc to my OBSERVATION 01a1abb6. Lesson: the example profiles lag the decision until Stage 22. Take an endpoint default from ADR-0061 and from the stand-in it replaces (tests/android-e2e platform.rs provisions false too), never from an example.

j141 (paused): architect-cto REPLY 01a1abb3 (seq 851561). The Pixel findings are recorded in plan §22 (#567, fa77a57aa). The baseline for the update-in-place case with the same PeerId from the same store is the first published release that onboards a new user. It runs from that release to its successor in the next phone window, and is a named limit until then, never a case against v0.2.0. The device record and README row cite §22's paragraph "The published Android releases cannot onboard a new user".

j141 branch (2026-11-05, at the j147->j141 switch): the paused note says commit on develop-qzapp/p2p-network-dev-01/docs/j141-android-update-downgrade-pixel. That branch was empty (at main c9ba8c215), and fabric-jobs switch refused because the worktree did not have the job's recorded branch checked out. So I reset feat/j141-j140-no-launch-action-owed-the to origin/main c9ba8c215 in the j141 worktree, checked it out, and deleted the empty docs branch. Commit the Pixel record (~/j141-handoff) on the feat/j141 branch, not the docs one.

Correction (j148, #569 review F1): embedded_document is not what the stand-in "always
wrote". The stand-in advertised the human endpoint until e473680b4 (2026-10-24) and had
neither the android: block nor the peer-cache provider. They agree only on advertise off
and the client-kind list, and the provision.rs docs now say so (0db4cfa68).

*Observed 2026-11-05 (p2p-network-dev)*

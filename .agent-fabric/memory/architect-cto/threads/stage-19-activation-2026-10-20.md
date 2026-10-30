---
role: "architect-cto"
class: threads
topic: "stage-19-activation-2026-10-20"
description: "Stage 19 (packaging, migration, release) activated 2026-10-20 on the owner's word, beside Stages 17 and 18; the §22 record (IW #317) is the packages' authorisation; the split by lane and what waits (macos, windows)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 83427c9e4e2b8bfd
---

## Stage 19 (packaging, migration, release) activated 2026-10-20 on the owner's word, beside Stages 17 and 18; the §22 record (IW #317) is the packages' authorisation; the split by lane and what waits (macos, windows)

The owner's word "activate Stage 19" (2026-10-20, relayed by fabric-coordinator, message 01a157d9-5e4d-78c8-8586-02e8f074d339) answered my recommendation after Stage 18's S0–S8 and S10 landed and the two Rust lanes ran dry. The record is plan §22's activation block on BlueTeam-OU/InterWeave#317 (3fd794a4): the stage runs beside Stages 17 and 18 as §19's Stage 16 ran beside Stage 15; the status value stays `stage-17-android-human-client`; the record is the authorisation for the stage's packages (CLAUDE.md §3).

Split: p2p-network-dev-01 (REQUEST 01a157dd-edc0) — packaging/linux, supervision units, lifecycle/uninstall rows, the Stage 13 carry (SIGHUP reload and surviving leases; flock on NFS homes), the ADR-0057 Android release run. rust-ui-dev-02 (REQUEST 01a157dd-f026) — upgrade / rollback / profile-config migration / PeerId preservation as host tests against the packaged build, the Stage 15 Slint attribution on the download page. packaging/macos and packaging/windows (with the Stage 18 Windows carry) wait on a lane and a host and open on their own record line. The stage closes on the owner's word once every §22 Validate row has a green test on main or a `limit` row.

2026-10-20 15:30Z: #317 round-1 review (on 897140fc, posted) found two
P2 + three P3 on my prose: F1 CLAUDE.md §9 names lintDebug/
testDebugUnitTest that only #311 (armed on my word, watcher running)
adds — resolves by folding origin/main after #311 merges, then a
re-review of the fix range; F2/F3/F5 + the §18 (9) autostart carry
fixed in 21665e6a (review-fix); F4 ci.yml comment is devex-tooling's
(INFO 01a159e9). #317 is 3 work, 6 fix; arms only on the owner's word.
Lesson: a CLAUDE.md sentence must be true of MAIN at the head, not of
an armed PR — write "(#311, when landed)" or wait for the fold.
2026-10-20 16:00Z: #317 FINISHED at a5e02b18 (3 work, 7 fix; rounds
1-3 posted, round 3 clean; finding 4 = ci.yml comment lands on
devex-tooling's #323 ee8041a2). Waits ONLY on the owner's word to arm
(under 8; fabric-coordinator put it to the owner 01a15898-9c08). The
Stage 17 closing record (j148) goes onto this same PR as a work commit
when its evidence is in — that would make it 4 work; still under 8.
fabric-review next quirk: it derived round 3 from round 1's base;
fix by editing the rr<N>.json (range, review_round, previous_findings
path) and rendering with `fabric-review brief rr<N>.json`.
2026-10-20 16:30Z Skia ruled (ADR-0057 decision 4, A 2026-10-20): the
prebuilt archive pinned by a committed SHA-256, file:// handoff, the
bindings' silent source fallback made a red run; the android check job
reads the same pin (REQUEST to devex-tooling). Supply 48296fd7..0ad8bc32
on for/p2p-network-dev-01/skia-prebuilt-pinned, to fold into their j94
PR with the release script. OWED on #317's next plan commit (with the
Stage 17 closing record): reword §22's "the Skia choice ... is
architect-cto's, on p2p-network-dev's question" to cite the amendment.
Validator lesson: a digest amendment line must sit INSIDE the ADR's
"### NNNN —" entry (before the next "## Cluster" heading).
2026-10-20 ~17:00Z: #317 ARMED on the owner's word ("arm 317" in
session) at a5e02b18, queued. Consequence: the Stage 17 closing record
(j148) no longer rides #317 — it goes on a NEW branch off main
(develop-qzapp/architect-cto-01/docs/stage-17-close), with the §22 Skia
sentence rewording and the plan's §20 record in one PR; a single work
commit there needs the owner's word to arm.

2026-10-20 17:01Z: #317 (3 work, 7 fix) merged as 8f51ee87 on the owner's word; the Stage 19 §22 activation record is on main. The Stage 17 closing record (j148) goes on a new branch off main, as planned.

2026-10-20 17:28Z: rust-ui-dev-02's #318 (16 work, 14 fix) merged 5121d46d: the §22 Track B rows for the closing record as they stated them (fresh install, upgrade, migration, PeerId, recovery drill GREEN in tests/desktop-e2e release.rs; rollback GREEN both halves; limits: no packaged build tested, no older human-desktop on the axis, pre-marker builds unprotected, reading not migrating proved; release page `limit` until first publish). j185 closed on it.
FINDING the same day: plan §18 clause (4) decided recovery mode as a blocking screen (Quit, Retry, path shown) but run.rs exits with a stderr line and no window on 65/78/75 since 196352d7; the §18 closing record carried only read-only open/export, so record and code disagree. DECISION 01a15a5f to rust-ui-dev-02: one refusal screen (Close; Retry on recovery; 75 raises the existing window), a job now; my §18 carry correction + §22 citation supply is j203.

17:40Z: record supply for the refusal screen is bc574d2f on for/rust-ui-dev-02/j31-startup-refusal-record (j203); rust-ui-dev-02 took the screen as j31, 77/74 included.

*Observed 2026-10-20 (architect-cto)*

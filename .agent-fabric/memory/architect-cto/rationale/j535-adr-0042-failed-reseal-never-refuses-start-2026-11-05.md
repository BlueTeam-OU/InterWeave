---
role: "architect-cto"
class: rationale
topic: "j535-adr-0042-failed-reseal-never-refuses-start-2026-11-05"
description: "j535 (2026-11-05): ADR-0042 A 2026-11-05 ruled for rust-ui-dev-01 j213 — a reseal refused otherwise than UserNotAuthenticated never refuses the Android start; committed 690d7f603 + F3 fix 9e95b3d6f on #567, rounds 3–4 posted (clean)…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 24591b2781e890df
---

## j535 (2026-11-05): ADR-0042 A 2026-11-05 ruled for rust-ui-dev-01 j213 — a reseal refused otherwise than UserNotAuthenticated never refuses the Android start; committed 690d7f603 + F3 fix 9e95b3d6f on #567, rounds 3–4 posted (clean); closes when #567 merges

rust-ui-dev-01's QUESTION 01a1abeb-f9ea (j213: the service start now
calls custody::reseal after unlock, p2p-02's finding 01a1abab that it had
no caller): what a reseal failed otherwise than UserNotAuthenticated does
to the start. Ruled (DECISION sent 2026-11-05 from the j534 session):
go on under the record's own policy byte, retry at every start; the
refusal logged and the pending change exposed in the status model by
name beside background_restart_requires_user_authentication; ProfileLocked
is the start's own lock question; refusing (in general or only toward
user-presence) rejected: offline for a fault the unlock survived, protects
nothing over the day before the change.

COMMITTED 2026-11-05 from the j535 session: 690d7f603 (triple + custody
doc + digest) and 641ac829b (a line break) on #567's branch, under the
blind review (round 3 of #567). The wording says the pending change is
shown "where background_restart_requires_user_authentication belongs",
not beside it: rust-ui-dev-01 (REPLY 01a1abf3) and the plan's §20 carried
list agree nothing emits that diagnostic yet. Sha sent to rust-ui-dev-01
(REPLY 01a1abfa-e6b9): no hold on the j213 PR for the person-facing
surface (ux-designer's spec, language-culture-en's words); the j213 PR
carries the reseal and the status member. j535 closes when the sha is on
origin/main via #567 (below minimum: 2 work, 1 fix; waits on #566's
deny.toml ignore and the owner's word).

**Why:** a decision and its record must land together; the reply named the amendment as written, so the fresh session owes the commit.
**How to apply:** watch #567 to main; the j213 PR folds 690d7f603 before its review. See [[j534-android-release-cannot-onboard-2026-11-05]].

**Round 3 review of #567 (2026-11-05, posted 5786037790):** F3 (P3,
certain): "neither silent nor only a log line" was false for
`OldKeyRemains`/`Unsynced`, whose record is written before the retire
and so already carries the configured policy. Fixed 9e95b3d6f: the
exposure covers "the other policy's key remains" too, in all five places.
OPEN THREAD (reviewer's risk, no finding): the rule "never refuses"
also covers `RecordNamesOther`, `Unreadable`, `NotProvisioned`,
`Identity`, `CipherShape` and the seal's `KeyInvalidated|KeyMissing|
Authentication` after a successful unlock — only reachable if the record
changed between the unlock (no lock held) and the reseal, or an adapter
misbehaves; then the start would run an identity the disk no longer
names. Settle it on the j213 branch or by a ruling: my lean is that a
record changed under the start is the lock's question (another writer),
answered by the start's lock take, and a misbehaving adapter is
`Unavailable`. Not yet ruled.


**CLOSED 2026-11-05:** #567 merged 8859f6366 2026-11-05T16:02:50Z; 690d7f603 and 9e95b3d6f are on main.

*References: j534-android-release-cannot-onboard-2026-11-05*

*Observed 2026-11-05 (architect-cto)*

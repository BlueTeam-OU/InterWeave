---
role: "architect-cto"
class: threads
topic: "j539-a40-api30-and-no-0-1-0-compat-word-2026-11-05"
description: "j539 follow-up to #567 — the A40 at API 30 reproduces the release onboarding defect (p2p-network-dev-01 INFO 01a1ac5a-9727, record on #570), and the owner's word of 2026-11-05 \"no need to maintain compatibility with 0.1.0\""
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ce00722537840666
---

## j539 follow-up to #567 — the A40 at API 30 reproduces the release onboarding defect (p2p-network-dev-01 INFO 01a1ac5a-9727, record on #570), and the owner's word of 2026-11-05 "no need to maintain compatibility with 0.1.0"

2026-11-05, while #567 was armed at c5813ccb8: p2p-network-dev-01
(INFO 01a1ac5a-9727) ran j141's Android cases on the samsung-a40 at
API 30 (minSdk). Release v0.1.0's Create reached "Identity created"
and refused at the seal with the same Storage refusal as on the Pixel
at API 37 — the defect is not API-37-specific. Downgrade refused and
update-in-place kept firstInstallTime there too. Records on
InterWeave #570 (packaging/android/evidence/ + README Validate row).
The PeerId half is recorded as not run under §22's baseline rule.

The owner said in p2p-01's session about 15:33Z: "no need to maintain
compatibility with 0.1.0" — a versioning word (ADR-0065) nobody has
recorded yet; p2p-01 left it to me.

**Why:** #567's §22 defect paragraph names only the Pixel finding and
"the A40 record of 2026-11-04 says the release was not measured"; a
push to the armed head would disarm it, so it stays.

**How to apply:** job j539 — after #567 and #570 merge, amend the §22
defect paragraph in the next batch: cite the A40 API 30 reproduction
beside the Pixel one, and record the owner's 0.1.0 word where ADR-0065
compatibility is stated. REPLY 01a1ac5c-c83c sent to p2p-01.
See [[j534-android-release-cannot-onboard-2026-11-05]].

2026-11-05 17:10Z, rust-ui-dev-02 OBSERVATION 01a1acbe-eeb0: the same §22
paragraph (plan L6059, fa77a57aa/c7066dfbc) goes stale when #569 lands —
its four present-tense claims (no first-run profile written, stand-in
only under dev-stand-ins, provision_embedded uncalled, fix owed) are
false at #569 head 4d6a9556 (A40 API 30 onboarded, device-runs/
2026-11-05-samsung-a40.md). They deferred F6 of #569 round 4 on this
message. So j539 batch = one edit of that paragraph after #569 merges:
past tense + the fix sha + the A40 reproduction + the 0.1.0 word.

#569 MERGED fab74dd40 2026-11-05T17:55:32Z (rust-ui-dev-02 INFO 01a1acde): the §22 paragraph cites it; not shown there: release-key signing, restore on a release build, in-place update from v0.2.0, the Pixel at API 37; v0.2.1 cut is the owner's act.
p2p-network-dev-01 OBSERVATION 01a1ace0 (same finding; their #574 fixes packaging/android/README.md counterpart): replied 01a1ace1-4c5f — line wording "fixed on main by #569; the release that ships it is the owner's".

**DELIVERED 2026-11-05:** BlueTeam-OU/InterWeave#575 (2 work, docs only; head 29870b438 after folding main 1aadab26f): 8d7dc6d00 ADR-0065 A 2026-11-05 "No release owes `v0.1.0` compatibility" (decision 2 in place, history note, log row, digest, README row; names v0.1.0 only) + 0d0afc2c7 the §22 paragraph in past tense, title kept because #574 quotes it. Blind review round 1 dispatched; arms on the owner's word (under 8). REPLYs sent to rust-ui-dev-02 (IN-REPLY-TO 01a1acbe) and p2p-01 (IN-REPLY-TO 01a1ac5a). Mishap: #571 had merged (1aadab26f 18:20Z) while I folded my commits onto its branch and pushed ad5b58965 there; the branch delete was refused by the classifier [Git Destructive], so origin's j540 branch carries two stray commits that are on #575 anyway — sweep reports it; the owner may delete it.

Round 1 (29870b438): F1 P2 — the amendment reasoned "nothing of v0.1.0 is relied on" from the APK defect, but the fleet runs installed v0.1.0 Linux programs with profiles (packaging/linux/evidence/2026-10-27-second-uid-port.md); fixed 9b19f7d4d: the word alone carries the rule, APK occasioned it, Linux installs covered by the same word. F2 P3 merge time: commit stamp 17:39Z vs PR merge 17:55Z; fixed b1584e876. Re-review round 2 dispatched at b1584e876.
Round 2 at b1584e876: CLEAN (F1, F2 fixed; posted 5790111241). #575 (2 work, 2 fix) awaits the owner's word at b1584e876.


**CLOSED 2026-11-08:** #575 (5 work, 6 fix) merged 4c4cf3587 at 2026-11-08T06:16Z on the owner's word 'arm 575' at head 2d6ca2bf3; F1 P3 lifted by --defer (cited §11 L1 on main via #579, 4w/3f, 62ba5cf01).

*References: j534-android-release-cannot-onboard-2026-11-05*

*Observed 2026-11-08 (architect-cto)*

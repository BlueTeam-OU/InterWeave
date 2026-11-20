---
role: "ux-designer"
class: threads
topic: "j126-network-grant-row"
description: "j126 — unlock note §11 rewritten for a network grant that ends the process (GrapheneOS); delivered #659 merged 260a2a43c"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 51a236a1e1c77d4b
---

## j126 — unlock note §11 rewritten for a network grant that ends the process (GrapheneOS); delivered #659 merged 260a2a43c

j126 (2026-11-13): PR #659, commit 3cee7cdb8, revises design/2026-11-05-unlock-policy-pending.md:
the §11 restarts list, the L1 row, a new paragraph "A network grant that ends the process", F11,
A16 bullet 2, the Built-by line, and a §9 hypothesis (no notice at the next launch).

architect-cto-01 answered my QUESTION 01a1d55d in REPLY 01a1d55f:
- The read-unkept loss after the grant is ADR-0044's reading (RETENTION.md §7, "Evaporates").
  It is not provisional. A notice at the next launch stays mine to propose, as long as it names
  no message.
- They are supplying a for/ branch on 3cee7cdb8: human-client-android.md step 2 and
  android-key-custody.md line 78, each split by whether the process survives the grant.
  Fold it, then re-review.

Also asked: language-culture-en, for the English pass (REQUEST 01a1d55e).
Review r1: brief in ~/review-reports/pr659.yaml and pr659-brief1.txt.
Arming: 1 work commit, so it needs the owner's word.
Related: [[j98-unlock-s11-pixel-review]], [[android-permission-grant-kills-process]].

Round 1 review (posted as 5893087898 on 3cee7cdb8): F1-F5 answered in e8549e1b0 (PR comment).
Report and request are in ~/review-reports/pr659-r1.md and pr659-rr1.yaml.
Lesson: the tree recorded one grant sighting, not the three in my memory. I told architect-cto
(01a1d564). Next: fold the architect for/ branch and the LC pass, then re-review (fabric-review
next) and ask the owner's word to arm.

Round 2 (review 5893571791 on ad461090b): F1-F5 fixed. F6 (P2): the recorded grant ran under Stay
reachable (j98 thread lines 38-39, 58-59), not foreground-only. The note was fixed in 4691e8000.
Asked architect-cto-01 for a for/ commit fixing human-client-android.md (REQUEST 01a1d57d), and
LC-en for a pass (REQUEST 01a1d57e). Folded so far: LC 6f512e15d and 81ffb562b, architect a01b42fe6.
Next: fold both answers, run re-review round 3 (rr2.json), then ask the owner's word.
Lesson: before I cite a run's conditions, read the run record's config lines, not my memory of them.

Round 3 (review 5894213279 on 3e2f07afb): clean. #659 was armed 2026-11-13 with 8 work commits.
Risks carried in a PR comment: "the restart was refused" names no refusal; A16 reads pidof only
once; the dialog vs the settings page after pm grant. For the next Pixel run: pidof again after a
wait, and dumpsys package for the permission flags.
Also on 2026-11-13: /home on the host filled to 100%. I deleted j122's target/ (2.9G, rebuild it on
resume) and told fabric-coordinator (01a1d5a3).

Re-armed 2026-11-13 at 1fa8c336 (9 work commits). Rounds 4-6:
- F7 (architect-cto's steps 4-5 said "not checked"; j98 line 57 observed both): fixed by 4e0d7f2aa.
- LC edits went in as 974f64e55.
- F8 (P3, step 5 says "posted that one notification" when the record holds only the log line
  written before notify()): deferred to architect-cto (OBSERVATION 01a1d5c7).
Lesson: a run record that quotes a log line proves where the code reached, not what the person
saw. Next run, keep a screenshot or a dumpsys notification for every claim the docs will cite.

Delivered: #659 merged 260a2a43c on 2026-11-13 (6 review rounds). Carried: F8 is with architect-cto. Risks for the Pixel run: pidof twice, check the permission flags first.

*References: android-permission-grant-kills-process, j98-unlock-s11-pixel-review*

*Observed 2026-11-13 (ux-designer)*

---
role: "architect-cto"
class: workflow
topic: "arming-words-2026-10-26"
description: "Arming words given or withheld on 2026-10-26 (InterWeave #462 given at 1e5821eb; #464 withheld because the head moved after the ask), and the check that decides it"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 30a6121dcee2a32f
---

## Arming words given or withheld on 2026-10-26 (InterWeave #462 given at 1e5821eb; #464 withheld because the head moved after the ask), and the check that decides it

Arming words on 2026-10-26 (my remit: below-minimum and boundary authority in InterWeave):

- j393: InterWeave #462 (ux-designer-01, 3 work, 3 fix) — GIVEN at
  1e5821eb (REPLY 01a17a0a with the AUTHORIZES header): one design file,
  five posted blind rounds chaining from main with no merge commit, round 5
  clean on the head, 0 threads, not a boundary.
- j394: InterWeave #464 (devex-tooling, CI apt cache) — WITHHELD for
  387acdc6: `fabric-pr arm --dry-run --any-owner` showed the head had
  moved to 08ad5218 with no review of it (REPLY 01a17a1c, REPLY-EXPECTED
  yes: they re-review and ask again naming the new head).
- j395: #464 — GIVEN at 08ad5218 on the re-ask (REQUEST 01a17a1d): re-review of
  387acdc6..08ad5218 posted clean (four help-comment lines), 0 threads, dry-run
  names that head; AUTHORIZES header in my REPLY.
- j396: #463 (rust-ui-dev-02, 4 work, 7 fix, own-card leaving rule) — GIVEN at
  b1367dec (REPLY 01a17a58): four ui-slint/e2e files, three rounds chaining from
  base, round 3 clean, 0 threads, not a boundary; reviewer could not run the
  suite (no registry in its sandbox), CI stands for it.

**How to apply:** before any word, run `fabric-pr arm <n> --dry-run
--any-owner --basis x` in the project clone: it names the CURRENT head,
whether the paths are a boundary, and the review state, in one line. A
request's head is stale the moment its sender pushes again; a word for a
stale head arms nothing and reads as consent. Read the diff and the
posted review bodies, not the request's summary of them.
See [[the-arming-word-is-given-on-a-finished-head-only]].

**2026-10-27 j400:** #466 (4 work, 5 fix; boundary hole_punch.rs) given at 018d296d: three rounds
chained from the merge-base, round 3 clean, 0 threads; reviewer ran no check (sandbox), CI pending
so the arm waits on it. Owed by me later: one DCUTR.md filter-paragraph sentence saying a reissued
dial's failure is the crate's dial's (next architecture batch).

- **#470 (2 work, 0 fix, boundary: hole_punch.rs test + cfg(test) recorder, j81 carried from #466)**: given at d14ea496 (j402, REPLY 01a17b8a, 2026-10-27). Read: full diff from merge-base bdc7ac89, round-1 review clean 0 threads, dry-run refused on the word alone; CI pending at the word. #466 merged (4 work, 5 fix); its DCUTR.md sentence still owed by me.

- **#481 (3 work, 4 fix, boundary: shared-space comment rewordings to (iii) + dcutr.rs wait tick, p2p-network-dev-02)**: given at 67c2d1e5 (j415, 2026-10-27), to arm AFTER #479 lands (the (iii) citations resolve there; the review's F1 "partly" is that ordering). Three rounds chained from merge-base 740ed001.

- **#482 (2 work, 2 fix, not boundary: add-a-peer 5.3 unasked-read focus, rust-ui-dev-01 j157)**: given at 58b6f8b7 (j417, 2026-10-27). Round 1 from merge-base 2d961966 (F1 P2 stale focus_sent compare, F2 P3 comment), round 2 fix range clean, 0 threads. Rules are ux-designer-01's on #480 (4 work, 6 fix, unmerged) — carried, not a hold.

- **#484 (4 work, 1 fix, not boundary: add/Show back arrow reads AddClose "Back", rust-ui-dev-01)**: given at b1494d32 (j419, 2026-10-27); round 1 from merge-base abd9b4ee (F1 P3 RTL copy untested) fixed, round 2 clean. #482 MERGED on my earlier word.

**13:05Z:** owner in session: "merge 484", "merge 479", "merge 481". Armed #484 (4 work, 1 fix, rust-ui-dev-01, --any-owner), #479 (3 work, 1 fix, mine, queued 1), #481 (3 work, 4 fix, p2p-02, --any-owner, queued 2). Both lanes told.

**13:40Z:** #479 MERGED 95be7c3e (rule 10 ii/iii on main), #481 MERGED 6d33a6af after it; #484 (4 work, 1 fix) queued. Sweep removed the supply worktrees (branches kept: note (ii) 85565595, 5b add-floor a8fe9ea1 ride #478).

- **#486 (1 work, 0 fix, not boundary: POST_NOTIFICATIONS asked at API 33+ only, rust-ui-dev-01 j161)**: given at 46a96518 (j420, 2026-10-27); round 1 clean.

- **#488 (4 work, 17 fix, design only: start-window U1-U4, contact-cards 5.4.1 tie-break, ux-designer-01 j77)**: given at 14c652c3 (j426, 2026-10-27); seven rounds chained from merge-base fde14292. Side effect: the per-peer endpoint bound now counts contact_routes too (20afc55a, 5th commit on the state-v10 for/ branch) so §11's "at most 63 same-device lines" holds.

*References: the-arming-word-is-given-on-a-finished-head-only*

*Observed 2026-10-27 (architect-cto)*

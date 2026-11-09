---
role: "ux-designer"
class: threads
topic: "j43-add-peer-peerid-wording"
description: "j43-j49 state: add-a-peer PeerId wording, Back during add, row-gone focus, pending lines per add, Stage 21 flows; PRs #396, #400, #405 (part 2 of 3)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f294c993be6ac636
---

## j43-j49 state: add-a-peer PeerId wording, Back during add, row-gone focus, pending lines per add, Stage 21 flows; PRs #396, #400, #405 (part 2 of 3)

2026-10-23: #391 merged ddf1db80; j41, j42 delivered. j43 (PeerId wording) on #396 b3ba5625, review running.
2026-10-23: #396 clean round 2 on e9833b33 (1 work, 2 fix); AUTHORIZES asked. After merge: deliver j43; tell rust-ui-dev to apply texts in labels.rs.
2026-10-23 07:55Z: j44 (rust-ui-dev-01 QUESTION 01a167be) answered on #396 7b676d33: list off only from a row (a); Back during add closes the page = unknown add. AUTHORIZES re-ask after round 3.
2026-10-23 08:05Z: #396 adds AddWaiting (folded 3faef206) and contracts-landed update 6cf25b9f (own_addresses 2.8, add_addresses/own_peer 2.9 recorded; RefusedLimit names a line); main merged c3377233; round 4 running. Tell rust-ui-dev-01: RefusedLimit names a line.
2026-10-23 08:07Z: #396 clean round 6 on 9927df50 (4 work, 7 fix, 1 merge); AUTHORIZES asked (arch18). Tell rust-ui-dev-01 after merge: RefusedLimit names a line; several-not-taken is a client check below 2.9; RefusedLimit text new.
2026-10-23 10:08Z: j45 (QUESTION 01a16839) on #396 6a1a03ef: second row-gone §8 row, focus however it got there; PeerRowGone placeholder until LC (01a1683a-0789). AUTHORIZES 01a167cc superseded.
2026-10-23 10:12Z: #396 round 7 F10/F11 on ed56237c; LC asked re PeerRowGone 4th case (app-placed focus) 01a1683d-7583. Tell rust-ui-dev-01 after: RemoveUnknown read back moves focus only if still on snackbar/row; read-back exclusion only for that row.
2026-10-23 10:20Z: #396 clean round 12 on 63680cb2 (6 work, 13 fix, 1 merge); AUTHORIZES asked (arch20); rust-ui-dev-01 told (ui22). After merge: deliver j43, j44, j45.
Owed next revision (s8): a target the app placed, dropped before focus reached it: next/previous/Add an address, nothing announced (rust-ui-dev-01 QUESTION 01a16856, answered 01a1685x).
2026-10-23 10:40Z: #396 CI red from RUSTSEC-2026-0385 (all PRs); fix is #386 (j115). After #386 merges: merge main into #396, re-review the merge, re-ask AUTHORIZES.
Owed next revision (s5.7): AddUnknown note goes when the field no longer holds the unknown add's lines; not restored by editing back (rust-ui-dev-01 QUESTION 01a16949, job j46). Also s8 placed-target-dropped-before-focus (01a16856).
Owed next revision (s5.7): pending per line for adds (like Trust per PeerId): every unknown add's lines pending until a read/answer decides or page closes; no-write Add including a pending line -> AddUnknown (QUESTION 01a16973, job j47). Corrects j46 reply's 'new add' sentence.
Owed next revision (s5.3, s8): a block settled by an unprompted read: focus stays, line announced once; if decided trusted and focus was on its Trust/Not now, focus as a decided block (QUESTION 01a169a2, job j48).
2026-10-23: #396 ARMED on owner's word ('merge 396'), queued at 2. After merge: deliver j43,j44 (j45-j48 go to next revision PR).
2026-10-23: #396 merged 46fa43d9; j43-j45 delivered. j46-j48 + j45 follow-up on #400 d504c32f, review running.
2026-10-23 18:37Z: #400 clean round 5 on 3f4062ad (1 work, 4 fix); AUTHORIZES asked (arch23). rust-ui-dev-01 told (ui28). After merge deliver j46-j48.
2026-10-23: #400 merged b82e437f; j46-j48 delivered. Queue empty. Still owed: rust-ui-dev-01 builds AddWaiting, unsettled lines etc. on their next PR; known risks: unasked read during TrustStepWaiting, double announcement on a focused changing line.
2026-10-24: j49 (architect REQUEST 01a16c4a, Stage 21 close step 5): flows already in add-a-peer s5.1/s5.4/s5.6; gaps closed on #405 dfaaaba1 (C4/R4 -> Connect in identity note; relay-only line in s5.4); reply 01a16c4e sent.
2026-10-24: #405 clean round 3 on 35f320f7 (1 work, 2 fix); AUTHORIZES asked (arch25). Deliver j49 after merge.
2026-10-24: #405 merged 5f4d9ecf; j49 delivered; architect told.
Before: [[j39-add-peer-unknown-retries]]; next: [[j50-add-peer-desktop-scan-show]].

*References: j39-add-peer-unknown-retries, j50-add-peer-desktop-scan-show*

*Observed 2026-11-03 (ux-designer)*

---
role: "rust-ui-dev"
class: threads
topic: "j68-handoff"
description: "j68 start window + owner's second-review findings (j73 P2, j74 P3-2, j75 P3-1) handoff 2026-10-27 - branches, worktrees, what is built, what is owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ba7f70c0cacee565
---

## j68 start window + owner's second-review findings (j73 P2, j74 P3-2, j75 P3-1) handoff 2026-10-27 - branches, worktrees, what is built, what is owed

State on 2026-10-27 ~18:50Z.

#483 (3b6db4e7) and #489 (24445504) MERGED. P2-1 start window runs in batches:
- B1 = PR #492, branch develop-qzapp/rust-ui-dev-02/feat/j68-start-window-b, worktree
  ../interweave-j68-four-human-client-findings-forwarded-as. Head 62d11cb5: endpoint bound, conversation_range,
  shown_conversations, lc text for NotSentEndpointLimit (532fce2c, Supplier-Review 01a17e7f), architect's f91f6c89 folded
  (b9e2eb00), P3-1 COUNT(*) (62d11cb5 = j75). Full suite ci5.log and a blind review were running; then post, fix, arm (>=8 work).
- B2 (model/protocol), B3 (views/desktop), B4 (Android/CLI/removal) still to build, each under 16 commits. B2 reads contacts via
  contact_directory_page to its end (told rust-ui-dev-01, REPLY 01a17ea5: keep its contact_id keyset; model sorts by contact_order).

Owner's second review (architect-cto REQUESTs 18:07Z):
- j73 P2 facade past stop + j74 P3-2 CLI stdin bounds: branch develop-qzapp/rust-ui-dev-02/fix/j68-android-facade-past-stop,
  worktree ../interweave-j73, head 5032e717 pushed, NO PR yet (opens after #492 merges; one open PR rule). Needs full suite +
  blind review then. Hub::retire/ended/facade_alive, generation on wants_listing/listed/update, test-hooks gate
  hold_commands_for_test; store BUSY_TIMEOUT explicit (rusqlite already defaulted 5 s). PR body: no device run.
- j74 refs map ruled (a) by architect-cto (01a17ead-19cc): ref capped at 256 bytes serialised, built d4a2249a (pushed).
- #492 review of 62d11cb5 posted (F1-F4 judged real). Folded 01bd1ddf (c597d447); F1 fixed db3586b2; F2 = shown_conversations
  reverted 6717c2fd (needs v11 covering kept index, REQUEST 01a17ebd to architect-cto; rebuild in B2 with Chats); F3 carried P3
  (write-at-open under quota degrades: backfill+migrate+refresh_name_keys, one later fix); F4 answered. Answers comment on PR.
  Round-2 re-review posted on 6717c2fd (F5 P3: my widened guard never landed - a failed `git revert` ended the && chain;
  fixed 3a2aab5c, correction comment posted). Architect folded v11 row/quota-above-open rule/index sentences (63a80592).
  Owed later: v11 kept index in B2; open's own writes run ABOVE quota (architect 81091c7d) with vacuumed test.
  Round 3 posted on 63a80592 (no P1/P2; F6 P3). F6 ruled (01a17ec7): build in the quota batch - accept_card indexes the
  route's (peer,endpoint); backfill version 2 adds contact_routes pairs; test deletes a route-only endpoint, still counted.
  39d51c3d folded (8c51e82f); round 4 clean; #492 ARMED 2026-10-27 ~19:45Z at 8c51e82f (16 work, 3 fix).
  #492 MERGED 80de290e. j73+j74 = PR #495 (5 work; main folded 9000415b); ci9.log suite + blind review running.
  Under 8 work: arming needs the word the dry-run names. Review of 9000415b: F1-F5 real (F1-F3 P2). Fixed:
  6483d06f (F1+F2 relist count), 9885c031 (F3 Held drop order), 4a5fd290 (F4 derived bounds), 07b24708 (F5 VmHWM tests).
  Rule 3 reworded by architect efbf2938, folded e75ed558. Round 2 (07b24708) posted: F6 = that rule (done), F7 P3 tests
  claimed tighter bound -> 7f51ca08 (counting reader exact; line test <16 MiB). Head 7f51ca08: ci11 + round 3 running.
  Round 3 clean on 7f51ca08 (posted); ci11 xtask OK; CI green. Asked architect-cto for AUTHORIZES #495@7f51ca08
  (REQUEST 01a17f3e); given 01a17f3f; #495 MERGED cb632c4c. j73/j74/j75 closed.
- Quota batch: branch develop-qzapp/rust-ui-dev-02/fix/j68-store-open-above-quota (worktree ../interweave-j73), pushed,
  fd51dedf (open writes above quota) + 7deee00d (route pair indexed at accept; backfill v2); architect fold fcc22724
  (72f58849). Architect ruled (01a17f5b): NO PR yet, carry B2 on this branch; land at ~16 work.
  B2 so far: 8de17d31 schema v11 (kept_inbound_by_kind wide) + 3d5d0e32 shown_conversations back, in the covering test.
  = 5 work + architect's fcc22724. NEXT: rest of B2 (ui-model windows/edges/gaps/ReadNotShown, app-core FacadeSide::start,
  Command::Page, Update Started/Paged/PageFailed/Counts/ContactPhoto, contacts w/o photos via contact_directory_page,
  labels PLACEHOLDER + lc request, start_memory test) - plan it against the tree first (see j68-batch-b-plan).
  At PR time: STATE.md v11 row says "not yet the build's" - architect's to past-tense. (was: NEXT after merge: open the j73+j74 PR from fix/j68-android-facade-past-stop (d4a2249a): fold main, full suite, blind
  review, arm. Then the quota batch (open writes above quota + F6 route indexing, backfill v2), then B2 (v11 + Chats read).
- j75 told architect-cto two STATE.md v10 phrases go stale once #492 lands (theirs to fix).

Carried from #477 as P3: F11 e2e "none leased" check after refused --endpoint cannot fail; F12 README `failed` line.

- 2026-10-28: the quota batch + B2a landed as PR #498 (branch fix/j68-store-open-above-quota, head 88ac94b6 after
  folding main): fd51dedf 7deee00d 8de17d31 3d5d0e32 eec75cd5 d1aa52d6 be52b2fd bb6843cb 481987ba 678286b7 3d26f9be +
  architect fcc22724 834229c7. ci12 suite + blind review running. Next batch (B2b) = C5-C11 per j68-b2-plan, on a new
  branch after #498 merges.
- #498 review round 1 (88ac94b6): F1 P2 (card counted twice; split card_received/card_arrived c3defb81 -
  app-core's drained cards must call card_arrived in B2b!), F2 3c0cfb65, F3 db67c251, F4+F5 23fe8092. Round 2 + ci13
  running. Asked architect REQUEST 01a18104 to make STATE.md "at or over" -> "over" (fold onto #498 before arming).
  architect's e6fd725e (for/rust-ui-dev-02/quota-over-not-at) to fold after ci13 ends.
- #498 ARMED 2026-10-28 ~06:35Z at 7833e45d (15 work, 4 fix) after 4 review rounds; architect folds e6fd725e, 4ed113ff.
  NEXT after merge: B2b (C5-C11 of j68-b2-plan) on a new branch off main; first the MUSTs noted in j68-b2-plan.
- #498 MERGED 3fe09e2b (2026-10-28). B2b branch develop-qzapp/rust-ui-dev-02/feat/j68-start-window-b2b off 3fe09e2b
  in worktree ../interweave-j73; nothing committed yet. Plan: C5 window.rs (model windows/edges/marks), C6 session
  bound + room=min(session, read-proposal), C7 held-row ceiling/gaps, C8 contacts directory/photos, C9 one contact per
  proposal, C10 app-core protocol + FacadeSide::start (+ desktop/cli command_name arms), C11 window commands; MUSTs in
  j68-b2-plan.

*Observed 2026-10-27 (rust-ui-dev)*

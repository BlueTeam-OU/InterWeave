---
role: "architect-cto"
class: threads
topic: "external-review-2026-10-27-main-1b37e0ef"
description: "The owner's third external review of the InterWeave client (main 1b37e0ef, 2026-10-27): P2 Android facade past its stop, P3-1 channel badge distinct count (my own STATE.md sentence), P3-2 human-cli stdin, P3-3 contact collation…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 39dc593770ba89e5
  - 7e4efceab6cea1e0
---

## The owner's third external review of the InterWeave client (main 1b37e0ef, 2026-10-27): P2 Android facade past its stop, P3-1 channel badge distinct count (my own STATE.md sentence), P3-2 human-cli stdin, P3-3 contact collation — verdicts, rulings, where each record and lane is

j432. Four findings judged by four parallel code-review dispatches against main 1b37e0ef (no PR; claim verbatim, repository path, questions), then ruled and routed the same day.

[… 9 earlier entries of this thread moved to external-review-2026-10-27-main-1b37e0ef-log.md on 2026-10-28, not drained …]
**j435 (REQUEST 01a17ebd-b402, 19:02Z):** #492 review F2/F3 + two pre-existing sentences. Ruled: v11 = kept_inbound_by_kind (kind, source_peer, source_endpoint, channel_id), index-only, built in B2 (fba63c05); quota rule = the open's own writes run above --store-max-pages, re-applied after, degraded if still over, migration failure stays recovery (81091c7d), NOT their degrade-and-retry; conversation_index keyed {peer, endpoint} (local endpoint at Stage 22), an indexed endpoint forgotten by discard alone (e3a6e7b8). Branch for/rust-ui-dev-02/v11-quota-and-index-sentences from 6717c2fd; REPLY 01a17ec1-e20c-744e-bdfc-b2df53fa42bb.
**19:15Z (QUESTION 01a17ec6-9700, #492 r3 F6):** a route-only endpoint (upgraded store) is forgotten with its route; ruled: build the index-on-accept + backfill v2 in the quota batch, and the sentence qualified now, 39d51c3d on the same for/ branch (REPLY 01a17ec7-4b47-7a1f-80c8-1ab3b4bb4191).
**19:31Z:** #494 (5 work, 4 fix, boundary) word GIVEN at 9c5a9305 (REPLY 01a17ed7-bc2e; the first send was refused by a presence answer during their 19:30 relaunch, the retry went): root-only provider, ~2.3 pins, sizes APK +280,080 / Linux archive +711,753 gzip, tailorings kept out (~896 KB).
**20:35Z:** #493 and #494 MERGED: human-client-ui.md s3 "How contacts are ordered" (5d4d3393) and the add-a-peer revision are on main; P3-3 closed. #492 at 8c51e82f (16 work, 3 fix, green, head reviewed) arms on its own count. Open: #492 landing (P3-1 + my STATE.md/HUMAN-CHAT.md supplies), the Android fix PR (P2 + P3-2), the quota/route-index batch, docs/j421 PR.
**21:10Z:** #492 (16 work, 3 fix) MERGED: P3-1 closed, STATE.md/HUMAN-CHAT.md supplies (f91f6c89, 01bd1ddf, e3a6e7b8, fba63c05 v11 row, 81091c7d quota rule, 39d51c3d) on main. #495 = rust-ui-dev-02 fix/j68-android-facade-past-stop (P2 + P3-2), open.
**j437 (20:43Z, REQUEST 01a17f19-dfa3):** #495 review F1/F2 (P2): unlisting dropped the newer facade's in-flight answer; a listing before the late end paid the debt stale. Hub now counts relists owed (6483d06f). Rule 3 reworded (owes a listing, view stays listed, pre-end listing does not pay): efbf2938 on for/rust-ui-dev-02/facade-rule-3-owed-listing from 07b24708; REPLY 01a17f1a-adaf-7d07-a305-0c7e07f634eb.
**21:25Z:** #495 (6 work, 5 fix, boundary) word GIVEN at 7f51ca08 (REPLY 01a17f3f-9d1e-733b-8b5e-58cbbad4e37f): P2 + P3-2 land with it; my 398e803a and efbf2938 folded.
**j440 (21:53Z, REQUEST 01a17f5a-4230):** quota batch built on fix/j68-store-open-above-quota (fd51dedf open writes above quota; 7deee00d route pair indexed on accept + index v2 backfill). My three owed clauses past tense: fcc22724 on for/rust-ui-dev-02/quota-and-route-index-built. Ruled: no PR under the floor now; carry B2 on the same branch (same lane and files, no consumer waiting). REPLY 01a17f5b-4f0b-7f37-9c91-f76fc0479c17.
**22:00Z:** #495 (6 work, 5 fix) MERGED (cb632c4c): P2 and P3-2 closed; all four findings of the review are on main except the quota/route-index clauses (fcc22724) riding B2. Open: B2 PR (carries fd51dedf, 7deee00d, fcc22724), my docs/j421 batch (f0fd2666).
**j432 B2 (04:40Z, QUESTION 01a180cb-ba04):** ruled (1) page room = min(session room, read-proposal room), caps stay 256, invariant in design 7.1 with an eviction test; (2) the two command_name arms ride app-core's commit; (3) v11 said built at 834229c7 on for/rust-ui-dev-02/v11-built from 3d5d0e32. REPLY 01a180cc-d1ef-70a9-9b2c-5f983dc876d3.
**j443 (05:38Z, REQUEST 01a18104-84ef):** #498 (13 work, 4 fix) = rust-ui-dev-02 B2 + quota + route index. Review F4: STATE.md said "at or over" degrades; code degrades only over (at = healthy). Ruled "over" with the reason; e6fd725e on for/rust-ui-dev-02/quota-over-not-at from 3c0cfb65; REPLY 01a18105-9754-76f1-b931-01b955af2e6c.
**06:30Z (QUESTION 01a18131-699e, #498 r3 F6):** my e6fd725e clause order read the healthy store as "no new row committed"; reordered 4ed113ff on for/rust-ui-dev-02/quota-clause-order from 35ad3b4f, to fold before the arm. REPLY 01a18132-3311-7397-a2a1-90a65516caae.
**07:40Z:** #498 (15 work, 4 fix) MERGED: quota rule, route index, v11, B2 and my supplies (fcc22724, 834229c7, e6fd725e, 4ed113ff) on main. Every finding of the review is on main. The plan bullets on docs/j421 rewritten past tense naming #492/#494/#495; the docs batch (6 work, no PR) still gathers j421 items. j432 waits on that PR alone.
**11:00Z:** #500 (8 work, 4 fix) ARMED at 3a181f5c at the minimum (the digest 0055 Rules/Security correction made the 8th work commit); six review rounds; the owner was told. Lesson: after a fold of main, a full review from the merge-base (round 4) found a figure the earlier rounds had passed (the 1,310,720 literal replaced by 1,245,184 in the same PR it cited).
**11:40Z:** #500 (8 work, 4 fix) MERGED: ADR-0055 (iii), ADR-0052 (iv), the plan bullets of both client reviews, the Stage 22 note. j421/j432 close.

*Observed 2026-10-28 (architect-cto)*

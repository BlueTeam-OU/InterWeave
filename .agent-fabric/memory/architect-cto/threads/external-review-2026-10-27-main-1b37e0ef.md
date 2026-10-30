---
role: "architect-cto"
class: threads
topic: "external-review-2026-10-27-main-1b37e0ef"
description: "The owner's third external review of the InterWeave client (main 1b37e0ef, 2026-10-27): P2 Android facade past its stop, P3-1 channel badge distinct count (my own STATE.md sentence), P3-2 human-cli stdin, P3-3 contact collation…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 39dc593770ba89e5
---

## The owner's third external review of the InterWeave client (main 1b37e0ef, 2026-10-27): P2 Android facade past its stop, P3-1 channel badge distinct count (my own STATE.md sentence), P3-2 human-cli stdin, P3-3 contact collation — verdicts, rulings, where each record and lane is

j432. Four findings judged by four parallel code-review dispatches against main 1b37e0ef (no PR; claim verbatim, repository path, questions), then ruled and routed the same day.

- **P2 (partly holds)** facade.rs close() leaves the thread past FACADE_CLOSE (2 s); wants_listing/listed/update carry no generation. Gap the finding missed: ServiceHost::stop never calls withdraw/stopped, so the old facade stays current until its own closing() or the next started(); store has no busy timeout; discard_store checks only running.is_some(). Rule: human-client-android.md "A facade past its stop" (398e803a on for/rust-ui-dev-02/android-facade-past-stop): stop retires the generation before returning; every facade→hub op carries one; a retired facade finishes accepted commands against the store, publishes nothing, its Received still counts a notice, its late end marks the newer view unlisted; discard refuses while a facade thread lives; no wait for a lingering facade. REQUEST 01a17e8a-1c0d to rust-ui-dev-02.
- **P3-1 (partly holds, MY defect)** COUNT(DISTINCT app_message_id) per channel merges two publishers' reuse of one id; model keys a channel item by publisher (model.rs:1619), read_pairs says two pairs; the kept-item reason I gave in A3 is unreachable by a count over unread_inbound alone. unread_counts has no caller yet. STATE.md corrected f91f6c89 on for/rust-ui-dev-02/start-window-b1-records (A 2026-10-27 (ii), rows one per row); query+test in batch B. REQUEST 01a17e8a-1ea5.
- **P3-2 (holds)** send - read_to_string unbounded before the session; listen --input read_until no ceiling, unbounded_channel, unbounded VecDeque. Shape ruled in the REQUEST (01a17e8a-1d53 to rust-ui-dev-02): take(N+1) as admin.rs peers add -; raw line ceiling as the bridge's HostLines, drained + input-refused ref null; bounded channel with blocking_send; ceilings derived from MAX_DECOMPRESSED_BYTES (196,608) with const asserts; record = README. The chat text has no largest-size constant (envelope ≤196,608 compressing to ≤49,152).
- **P3-3 (partly holds)** no collation anywhere; four sites order four ways (view lowercase+contact_id tie; contact_names_on lowercase; store/model/CLI byte order; standing queries byte order); design §6.2 tie is the smallest route, not contact_id; add-a-peer Q-collation was never answered. Rule: human-client-ui.md §3 "How contacts are ordered" (5d4d3393 on for/rust-ui-dev-01/contacts-order): ICU4X icu_collator (family already in the graph under Unicode-3.0, allowed), root order until a locale, one ui-model function, pages of that order. REQUEST 01a17e8a-2016 to rust-ui-dev-01 (measure APK/archive size delta); DECISION 01a17e8a-2164 to ux-designer-01 to revise add-a-peer §5.2/§11.
- Plan §20/§29 carried bullets: f0fd2666 on docs/j421 (still no PR).

**How to apply:** if the review is pasted again, answer from these four branches and `git merge-base --is-ancestor`; nothing else is owed by me. Related: [[external-review-2026-10-20-main-a4538593]], [[owner-four-client-findings-j68-2026-10-27]].

**18:08Z:** rust-ui-dev-01 took P3-3 as j175, branch develop-qzapp/rust-ui-dev-01/feat/j175-contacts-collation folding 5d4d3393 (REPLY 01a17e8c-afa8); PR number to follow. origin/main moved to 32ade966 (v0.2.0, #487) after the judgement; none of the ruled files changed but the human-cli README --version line.

**18:09Z, rust-ui-dev-02 (REPLYs 01a17e8c-de62/-e495/-ea72):** P3-1 on #492 (feat/j68-start-window-b, f91f6c89 folded, COUNT(*) both groups, test asserts 2); P2 and P3-2 together on fix/j68-android-facade-past-stop, cut from main, opened as its own PR after #492 merges (one open PR per agent; 398e803a folded; no device run, said in the body; refs map growth answered with a test either way). Nothing asked of me.

**18:41Z (REPLY 01a17eab-1008):** P2 built on rust-ui-dev-02 fix/j68-android-facade-past-stop head 5032e717 (398e803a folded as 4e04c36a): Hub::retire from close() after the grace, generation on wants_listing/listed/update, Hub::ended via a drop guard relists the newer view, busy timeout set by the store (5c735753), discard refused while a facade thread lives; 11 mutations caught; service-level test with the real embedded runtime; no device run. PR opens after #492.

**18:45Z:** P3-1 done on #492 at 62d11cb5 (COUNT(*) both groups; f91f6c89 folded as b9e2eb00 with one conflict resolved by them). Two phrases of mine made past tense: 01bd1ddf on for/rust-ui-dev-02/state-badge-count-landed (cut from 62d11cb5), REPLY 01a17ead-eb08. P3-2 built at 5032e717 (send - 256 KiB; line ceiling 1,310,720 = 6x196,608+64 KiB; channel of 4; blocking_send); refs map ruled (a): ref capped at 256 bytes serialised, longer refused input-refused ref null; (b) is the later product rule (REPLY 01a17ead-19cc). ux-designer-01 #493 (1 work, 3 fix, clean at ca22809c) asked my word: WITHHELD because the note cites human-client-ui.md s3 not on main; asked them to fold 5d4d3393 unrebased, re-review the fold, ask again (REPLY 01a17ead-26fe); word on the new head on the ask.

**18:46Z (REPLY 01a17eae-e213):** ref cap built d4a2249a on the fix branch (256 bytes serialised, 257 refused); 01bd1ddf folds onto #492 after its running suite on 62d11cb5, then suite + re-review before any arming. Nothing asked of me.

**18:48Z:** #493 (2 work, 3 fix) word GIVEN at 82637f90 (merge of 5d4d3393 only; blind review on the head; 0 threads): AUTHORIZES in REPLY 01a17eb0-c76b-7190-b7e3-6a26e8cb2ea9. human-client-ui.md s3 lands on main with #493; rust-ui-dev-01 j175 folds the same 5d4d3393 (no-op once merged).

**19:01Z (REPLY 01a17ebd-1b83):** P3-3 = PR #494, branch feat/j175-p3-3-contacts-ordered-by-unicode (the earlier branch name was wrong), 5d4d3393 folded; ui-model order.rs contact_order, model holds contacts in order, view no longer sorts, store queries order by contact_id; icu_collator/provider/normalizer ~2.3 (a bare 2.3 pin moved icu_* to 2.4), cargo deny clean, root data only (tailoring markers refused by a provider, tested); paging agreed with rust-ui-dev-02 (batch B reads every page, model sorts whole); size deltas owed in the PR body; under the minimum, word asked after review.

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

*References: external-review-2026-10-20-main-a4538593, owner-four-client-findings-j68-2026-10-27*

*Observed 2026-10-28 (architect-cto)*

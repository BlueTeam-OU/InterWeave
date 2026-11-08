---
role: "architect-cto"
class: threads
topic: "owner-four-client-findings-j68-2026-10-27"
description: "owner's four human-client findings (2026-10-27) judged by rust-ui-dev-02 as j68 — placement in plan records, store v10 ownership, paged start listing vs RETENTION; I owe §29/§20 bullets + STATE.md contacts row on a for/ branch"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5923c3ebbbabfd15
  - f4744159793b906c
---

## owner's four human-client findings (2026-10-27) judged by rust-ui-dev-02 as j68 — placement in plan records, store v10 ownership, paged start listing vs RETENTION; I owe §29/§20 bullets + STATE.md contacts row on a for/ branch

j406 (QUESTION 01a17bcc, answered by REPLY 2026-10-27). The owner handed four findings via p2p-network-dev-01; the review class judged them at f2295901: P2-1 start listing is one Update of every row + contacts, O(K×C) card standing, memory exhaustion by a trusted peer's backlog (holds); P2-2 a command accepted while the Android facade loop closes is dropped with no update (partly holds); P3-1 photo read has no deadline/cancel; P3-2 late photo pick replaces a newer one. Order: P2-2, P3-2+P3-1, P2-1.
**Ruled:** P2-1/P3-1/P3-2 → Stage 21 §29 carried list; P2-2 → Stage 17 §20 carried list; bullets are mine, supplied on a for/ branch onto the j68 branch, each naming the fix commit. Store v10 (name-key column + backfill) is rust-ui-dev's (ADR-0060, STATE.md Migrations); STATE.md's contacts row is mine, supplied with the bullets once the column is named; pointed them at design 5.4.1 same-name three-keys gap ([[same-name-three-keys-gap-2026-10-25]]). Paged listing: consistent with RETENTION.md/HUMAN-CHAT.md (read state begins at presentation) under four constraints: badge/notification count all unread (count content-free); retries read the store not the window; an unshown proposal survives exit; Keep/Dismiss/Cancel/Undo unchanged in the window. Window order = ux-designer-01's.
**Owed by me:** the bullets + STATE.md row when the j68 branch and column are named.

[… 4 earlier entries of this thread moved to owner-four-client-findings-j68-2026-10-27-log.md on 2026-10-28, not drained …]
**A5 (14:50Z):** "distinct" = among conversation_index rows for the PeerId (durable, store-bounded, no memory set; forgotten only when the conversation leaves the index via delete/discard); the check in transport-client's commit path before the store. Text ef435647 (second commit on the same for/ branch). Lesson: a bound "per peer" needs its lifetime named in the same sentence (store-indexed vs seen-this-session).
**15:00Z:** added the v10 index as contacts (local_endpoint_id, name_key) — 86708be2, third commit on the for/ branch (their migration names it contacts_by_name_key). rust-ui-dev-02 folded 92f56bed + ef435647 into feat/j68-start-window.
**15:05Z:** corrected: v10 index = contacts (name_key) (ad1d2f78, 4th commit); local_endpoint_id on contacts is Stage 22's, which widens the index then.
**15:30Z:** the 65th-endpoint rule widened: distinct = union of conversation_index + contact_routes rows per PeerId (20afc55a), because a route outlives its conversation and #488 §11 bounds same-device lines at 63 on it.
**15:35Z:** #489 (14 work, 0 fix) = batch A (schema v10 as STATE.md reads, versions recomputed at open, card standing by three indexed reads, conversation_page + unread_counts, hidden_inbound, contact directory without photos); my five commits folded. The 65th-endpoint CHECK is batch B → added 8abb329a (6th commit) so the contract sentence names its build batch (an owed clause is false the moment the PR merges). list_all still runs at start until batch B.
**15:45Z, #489 review:** F1 P2 — unread_counts/hidden_inbound read every row's payload under SQLCipher (kind after payload; 10-17x slower measured). Added covering indexes unread_inbound (kind, source_peer, source_endpoint_key, channel_key, app_message_id) and kept_inbound (kind), and corrected my paging-index text: the inbound tables have NO conversation_id column — keys are (source_peer, source_endpoint_key, …) and (channel_key, …), the _key columns generated non-null forms. 6485eba1, 7th for/ commit. Lesson: name index columns from schema.rs, not from STATE.md's row list (which named conversation_id for unread_inbound — a STATE.md inaccuracy to fix later: its unread/kept rows list conversation_id).
**15:50Z:** rust-ui-dev-02's own ask was wrong: the covering index must be over source_endpoint, channel_id (stored), not the VIRTUAL _key forms (SQLite recomputes them → USING INDEX, rows re-read). Eighth commit on the for/ branch.
**16:00Z, #489 round 2 F2 partly:** HUMAN-CHAT pointed at plan §29 text not on main (my unmerged docs/j421); STATE.md's at-most-64 sentence read as built. Fixed: cherry-picked be49fbe8 onto the for/ branch (00516e52) and c45bbbbb (STATE.md clause + §29 bullet rewritten: schema batch #489 first, window+check second). Lesson: a contract sentence may point only at record text that exists at the PR's head — carry the record onto the same branch.
**17:25Z, batch B (j430):** #489 MERGED 24445504 (batch A + my eight commits incl. the §20/§29 plan bullets via cherry-pick). Found: conversation_index was written by mark_card_shared ONLY, so the per-peer bound counted ~nothing → ruled: every direct commit indexes its conversation (INSERT OR IGNORE) + open-time backfill (conversation_index_version); own sends count and a 65th own send is refused before its row; endpoint_bound reason, metric only; failed store read → dropped_unstored; CLI streams pages; peak-allocation test = #[ignore] 20k + CI 2k (devex); RETENTION clause supplied; no pending ceiling. Records a662454c on for/rust-ui-dev-02/start-window-b1-records (from main). B = four PRs B1-B4.

**2026-10-30 ~04:55Z, B3 (rust-ui-dev-02 QUESTION 01a18b25):** A1 ruled: test-only cargo feature in the tree's `test-hooks` shape (empty feature, enabled only by dev-dependencies, cfg-gated code, never default/shipped; the PR says which §10 items 7/11/12/14/15 are proven headless, none over AT-SPI, and how the release feature set was read). A2 ruled: show the §3 preview cut and §6.1 RowsNotShown on Android's listed mode now (correct in either mode), the Pixel run named as not done, B4 carries it. Four B2b window-model fixes, one commit + test each: fine. REPLY sent.

*References: same-name-three-keys-gap-2026-10-25*

*Observed 2026-10-30 (architect-cto)*

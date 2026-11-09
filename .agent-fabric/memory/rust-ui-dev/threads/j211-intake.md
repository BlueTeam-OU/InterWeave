---
role: "rust-ui-dev"
class: threads
topic: "j211-intake"
description: "DONE (merged d834ecf21) j211 (architect-cto REQUEST 01a1a689): PR #556 with TM-56 ui-slint test, TM-45 desktop log e2e, window cost at 20k rows; Android TM-45 a named limit; waits on review + architect for/ rows"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 426777821f673194
---

## DONE (merged d834ecf21) j211 (architect-cto REQUEST 01a1a689): PR #556 with TM-56 ui-slint test, TM-45 desktop log e2e, window cost at 20k rows; Android TM-45 a named limit; waits on review + architect for/ rows

j211, taken 2026-11-04, against origin/main b563dd00c. PR #556 (3 work commits, d136d5f4e / fe6146081 / a4745ac30), REPLY 01a1a6e2 to architect-cto with the paths and the figure.
1. TM-56: crates/human/ui-slint/tests/cards.rs::two_same_name_contacts_are_drawn_told_apart_by_their_short_peer_ids.
2. TM-45 desktop: tests/desktop-e2e/tests/human_app/content_free_logs.rs (shipped binary, planted text/card/pending, stderr checked). Android: LIMIT, its log and notification code is Android-only (activity/jni/onboard/store.rs, NetworkService.kt).
3. crates/human/app-core/tests/window_cost.rs: counting GlobalAlloc (allow(unsafe_code) with reason, as jni.rs) PLUS sqlite3_memory_used/highwater, since bundled SQLite mallocs past a Rust global allocator (review F1). 20k release, Rust+SQLite: start 31.9 ms 0.02+3.53 MiB (SQLite grows with backlog: page cache from index-scan counts, no cache_size set), first page 2.4 ms 0.23+0.02, walk stops at HELD_ROW_CAP 1024. Correction sent REPLY 01a1a6f7.
Non-obvious: "#489's peak-allocation test" does not exist (#489 has a query-count test); seeding 20k rows takes about 74 s (one fsync per commit).
Reviews: r1 F1-F3 fixed (9037d277); fold of architect-cto's for/ supply (REPLY 01a1a727) reviewed, F4 (Cargo comment, mine) and F5 (TM-56 'draws' overclaim, architect-cto's rewording e19c8a265) fixed; head 33198158 (7 work, 5 fix) reviewed clean and posted. REPLY 01a1a752 asks architect-cto's AUTHORIZES for 33198158. Armed on AUTHORIZES 01a1a753; MERGED d834ecf21 2026-11-04T16:53Z. DONE. Left by architect-cto's choice: test name keeps 'drawn' (rename with its matrix path in a later batch).
Related: [[human-store-realistic-size]].

*References: human-store-realistic-size*

*Observed 2026-11-04 (rust-ui-dev)*

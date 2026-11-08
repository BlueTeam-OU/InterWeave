---
role: "rust-ui-dev"
class: threads
topic: "j68-b4c-plan"
description: "j68 B4c (2026-10-30 code-plan): CLI on the windowed start via a two-pass walk, then listed-mode removal; commit list, design facts, questions out to architect-cto"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d93717c384a9046c
---

## j68 B4c (2026-10-30 code-plan): CLI on the windowed start via a two-pass walk, then listed-mode removal; commit list, design facts, questions out to architect-cto

Branch develop-qzapp/rust-ui-dev-02/feat/j68-b4c-cli-windowed off main 13256d30 (#517 merged), worktree
../interweave-j68-four-human-client-findings-forwarded-as-2. 8e5d494e = AndroidSurface comment (carried nit).

Design facts (verified by the planner on 13256d30):
- A row of a conversation not open is never held (received/card_arrived return) -> listen lines must be built
  from the update itself (needs ui-model message_of / proposal_item).
- Window input goes through ModelSide::window(input) then step(); one page per step.
- let_go/drop_items do not add to `released`: a gap page re-holds rows the ceiling dropped -> print by a cursor.
- Edge::Top reappears after the ceiling moves the top down; a re-open resets the window -> track window_opening seq.
- MarkRead does not need the row held. Pending rows always held (can sit inside a gap span -> barrier).

Commits phase A: A1 ui-model message_of/proposal_item; A2 ui-model DrawnAt + pub drawn + window_gaps(GapSpan) +
window_opening; A3 human-cli walk.rs (Up: Place oldest, Left+Reached Top, ShowAnyway at AtLimit, fail on Failed;
Down: cursor + barrier = smallest gap.above with below>cursor, chunks <=64, Place last printed, reach gap upper
side, re-read gaps each round; `late` for mid-walk rows at/below cursor) + tests T1 (cap+300, ties, kept, pending,
card), T1b (bounds 8/8/4), T2 first pass prints/marks nothing (page fault after 2nd page), T3 mid-walk row last,
T4 ShowAnyway at session bound, T5 reopen mid-walk; A4 listen lines from the update; A5 messages/read on the walk
(read chunked: mark, keep, print); A6 proposals pass 1; A7 unkeep walks; A8 switch Session::new to start(),
counts from summaries, contacts-unread refusal, listen --consume drain on walks, relist on SummariesListed, README.
Phase B (after A8): B1 android-platform hub tests to Started; B2 android-e2e-cases drop Listed arm; B3 desktop-e2e
codes.rs; B4/B5 app-core tests root.rs/cards.rs (Root::open helper); B6 app-core remove Listing/listing()/list_all/
Update::{Listed,UnreadListed,ContactsListed}/windowed flag; B7 ui-model prose; B8 ui-slint comments.
Keep list_unread (proposals_again uses it). unread_listed/kept_listed/contacts_listed stay (page_arrived uses them).

Questions sent to architect-cto-01 2026-10-30 (QUESTION): Q1 status kept (propose count in start), Q2 unkeep walk,
Q3 proposals walk, Q4 reply_to unavailable past ceiling, Q5 read chunked, Q6 stream output, Q8 fail page at once;
proposals_again cost as its own job. Build to the defaults; A8 last. Send architect the B4c PR number when it opens.
RULED by architect-cto 01a18ecf (2026-10-30): Q1 kept = store count of shown-kind kept rows, one query at start and relist (hidden kept stay in undisplayable). Q2 unkeep walks pass 1, stops at first hold, today's refusal wording; TEST: unkeep of a kept row older than HELD_ROW_CAP succeeds and marks nothing read. Q3 proposals pass 1 over unread>0 accepted; card count in StoredSummary = later job after measuring vs SPIKES read-walk baseline. Q4 reply_to unavailable accepted; README names both new cases; never a fabricated id. Q5 chunked read on 3 conditions each a test: (1) all --keep ids checked by pass 1 before chunk 1 reads; (2) each chunk printed+flushed before next is marked read; (3) mid-walk failure exit code says stopped, README: later rows stay unread, second read continues. Chunk constant comment names the facade's read-copy bound. Q6 stream to stdout chunk by chunk; json one object per line; pre-read checks before first byte. Q8 Failure::Store at once naming the conversation; --wait never bounds the walk. proposals_again: keep renamed, open a job after B4c lands. Switch commit last; send PR number (architect j458 waits). ux-designer 01a18ecd: reply present = held or stored; store check not in B4c -> carried gap, open as a job (store index + window read + model cache). Device record 05a003c7 committed (rust-ui-dev-01 j179 both phones pass at 79d72395). A1 in progress (message_of/proposal_item, uncommitted).
A2 committed: drawn_order/DrawnAt, window_gaps/GapSpan, window_opening.
A3 committed: walk.rs + walk_tests (6 tests, 7 mutants killed); late = every mid-walk arrival by origin (ceiling may let it go at once). Next A4 listen lines from the update.
A4 committed 3c8edf43: listen lines + filter conversation from the update; test listen_prints_a_message_and_a_card_for_a_conversation_not_open (3 mutants killed; filter fails OPEN on an unknown conversation, so the control is a filter naming another). Next A5 messages/read on the walk.
ux-designer-01 INFO 01a18f10 (2026-10-30): #518 merged 7631498a; reply line rules (Dismiss-hidden = not held; stored = this conversation unread/kept; no announce/focus); item 17(a) runnable on B4c (Dismiss/Undo/snackbar, release in view, desktop return via a11y tree); 17(b) waits architect j462.
A5 committed: befa22d1 ui-model items_viewed (2 mutants); 2f1b08d0 human-cli reading.rs (messages/read stream via verbs::stream, main routes them to stdout; run buffers) + 4 tests in walk_tests (Q5 1-3 + messages), 5 mutants killed; README bullet. Late rows in read printed unread. Next A6 proposals pass 1 / proposals read.
A6 8905af06 (proposals/proposals read pass 1 via Walk::up_to_the_top/close; card mid-conversation test). A7 committed (unkeep walked_to, test in walk_tests). Next A8 switch (start(), status counts from summaries + Q1 kept store count = app-core/store work, contacts-unread refusal, listen --consume drain on walks, relist on SummariesListed, README Q4 reply_to, e2e human_cli).
A8 CLI switch committed d2937263: store 9a08d821, app-core ed9f10f5 + hook fa85fe56. Next: rebuild binary, e2e human_cli.rs, then phase B.
PR #519 opened 2026-10-31 (head 06ecd286 after folding main; 14 work). Phase A only; phase B (listed-mode removal) = next batch B4d. CLI e2e 13/13, human crates suite + guard + android check green. Blind review round 1 dispatched (brief scratchpad/review-b4c.yaml). PR number sent to architect-cto-01 (REPLY 01a18f4c). After merge: open proposals_again job, reply-target lookup is architect j462, A40 device row to j77.
#519 r1 (06ecd286): F1 P2 read's own steps dropped arrivals -> e5a73d11 Walk::take_arrivals (test read_prints_a_row_arriving_during_its_own_step_last_and_unread, multi_thread + block_in_place send in flush); F2 3cec019f, F3 d9fa8d38 doc moves. R1 (keep by ItemKey) judged real but unreachable: store refuses re-commit of a read id (AlreadyRead) -> no change, carried. Next batch owes: reading.rs module doc "a failure leaves no row read and unwritten" says store failure not stdout (R2). r2 re-review dispatched on d9fa8d38.
#519 r2 (d9fa8d38): F1-F3 fixed; F4 P3 doc -> 8c9313d1; r2 risk1 (first take untested) -> 3f899710 test keeps u100. r3 re-review dispatched (brief from review-b4c-rr2.json; next needs the previous round's request).
#519 r3 clean on 3f899710; ARMED 2026-10-31 (14 work, 5 fix). wait-merged running. After merge: proposals_again job, phase B (B4d) branch off new main, reading.rs module-doc stdout wording, R1 keep-by-row carried (unreachable: AlreadyRead).
#519 MERGED a32d03b4. j78 opened (proposals_again). A40 row noted in desktop-paging-relayout-cost. Next: B4d off a32d03b4.
B4d PR #520 opened (8 work: 008234c7 0caab6cb a1d214fc ac8a912c 91b88b6b db403de1 6254f757 d2733339); blind review r1 dispatched (review-b4d.yaml). INFO 01a18fa1 to architect-cto-01: list_all removed, SPIKES/plan cites theirs.
#520 r1: F1 F2 P3 docs fixed (8c6ec41b, 7cb13d7d); r2 clean + risk (listed wording class) fixed 99d9bf83 a29d64bc ee3fa66a afcc4c48; r3 dispatched on afcc4c48.
#520 r3 clean on afcc4c48; ARMED (8 work, 6 fix). Carried pre-existing: listing.rs test name a_store_classified_by_no_client_is_classified_at_the_first_listing + its doc claims next start leaves them alone without asserting; facade.rs:1501 delete_contact doc 'listed again'. After merge: j68 done? check job items, then fabric-jobs switch.
j68 COMPLETE 2026-10-31: findings -> #483 (Android commands, photo picks), P2-1 start window #489..#520. Switch to next job.

*Observed 2026-10-30 (rust-ui-dev)*

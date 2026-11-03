---
role: "rust-ui-dev"
class: threads
topic: "j68-b4-plan"
description: "j68 B4 plan (2026-10-30 code-plan) - three PRs B4a/B4b/B4c, commit lists, the listed-mode removal decision, questions out"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9a09dadc0be801a0
---

## j68 B4 plan (2026-10-30 code-plan) - three PRs B4a/B4b/B4c, commit lists, the listed-mode removal decision, questions out

Branch develop-qzapp/rust-ui-dev-02/feat/j68-b4-windowed-everywhere (worktree ../interweave-j68-four-human-client-findings-forwarded-as-2),
7ca62b5a = Contacts title heading (done, counts in B4a).
B4a (test hooks + headless failure paths + window fixes, ~14): A1 ui-model one limit row on the side that fetched
(gap_settled must not reach the other side into AtLimit); A2 Left on Wanted -> Armed; A3 card_marks removed on
resolve/release/let_go/drop (+count test); A4 ui-model Bounds{session_items,held_rows,read_proposals} field defaulting
to the consts + `test-hooks = []` with cfg'd set_bounds_for_test (self dev-dep); A5 app-core test-hooks forwarding
ui-model's, cfg'd StoreFaults (page/contact read Pass/Fail/Hold via Condvar released on drop, photo delay, unreadable
ids), FacadeSide::with_store_faults, ModelSide::set_bounds_for_test, tests in app-core tests/window.rs; A6 ui-slint
clear app_focus when focus leaves its target; A7 Connect closes the window (ONLY if ux says yes); A8 test: contacts-unread
proposal menu/actions offer no Accept; A9-A14 human-desktop headless.rs with app-core test-hooks dev-dep: items 3b+7,
11 failure path, 12 small session bound, 14(a)(c), 14(b)(d)(e), 15 slowed photos. PR shows `cargo tree -e features
--edges normal -p interweave-human-desktop` has no test-hooks; says workspace test builds unify the feature into the
e2e binary; items headless-only, none over AT-SPI.
B4b (~9): B1-B4 desktop-e2e human_app window.rs (seed_unread in world.rs; items 1,3; 2,9; 13 by seeding undecodable;
5 via a11y grab_focus only if accesskit honours GrabFocus - probe first); B5 ui-model second `started` on a live model
reopens on a missed row (Android relist); B6 android-e2e-cases accept Started + report Opened; B7 AndroidSurface::take_window;
B8 android-platform facade start() + tests; B9 Pixel device-run record (signer issue: rust-ui-dev-01's key or wipe on owner's word).
B4c (~11): C1-C4 human-cli two-pass walk (page to top, then down through gaps, print once all older printed, ShowAnyway
at the limit), start(), listen relist arm SummariesListed; C5 desktop-e2e codes.rs start(); C6 android cases drop Listed;
C7-C8 app-core root.rs/cards.rs fixtures on start() (Root::open helper); C9 remove listing/list_all/list_unread/Listing/
Update::{Listed,UnreadListed,ContactsListed}/facade windowed flag/ModelSide arms; C10 ui-model docs: windowed:None stays
as "the model before a start" (~270 test sites, B5 follow-up names unread_listed/kept_listed/contacts_listed unused).
Questions to send: architect (CLI unread count now includes cards - "output unchanged"?; two-pass walk OK under A5?;
past-tense SPIKES.md:631,672 + close plan P2-1 :6921 after C9); ux (Connect closes window?; marks inside Accept span?);
devex (continuous check no shipped normal-edge graph carries test-hooks); rust-ui-dev-01/owner (Pixel signer).
Rulings 2026-10-30: ux 01a18cfd-7576 (provisional, note PR to follow): Connect + Peers-from-Connect CLOSE the window (A7 in); marks inside Accept span count INSIDE (add to B4a: swipe::in_accept_span knows mark focus keys by place). devex took the test-hooks graph guard as j259 (APK invocation sent). A1 in progress (reach: other side AtLimit -> Armed).
ux INFO 01a18cff: rulings in PR #513 (head 1ea42d03, in review). Item 12 walks every s7.1 page in A's place on a wide window (Connect via a test hook), limit row shows again each return; Details and a contact page from a proposal left with Back: next page loads without asking. Q2 names marks inside span: EarlierUnread, ReadNotShown, gap line, gap failure/progress/limit row + their Try again/EarlierShowAnyway; check: gap failure row after confirmation row, wait >6 s on Try again, Undo still reached.
architect 01a18d00: Q1 store's count (conversations = per-conv unread_counts; status = sum; undisplayable line kept, README says cards counted + hidden apart). Q2 two-pass walk OK IF (a) first pass marks nothing read, prints nothing; (b) test: conv > ceiling prints every row exactly once oldest first; a row committed mid-walk prints after the walk's last row. SPIKES/plan edits theirs after B4c merge: SEND B4c PR number when it opens. B4a so far: 7ca62b5a, 89ce5f2f A1, 7fca8688 A3 (A2 decided out: deferred press must survive Left).
B4a: 90cf4a40 A4 ui-model Bounds + test-hooks (set_bounds_for_test; Cards.cap; graphs measured 0). 4 work + 7ca62b5a = 5 work. Next A5 app-core test-hooks StoreFaults.
B4a: A5 app-core test-hooks committed (StoreFaults, with_store_faults, set_bounds_for_test fwd). 6 work. Next A6 app_focus clear, A7 Connect closes (ux yes), accept-span marks inside (ux), A8 menu test, then A9-A14 desktop headless.
B4a: 73f1d678 A6 app focus once moved; cf096841 A7 Connect closes; <span> marks inside Accept span. 9 work. Next A8 menu test (maybe fold), A9-A14 desktop headless (aim 5-6 commits) -> ~15.
B4a A9 in progress (uncommitted): apps/human-desktop/tests/window_faults.rs (own helpers; dev-deps app-core[test-hooks] + slint), test items 3b+7 PASSES via focus (scroll too slow: held reads pass after HOLD_LIMIT 20 s). Headless: the app's open focus request on the line never lands, so focus_line() steps off and back. Focus after Try again = the line (correct). PERF: one find_all after the Try-again page took 37 s debug -- measuring in release (possible real freeze). ux 01a18d33: #513 merged 830733da; item 12 presses ShowAnyway before each s7.1 page; Connect step needs B4a's hook (architect REPLY 01a18d0b, not seen by me).
B4a commits 2026-10-30 (cont.): 65175c2d Remove photo on an unreadable photo (was a no-op, guard read photo bytes); b612fa7d desktop tests/window_faults.rs (items 3b+7, 11, 12, 14, 15; 6 mutants killed); 3ccb8fc9 ui-model open_connect_for_test; 2df28eac ui-slint walk over Trust/Peers/Contacts/Your card/Connect (+Details keeps). 14 work. No Your identities page in this build. Q2 e2e span check (gap failure after confirmation row, >6 s, Undo) carried to B4b; perf finding is j77. Next: fold main, suite, PR, review.
B4a PR #515 OPEN (head f1a45d36, gate 13 work 0 fix), suite green (14 AT-SPI fails outside display only). Blind review r1 dispatched (review-b4a.yaml in scratchpad, base 50261afb). After it: post-review, fix, arm.
#515 r1 review posted (F1 P2 card marks stale; judged real, newest-message case did NOT reproduce, oldest-card gap did). Fixed f85539da (card_marks_follow at every proposal change; Answers trailer), reply comment posted. Re-review r2 dispatched (brief-b4a-r2.md). Then post-review r2, arm with fabric-pr arm 515 --basis.
#515 r2 re-review posted (F1 fixed, Finding none) and ARMED 2026-10-30 (13 work, 1 fix); wait-merged running. Carry to B4b: r2 risk -- card_restored/holds_card calls seen() for any open window (an Undo of A finishing after B opens could raise B's newest_seen); test or gate seen on open_for.
#515 MERGED c81f0168 2026-10-30 (13 work, 1 fix). B4a DONE. Next: B4b on a fresh branch off origin/main (AT-SPI e2e items 1,2,3,5,9,13 + Q2 span check; ui-model second started; Android windowed; Pixel run) + the carried seen() risk.
B4b branch develop-qzapp/rust-ui-dev-02/feat/j68-b4b-windowed-e2e-android (same worktree), 2026-10-30: ddcd2ab8 seen() gated on conversation (r2 risk fixed); 4b12f60d second started re-bases (B5); da7b3985 AndroidSurface take_window (B7); e8c9995a e2e-cases accept Started + open (B6); 021ace2c android facade start() (B8); e71408b4 hub doc; 6333816f ui-model MessageItem.arrived; 2159932f ui-slint announces only arrivals (REAL DEFECT found by e2e: windowed opening announced "New messages...: 64"); af32231f desktop-e2e window.rs items 1+2, 3+9, 13. 9 work. AT-SPI exports only rows in view: the line at the top is out of view, so item 3's line count is read as A's badge. Left: ux Q2 span check (headless, window_faults.rs), item 5 focus probe, Pixel run (signer), PR.
B4b PR #517 OPEN 2026-10-30 head b1cdc54a (10 work, 0 fix): e2e fmt fixup squashed into 6b9bb568, span test b1cdc54a (needs TWO clock steps: one tick arms the deadline, next closes). Blind review r1 dispatched (review-b4b.yaml, brief-b4b.md). Pixel REQUEST 01a18e66 sent to rust-ui-dev-01 (install + human_chat case). Item 5 stays headless (no keys over AT-SPI).
#517 r1 F1 (P2 stale view reports on Android model swap) + F2 (P3 e2e open order) judged real; fixed 0355f4e4 16dedd62 cfbae505 79d72395 (Answers trailers), pushed, answer comment posted; round 1 saved; re-review r2 dispatched (brief-b4b-r2.md). rust-ui-dev-01 told head 79d72395 + asked a restart-with-conversation-open check.
#517 r2 re-review posted (Finding none), round 2 saved, ARMED 2026-10-30 (10 work, 4 fix) head 79d72395; wait-merged running. Carry to B4c: AndroidSurface doc 'one per attach' -> also per running state (r2 pre-existing nit); host test of ViewSide over a real View (Running+Started one batch -> page asked) if cheap; Pixel/A40 run via rust-ui-dev-01 j179 at 79d72395 then device-run record.
#517 MERGED 13256d30 2026-10-30 (10 work, 4 fix). B4b DONE. B4c: code-plan dispatched for the CLI windowed walk + listed-mode removal; branch to make off origin/main after it.

*Observed 2026-10-30 (rust-ui-dev)*

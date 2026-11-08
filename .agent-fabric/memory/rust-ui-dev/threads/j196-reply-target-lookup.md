---
role: "rust-ui-dev"
class: threads
topic: "j196-reply-target-lookup"
description: "j196 (architect-cto REQUEST 01a1a079): reply-target lookup per start-window §5.2 — store v13 index + reply_target_present + model answers; state and first steps"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ce0f01fc136b204d
---

## j196 (architect-cto REQUEST 01a1a079): reply-target lookup per start-window §5.2 — store v13 index + reply_target_present + model answers; state and first steps

j196, from architect-cto-01 (REQUEST 01a1a079, relay seq 779775, `gzcoord-inbox --replay 779775`), 2026-11-03. Their j462 decided that the lookup is an indexed store read, not an accepted unavailability.

Scope, as one PR of mine:
1. Store schema v13: an index on unread_inbound and kept_inbound over (conversation_id, app_message_id). Add `reply_target_present(conversation_id, app_message_id) -> bool`, which answers from the index alone and never reads the payload. pending_outbound gets no index (a pending target is always held, §3). The same id in another conversation counts as not stored.
2. Model: a reply whose target is not held asks the store once, when the row is built or when the target leaves (released by the ceiling, §5.1; dismissed after the snackbar closes). The answer is ReplyPresent or ReplyUnavailable per §5.2. Dismiss with the snackbar still open stays ReplyUnavailable, and the line flips back when the target loads again.
3. Tests: §10 item 17 (b) is the acceptance. Add a store test that EXPLAIN QUERY PLAN names the index.

References: design/2026-10-27-start-window.md, architecture/clients/human/STATE.md, crates/human/store/src/schema.rs, SPIKES.md, #518, #521. STATE.md's v13 paragraph and Migrations line are architect-cto's, supplied on a for/ branch cut from my head (as for v12, #522).

First steps in the fresh session:
- Cut the branch off origin/main.
- Push it, and send a REPLY naming it. The REPLY also covers their j178 question: feat/j178-android-platform-hub-rs-313-before (415888e4) is patch-equivalent to main's 827d44e9 (via #527), per git cherry `-`. So delete it from origin, which the hook refused in the herdr session, and say so.
- Then fold their supply when it arrives, build, run the full suite, do one blind review, and arm.

Handover 2026-11-03 ~08:25Z: REPLY 01a1a07b sent. It named the branch `feat/j196-reply-target-lookup` and promised an INFO with the head. fabric-jobs switch made the branch develop-qzapp/rust-ui-dev-01/feat/j196-next-job-the-reply-target-lookup in ~/projects/interweave-j196-next-job-the-reply-target-lookup instead. Use that branch: push it, then send the INFO with its real name and head, correcting the REPLY. Also delete the j178 branch from origin.

2026-11-03 session 2: branch pushed, INFO 01a1a07c sent (real name), j178 branch deleted on origin. architect-cto first said no v13 (the UNIQUE key would do). I measured that a channel conversation cannot use it (the key leads with source_peer) and that kind sits after payload. DECISION 01a1a081 chose (a): v13 adds unread/kept_inbound_by_message (app_message_id, channel_id, source_peer, source_endpoint, kind), covering. STATE.md v13 paragraph comes as a for/ commit, develop-qzapp/architect-cto-01/for/rust-ui-dev-01/state-md-v13, with an INFO giving its sha; fold it before the suite and the review.
Model design: per-target record (held replies count, last shown answer, in flight); ask on build with target not held and on drop_items; keep the shown line until the answer comes; hidden holder = Unavailable; new Reply::Stored variant; CLI output.rs maps it.
Built 2026-11-03: c98f38641 store v13, 20f165a04 ui-model (Reply::Stored, model/replies.rs), 3923e0d98 ui-slint, 695bfa6de app-core (WindowCommand::ReadReplyTarget, Update::ReplyTarget), 6dfecac22 cli, adee56ab8 desktop label; fold 2e1d61206 of cb1b1a674. 6 work commits, below the minimum of 8, so the PR needs the owner's word or the arming authority's. Each test was checked with a mutation. Still owed: xtask ci, push, PR, blind review, arm. design §5.2's "Until the store can be asked" interim paragraph goes stale on landing; it is ux-designer's or architect's to update, so flag it in the PR and the REPLY.
PR #535 opened 2026-11-03. Review rounds: r1 found F1-F4 (judged real, fixed); r2 F5 (fixed); r3 F6 (P2, vacuous hidden-guard test), F7, F8, all fixed at d729a8688. Next: xtask ci, push, round-4 full review, then ask architect-cto for AUTHORIZES on that head (below the minimum of 8: 7 work). A re-review render refused ("no review a re-review can start from"), so each round has been a full mode: review, which carries the open findings itself.
2026-11-03 ~12:00Z: #535 head 21792a6e2. Round 5 clean; F1-F9 fixed. Unarmed: 7 work, 10 fix, minimum 8, boundary. Sent REQUEST 01a1a140 to architect-cto for AUTHORIZES (its REFERENCES came out garbled; INFO 01a1a140-f39b carries the evidence). On the answer: `fabric-pr arm 535 --authorization <id> --basis "..."`; a push voids it. j197 queued: the kept_disable flake.
2026-11-03 ~11:55Z: AUTHORIZES 01a1a141 received; #535 armed at 21792a6e2, and wait-merged is running. Do not push to the branch. After the merge: fabric-jobs switch (job j197 is queued).
DONE 2026-11-03: #535 merged 6f302ae8; j196 closed by sweep. Left over: the local j178 branch is patch-equivalent to main (origin copy deleted), and the switch kept it as 'commits off main'; delete it at the next branch sweep. j197 is next: branch develop-qzapp/rust-ui-dev-01/fix/j197-desktop-e2e-kept-disable-flakes-under, worktree ~/projects/interweave-j197-desktop-e2e-kept-disable-flakes-under; fix endpoints_admin in tests/desktop-e2e/tests/human_app/kept_disable.rs to retry BackendUnavailable until the deadline too.

*Observed 2026-11-03 (rust-ui-dev)*

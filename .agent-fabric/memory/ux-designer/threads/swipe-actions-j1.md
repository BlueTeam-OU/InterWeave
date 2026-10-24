---
role: "ux-designer"
class: threads
topic: "swipe-actions-j1"
description: "Status of j1, swipe actions on messages in the Android client — spec merged, build merged; the device review is partial and blocked on the phone link, TalkBack items deferred (IW #297)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 743fe03e57971a21
---

## Status of j1, swipe actions on messages in the Android client — spec merged, build merged; the device review is partial and blocked on the phone link, TalkBack items deferred (IW #297)

Owner's words (2026-10-16, via fabric-coordinator's REQUEST
01a14396-0f46-797c-9675-e9ac03c4b554): "Swipe right -> Keep / Like /
Accept; Swipe left -> Discard / Reject / Skip".

2026-10-16: sent the QUESTION 01a14397-d329-7c1c-ba81-a16d4bbd6859 to develop-qzapp/user
with a per-kind sketch (right = Keep / Retry; left = Hide-for-now /
Unkeep / Cancel, each undone from a snackbar; Like and Accept/Reject left
out because the product has neither). Owner answered 2026-10-16 (REPLY 01a143a5-d809-7b5b-8973-b1c466555d20): the
sketch as-is, snackbar Undo enough, Like/Accept go to architect-cto,
Chats-list swipes proposed back first.
Spec: design/2026-10-16-message-swipe.md on contributor branch
develop-qzapp/ux-designer-01/for/user/message-swipe (58759d0d), for
fabric-coordinator to fold (#282 is unmerged). I chose to defer Unkeep
as well as Cancel until the snackbar closes, so a kill fails safe.
Sent: Chats-list/RTL QUESTION 01a143a9-9ceb-7924-8a38-d624d6b60d12 (owner);
product+rule QUESTION 01a143a9-a1ca-7cee-9e5b-cf4d797a1eb2 (architect-cto-01);
build REQUEST 01a143a9-a38a-722e-a2ad-d8b4ab0fdbbb (rust-ui-dev-01). 
Later 2026-10-16: rust-ui-dev-01 took it as their j82 (after their #283),
and found the kill case, so Unkeep/Cancel now run at once and Undo is
the reverse action ([[undo-by-reverse-not-deferral]]). Owner ruled: the
Chats-list left swipe hides a conversation holding nothing unread,
pending or kept; RTL mirrors. architect-cto-01: reactions not now
(need own record, never a receipt), contact requests refused, Hide
within §6. Branch was at 95b9079b (4 work commits; no Undo when copy_gone; transport message id of a re-queued cancel is the network role's, rust-ui-dev-01 asks them). Open: architect-cto's
check of re-queueing the same envelope on Undo of Cancel (asked in
REPLY 01a143ae-cb40-758b-986e-91b7fa52f650). Then tell fabric-coordinator the spec is final
(they fold it into #282), then review rust-ui-dev's build against §11
items 1-18.
Hide-for-now would be a new intent and Like a product change; both
are architect-cto's if the owner wants them.

Persona done: "AI Margaret Lovelace, ux-designer" (agent-fabric #281, 6240d7e4).
Related: [[human-client-message-kinds-and-actions]]

2026-10-16, final: spec FINAL at b8f532da (5 work commits). architect-cto-01
ruled Undo of Cancel restores the cancelled record under both ids
(app + transport message id) with its stored bytes, never a fresh row
(receiver dedups by transport id), and withdrew the durable hold; they
write it into human-client-ui.md §6/§13 in a docs PR. Fold asked: REQUEST
01a143b3-7b48-7096-9212-57b05d689bf3 to fabric-coordinator (into #282). Remaining: review
rust-ui-dev-01's j82 build against §11 items 1-18 on a device.
Fold verified: merge 56e2cbad on #282 carries b8f532da (2026-10-16).
Amended at 00f421e2 after architect-cto's #284: no accepted-while-cancelling case, Cancel snackbar follows Cancelled{may_have_reached} label, snackbar capped at 2 min (< ADR-0019 5-min dedup). Fold asked: REQUEST 01a143bb-6c94-77ea-a6ef-ff1ab1ba2a80.
#282 round 1 review (j2): F2-F5 + risk answered at bc0f7f17 (review-fix commits), F7 already done; F2 residual (does RETENTION §2 need a restore edge?) asked to architect-cto-01 QUESTION 01a143c2-d2bb-7bd0-9965-bdb938f1baa8; Undo of Cancel waits on #284.
F2 settled: architect-cto put the undone cancel into RETENTION.md §2 on #284 (28efec49, conformance case 16, no new edge). Spec at 8e2c23a2; fold asked REQUEST 01a143c4-e1ab-7a95-9b3d-86e1259aba01.
Fold of 8e2c23a2 verified on #282; coordinator asked: no more commits on the branch until round-2 re-review reports unless a finding asks.
PUSHED (was held): 8ade303a (attempt count as 6th restore input, per architect-cto #284 ec8a6d84) committed but NOT pushed; push and ask coordinator to fold once #282's round-2 re-review reports (coordinator's ask).
Round 2 (j3) N1-N5 answered + 8ade303a pushed; head 064641d4; fold asked. Hold further commits until re-review reports.
#284 merged d7634b3d 2026-10-16: Undo of Cancel no longer waits; spec prose 'waits on #284 landing' / 'writes it ... in #284' can be put in past tense after #282's re-review (holding commits now).
#282 merged 3a673136 (2026-10-16): spec on main. Coordinator's j259 will bring round-3 P3 follow-ups (remit roles, §4 icon colour, refused Unkeep edge, dedup wording, Cancel spring-back timing) to me later; bundle the past-tense #284 touch-up with that. Local spec branch swept.
j4 (round-3 follow-ups) delivered 2026-10-16 on develop-qzapp/ux-designer-01/for/user/swipe-followups (d6f79245, based on coordinator's dc27c626); coordinator folds and opens the PR. Also put #284 sentences in past tense.

2026-10-16 later: rust-ui-dev-01's build is InterWeave#289 (merged). Owner renamed the message action Dismiss (conversation keeps Hide); language-culture-en finalised all strings in spec §9. I ruled on the build: fling = release within 100 ms of first move seen (Slint delivers presses ~100 ms late), focus moves only from inside removed row, emptied conversation focuses header. #286 fixes delivered at c90333f6 on for/user/swipe-followups. Remaining: device review of #289 against §11 items 1-19.
Lesson: my inbox watch was down for hours and rust-ui-dev waited on 3 decisions — re-arm the watch every time it exits.
#286 round 2 (j8) answered at 14cafe23 on for/user/swipe-followups: F1-F4 + fling from touch-down (item 3 = 75 dp/150 ms), snackbar order while Cancel waits, silent last-moment refusal kept. Build's gesture.rs still speed-based fling -> rust-ui-dev's fix PR.
Device review: owner chose (b)+(c) 2026-10-16: rust-ui-dev-01 scripts §11 items on the Pixel (minus 12, 14), records + per-item log readable by me; a person does 12 (TalkBack) and 14 (Remove animations); I review evidence, failing items -> rust-ui-dev findings. No device access for me.
2026-10-16: owner decided ux-designer-01 also gets the Pixel 9a. One holder at a time: run as `fabric-lease pixel-9a -- <run>`, end with `adb disconnect` + `adb kill-server`. The owner pairs my login (Wireless debugging); a re-pair is the owner's step. TalkBack and gesture feel stay a person's check. rust-ui-dev-01's recorded run (their j88) still goes ahead for #289; from the next spec on I run my own device review.
2026-10-16 ~22:00 local: owner asked me to check #289 on the Pixel myself. Connect address a private-network address:43191 (the owner gave it; mDNS does not reach this VM). Installed app was a pre-final build (no 'Action undone'); owner chose no reinstall (my debug key differs, uninstall wipes data). Asked rust-ui-dev-01 to install main@925bd6e9 with their key (REQUEST 01a1464e-ff10-76df-ac2a-b6c7b80b3056).
Phone: rust-ui-dev-01 installed main@761d72b3 (incl. #289) with its key 2026-10-16 20:11Z; holds lease for j88 (~2 h). I review after; asked them to leave j88's conversations on the phone.

2026-10-16 ~21:25Z, own device run of main 761d72b3 (partial; the wireless link dropped twice, the second time unrecovered):
- item 5 reproduced (right drag on kept K7, then the first left drag does nothing; second commits Unkeep).
- item 17 reproduced: a left drag on unread Chats row h5yFGQBF cleared its 2 unread while the pane showed the previous conversation.
- Keyboard focus: no visible focus on the Chats list; in a conversation only a faint border. Tab order includes the peer-id text.
- Snackbar ~7 s, ignoring accessibility_interactive_ui_timeout_ms=30000 (not re-checked after an app restart: link dropped).
- uiautomator sees no app nodes: the Slint window exposes no accessibility tree (bears on TalkBack, item 12).
Not yet reported to rust-ui-dev-01.

2026-10-17 05:43Z, device review of main 9a5218c9 (#292's fixes), started:
- item 17 PASS: left and right drag on unread "...ukjyegnF" moved nothing, opened nothing, its "2 unread" stayed.
- Chats list after reinstall: rM49thLa and h5yFGQBF gone (read before restart), as retention says.
- The link went offline mid-run (05:44Z) and did not come back (connect ok, wait-for-device timed out). Item 5 untested; a right drag on K7 may or may not have reached the phone.
Still to run: items 1-11, 15, 16 and the N3 re-check (N3 and the ring are in #295, not this build).
2026-10-18: architect-cto-01 DECISION 01a14e7d (the owner's word): TalkBack on Android is deferred to the release gate (Slint 1.18.1 has no AccessKit adapter on Android; bridge prototyped, deferred). Swipe spec items 12, 13 and item 21's TalkBack rows stay unmet with that reason, not failed; no wording change. Record: plan §20 "Decided 2026-10-18", IW #297. My look review proceeds on main 400b42b6 once the owner frees the phone.

*References: human-client-message-kinds-and-actions, undo-by-reverse-not-deferral*

*Observed 2026-10-16 (ux-designer)*

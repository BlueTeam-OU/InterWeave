---
role: "rust-ui-dev"
class: threads
topic: "j155-connect-screen"
description: "InterWeave j155 Connect (add-a-peer s5.1) batch: what is built on the branch, the design choices made, what is left"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 997d476a36b89545
---

## InterWeave j155 Connect (add-a-peer s5.1) batch: what is built on the branch, the design choices made, what is left

Branch develop-qzapp/rust-ui-dev-01/feat/j155-29-step-5-carry-real-scan (worktree interweave-j155-...), pushed at 8918a1fa 2026-10-27, no PR:
- 6b5e12bc folds test/j155-drop-store-key-hook (9db1ee1e store-key hook, 50778fb9 Wi-Fi chat case).
- a6f0fb50 ui-model: src/connect.rs (Connect, ConnectGate Open/Settled/Deferred, ConnectLine, ConnectFocus, ConnectInput Add/Show/Trust/Leave); UiModel::connect_owed/connect_turn/connect_input/connect; Peers gets OpenAddFromConnect, TrustFromConnect(peers), AddPage.from_connect (Unchanged->Checked), take_connect_adds, list_answered; TrustSettings::read_answered. tests/connect.rs 8 tests.
- 8918a1fa app-core: ViewEvent::Connect, connect_turn in the turn's empty-events branch, Opener::connect_settled (default no-op).
- Choices: Connect is Android-only (only Android has onboarding). "Owed" survives restarts as a marker file in app data (to build in human-android: written at C4/R4 Continue, removed on connect_settled; ViewSide factory calls connect_owed when it exists, since a service restart makes a fresh ModelSide). Gate decided once per ModelSide. A Trust awaited at Done, or one lost and unread, counts as unknown.
- Assumption to put to ux-designer-01: once a line shows, Continue is the primary, Add/Show stay, Not now goes.
- 2026-10-27 (later): built and pushed to ca7f6b43: e2d3933c fmt fix; cb95e23a ui-slint Connect overlay (src/connect.rs, Page::Connect, panes hidden under it, add/Show drawn in Peers' place via peers on_page, Back/Escape=leave, focus heading/opener/"heading" on leave, Continue reuses IdContinue since label table refuses duplicate text, handed_over() helper kept take_events under clippy's 200 lines); 14fb269c line said on EVERY return with a line (ux REPLY 01a17cc5-7b6d, spec PR #480); f2b992ea android-platform connect-owed marker; ca7f6b43 human-android sets it on Seal Ok/restore Ok, factory reads it, connect_settled clears it. ux confirmed: Not now goes once Continue shows; Add/Show equal tonal. Desktop e2e human_app 33 ok; Android-target clippy OK. Mutations all caught.
- Still left: device run on the A40 (fresh identity -> Connect after C4; needs owner's word to clear the A40 app data + its adb address), TalkBack under the viewfinder (person's check), then PR (9 work commits) + blind review.
- Old plan line: ui-slint Connect page (pane slot like peers-pane; Add/Show render in the peers pane over it; focus Opener/ShowOpener map to Connect's controls), human-android marker + C4/R4 routing, mutations, desktop-e2e unaffected check, device run on the A40 (fresh install shows Connect after C4; acceptance s8 item 2), TalkBack under the viewfinder.
Related: [[j155-paste-batch-451]] [[j155-real-scan-418-done]]

2026-10-27, ux-designer-01 REPLY 01a17cd6 (provisional until #480's review is clean). Two changes to Connect as built:
1. A page opened from Connect (its Add an address, or step 2 reopened by the line's Trust) closes with NO snackbar. Connect's line is the feedback.
2. The line is announced on a return only if that page did something §5.1 counts: an add counted, a Show that drew addresses, or a trust decided in step 2. Back, a refusal, a failure, an unknown add or an empty Show announces nothing. This replaces correction 01a17cc5.
Also, Escape on Connect opens Chats as Back does, and it has a focus row now. Read #480 (cd57a6c0, a41a38bb) on main before building.

2026-10-27 (afternoon), after #480 merged at 4d174118 (so 01a17cd6 is no longer provisional):
- 887e13e7 folds origin/main. Two import-list conflicts were resolved as unions: activity.rs and ui-model lib.rs.
- 570db6c3 ui-model: a page opened from Connect closes with no snackbar. `Peers::take_connect_return() -> Option<Vec<ConnectAdd>>` is None when the page did nothing counted; this replaced take_connect_adds, and a bool would have tripped clippy's struct_excessive_bools. `Connect::said()` counts the returns that are said. Back on step 2 is Done, so it counts.
- 5cb99e1b ui-slint: the line is said when `said()` moves (replacing page_was_up). Escape on Connect = Not now was already built.
- Mutations: 4 on the model and 2 on the view, all caught. Then the push and `cargo xtask ci` (running).
- Next: blind review, then open the PR (11 work). A40 device run + TalkBack still wait on the owner.
- 2026-10-27 ~19:38-19:47 phone time, A40 (a private-network address:35361, API 30), the owner's word to clear: pm clear + install -r of 5cb99e1b (sha 1f47032c, a scratch path). The owner did C1-C4 by hand, so the words never went through adb. Results:
  - Connect shown after C4 (acceptance s8 item 2): title, body, two tonal buttons, Not now.
  - Show then Back: ConnectDoneShown line + filled Continue in Not now's place.
  - Add then Back with nothing: line kept, no snackbar.
  - Onboarding and Show capture blank (FLAG_SECURE). The A40 dropped adb twice; a plain re-connect worked. Screens 03/05/06/08 are in a scratch path.
  - Not run: TalkBack announcement (person's check); an add from Connect with a real peer (needs a second device's address).
- PR #490 opened (11 work). Round 1 review 5670735453 found F1 P2 (no test of Settled reaching connect_settled), F2 P3 (stale trust-unknown line) and F3 P3 (a comment). A judgement found all three real. Fixes: ac65274a + d450a19b (F2: trust_read each turn/press, plus the view's focus move to Continue), bfb95f80 (F3), 2a44d830 (F1, app-core root test). Answers comment 6447926479. Re-review of 5cb99e1b..2a44d830 dispatched.
- A40 20:47: Continue -> Chats, marker gone, restart -> Chats (no Connect). Posted on #490.
- Open question for ux-designer-01: whether an Activity re-attach (not a service restart) counts as a start that reopens Connect.
- Round 2 (5671033173): F4 P2 (the late answer not drawn: connect_turn ran after the render) fixed in 19b1809c (ModelSide::turn re-renders when Connect changed; root test). F5 P3 fixed in 70c23cde. Round 3 (5671340832) clean. #490 ARMED 2026-10-27 at 19b1809c (11 work, 6 fix).
- ux REPLY 01a17e5a: (a), provisional on design PR #491. A late answer that changes the line's text is announced once, politely. Focus stays, except Trust -> Continue. While Add/Show is up, nothing is said; the return announces the line once even if that page did nothing counted (s8 return row amended). Queued as a job; build after #491 merges.
- ux REPLY 01a17e63-f163: the lost Trust's read-back list counts as a read; #491 8add61b5 drops 'never applied'. #490 and trust.rs read_back stand as built.

*References: j155-paste-batch-451, j155-real-scan-418-done*

*Observed 2026-10-27 (rust-ui-dev)*

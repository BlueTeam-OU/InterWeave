---
role: "rust-ui-dev"
class: threads
topic: "j217-locked-restart-plan"
description: "j217 (#579 s11 L1-L4, F10-F14, A14-A20) DONE: #578 merged 20ce43c8 (12 work, 6 fix); no device run; double-prompt race left as j218"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d22d5d40940de90b
---

## j217 (#579 s11 L1-L4, F10-F14, A14-A20) DONE: #578 merged 20ce43c8 (12 work, 6 fix); no device run; double-prompt race left as j218

2026-11-05: #579 merged (62ba5cf01), so j217 is unblocked. Its work goes on #578's branch, develop-qzapp/rust-ui-dev-01/feat/j216-577-follow-up-the-activity-s, which is unarmed.

Design, part 1:
- The routing decision lives in Rust, as a pure module `apps/human-android/src/unlock_route.rs` that the host can test. It holds three flags: `declined` (process-wide, cleared by a launch or by a start that ran), `person_asked`, and `shown`. It answers Prompt, Notice or Nothing.
- jni start: a -10 answer calls `ServiceHost::refused_locked` (new `StoreAnswer::Locked`) and `onboard::refused_locked` (sets AFTER_PROMPT=RESTART and the hub flag, then routes). Any other answer clears authentication.
- identityState takes a `ready` argument. When ready: drop only the queued UNLOCK asks; if the host is running, answer READY; otherwise check the record for authentication only.
- Kotlin: `unlockDeclined` moves to Rust. New Native calls: `shown`, `launched` and `restartAsked` (for the network grant). `prompted` gains an `unlock` flag.
- ui-slint: the session NoticeCard gets focus-key "notice". `View::focus_locked_notice` acts only on Chats or a conversation. F14: when the locked notice leaves with focus on it, focus goes to the heading or the composer.
- store_follow: a Locked answer after a discard hides the page and holds the discard's notice until a later Opened or OtherRefusal.

Part 2 (open): s11 says the list and the open conversation stay across a refused restart. But ViewSide makes a fresh, empty model at Running(false) (review F3). Look into this after part 1.

State on 2026-11-06:
- 081b8ac49 is committed (android-platform Locked).
- Not committed yet: the ui-slint notice focus, the app's unlock_route/store/onboard/jni/activity changes, and the Kotlin changes.
- View tests pass, and a mutant was killed.
- Focus ids: both notice cards report "notice", which the Tab tests and android-e2e `unlock` use. The request targets are "chats-notice" and "pane-notice".
- A card made in the same render as the request does not take focus under the test backend (no layout between timers). The test asks after the notice is drawn. The L2 return case needs a device check.
- My schedule_focus change turned out to be unneeded, so I reverted it.

State on 2026-11-06, later:
- Part 2 and ux's read-out condition are pushed on #578 at 328160890: 11 work commits, unarmed.
- Commits: ad551ee5b (ui-model drafts), b641ab387 (app-core stopped/drafts/remade), 9808254d5 (ViewSide sets the stopped model aside and shows it again once locked), 983fba985 (test of remade), 328160890 (a notice action's description is the notice text).
- ux REPLY 01a1ae33 (no reply expected): S2 focus right; the Unlock read-out must include the notice text, which is done. ux checks on the phone in their j95.
- Next: xtask ci, blind review of main..head, post it, arm. Then INFO to ux with the head. No device run of mine yet.

Closed 2026-11-06: #578 merged as 20ce43c8 (head 9bf4730d; 12 work, 6 fix) after six review rounds (F1-F6 all fixed). INFO 01a1aecb sent to ux-designer-01; their phone review of the rows is still open. Evidence is unit/view tests and CI only: no phone run, TalkBack not exercised.
Left over: r5 risk 1, two Unlock prompts when the Activity's check and a Service start find the lock in one showing (nothing tracks an outstanding prompt in unlock_route) — queued as j218. Other unproven risks from r5: notice focus behind an open menu/Details/dialog, drafts lost when kept is dropped by a non-Locked answer, store.rs HELD discard notice lost on an Activity recreation.

*Observed 2026-11-05 (rust-ui-dev)*

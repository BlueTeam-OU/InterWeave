---
role: "rust-ui-dev"
class: solution
topic: "j282-r2-check-holds-words"
description: "j282 — R2 holds its words while Check my words runs: final rule on ux #683 (f71f27d0c, REPLY 01a1e4e6), where it goes in the code; blocked on #683 merging"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 780c088755b036d4
---

## j282 — R2 holds its words while Check my words runs: final rule on ux #683 (f71f27d0c, REPLY 01a1e4e6), where it goes in the code; blocked on #683 merging

ux REPLY 01a1e181-5b30 (2026-11-15, provisional until design/2026-10-19-android-identity.md §8 says it): from the press of Check my words until its result (R3, a §6 banner, the system prompt's answer, IdGoingOffline's end) no position chosen, no chip taken, no letter typed or deleted, full list not opened; Back keeps §7. Focus stays on IdCheckWords, shown disabled; at the result the existing rules place focus. Do not build before ux's note PR lands.

2026-11-16, read on dbc969d7e: cheap to build.
- Model: crates/human/ui-model/src/identity_flow.rs `words_held()` (discard_asked || reviewing) guards choose/press/delete/clear via picker_mut; add R2 `checking`. `edit(position)` checks only discard_asked: guard it too. `review_words` already refuses while checking.
- View: `page.primary_enabled` is already false while waiting (ui-slint src/identity.rs). Keypad keys already have `enabled-action` + 0.38 opacity (app.slint ~3081); grid slots and chips have none, so add one. The `changed page` focus rule (app.slint ~2573, to-check/to-keypad) must not move focus while checking.
- Tests: tests/trace.rs (Identity helpers), mutation-check each guard.

FINAL (2026-11-16, ux REPLY 01a1e4e6, spec on #683 f71f27d0c, open): differs from the provisional reply:
- Back and Escape WAIT during the check (no discard dialog), not "Back per §7".
- Held: letter key, typed letter, Backspace, delete, clear, chip, Enter, grid position, show-all. Focus never moves; a tap moves none, so focus stays where it was. Held input leaves the mode unchanged.
- Only IdCheckWords disabled and the back arrow inactive; keys KEEP their look (no extra disabled styling on slots/chips).
- §6 gains a "Check running" row; acceptance 15 a case: a test hook holds the check, "a", tap position 5, Escape, Enter change nothing, focus on IdCheckWords; a failed checksum then focuses the banner.
- ux's diagnosis: `checked()` sends RestoreRecorded with entry.phrase() at result time, so an edit during the check would reach R3/restore.

j282 blocked on pr:BlueTeam-OU/InterWeave#683. No message owed to ux, since ux asked only to hear if the change were costly.

BUILT 2026-11-17 on PR #697 (branch develop-qzapp/rust-ui-dev-01/feat/j282-interweave-r2-words-fixed-while-check, base 0ac77f044): b82212492 ui-model (edit() and words_held() refuse while R2 checking; test no_input_changes_the_words_while_the_check_runs), a915be1d3 ui-slint (IdentityPage.checking, app.slint `mode()` keeps typing/navigating unchanged while checking; identity.rs entry_grid() helper split out because page() hit clippy too_many_lines 201/200; trace test the_words_are_held_while_the_check_runs). 5 mutations killed. 2 work commits: arming needs the owner's word plus a posted review (security boundary: identity_flow.rs, identity.rs).
Local `cargo xtask ci` fails only check_unused_dependencies (cargo-machete not installed in this account, exit 2); CI's tree checks run it.
2026-11-17: review r1 posted (F1 P3: five mode sites untested); fixed 4a93ad292 (trace a_held_input_with_letters_typed_leaves_the_mode_navigating, pos 5 cut to "cab"; pos 24 is off-screen so untappable; a Check tap moves no focus, so Enter after each probe presses delete); re-review F1 fixed, posted. AUTHORIZES asked of architect-cto-01 (REQUEST 01a1e6cf). Review's pre-existing P-1/P-2 (R3/R4 title focus; repeated notice) queued as j285.
ARMED 2026-11-17 at 4a93ad29 on architect-cto-01's AUTHORIZES (01a1e6cf-f1e0); wait-merged running; deliver j282 after merge. architect-cto: take j285 P-1 (R3 Yes gets focus, second Enter confirms unread PeerId) ahead of P-2.
DONE: #697 merged 2e8f7ba8a 2026-11-17; j282 delivered.

*Observed 2026-11-17 (rust-ui-dev)*

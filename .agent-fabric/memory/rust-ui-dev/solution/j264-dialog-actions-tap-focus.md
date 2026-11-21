---
role: "rust-ui-dev"
class: solution
topic: "j264-dialog-actions-tap-focus"
description: "j264 / PR #644 — screen actions held behind the discard dialog; a keypad tap moves no focus (measured), so Return after a tap presses the previously focused key"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 414a394059906b02
---

## j264 / PR #644 — screen actions held behind the discard dialog; a keypad tap moves no focus (measured), so Return after a tap presses the previously focused key

j264 (PR #644, opened 2026-11-12, 4 work commits + 1 merge, minimum 8 so it arms only on the owner's word): the round-3 review of #634 (5873906315) listed pre-existing items. These are the answers:
- `IdentityFlow::primary`, `try_again` and `show_words_again` return early while `discard_asked()` (b3d1c6594). Unit test `no_screen_action_runs_behind_the_dialog`; trace tests `check_queued_behind_the_discard_dialog_checks_nothing` and `c3s_{try_again,show_words_again}_queued_behind_the_discard_dialog_does_nothing`. Each guard was mutated out and its test failed.
- Measured, contrary to both my guess and the review's: on R2/C3 a pointer tap on an `IdentityStop` key does NOT take focus. Its TapArea child takes the pointer. Focus stays where it was (a fresh R2 = q), so a hardware Return after a tap presses that key, and the `enter-takes()` no-control branch is never reached. Pinned by `a_return_burst_behind_a_tapped_key_presses_the_focused_control`. The trace line dedups `focus=identity-key`, so traces cannot show which key holds focus: probe with TAB+Enter on a fully enabled keypad.
- Each `Identity::new` in trace.rs calls `init_no_event_loop`, which panics the second time on one thread: one `Identity` per #[test], never a loop over two.
- QUESTION 01a1d095 to ux-designer-01: should a tap move focus, and design §4 line 386 says "a chip the person taps" though Enter also takes it.
Related: [[slint-android-input-batch-stale-page]] [[j261-r2-enter-art]] [[key-driven-ui-settled-focus]]

2026-11-12 ~17:20Z: #644 ARMED at 48ca9b73 on architect-cto's word (REPLY 01a1d0c0): 4 work, 2 fix, minimum 8. Round 1 review 5878174990 had P3 F1-F3. F1: the tap test on "ac" passed even under a disabled-key-rejects-Return mutation; it now settles "act" and taps "i". All fixed in ab0a5eae2 and 48ca9b736. Re-review 5878755805: Finding: none.
ux-designer answered j119 on #645, still in review: a tapped enabled letter, delete or clear counts as typing, and no tap moves focus. That replaces #644's tap test. Queued as j266, blocked on #645. The build must tell a touch press of a key from an Enter press.
DONE 2026-11-12: #644 merged a16003b33 (4 work, 2 fix, 1 merge). The follow-up is j266, blocked on #645.

*References: j261-r2-enter-art, key-driven-ui-settled-focus, slint-android-input-batch-stale-page*

*Anchors: `crates/human/ui-model/src/identity_flow.rs::show_words_again`, `crates/human/ui-slint/tests/trace.rs::a_return_burst_behind_a_tapped_key_presses_the_focused_control`, `crates/human/ui-slint/ui/app.slint::enter-takes`*

*Observed 2026-11-12 (rust-ui-dev)*

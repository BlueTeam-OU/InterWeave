---
role: "rust-ui-dev"
class: solution
topic: "j266-tap-is-typing"
description: "j266 (#645 ruling): how IdentityScreen tells a tap from an Enter press (by-tap out property), returns typing after a chip or position, and moves focus off a chip the letters remove"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2eb667749031a065
---

## j266 (#645 ruling): how IdentityScreen tells a tap from an Enter press (by-tap out property), returns typing after a chip or position, and moves focus off a chip the letters remove

j266 (2026-11-13), branch develop-qzapp/rust-ui-dev-01/feat/j266-j119-tap-focus-and-4-item, built from design/2026-10-19-android-identity.md §8 as #645 (merged 6c9327055) revised it:
- IdentityStop and ActionButton carry `out property <bool> by-tap`, set true just before `pressed()` from their TapArea and false from Enter/Space or the accessible default action. A key's handler sets `typing = self.by-tap`. The accessible action counts as a press (navigating); ux-designer confirmed it (REPLY 01a1d262: a screen reader moves focus first, so activate = Enter on a focused key; "pp" must work). They will name it in §8 under their j121.
- A chip taken (take()) and a grid slot pressed set typing = true. Focus goes to the keypad on the page whose counter moved: `chips-taken` (existing) or the new `positions-chosen` (IdentityFlow::positions_chosen, counted in edit()). Every changed-page branch that resets chips-shown must reset positions-shown too, or a stale difference moves focus later.
- Focused chip: its word kept in `chip-word` (never traced). On a rebuilt page with at-seg == 2, chip-index(word) over at most four chips (MAX_CHIPS, pinned by trace.rs the_chip_row_holds_at_most_four): found -> again(2, i), else keypad().
- Test traps: one Identity per #[test] (init_no_event_loop panics a second time); the trace dedups a repeated focus line, so "focus stayed" means last() unchanged, not a new line. A chip test whose chip index does not move cannot tell "follow the word" from "keep the index".
Related: [[j264-dialog-actions-tap-focus]] [[j199-identity-keyboard-reach]] [[slint-android-input-batch-stale-page]]

#654 (2026-11-13): reviews r1-r4 posted, head 60ecc7e2 (3 work, 2 fix), gate met; arming asked of architect-cto-01 (REQUEST 01a1d38e, AUTHORIZES line). Lessons: a Slint FocusScope focuses on click by default, so any identity stop whose TapArea can be disabled (IdentityStop, IdentityPrimary) and the identity FocusNotes (banner, K1 title) set focus-on-click: false (only the identity design states "no tap moves focus"; FocusNote itself left alone). by-tap is per element, and a repeater key is a new element each page, so a stale by-tap is only reachable within one input batch (tap_queued then activate). Test WORDS has no "ap" word: acceptance 15 is pinned as "ra", b, "rabb".

*References: j199-identity-keyboard-reach, j264-dialog-actions-tap-focus, slint-android-input-batch-stale-page*

*Observed 2026-11-13 (rust-ui-dev)*

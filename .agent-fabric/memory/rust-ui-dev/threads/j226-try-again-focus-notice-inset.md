---
role: "rust-ui-dev"
class: threads
topic: "j226-try-again-focus-notice-inset"
description: "j226 (ux OBSERVATION 01a1b81a): A14 focus after R goes, A15 focus statement after a prompt, Chats notice inset, network-denied reads offline; 5 commits on feat/j226"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 35cc0dde3c4703b2
---

## j226 (ux OBSERVATION 01a1b81a): A14 focus after R goes, A15 focus statement after a prompt, Chats notice inset, network-denied reads offline; 5 commits on feat/j226

2026-11-08: j226 has 5 work commits on develop-qzapp/rust-ui-dev-01/feat/j226-j98-interim-observation-01a1b81a-from-ux (1a85a6d01..4a2d391ef), off main 3e3c6b902. There is no PR yet.
- A14: `View::show_identity(None)` hands focus on when focus was inside the screen. It goes to Chats' heading, or to an open conversation's header ("h"), and a locked-notice ask wins. Slint moves focus nowhere when the focused element is destroyed.
- A15: focus stays on Try again; the trace only wrote changes. Now `View::trace_focus_settled` (ui-trace) states the focus 200 ms after a prompt's answer, unless a move was said meanwhile. Android calls it from `onboard::prompted`.
- (3): the Chats session NoticeCard is wrapped in 12 px side padding, the same as no-reach and the pane's card. ux said "26 dp"; I did not reconcile that number.
- Minor: `UiModel::connectivity` is Offline while network_denied, the same rule as locked.
- Lesson: the first `focus-seq` bump is missed by a control whose tracker never read it, even if that control existed before. Use focus_key_after_draw. In headless tests, walk the tree before settle so `if` elements exist.
- Lesson: `git apply` takes `-C1`, not `-U1`. A failed `&&` chain then swept every hunk into the next `git add <dir>`. Diff the end tree against the saved final ([[split-commits-fail-fast]]).
- (2), the grant kills the process: I sent ideas only. Asking from the app's settings page (opaque, so the Activity stops and saves its state) may let the system recreate it. This is unverified.
Related: [[j222-discard-hook]] [[slint-focus-new-element]]
- ux REPLY 01a1bb84: yes to both A14 targets, provisional until their note commit lands with the A17 note fix. Inset is re-measured at 3b; their 26 was read off a 1080 px screenshot. Network grant: they try both paths at 3b. **The PR body must say that Offline-while-network-denied is connectivity vocabulary, so architect-cto sees it.**

*References: j222-discard-hook, slint-focus-new-element, split-commits-fail-fast*

*Observed 2026-11-08 (rust-ui-dev)*

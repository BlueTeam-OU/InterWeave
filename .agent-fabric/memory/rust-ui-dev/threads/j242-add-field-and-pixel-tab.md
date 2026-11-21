---
role: "rust-ui-dev"
class: threads
topic: "j242-add-field-and-pixel-tab"
description: "j242 DONE (REPLY 01a1bfaa): A40 add 'hang' = untraced refusal of a key-event-lowercased PeerId; Pixel TAB lost to the POST_NOTIFICATIONS prompt after pm clear; supply for/p2p-network-dev-02/step5-field-and-focus efa081bdf"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 73404414bbf97a97
---

## j242 DONE (REPLY 01a1bfaa): A40 add 'hang' = untraced refusal of a key-event-lowercased PeerId; Pixel TAB lost to the POST_NOTIFICATIONS prompt after pm clear; supply for/p2p-network-dev-02/step5-field-and-focus efa081bdf

Answered OBSERVATION 01a1bf79 from p2p-network-dev-02 with REPLY 01a1bfaa on 2026-11-09. The supply is `develop-qzapp/rust-ui-dev-01/for/p2p-network-dev-02/step5-field-and-focus`, 1799f3aa8..efa081bdf. The same three commits are on my branch feat/j242-step-5-on-d1be8eb89-pixel-tab. I opened no PR: p2p-network-dev-02 folds them into their row-2 PR.

- c07d69011 (ui-slint): the trace prints `note=<kind>` for every add-page note, and `note=none` when it clears. Pressing Add again clears the last note before the answer comes, so a driver must skip `none`. The first test run caught exactly that case.
- 5dcc4c7ea (android-e2e): `typable()` refuses capitals, `:` and `_` (the Slint 1.18.1 Shift limit). `wait_answer` fails on the first `note=` line that is not `none`.
- efa081bdf (android-e2e): `launch()` fails when `mCurrentFocus` is not the app's window (`platform::input_focus`).

Pixel cause, measured: on API 33+, `MainActivity.askNotifications` asks at every launch while POST_NOTIFICATIONS is ungranted. After `pm clear`, the GrantPermissionsActivity prompt takes every key. A driver run needs `pm grant ... POST_NOTIFICATIONS` after a clear. The A40 is API 30 and has no such prompt.

Not run on a phone: the three commits.
Related: [[key-driven-ui-settled-focus]] [[android17-internet-runtime-permission]] [[for-supply-close-as-delivered]]

*References: android17-internet-runtime-permission, for-supply-close-as-delivered, key-driven-ui-settled-focus*

*Observed 2026-11-09 (rust-ui-dev)*

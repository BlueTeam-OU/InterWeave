---
role: "rust-ui-dev"
class: solution
topic: "j153-superseded-address-state"
description: "j153 (2026-10-24/25): add-a-peer superseded read-back built from #425 s5.7; branch state, what it proves, what is left (acceptance 10 hook, j157)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3cbe39fe50303833
---

## j153 (2026-10-24/25): add-a-peer superseded read-back built from #425 s5.7; branch state, what it proves, what is left (acceptance 10 hook, j157)

j153 builds design/2026-10-22-add-a-peer.md §5.7's superseded table and §5.3's superseded Trust (#425, merged), after IPC 2.10 (#426, merged 2026-10-24).
- LANDED as #428, merge 4c2674b6 (2026-10-25): 2 work, 3 fix. Armed below the minimum on architect-cto-01's AUTHORIZES REPLY 01a17084, given on request; for a small follow-up that architect-cto carries by name, ask architect-cto for AUTHORIZES before waiting on the owner.
- Review round 1 (de98c4fd) found F1: an unasked read closed the page for an Unknown or SupersededUnread block, which also fixed the pre-existing Unknown case. It found F2: a superseded read back left listed unknown lines unsettled, so `page.unknown = None` must stay; my mutation run had wrongly called it unobservable, but it shows on the path A, then B, then back to A. It found F3: no test held max(ms, 10 s). All three were fixed; round 2 on f5ffa621 was clean.
- Mechanism: the facade already reads the list back after every change, refusals included (client.rs change_address / add_addresses). So a superseded refusal is decided on `answer.list`, with no extra read; a failed read owes no further read.
- TrustSettings::set on Superseded: it ends `lost` for the peer and sets `unread_write = sent`, so a read started before the refusal confirms nothing (unit test a_superseded_change_ends_pending_and_an_earlier_read_confirms_nothing). TrustBlock.checking is now Option<ReadBack {Lost, Superseded}>, because clippy refuses a fourth bool.
- The no-action snackbars are max(10 s, snackbar_ms). snackbar_ms is already the Android accessibility timeout (FLAG_CONTENT_TEXT|CONTROLS). The Snackbar component's TouchArea reports `pointer-on`, and close_when_free re-checks every 1 s. It has no focusable element, so the "focus on it" clause cannot arise.
- Not done: acceptance 10's superseded e2e cases, which need a hook that holds the app's call before it is sent. Also j157 residual: the §8 focus move when focus was on the block's Trust or Not now, and Done for a one-block page. That needs ux-designer-01.
- Q-superseded-own answered to ux-designer-01 (REPLY 01a16fe3): no. The facade opens one port per call, and the model sends one change per key at a time.
Related: [[superseded-not-a-not-made-line]]

*References: superseded-not-a-not-made-line*

*Anchors: `crates/human/ui-model/src/peers.rs::add_superseded`, `crates/human/ui-model/src/trust.rs::TrustSetFailure::Superseded`, `crates/human/ui-slint/src/peers.rs::close_when_free`*

*Observed 2026-10-24 (rust-ui-dev)*

---
role: "rust-ui-dev"
class: solution
topic: "j268-connect-launch-count"
description: "j268 (+ j269 items 1, 3a) — owed Connect's launch count from Chats showing, built on #658; how StartRules now holds the order and the baseline"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c5dcb491b9018dcf
---

## j268 (+ j269 items 1, 3a) — owed Connect's launch count from Chats showing, built on #658; how StartRules now holds the order and the baseline

j268 (2026-11-13) built add-a-peer §5.1 as #655 wrote it, on #658's branch (develop-qzapp/rust-ui-dev-01/feat/j267-tm-58-the-a40-run-of), work 2ffaf850b, dbef97f36, 5c4d2d066; r1 fixes 7129081fe..2608e4ff9.

- The count of openings only grows: `View::opened_from_chats` = taken + queued + the window's `menus-opened`, which `open-chats-menu()` / `open-overflow-menu()` bump at each opening. Every menu button must call those, never set `*-menu-open = true` itself (views.rs `opening_something_from_chats_moves_its_count` presses the buttons; mutation-checked).
- `StartRules::count_from(opened)` sets the baseline: Root::attach (Chats shows at this launch) and `onboard::person_asked` → `activity::start_asked` (invoke_on_window; a later start's ask). A reattach keeps it. `launch` gives up when opened != baseline, even before the clock arms (opening before start success counts).
- `StartRules::turn(side, held, opened, now)` runs pumped, deferral, launch rule, then `side.turn()`: the order is host-tested (`the_rules_decide_before_the_turn_the_reads_open_connect_in`). Root::pump now runs store::follow and the lost-link reattach BEFORE the rules.
- Carried, not fixed (r1 pre-existing): a hub reattach restarts the 10 s bound (`reattached` clears owed_since; root epoch is fresh per attach). Unproven r1 risk: a recreated Activity in the same process re-reads the marker (only connect_settled clears it), so a deferred Connect may be owed again.
- design/2026-10-22-add-a-peer.md §5.1 "The build of #651 differs…" and §8 item 12 "No host test…" are stale after this; ux-designer's to revise.

Related: [[j265-continue-hold-plan]]

**2026-11-13 state:** r1 (4 P3) fixed 7129081fe..2608e4ff9, r2 found F4 partly (RTL buttons), fixed 2cbbe6b79; r3 clean, all posted. REPLY 01a1d4bb to ux-designer-01 sent (delivery + stale §5.1/§8 text + the two carried items). #658 unarmed at 5 work, 5 fix (floor 8): j270/j271 go on as commits there. j269 dropped as folded into j268.

*References: j265-continue-hold-plan*

*Anchors: `crates/human/android-platform/src/start_rules.rs::StartRules::turn`, `crates/human/ui-slint/ui/app.slint::menus-opened`*

*Observed 2026-11-13 (rust-ui-dev)*

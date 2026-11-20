---
role: "rust-ui-dev"
class: threads
topic: "j265-continue-hold-plan"
description: "j265 (add-a-peer §5.1 hold of C4/R4 after Continue until Connect is decided) — what was undertaken and the build plan"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - edd53fdfdee85d6e
---

## j265 (add-a-peer §5.1 hold of C4/R4 after Continue until Connect is decided) — what was undertaken and the build plan

j265 started as ux-designer-01's QUESTION 01a1cfc9; answered by REPLY
01a1cfcb (2026-11-12, sent from the j263 session): the hold is buildable,
one model input, both hooks (reads hold + host test) mine, built after
§5.1's language-culture pass. That pass and the spec landed on main with
#642 (design/2026-10-22-add-a-peer.md §5.1 "Between Continue and the
decision", §8 item 12). So j265 is the BUILD, branch
develop-qzapp/rust-ui-dev-01/feat/j265-add-a-peer-5-1-can.

Why Chats flashed: onboard.rs render() at flow Done calls
show_identity(None) and hands over (done() -> Root::attach, whose fresh
model is told connect_owed and sends the reads), so Chats shows for as
long as the reads take.

Plan (one commit per package):
1. ui-model labels: ConnectStarting ("Starting InterWeave…", final in §9).
2. ui-model: UiModel::connect_defer() — Owed -> Decided{deferred} (the 10 s
   bound, or the person opened something from Chats at a launch).
3. ui-model: pure hold clock (ms): 1 s ConnectStarting from the press,
   reset by a prompt grant; 10 s bound from commit (C4 seal ok / R4 press),
   paused while the service-start prompt is up.
4. ui-model identity_flow: C4/R4 held after a press (primary disabled,
   StartService without OpenChats), decided() -> Done + OpenChats.
5. ui-slint: held page — Continue disabled + focused, ConnectStarting line
   with WaitMark under it, one polite announcement.
6. human-android: attach Root at the press, keep the page until
   connect_phase leaves Owed / start ok when not owed / refusal / bound;
   FLAG_SECURE untouched until the decided page; launch 10 s rule;
   trace lines (press, seal ok, start ok, prompt result).
7. debug hooks: hold the start's address+trust reads; hold the service start.

Progress 2026-11-12 evening: committed 1-4 (7011c6adf label, connect_defer,
0edbf77fa StartHold, d681920d5 flow held/decided/start_held/set_starting).
Uncommitted, written: ui-slint (IdentityPage.starting + WaitMark line,
announce once in show_identity, after()/Wake, trace start=pressed|sealed|
starting|running, View::opened_from_chats); android-platform
(Hub::store_generation, ViewSide::running/start_decided + test); app-core
ModelSide::connect_defer + root.rs test; human-android onboard.rs (hold
clock wiring, render Step::Attach/Handover, held_start/hold_bound_passed/
start_decided, HOLD_TIMER) and activity.rs (Root launch_rule/held_rule,
defer_next, turn_soon). Not yet built for the Android target. Hooks
(reads hold, start hold) still to write.

2026-11-12 late: all 8 work commits on the branch (7011c6adf..b56b2fbdc),
not pushed yet. Host tests green (ui-model, ui-slint trace/views/lib,
app-core root, android-platform view_side/hub, human-android start_hook);
check_android_target.sh OK before the last fixes. Debug hooks are armed
from launch extras: `am start -n org.interweave.human/.MainActivity
--ei holdReadsMs 2000` / `--ei holdStartMs 12000` (the Service's broadcast
receiver is not up before a first run's start). Trace page= under an
identity screen is held until identity=closed. Still owed: device run on
a phone (acceptance 12), blind review, PR. j267 (TM-58 A40 run) queued
behind; A40 order: rust-ui-dev-02 j95 run, then p2p S12 rows, then TM-58.

**2026-11-12 late:** PR #651 opened at d8d78b45c (8 work, 0 fix, 1 merge; review base aa354de0a). xtask ci green except tests/security kademlia_adversarial full-bucket case, a load flake (3/3 alone). Round-1 blind review dispatched; then post, judge findings, arm. Device run awaits the owner's word to uninstall on the Pixel.

**2026-11-13:** r1 F1-F4 (refused start kept defer_next; queued opens uncounted; stale seal notice; untested Android-only rules) fixed in 4 fix commits; rules moved to android-platform `start_rules.rs` (host-tested, mutation-checked). r2 re-review clean; #651 ARMED at 9a7e36a3 (8 work, 4 fix). REPLY 01a1d217 to ux-designer-01 asks whether an opening already open at the start should give Connect up (r2 risk 1). Device run still awaits the owner's word on the Pixel uninstall.

**2026-11-13:** #651 MERGED a04400b39. Follow-up j268 (ux REPLY 01a1d218): Connect at a launch only if nothing was opened from Chats since Chats showed at this launch (flag, closed-again opens count, a launch not showing Chats first counts as opened).

j269 (ux REQUEST 01a1d2dd, 2026-11-13) folds into j268: per #655 §5.1 the launch count starts when Chats shows at this launch (NOT at the service start's success, which is #651's anchor and wrong); a menu opened and closed again counts; a later start in the same process counts from that start's ask. Host test owed: the root asks StartRules before the reads' turn applies (queued case). ux REPLY 01a1d2de corrected my wording. Build from #655 once on main.

*Observed 2026-11-12 (rust-ui-dev)*

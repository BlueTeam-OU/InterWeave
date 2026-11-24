---
role: "rust-ui-dev"
class: solution
topic: "j274-start-rules-per-start"
description: "InterWeave Android owed-Connect launch rule (add-a-peer §5.1) across a hub reattach and a remade Activity window — where the state lives and how a start is told from a later one"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c5346e8465c5fd12
---

## InterWeave Android owed-Connect launch rule (add-a-peer §5.1) across a hub reattach and a remade Activity window — where the state lives and how a start is told from a later one

j274 + j275 (ux-designer REQUESTs 01a1d5f2, 01a1d5f8; #661 §5.1 at a84f1c782), built 2026-11-13 on #662 as 79859e612 + 947069371:

- A reattach and a window made again while the service runs are the SAME launch: the 10 s bound goes on from the start's success, what an earlier window opened still counts, and a model made again shows what the start decided (deferred/settled -> deferred at once; open -> reopened from the reads, launch rule skipped).
- The hub says `ToView::Start(count)` (the facade generation) just BEFORE `Running(true)`, at a start and at an attach — before, not after, so a pump that sees Running(true) has the count already. `ViewSide::start()` reads it.
- `StartRules` is process-wide in activity.rs (`static RULES` + `EPOCH`), not a Root field: each Activity runs on its own thread with its own thread_local ROOT, so per-window state died with the window. `Root::new_window` calls `new_window(opened)`; the reattach path calls `reattached()`; the root wakes at `Turned::wake_in_ms` (re-armed for what is left after a reattach, since the old window's timer may be gone).
- A start is told by its hub count AS WELL AS the running edge (#662 F1, fixed 515d3eead): `ViewLink::take` drains everything queued, so a restart's Running(false), Start(g+1), Running(true) can land in one pump with no edge; edge-only kept the earlier start's decision and Connect opened unbounded. Test `a_restart_in_one_pump_is_a_later_start`. Counts never repeat: the hub lives in `ServiceHost::global()`'s OnceLock.
- Merged as #662 (e4a962472, 2026-11-13; 3 work, 2 fix, armed on the owner's word by j276).
- Known limits sent to ux: a reopened Connect starts blank; a settled start whose marker clear failed traces connect=deferred.
- Device check (Don't keep activities, item 12) not run: phone address with the owner.
- j275 was a correction of j274's point 2 (ux first read "a remade window is a launch"; #661 review F1 reversed it): built in the same two commits, delivered together. Lesson: delivering both before the owner said "arm" locked this session out of `fabric-pr arm` (ADR-037 rule 3); the arm went to j276 in a fresh session. See [[deliver-after-merge]].

*References: deliver-after-merge*

*Anchors: `interweave:crates/human/android-platform/src/start_rules.rs`, `interweave:apps/human-android/src/activity.rs`*

*Observed 2026-11-13 (rust-ui-dev)*

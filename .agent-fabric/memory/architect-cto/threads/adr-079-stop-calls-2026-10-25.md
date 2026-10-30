---
role: "architect-cto"
class: threads
topic: "adr-079-stop-calls-2026-10-25"
description: "gzapp ADR-079 (Ephemeral Stop Calls and Demand Consolidation) drafted 2026-10-25 as PR #1166 (draft, 2 work): what it records, what it amends, what it waits on (review from a relaunched session, owner's acceptance), and PR 9's dependency…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2ffad196e88fcfff
---

## gzapp ADR-079 (Ephemeral Stop Calls and Demand Consolidation) drafted 2026-10-25 as PR #1166 (draft, 2 work): what it records, what it amends, what it waits on (review from a relaunched session, owner's acceptance), and PR 9's dependency on it

**2026-10-25 ~04:40Z.** gzapp #1166 (draft, 2 work, 0 fix) on `develop-qzapp/architect-cto-01/docs/adr-079-stop-calls` off main 70b59e95c: ADR-079 "Ephemeral Stop Calls and Demand Consolidation: One Vehicle Stop Serves Every Passenger Bound for It" (Proposed) + master/digest/index/clusters, and consequential amendments ADR-053 §2.5/§2.6, ADR-058 §2.9, ADR-059 §2.6/§2.8, ADR-060 §2.4, ADR-028 §2.1 (counts 10/27/19/5/12). Written from plan §9/§11/§13/§14/§15/§16/§20/§26/§27 and the landed code (PRs 6–8b, 13a) plus my rulings (8b, 13a, 13b, PR 9 R1–R8). validate_adr_index 80 OK; scan_semantic_collisions OK.

**Waits on:** the blind review dispatched from a RELAUNCHED session (ADR-065; this session must not dispatch), then the PR leaves draft and arms on the owner's word (own PR, under minimum); the owner's acceptance of the ADR (Proposed → Accepted is a later amendment or the owner's word in the PR). backend-dev-02 asked to read it against the code (INFO). PR 9 (their j37, branch `…/feat/j37-next-completion-plan-pr-9-the`) builds as a draft behind it and cites ADR-079 by number; its event-kind 1.36.0 bump is theirs.

**Owed by me around it:** ADR-044 §2.3 "owed" → `tools/release/release.sh` + `docs/operations/RELEASE.md` once #1165 (2 work, 2 fix) folds main (supply onto #1165, then AUTHORIZES); #1160 (8 work, 5 fix) word after #1159's guard is folded + re-review; 13b's ADR-028 retention rows for its three tables once backend-dev-01 names the branch; the planning contract flip with 13b; the InterWeave docs PR from s21-docs after relaunch.

**04:45Z backend-dev-02's read (OBSERVATION 01a17143) folded at 9e4a8dfc8 (#1166 now 3 work):** O1 lock issues the ticking journey's directive; O2 ideal = chainage today, point identity with PR 9 (db-admin columns, REQUEST 01a17139); O3 Terminal/Transient cases in full from ClassifyPinnedDropoff; O4 RULED causes: ended run by end_reason (evidence_gap→vehicle_stale, else vehicle_changed_line), gone row→vehicle_changed_line, marked→stop_call_unservable, passed or next run→stop_call_passed; O5 run detection not mode-gated, shadow after the live boarding's dropoff too; O6 three additive payload fields (stop_call_id, cascaded_from_directive_id, inherited_from_stop_call incl. _issued). NEXT SESSION: dispatch the blind review of #1166's head (code-review, fable), post it, leave draft, ask the owner's word (own PR, under minimum) and acceptance of the ADR; the same for the InterWeave s21-docs PR once opened.
- 2026-10-25 04:55Z #1165 (2 work, 2 fix) folded main (468246241); my ADR-044 'owed'→`tools/release/release.sh` + `docs/operations/RELEASE.md` fill = dfe966ce9 on `for/devex-tooling/adr-0044-release-script-named` (body, row cell, digest; history note untouched). Next: their fold + re-review → my AUTHORIZES for #1165. gzapp worktrees gz-13a/gz-8b/gz-adr079 were pruned (branches all pushed); gz-1165 and gz-adr079's branch exist on origin.

- 2026-10-25 09:35Z: backend-dev-02's three code-past-text places (QUESTION 01a17262) written into §2.2 (no lock on a stale/unmatched observation or below the passed clearance; passed unlocked via the progress closer), §2.4 (replan from the pinned dropoff's current point, R1), §2.5 (scan skip only for a live demand whose directive the active path issued) — 419eeaf80; #1166 now 4 work, 0 fix, review still owed (from this relaunched session it can be dispatched).
- 09:40Z blind review of #1166 (4 work, 0 fix) DISPATCHED from this relaunched session on 419eeaf80 (brief review-1166.yaml in the scratchpad); post with fabric-pr post-review 1166, then leave draft until the owner's word and acceptance of the ADR.
- 09:55Z round 1 (partial, NOT coverage — tools bound to the InterWeave clone) posted on 419eeaf80: F1 vehicle_runs is Regime 1 not 3 (migration 0020, ADR-060 §2.6 r5; ADR-028's row moved to Regime 1), F2 PR 9's code marked as PR 9's (§2.5 closers/skip, §2.6 today = pin_unstamped + awaiting_boarding, §2.7 retreat, §2.8 2.1.0, §8 narrowed). Both fixed in one Answers commit. NEXT: the coverage review of #1166 must be dispatched from a gzapp-bound session (switch into a gzapp job); the plan's §27 sentence "vehicle_runs is Regime 3" is a pre-existing stale line, backend-dev's plan or mine — fix with the next plan touch.
- 2026-10-25 ~10:00Z OWNER: "merge 1166, interweave-deploy created, merge pr 9". For #1166 (4 work, 1 fix, head 57dbabf0b, draft): the gzapp-bound session dispatches the coverage review (brief: scratchpad review-1166.yaml of session 874290c4, regenerate with range 70b59e95c..57dbabf0b and the posted partial round as previous findings is NOT needed — it is a full review), posts it, fixes findings, `gh pr ready 1166`, then `fabric-pr arm 1166 --basis "the owner's word ('merge 1166', 2026-10-25 ~10:00Z)"`. PR 9 (#1167, 13 work) lands after it under backend-dev-02 by the count rule; the owner's word relayed (INFO 01a17276-7d60). interweave-deploy exists (INFO to p2p-01 01a17276-298e).
- 10:15Z #1166 head 08feb0919 (5 work, 1 fix, 1 merge — the j360 switch folded origin/main as d0be51c0c): §2.6 names the five legacy_fallback reasons (no_demand, legacy_issued, pin_no_run, pin_stamp_failed, mode_retreat), once per leg, the uncounted no-demand boarding stated (backend-dev-02 REPLY 01a17277). PR 9's local head a8c644df3 matches §2.5–§2.7 otherwise. The gzapp session (j360): review this head, post, fix, `gh pr ready 1166`, arm on the owner's word.

**2026-10-25 18:15Z (from the interweave session):** two supplies on `develop-qzapp/architect-cto-01/for/backend-dev-02/adr-079-references` off #1167's head 0f4e7e2e: 45d0b44eb (References line: StopDemandRetirement replaces TryCancelAsync, #1167 review F3) and e35ec3501 (§2.5/§2.7 bound to `StopCallTick.FinishesOnItsOwn`: a live directive not backed by the leg's attached demand's call finishes on the legacy path, vehicle_stale/vehicle_changed_line kept; #1167 round-2 F8). ADR-079 still Proposed → in-place edits, no history triple. backend-dev-02 folds both into #1167 (27 work, 6 fix); the owner asked for #1167 armed now. My j367 drives it after they fold.

*Observed 2026-10-25 (architect-cto)*

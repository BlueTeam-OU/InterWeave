---
role: "rust-ui-dev"
class: threads
topic: "j167-fleetdeck-focus-baseline"
description: "j167 FleetDeck focus-only (REQUEST 01a1797f from fabric-coordinator): code map, baseline numbers measured 2026-10-26 21:50, design direction"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8d3ae09496d90f2a
---

## j167 FleetDeck focus-only (REQUEST 01a1797f from fabric-coordinator): code map, baseline numbers measured 2026-10-26 21:50, design direction

j167 is herdr fork work. The fork is ~/projects/herdr, and its checkout sits on an old branch, so start a worktree from origin/master. There are two PRs, part 1 then part 2.

Map, at origin/master:
- Panes:
  - fabric_deck.py `_restore_account` (1169) builds harness, shell, status and agent panes.
  - `_arm` (1539) types `moveto <login> [--wait|--watch]` with `herdr pane run`.
  - The ssh itself is agent-fabric's moveto `--via ssh`; the deck never builds `ssh -i`.
- Focus signals:
  - herdr emits `tab.focused`, `workspace.focused` and `pane.focused`, through a socket `events.subscribe` or a plugin `[[events]]` hook. There is no "lost" event: a loss is the next tab.focused.
  - DECSET 1004 reaches only a tab's focused pane.
- fabric_view:
  - Its agent pane already fetches only when focused (`Refresher.lazy`).
  - Its loop is select at TICK_S 0.25 plus tick.
- Stop: `boot.hang_up` (boot.py:581) SIGHUPs a pane's foreground pgid, and `stop_all` is in fleet_deck.py:503.

Baseline, 2026-10-26 21:50 (load 39):
- ssh with fabric_deck: 86 (29 --wait, 29 --watch, 28 shell).
- user-1000 cgroup CPU over 10 s: 207%.
- 30 fabric_view processes: 79% together. Unfocused ones are about 2-3% each, single thread, no children, so the idle loop itself costs.

Direction for part 1:
- The controller subscribes to tab.focused. It arms shell and status in the focused tab and hangs up the previous tab's, then shows "paused: tab not focused".
- fabric_view stops ticking and rendering while unfocused.

Part 2 needs a design note to fabric-coordinator before any code. The launcher, moveto and bootstrap's units are theirs.
Related: [[release-disk-after-tests]]

Progress, 2026-10-26 ~23:30: part 1 is built on develop-qzapp/rust-ui-dev-01/feat/j167-focus-only-panes in worktree ~/projects/herdr-j167-focus. Not pushed and no PR yet.
- b0122b18 fabric-view rests unfocused. In a pty, 3.9 -> 0.2 wakes/s.
- 7ee2636b deck `_follow_focus`:
  - focus is read each look from `workspace list` (the focused workspace's active_tab_id);
  - decide_pending skips the shell and status panes of unfocused tabs;
  - `_pause` hangs up and re-queues; the `ending` set covers focus coming back before the hang-up lands, a real bug the new test found;
  - the Harness injects `hang_up`, so tests send no real SIGHUP.
- All 494 fleet-deck tests pass, and the rules are mutation-checked.
- Account side: 30 watch.py use 11% themselves and 103% in their children over 10 s, about 3.8% each.

PR is now #49 (#48 closed: subjects reworded to perf(fleet-deck), force-push refused, the user chose a new branch); review round 1 dispatched on 62264bef.
Next:
- the pull-requests skill, then open the PR (2 work commits, so arming needs the owner's word or the arming authority);
- the review dispatch;
- the after-numbers need the owner's deck restarted on the new code;
- then the part 2 design note to fabric-coordinator.

2026-10-26 late, #49 (2 work, 2 fix) at d6f88370:
- Review round 1 (dispatch 67a68a39) found F1–F6. All are fixed in aea40742 and d6f88370, mutation-checked; 501 tests pass.
- post-review REFUSED the report: the fabric-review MCP tools are bound to the session's clone (interweave), so a review dispatched from an interweave session cannot read a herdr diff or list its range.
- A counted review of #49 needs a session in the herdr working copy, via fabric-jobs switch to j167 → fabric-fresh.
- Commit subjects must be conventional (`type(scope): subject`, scripts/conventional_commits.py). The force-push to fix them was refused, so #48 was closed for #49.

Part 2 design sent as QUESTION 01a17a4f to fabric-coordinator, 2026-10-26.
- Design: a per-account headless FleetDeck server (`--session harness`) as a user unit whose pane runs `enter --wait`. enter-ssh gets a fifth word, "attach", which execs `terminal attach <pane>`. The deck hangs that ssh up off focus.
- Rejected:
  - saved machines and --remote: a whole-UI switch, and remote-client-bridge;
  - tmux: a second multiplexer and prefix;
  - dtach and abduco: not installed.
- Agent-fabric items 1-5 are theirs:
  1. the unit;
  2. a system-wide binary;
  3. enter-ssh attach, which amends ADR-048 §5 rule 4;
  4. moveto --attach;
  5. stop --all through the units.
- To verify: the nested guard (HERDR_ENV is not forwarded by ssh), and the attach's size and detach behaviour.
- Waiting on their yes before any code.

**2026-10-30:** INFO 01a18e96 from python-dev-03 says the agent-fabric half of FleetDeck part 2 is agent-fabric#435 (unmerged). My acceptance can run once the owner installs the binary. They assumed some herdr CLI output shapes (status server --json, workspace/pane list, pane process-info, terminal attach). Check those against the fork's real output when j167 resumes, and tell them about any difference.

*References: events, release-disk-after-tests*

*Observed 2026-10-30 (rust-ui-dev)*

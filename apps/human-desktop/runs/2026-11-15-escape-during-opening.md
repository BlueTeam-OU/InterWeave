# Desktop runs, 2026-11-15: Escape while a long reader opens, quieter host

The runs ux-designer-01 asked for (REPLY 01a1da48) on a quieter host,
after the 2026-11-14 runs (`2026-11-14-escape-during-opening.md`) had two
distinct-heading closes past 0.15 s at a load of 48-60. Same case, same
traced release build, same shapes and judge as that record's "How it was
run"; the case's code is that of PR #677 at e8bfe49ef.

The host: `develop-qzapp`, 10 CPUs, shared. The three runs went back to
back from 06:39Z once the one-minute load had fallen below 10; the load
(`/proc/loadavg`, 1/5/15 min) was 8.3/11.9/20.6 before run 1, 16.6/14.6/
20.5 after it, 12.8/14.0/19.5 after run 2 and 12.7/13.8/18.8 after run 3.
Runs 2 and 3 were therefore at one to one and a half times the CPUs, not
below them; none was on an idle host.

## Results

Escape to close, in ms; limits as before (1 s for the list, 0.15 s for
the stress shapes). The rows line is the `reader=drawn .. frame=untold`
line: the rows handed to the window, not a frame. Focus was on the row's
Open the whole message after every round that passed.

| Run | List plain opens (ms) | List rounds | distinct headings | distinct links |
|---|---|---|---|---|
| 1 | 46, 43, 31 | 39, 35, 11 | 37, 63, 28 | *void*, *void*, *void* |
| 2 | 35, 43, 39 | 30, 35, 29 | 25, 74, 15 | *void*, *void*, *void* |
| 3 | 56, 24, 27 | 28, 47, 24 | *void*, 76, 36 | *void*, *void*, *void* |

- Passed: 17 of 27 rounds: the list 9 of 9, distinct headings 8 of 8
  judged (15-76 ms).
- Failed: none.
- Void: ten. Distinct links nine of nine: its rows line came 391-497 ms
  after the press (run 1: 412, 445, 391; run 2: 409, 442, 399; run 3:
  428, 497, 428), before the Escape at 0.6 s. Distinct headings one, run 3
  round 1: rows line at 567 ms, 33 ms before the Escape.

## What this says about the 2026-11-14 runs

- The two late closes of distinct headings (192 and 312 ms) did not
  recur: eight judged rounds closed in 15-76 ms at a load of 8-17. That
  points at the host's load, as that record held possible; it does not
  prove a close past 0.15 s cannot happen on a loaded desktop.
- Distinct links opens faster here than the Escape time A5 sets for it:
  every round void. Under ux-designer-01's rule (more than a third of
  its rounds void), it goes back to them for a desktop time in A5. Their
  rule placed that time between 0.5 s and the shape's first screen; the
  shape's rows line came at 391-497 ms here, below 0.5 s, so that range
  is empty on this host and the time is theirs to set otherwise.

Not run, as before: the shipped build, Wayland, a screen reader during
the opening, and a frame-level check that no frame showed the bar.

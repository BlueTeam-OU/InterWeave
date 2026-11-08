---
role: "architect-cto"
class: threads
topic: "j513-spike-011-20k-figures-corrected-2026-11-04"
description: "j513 (SPIKE-011 20k figures from rust-ui-dev-01's #556): use the CORRECTED figures of REPLY 01a1a6f7 (Rust heap + SQLite heap, commit 9037d277), never REPLY 01a1a6e2's Rust-only ones"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9f8bc9fa59cc0da9
---

## j513 (SPIKE-011 20k figures from rust-ui-dev-01's #556): use the CORRECTED figures of REPLY 01a1a6f7 (Rust heap + SQLite heap, commit 9037d277), never REPLY 01a1a6e2's Rust-only ones

rust-ui-dev-01's REPLY 01a1a6e2 (queued as my j513) gave SPIKE-011's 20 000-row
figures from Rust's counting allocator alone; SQLite allocates with libc past
it (the review's F1 on InterWeave #556). Their CORRECTION 01a1a6f7-5e53
(2026-11-04 14:29Z, fixed in 9037d277 on #556) is the one to place: release,
one run, peaks as Rust heap + SQLite heap — start 31.9 ms, 0.02 + 3.53 MiB;
first page 2.4 ms, 0.23 + 0.02 MiB; the walk 34.8 ms, 3.14 + 0.02 MiB over 16
pages, ending at 1024 held rows with the oldest not reached (HELD_ROW_CAP).
The start's SQLite peak grows with the backlog (0.10, 0.42, 3.53 MiB at 200,
2000, 20 000 rows): indexed counts read every index page into the page cache,
bounded by SQLite's default per-connection cache_size; not measured past
20 000. The CI case gates the Rust peaks and the first page's SQLite peak and
records the start's SQLite peak ungated. Test paths unchanged.

**How to apply:** when j513 places the figures in SPIKES.md, read them from
#556 at or after 9037d277, and say both heaps.

**15:3xZ DELIVERED:** for/ branch
develop-qzapp/architect-cto-01/for/rust-ui-dev-01/j211-rows-and-spike-011-figure
at 9df29a900 off #556's head 9e0741c3d, three work commits (a31b89839 TM-56
row + plan §21; 6f7d66256 TM-45 row, Android as the limit, + plan §21;
9df29a900 SPIKES.md SPIKE-011 A 2026-11-04 both heaps + plan §29 item out of
the open list). REPLY 01a1a727-ae72 to rust-ui-dev-01. xtask checks 36/39,
the three not evaluated (cargo-deny, cargo-machete absent here). The job's
working copy stayed the j509 worktree (fabric-jobs project refuses a
re-point once a job has a branch), so the for/ branch was cut there and
the checkout goes back to the j509 branch at switch. Closes when 9df29a900
is an ancestor of origin/main (#556 merged with the fold).

*Observed 2026-11-04 (architect-cto)*

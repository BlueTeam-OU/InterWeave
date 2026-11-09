---
role: "rust-ui-dev"
class: threads
topic: "j174-connect-late-announce-pr496"
description: "j174 / PR #496: Connect's line said once on a late trust change (add-a-peer §5.1, §8); model said rules, the FocusNote line, the sync trap"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d230001e0fb8b8c5
---

## j174 / PR #496: Connect's line said once on a late trust change (add-a-peer §5.1, §8); model said rules, the FocusNote line, the sync trap

PR #496 (2026-10-27), branch develop-qzapp/rust-ui-dev-01/feat/j174-connect-late-trust-announce, base af371f4e.
Commits: 90d34e47 ui-model, 47839f60 ui-slint, 72099ee2 ui-model fix. 3 work, 0 fix: below the minimum of 8, so it needs the owner's word or architect-cto's AUTHORIZES.
- Model (`Connect`): `said` is bumped in three cases:
  - a return whose page did something counted;
  - a return whose line differs from the line when the page opened (`opener` holds `(focus, line_then)`);
  - `trust_read` changing the line while no page is up.
- `confirm` treats `trust.trusted_here` as trusted when no confirmed list exists. This is §5.1's "late answer made, no read back".
- `returned` confirms before it says the line, so the return never says "could not check" and then changes silently a turn later.
- TRAP: `connect_sync` is called from `peers_turn` with no page up. If `returned` confirms there, the late change lands before ModelSide::turn takes its Connect `before` snapshot, so nothing redraws. app-core root a_late_trust_answer_is_drawn_on_connect_by_the_turn_that_takes_it caught it, but only under the full `cargo xtask ci`. So `returned` is a no-op when nothing counted and no page closed.
- View: the line is a FocusNote with `focus-key` "connect-line" and `live: false`. When the window's focus-id is "connect-line", the render returns no announcement, because the new words are read where focus is. A `!moved` guard turned out unreachable (opening a page always moves focus into it) and was removed.
- ui-slint view tests: one View per #[test]. A second `view()` call in the same test panics with "platform already initialized".
- INFO 01a17ef9 to ux-designer-01: the line is a Tab stop now, which they can reverse. Evidence is headless only: Connect is Android-only and Android has no screen-reader bridge.
Related: [[j155-connect-screen]] [[j157-unasked-read-focus]]
- Review round 1 (5674483150) found F1 and F2, both P3: stale test docs, and "never" comments with no test. An adjudication judged both real. They were fixed in 41ce9820 and 44123456: the Show-return test, the liveness Off assertion, and the same-turn Add test. The re-review (5674588385) is clean at 44123456.
- Arming REQUEST 01a17f41 to architect-cto-01 (seq 628483) asks for AUTHORIZES #496@44123456. Once it arrives, arm with --authorization <id>. Never push onto the PR after that. Status: 3 work, 2 fix.
- Brief tips: lenses are restricted to the fabric's list (`accessibility` is not one). For range, use the merge base, not origin/main after a fetch. A re-review takes no previous_findings: it reads the last posted review.
- MERGED as ee88a1ff (2026-10-27), armed on AUTHORIZES 01a17f42. j174 delivered.

*References: j155-connect-screen, j157-unasked-read-focus*

*Observed 2026-10-27 (rust-ui-dev)*

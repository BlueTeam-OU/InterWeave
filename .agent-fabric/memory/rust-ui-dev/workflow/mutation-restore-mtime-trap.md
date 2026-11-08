---
role: "rust-ui-dev"
class: workflow
topic: "mutation-restore-mtime-trap"
description: "A mutation restore can silently be wrong: mv/cp -p keeps the old mtime (cargo runs the mutant), and a sed pattern restore rewrites other lines that match the mutant text"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 276d4a0167066206
  - a07c4ab20b5b73b6
---

## A mutation restore can silently be wrong: mv/cp -p keeps the old mtime (cargo runs the mutant), and a sed pattern restore rewrites other lines that match the mutant text

In a mutation check, restoring the source with `mv file.bak file` (or `cp -p`) gives the file back its original mtime. That is older than the mutated build's artifacts, so cargo sees nothing to rebuild and keeps running the MUTATED code. The next "green" or "red" run then says nothing about the real code. Seen on 2026-10-22 (#359, `card_photo.rs`): after the restore, tests kept failing exactly as they had under the mutation.

**Why:** cargo decides freshness by mtime, not by content.

**How to apply:**
- Restore by rewriting the file. A Python `open(p, "w").write(orig)` in a `try/finally` gives it a fresh mtime, or `touch` it after an `mv`.
- Run the suite once more after every restore before trusting it.
- **Never restore by pattern substitution.** On 2026-10-23 (#397, commit 002ad801), a mutant's text (`R1 || self.can_restore()`) was also the body of a sibling function (`warns_active_elsewhere`). The sed restore rewrote both lines, and the shipped code dropped the ADR-0043 warning on K1. The blind review caught it as a P1, with four tests failing at the head. Mutate and restore by line number (or `git stash`/`git checkout -- file` when nothing else is uncommitted), and compare the file's sha256 before and after the restore.
- **A mutation that did not land reads as "the test missed it".** On 2026-10-25 (#437) it happened twice: a python replace that asserted the old text was present failed on indentation, and a sed line found by `grep | head -1` hit the same line in an EARLIER test of the file. Both runs came back green, which looked like a weak test. Before the test run, check that the diff shows the mutation and that it sits inside the test's own function (find the line with awk, starting from the test's `fn` line).
- Related: [[pgrep-f-matches-itself]].

**Never restore a mutant with `git checkout -- <file>`** when the file carries uncommitted work: it goes back to HEAD
and the work goes with it (2026-10-28, WindowCursor::at, caught only by the sha check). Copy the file aside before
the mutation and copy it back, then compare the sha.

*References: pgrep-f-matches-itself*

*Anchors: `crates/human/app-core/tests/card_photo.rs::a_received_photo_whose_header_lies_about_its_size_is_dropped_before_the_decode`*

*Observed 2026-10-22 (rust-ui-dev)*

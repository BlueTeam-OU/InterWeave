---
role: "p2p-network-dev"
class: workflow
topic: "run-file-figures-from-quoted-lines"
description: "an evidence run file's durations must be computed from lines it quotes; a script-clock zero beside journal lines cost three review rounds on #554"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e604bce27f342fff
---

## an evidence run file's durations must be computed from lines it quotes; a script-clock zero beside journal lines cost three review rounds on #554

On #554 (j144, merged 2d1e9853, 2026-11-04), the run file
`packaging/linux/evidence/2026-11-04-gzcoord-units-and-cross-uid-port.md`
had two kinds of figure. The "active" ones were measured from the
script's monotonic clock just before its `systemctl` call, using
ActiveEnterTimestampMonotonic. The "serving" ones were measured from a
journal line. The script's clock was 0.166 s ahead of the journal's
`Sent signal SIGKILL` line, so the file said active at 5.391 s but
serving at 5.27 s, which is impossible. The review caught it, and the
fix took three rounds: the figures, then the offsets I added to explain
them (F4), then a rounding (0.45 → 0.44).

**Why:** a reader can only check a figure against what the file quotes.
An unquoted zero makes the record unverifiable, and mixing two zeros
makes it contradict itself.

**How to apply:**
- In a units or timing run, have the script print every timestamp it
  measures from, and quote those lines.
- Or compute every duration from the journal lines you excerpt, and
  round from the quoted digits.
- Before committing, recompute each figure from the file's own text.
- Also: never commit during a detached `xtask ci`. Its version test
  compares the built binary's commit with HEAD
  (`apps/claude-channel/tests/version.rs`). See
  [[background-verify-reads-the-live-tree]].

*References: background-verify-reads-the-live-tree*

*Anchors: `packaging/linux/evidence/2026-11-04-gzcoord-units-and-cross-uid-port.md::Every figure here is from the journal lines quoted`, `apps/claude-channel/tests/version.rs::version_is_the_workspace_version_and_the_commit`*

*Observed 2026-11-04 (p2p-network-dev)*

---
role: "rust-ui-dev"
class: workflow
topic: "authorizes-binds-head"
description: "an architect-cto AUTHORIZES names one PR head; new work waiting for it goes on a fresh branch, never pushed onto the authorized PR"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b5139394129f3d3b
---

## an architect-cto AUTHORIZES names one PR head; new work waiting for it goes on a fresh branch, never pushed onto the authorized PR

architect-cto's `AUTHORIZES: <repo>#<n>@<sha>` holds "for head <sha> and no later push" (01a17d74, #484, 2026-10-27). `fabric-pr arm <n> --authorization <message-id> --basis ...` takes it; the dry-run refuses without `--authorization`.

**Why:** a commit pushed onto the PR before arming voids the word, so the request has to be made again.

**How to apply:** when the next job starts while an authorized PR is still unarmed, set its edits aside first (save the diff to ~/scratch), then arm. Put the new work on its own branch off origin/main, and open its PR after the armed one lands. j161 46a96518 was done this way. Related: [[arming-tool]]

*References: arming-tool*

*Anchors: `apps/human-android/android/app/src/main/kotlin/org/interweave/human/MainActivity.kt::askNotifications`*

*Observed 2026-10-27 (rust-ui-dev)*

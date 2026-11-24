---
role: "language-culture"
class: workflow
topic: "head-moved-mid-pass"
description: "when the caller's PR head moves inside my lines during a pass, base the for/ branch on the new head after the cold read"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cac15b41d0b2b4d0
---

## when the caller's PR head moves inside my lines during a pass, base the for/ branch on the new head after the cold read

On #661 j311 (2026-11-13), ux-designer-01 moved the PR head (b417dd906 to 0a0e13407) inside the lines I was passing, and offered to fold my pass on the old base. I waited for the cold read to finish, then moved my for/ branch to the new head and applied the pass there, with the new sentences included (commit 0adeb14d2).

**Why:** a fold onto changed lines makes the caller choose between two meanings and my sentences a second time. That is the conflict j310 had (merge c60473b49).

**How to apply:** never move the tree while a cold read reads it. After the read, reset the for/ branch to the new head, apply the edits from a scratch copy, and say the base in the REPLY. Related: [[supply-base-moves-merge]] (a branch already pushed is merged, never rebased).

*References: supply-base-moves-merge*

*Anchors: `design/2026-10-22-add-a-peer.md::Revised 2026-11-13 (ux-designer-01's j124)`*

*Observed 2026-11-13 (language-culture)*

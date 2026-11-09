---
role: "architect-cto"
class: rationale
topic: "bounds-wide-cluster-pieces-by-measure-2026-11-07"
description: "HUMAN-CHAT Bounds A 2026-11-07 ruled and DELIVERED — a wide cluster (East_Asian_Width W/F or emoji presentation) counts as two in the figures, a drawn line past the 8,000 px ceiling is split by MEASURE never by a count, U+FDFD a known…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a4d78b4c36715ca5
---

## HUMAN-CHAT Bounds A 2026-11-07 ruled and DELIVERED — a wide cluster (East_Asian_Width W/F or emoji presentation) counts as two in the figures, a drawn line past the 8,000 px ceiling is split by MEASURE never by a count, U+FDFD a known limit of the figures only; contract triple 0b0b11c1a on the for/ branch for rust-ui-dev-02 j95 PR #583, sha sent (REPLY 01a1b6bb-019e)

rust-ui-dev-02 OBSERVATION 01a1b6b2-3e5e (2026-11-07, desktop debug build
under Xvfb at scale 4, main 20ce43c8c): pieces of 1,600 clusters draw ONE
text item of 9,981 logical px for 2,000 "W" or CJK U+6F22, 13,947 for emoji,
17,434 for U+FDFD at 200 % text, 360 dp (control "x": 5,405); a whole body of
2,000 U+FDFD has a row of 87,236 px (~117 Pixel viewports). No panic: text
takes the vector path, fills tiled. INFO 01a1b6b5-26ce: weighted pieces fix
W/CJK/emoji but U+FDFD stays past 8,000. ux-designer-01 INFO 01a1b6b4-47f2
proposed: a cluster with East Asian Width W/F or emoji presentation counts as
two (Unicode properties, not the font), U+FDFD a known limit.

Ruled (DECISION 01a1b6b7-afbe to rust-ui-dev-02, INFO 01a1b6b7-b4b4 to
ux-designer-01): (1) the invariant stands; it is kept by MEASURE — a drawn
line past the ceiling is split by its measured extent in the layout, never
by a count (their (c) for pieces); a failed measurement is a defect of the
split, no longer "make pieces smaller"; (a) and (b)-in-pieces declined: a
count predicts a font. (2) the FIGURES take the weight (W/F or emoji
presentation = two) in every count incl. the pieces' 1,600 ceiling; cut
still width/scale independent. (3) U+FDFD a known limit of the figures only
(figure × widest glyph = a row's height, tiled); the invariant covers it.
(4) a tall whole body's row is A3's, the design's, not a contract condition.

STATE 2026-11-07 (j549): DELIVERED. Contract triple committed as
0b0b11c1a966881e111ce8538b214d8eb7e1a67b (Kind: work, validate_adr_index
OK) on develop-qzapp/architect-cto-01/for/rust-ui-dev-02/pieces-by-measure
off origin/main 20ce43c8c, pushed; REPLY 01a1b6bb-019e to rust-ui-dev-02
(IN-REPLY-TO 01a1b6b2-3e5e) names it; their j95 PR #583 (draft, head
6a5394c8, body already names the for/ branch to fold unrebased) carries
it. Closes when the sha reaches origin/main. design/2026-11-04-long-
message.md lines 470/644 still say "a failure makes the pieces smaller":
ux-designer-01's, informed by INFO 01a1b6b7-b4b4. The job's working copy
is the j539 worktree, so the for/ branch was checked out THERE (the job
hook refuses a commit in another worktree, ADR-037 rule 3).

**Why:** a count of clusters can carry a person's figure (Unicode properties are stable) but never a renderer's pixel invariant (the font decides); the invariant is kept where the extent is known, the layout.
**How to apply:** when a figure and an invariant share one mechanism, split them: the figure by a text property, the invariant by measurement. See [[j532-drawn-lines-item-bound-2026-11-05]], [[j531-per-cluster-drawn-bound-2026-11-05]].

*References: j531-per-cluster-drawn-bound-2026-11-05, j532-drawn-lines-item-bound-2026-11-05*

*Observed 2026-11-07 (architect-cto)*

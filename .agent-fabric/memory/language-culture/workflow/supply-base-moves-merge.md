---
role: "language-culture"
class: workflow
topic: "supply-base-moves-merge"
description: "When a caller moves the base of a pushed for/ supply branch, merge the new base in; never rebase or force-push"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 91a31c10fef01335
---

## When a caller moves the base of a pushed for/ supply branch, merge the new base in; never rebase or force-push

A supply on `<host>/<login>/for/<caller>/<what>` is folded unrebased by
the caller. When the caller asks for the supply "based on" a newer commit
after mine is already pushed, and that commit descends from my base, I
merge it into the for/ branch (`git merge <sha>`) and add the new commit
on top: no force-push (CLAUDE.md §9 forbids it unasked), and the fold
still works. Done for ux-designer-01 on InterWeave's contact cards,
2026-10-21: 3f693cb5, merge 98e38cf5 of 2569ed90, 97ae68be; folded as
afc181df, and the caller then asked "merge, as you did" for the next base.
Before merging, check `git merge-base --is-ancestor <old> <new>`; if the
new base does not descend from mine, ask the caller.
Related: [[check-commit-before-reply]]

*References: check-commit-before-reply*

*Observed 2026-10-21 (language-culture)*

---
role: "rust-services-dev"
class: solution
topic: "spike-001-disagreement-list-amended-marks"
description: "SPIKE-001 README's \"Where the architecture disagrees\" list keeps superseded bullets, each ending *Amended <date>*"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 68fba6221f37a4d8
---

## SPIKE-001 README's "Where the architecture disagrees" list keeps superseded bullets, each ending *Amended <date>*

Since #532 (merged 2026-11-03), `spikes/spike-001/README.md`'s "Where the architecture disagrees" list opens with a rule: each bullet quotes its sentence as the document stood when the section naming it was written, and a bullet ending `*Amended <date>*` quotes a sentence the document has since replaced, kept as the record. Nine bullets carry the mark (2026-10-03, 2026-10-24, 2026-11-03), dated by `git log -S` on the named document.

**Why:** the list had silently gone stale — 9 of 14 bullets quoted text the architecture no longer held, three of them from #531, which amended the documents in the same PR without touching the list. Deleting them lost the record (a review found that, F2 on #532).

**How to apply:** when a new S-section adds a bullet, quote the document's current text verbatim; when an amendment lands from a fact, mark the bullet rather than delete or rewrite it. A script that greps each bullet's quotes in its named document finds the unmarked stale ones in one pass.

*Anchors: `spikes/spike-001/README.md::A bullet ending *Amended*`*

*Observed 2026-11-03 (rust-services-dev)*

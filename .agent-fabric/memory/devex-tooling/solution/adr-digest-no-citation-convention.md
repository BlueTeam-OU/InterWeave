---
role: "devex-tooling"
class: solution
topic: "adr-digest-no-citation-convention"
description: "why validate_adr_index checks \"entry edited\" not \"amendment cited\" — the digest has no citation convention"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6cd4d8aff899f06f
---

## why validate_adr_index checks "entry edited" not "amendment cited" — the digest has no citation convention

InterWeave's ADR-DIGEST.md has no amendment-citation convention: entries cite as
`Amendment D`, `A D`, `D amendment`, `amended D`, or fold with no date (ADR-0048
asks a current-state summary only). Same-day amendments exist (ADR-0041, two on
2026-10-19, the first withdrawn and rightly absent from the entry), so a count of
dated mentions against Amendments-table rows fails correct trees. The guard added
for that gap (j163, PR #307) is diff-scoped instead: AMENDED fails an ADR whose
table gained a (date, title) key since the merge-base with origin/main while its
`### NNNN` entry is byte-identical. Self-tests pass `--base none` / `--base HEAD`
because the tool self-tests clone is depth 1 with no origin/main; tree-checks has
fetch-depth 0. A content check needs architect-cto to set a convention in ADR-0048.

*Anchors: `tools/checks/validate_adr_index.sh::AMENDED`, `tools/checks/validate_adr_index.sh::--base none`*

*Observed 2026-10-19 (devex-tooling)*

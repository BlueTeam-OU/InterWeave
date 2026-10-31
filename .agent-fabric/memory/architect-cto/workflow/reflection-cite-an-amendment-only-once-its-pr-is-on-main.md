---
role: "architect-cto"
class: workflow
topic: "reflection-cite-an-amendment-only-once-its-pr-is-on-main"
description: "A lane cites an amendment of mine (\"ADR-NNN §x (A date)\") only once that amendment's PR is merged, or its own PR arms after mine; the blind review reads origin/main, so a citation that resolves only on an open branch returns as a P2 every…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6c2e9d97aedd1776
---

## A lane cites an amendment of mine ("ADR-NNN §x (A date)") only once that amendment's PR is merged, or its own PR arms after mine; the blind review reads origin/main, so a citation that resolves only on an open branch returns as a P2 every round

Three times in 2026-09-16..18 (gzapp) a lane cited an amendment that
existed only on my open PR: ten comment sites citing "ADR-025 §2.6 (A
2026-09-17)" when that text was not on origin/main (01a0ae23-4b8f);
the ruling "land yours after #822 is on main, cite it" (01a0ae79-f451);
"#844 arms AFTER #842 is on main" (01a0b07b-412e). The rule: an
amendment is citable when its merge commit is on main; until then the
consuming PR names the PR and head it depends on and arms after it.
Sibling of [[landed-means-merge-commit-on-main]]. Verified 2026-10-22:
no gzapp skill or ADR README states a cite-after-merge rule.

*References: landed-means-merge-commit-on-main*

*Observed 2026-10-22 (architect-cto)*

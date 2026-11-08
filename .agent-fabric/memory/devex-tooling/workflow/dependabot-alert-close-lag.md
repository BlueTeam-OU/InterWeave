---
role: "devex-tooling"
class: workflow
topic: "dependabot-alert-close-lag"
description: a Dependabot alert closes ~30 min after the fix merges, well after the SBOM shows the patched version; poll, never dismiss
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 74218a79fd606617
---

## a Dependabot alert closes ~30 min after the fix merges, well after the SBOM shows the patched version; poll, never dismiss

InterWeave Dependabot alert 22 (brotli, pip, tools/requirements.txt):
#415 merged 13:24:01Z on 2026-10-24; the dependency-graph SBOM listed
brotli 1.2.1 by 13:45; the alert's updated_at stayed at its pre-merge
value until it flipped to fixed at 13:54:56Z — 31 minutes after merge.

**Why:** a "still open" read in the first half hour is lag, not a
missed manifest; dismissing by hand is the owner's call.
**How to apply:** after a fix merges, check `git grep` on origin/main
for other pins and the SBOM (`gh api .../dependency-graph/sbom`), then
poll `dependabot/alerts/<n>` once a minute for up to an hour before
calling it stuck. Deliver the job only on `state=fixed`.

*Observed 2026-10-24 (devex-tooling)*

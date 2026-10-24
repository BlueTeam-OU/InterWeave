---
role: "p2p-network-dev"
class: workflow
topic: "re-review-brief-needs-card-base-and-previous-findings"
description: "fabric-review brief refuses a re-review whose base is not on origin/main unless card_base names the first round's base, and refuses any re-review without previous_findings (a file)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a88a2d7e9084ef5a
---

## fabric-review brief refuses a re-review whose base is not on origin/main unless card_base names the first round's base, and refuses any re-review without previous_findings (a file)

On 2026-10-19 (#305) `fabric-review brief` refused two re-review requests:

- **A base not on main.** A range whose base is a PR commit (`143fe917..ff3c6627`) is refused with "base ... is not an ancestor of origin/main". The project review card is read from the base, so the request needs `card_base: <full sha of the first round's base, on main>` (there, `dc897c90...`).
- **No previous findings.** `previous_findings:` is required in every re-review: the path to the previous round's posted report.

Two other habits from the same PR:

- **Posting needs the head.** `fabric-pr post-review` refuses a report with no dispatch block unless it is given `--head <sha>`, so post the review class's report with its `agent-fabric-dispatch v1` block first.
- **Reviewers can't clean up.** A reviewer that builds leaves a 5–6 GB CARGO_TARGET_DIR in the session scratchpad, because its hook refuses its own `rm -rf`. The dispatcher measures it with `du -sh`, then removes it.

See [[review-dispatch-shape-in-interweave]].

*References: review-dispatch-shape-in-interweave*

*Observed 2026-10-19 (p2p-network-dev)*

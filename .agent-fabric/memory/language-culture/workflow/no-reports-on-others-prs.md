---
role: "language-culture"
class: workflow
topic: "no-reports-on-others-prs"
description: Do not report the status, counts or readiness of pull requests I do not own
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cc2b358598ce9a9a
---

## Do not report the status, counts or readiness of pull requests I do not own

Do not report another agent's PR (state, counts, gate, armed or not) to
the owner. As a supplier, report only my own artifact: the commit I
pushed, on which branch, verified.

**Why:** the owner, 2026-10-10: "you should not report pr that you don't
own" — after I listed #1075 (web-dev-01's) as gate-ready and unarmed,
which invited the question why I was not arming it.

**How to apply:** after a supply, say "pushed <sha> onto <caller>'s
branch, verified" and stop. A PR's status is its owner's to report. The
Stop hook's demand for counts applies only when I state a PR's status, so
do not state it.
Related: [[gzapp-english-holder]]

*References: gzapp-english-holder*

*Observed 2026-10-10 (language-culture)*

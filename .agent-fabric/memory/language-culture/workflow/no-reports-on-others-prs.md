---
role: "language-culture"
class: workflow
topic: "no-reports-on-others-prs"
description: Do not report the status, counts or readiness of pull requests I do not own
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 08dbf40921696f59
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
do not state it. On 2026-11-17 (j337) a resume summary that only named the
job's title ("English pass on #693's dd38a8df4") was read as a status and
refused; I answered by declining to report #693. In a status line, name the
caller's commit and my branch, not the caller's PR number.
Related: [[gzapp-english-holder]]

*References: gzapp-english-holder*

*Anchors: `agent-fabric:runtime/claude-code/hooks/pr-counts.py`*

*Observed 2026-11-17 (language-culture)*

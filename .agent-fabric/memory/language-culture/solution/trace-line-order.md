---
role: "language-culture"
class: solution
topic: "trace-line-order"
description: "before wording \"X followed by Y\" over ui-trace lines, check the order trace.rs writes them in"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - aa6c32d7d8773962
---

## before wording "X followed by Y" over ui-trace lines, check the order trace.rs writes them in

The human client's ui-trace writes one line per changed part, in a fixed
order inside one render: `page=` before `connect=`
(`crates/human/ui-slint/src/trace.rs`, `changes`). So when Connect opens,
`page=connect` comes before `connect=open`. The e2e driver reads the last
line of each kind (`tests/android-e2e/src/screen.rs`, `last_of`). The
first `connect=` line after a C4/R4 continue is `connect=owed`.
On #633 (def7feafd, 2026-11-12) the spec said "connect=open followed by
page=connect", which a correct build fails. I asked the author (Q1/Q2 in
REPLY 01a1ce72) and did not reword it. Related: [[read-the-cited-section]],
[[quote-a-condition-verbatim]].

*References: quote-a-condition-verbatim, read-the-cited-section*

*Anchors: `crates/human/ui-slint/src/trace.rs::changes`, `tests/android-e2e/src/screen.rs::last_of`*

*Observed 2026-11-12 (language-culture)*

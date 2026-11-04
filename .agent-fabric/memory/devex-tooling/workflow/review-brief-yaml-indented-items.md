---
role: "devex-tooling"
class: workflow
topic: "review-brief-yaml-indented-items"
description: "fabric-review brief refuses a request whose list items sit at column 0; indent `- item` under its key"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7bb52b07fca42dde
---

## fabric-review brief refuses a request whose list items sit at column 0; indent `- item` under its key

`fabric-review brief <req.yaml>` reads a flat YAML subset: a `key:` block's
`- item` lines must be indented (`  - item`). At column 0 it fails with
"request line N: not `key: value`", naming the first item, which reads like a
colon problem in that item and is not. Also: a PR re-review takes no
`previous_findings` (it reads the last posted review). Seen on #455, j239,
2026-10-25.

**Why:** the error names the wrong cause; two re-renders were spent on it.
**How to apply:** write lists indented two spaces from the start. See [[review-brief-card-base]], [[rereview-reads-posted-review]].

*References: rereview-reads-posted-review, review-brief-card-base*

*Observed 2026-10-25 (devex-tooling)*

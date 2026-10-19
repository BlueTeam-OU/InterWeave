---
role: "language-culture"
class: workflow
topic: "cold-read-on-sonnet"
description: "Cold reads are Explore dispatches on model sonnet, not opus (since 2026-10-10)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2c544943c97ae2de
---

## Cold reads are Explore dispatches on model sonnet, not opus (since 2026-10-10)

A cold read (a fresh reader with no context judging a finished text) is a
read-only Explore dispatched with model "sonnet", never "opus". The
locale-worker (other locales) stays on opus per agent-fabric ADR-027.

**Why:** fabric-coordinator DECISION 01a125b7, on the owner's word
("2 ok", 2026-10-10 12:09Z): every opus Explore prompted the owner, both
accounts were near 77% of the weekly allowance, and a cold read is
comprehension and naturalness, which sonnet does well.

**How to apply:** dispatch cold reads with model sonnet. If a sonnet read
misses something a stronger read would have caught, report it to
fabric-coordinator with the example; the rule changes on evidence.
Reviews (code-review class) are unchanged: fable.
Related: [[cold-read-without-meanings]]

*References: cold-read-without-meanings*

*Observed 2026-10-10 (language-culture)*

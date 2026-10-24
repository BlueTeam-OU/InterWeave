---
role: "architect-cto"
class: rationale
topic: "reflection-a-newly-declared-default-is-the-value-that-ships-today"
description: A default declared for an existing or optional member is the value served today, not the one the author prefers, since a different default silently changes every record written without the member on the next release
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 09b18d8bb48241ae
---

## A default declared for an existing or optional member is the value served today, not the one the author prefers, since a different default silently changes every record written without the member on the next release

gzapp 2026-10-09: correcting my own DECISION (…8ff23ebcf708): "a
bottom default would move every unplaced splash on the next release
with nobody having chosen it" (…de6c6285c09b); "declares default on the
five configured members with today's served values, which become the
contract's" (…e2e84fde910f); shift-traversal 1.1.0 "default: 10", the
code default at the time (…6f787395377e). ADR-044 says a served knob
declares its default, not which value; a candidate sentence for §2.5.

*Observed 2026-10-23 (architect-cto)*

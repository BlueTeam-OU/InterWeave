---
role: "language-culture"
class: workflow
topic: "holder-addenda-installed"
description: "My English-holder charter and brief addenda (agent-fabric ADR-060) live in my store, not the shared charter; edit them through fabric-coordinator"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a3a389f7bf55bd52
  - d01b55e884ccab8f
---

## My English-holder charter and brief addenda (agent-fabric ADR-060) live in my store, not the shared charter; edit them through fabric-coordinator

Since 2026-10-17 (store commit 3b780f6, installed by fabric-coordinator on
the owner's approval) the English-specific rules are my holder addenda,
rendered after the shared language-culture charter and brief:
charter-addendum (global English; no locale-worker, one text sent once, no
rendering, notes English and unmeasured; everything else binds) and
brief-addendum (cold read as the worker's stand-in, judge its findings,
if/else whole, global-English checks, templates). Stage 5 removes the
English paragraph from the shared charter.

**How to apply:** a change to them is a new draft, checked with
addenda_check.validate_set and sent to fabric-coordinator, who installs
a copy set aside by the coordinator, sha256-checked; never write
settings/roles/ in my store myself.
Related: [[keep-else-whole]], [[cold-read-without-meanings]]

*References: cold-read-without-meanings, keep-else-whole*

*Observed 2026-10-19 (language-culture)*

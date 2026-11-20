---
role: "architect-cto"
class: workflow
topic: "memory-store-directory-is-the-project-scope"
description: "A memory's project is the store directory it sits in (harvest stamps the project from the launch directory's working copy), so text about a sibling project, or a production host's identifiers, written into the InterWeave store lands in…"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6528275f7e05709c
---

## A memory's project is the store directory it sits in (harvest stamps the project from the launch directory's working copy), so text about a sibling project, or a production host's identifiers, written into the InterWeave store lands in InterWeave's PUBLIC corpus at the next drain

fabric-coordinator's REQUEST 01a1bc26 (2026-11-08, from InterWeave #595's round-1 F3/F4): four of my InterWeave-store memories carried sibling-project PR threads and arming words, and a production host's node name, principal, token id, bridge and route layout and a local plan path. The drain had to hold them out by hand; the sources would have brought them back.

**Why:** the InterWeave repository is public and the corpus is committed into it; the sibling project is never named in another project's slices (memory/README.md "sibling projects"); a host's identifiers are read from the control plane, never remembered.

**How to apply:** a fact about another project goes in THAT project's store (`~/.claude/projects/<its launch-dir slug>/memory/`), even when learned in an InterWeave session; a thread that crosses both is split, one file per store, with no cross-project pointer by name. Before a memory names a node, a principal, a token, an address or a path outside the repository, drop it and say where it is read from instead. Grep the store for `gzapp ?#` and identifiers before a drain is asked. Fixed 2026-11-08: the four sources moved or split, REPLY 01a1bc28-9ee6.

*Observed 2026-11-08 (architect-cto)*

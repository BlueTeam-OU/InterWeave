---
role: "fabric-coordinator"
class: workflow
topic: "drain-cross-corpus-slices-both-ways"
description: "a project-half drain carries other repositories' slices in BOTH directions; a slice held from one half may already be superseded in the other"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - c9a809a5ad396399
---

## a project-half drain carries other repositories' slices in BOTH directions; a slice held from one half may already be superseded in the other

The 2026-11-08 drain's InterWeave half (#595, j654) took in two of architect-cto's gzapp
rationale slices (gzapp ADR-020 client abort, the gzapp analyzer-warnings ratchet). The
bundle was InterWeave's, but those slices were about gzapp. gzapp#1231 F9 had held two
InterWeave slices the other way. Those two, from `rehome-to-interweave`, were earlier
"awaiting amendment" versions of decisions the InterWeave bundle already carried as landed
(j528, j531). Adding them would have duplicated those decisions.

The same half also carried two generated placeholder descriptions ("<slug> (threads)") over
main's good ones. A description holding a YAML `\n` escape split its heading and drifted the
INDEX, which lint reported (j670).

**Why:** an author's bundle is keyed by the working copy they were in, not by the claim's
subject, so either half can hold the other's slices.

**How to apply:** scan every new slice's description for another repository's name, then move
the matches to `rehome-to-<project>` and scrub them from `last-drain-report.json`. Before adding
a rehomed slice, compare it with this half's slices on the same decision. Restore a placeholder
description from main. See [[drain-run-the-project-tree-checks-first]] and
[[drain-rerun-resets-the-fabric-tree]].

*References: drain-rerun-resets-the-fabric-tree, drain-run-the-project-tree-checks-first*

*Anchors: `agent-fabric:tools/fabric/assemble.py::--working-copy`*

*Observed 2026-11-08 (fabric-coordinator)*

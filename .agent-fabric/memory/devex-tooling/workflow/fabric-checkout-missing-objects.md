---
role: "devex-tooling"
class: workflow
topic: "fabric-checkout-missing-objects"
description: "fabric upgrade fails in git fetch on a missing object — refetch, then rebuild the commit-graph; leave the move to the upgrade"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 2ad0cf3549dfc50e
---

## fabric upgrade fails in git fetch on a missing object — refetch, then rebuild the commit-graph; leave the move to the upgrade

2026-11-01: `fabric-ctl devex-tooling upgrade fabric` failed in git fetch ("missing object", suggests
`git fetch --refetch`). In ~/projects/agent-fabric the HEAD commit itself was unreadable
(`git cat-file -t HEAD` "could not get object info", `git status` "bad object HEAD"), one blob missing, and
`.git/objects/info/commit-graph` named commits absent from the store. Not partial, no alternates, not shallow.
Repair: `git fetch --refetch origin`, then `rm -f .git/objects/info/commit-graph; rm -rf
.git/objects/info/commit-graphs; git commit-graph write --reachable`; `git fsck` then shows only dangling.
**How to apply:** do not fast-forward the checkout yourself (read-only lane); reply to fabric-coordinator
and let it re-run the upgrade. Cause of the loss was not established.

*Observed 2026-11-01 (devex-tooling)*

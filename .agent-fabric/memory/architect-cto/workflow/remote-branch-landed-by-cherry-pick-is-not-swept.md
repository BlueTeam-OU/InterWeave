---
role: "architect-cto"
class: workflow
topic: "remote-branch-landed-by-cherry-pick-is-not-swept"
description: "an own origin branch whose commit landed under another sha stays after fabric-branches --sweep; how to prove and delete it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 41b03f30758cd7fc
---

## an own origin branch whose commit landed under another sha stays after fabric-branches --sweep; how to prove and delete it

`fabric-branches --sweep` deletes an own branch on origin only when its tip is an ancestor of the default branch. A supply commit the caller cherry-picked or folded lands under another sha, so the branch counts above 0 and stays for ever (j680, 2026-11-14: the j616 and j653 branches, landed via #624 and #650).

**Why:** the sweep cannot prove a rewritten commit merged; the person can.

**How to apply:** on the owner's word naming the branch: `git cherry origin/main <tip>` must print `-` for every commit (patch already on main), `gh pr list --head <branch> --state all` must show no open PR, then delete with a push leased to the tip that was checked (`--force-with-lease=refs/heads/<b>:<sha> :refs/heads/<b>`) and confirm with `git ls-remote`. Never without the word, never another login's branch.

*Anchors: `agent-fabric:bin/fabric-branches::--sweep`*

*Observed 2026-11-14 (architect-cto)*

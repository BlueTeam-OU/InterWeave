---
role: "rust-services-dev"
class: workflow
topic: "blocked-job-refuses-arm"
description: "after blocking a job on an arming request, `fabric-jobs start` it before fabric-pr arm, or the hook refuses"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 3993a68246ca2e7c
  - bb3a03a70ece8928
---

## after blocking a job on an arming request, `fabric-jobs start` it before fabric-pr arm, or the hook refuses

Blocking the active job on a GZCoord request (`fabric-jobs block j<n> --on-request <id>`) takes it out of `active`; when the authorization arrives, `fabric-pr arm` is then refused by the PreToolUse hook ("This session worked job j<n>, which has left active ... run fabric-jobs switch"). Run `fabric-jobs start j<n>` first, then arm — in a SEPARATE tool call: the hook checks the job state before the whole command line runs, so `fabric-jobs start j<n>; fabric-pr arm …` in one call is still refused (InterWeave #541). Seen on InterWeave #539, 2026-11-03.

Also on #541: before arming, compare the AUTHORIZES header with the head yourself — architect-cto's first word carried `@2c8de9f0d` (a stray digit), no prefix of the head, and arm refused it; a corrected REPLY was needed.

Also seen there: `fabric-pr arm` refuses when GitHub answers 500/504 on the settings repository's arming policy ("the count rule is not applied to a guess"); a retry after ~30 s went through.

Hit a third time on #557 (2026-11-04) with this memory loaded: the first act on the authority's delivery is `fabric-jobs start j<n>` alone, before writing any arm command.

A DELIVERED job does the same (InterWeave #684, 2026-11-16): j104 was delivered while #684 sat unarmed under the count, and the owner's later "arm 684" was refused until `fabric-jobs start j104` brought it back to active. Once the PR merged, `deliver` refused ("merged: the job is done") and `fabric-jobs done j104 <repo>#<n>` closed it.

**Why:** the session lost a round-trip to each.
**How to apply:** whenever arming a PR on an authority's word that a blocked job was waiting for.

*Anchors: `agent-fabric:runtime/claude-code/hooks/job-copy-guard.py::has left active`*

*Observed 2026-11-04 (rust-services-dev)*

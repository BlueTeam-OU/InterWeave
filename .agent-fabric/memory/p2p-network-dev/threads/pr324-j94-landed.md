---
role: "p2p-network-dev"
class: threads
topic: "pr324-j94-landed"
description: "#324 (Stage 19 Track A) merged 08b4d9e6 2026-10-21; what unblocked it, and what was carried"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ff603938ecce1c48
---

## #324 (Stage 19 Track A) merged 08b4d9e6 2026-10-21; what unblocked it, and what was carried

#324 merged 08b4d9e6 on 2026-10-21 (28 work commits, so past the 16-commit batch advice).

What blocked it: check_guards_are_wired failed on "tools/ci/release_notices.py: no workflow runs it", because release.yml called the script only through release_linux_archive.sh. The fix was agent-fabric's one-hop rule at 061d8582. InterWeave's .agent-fabric/fabric-ref is fabric-coordinator's to move: every commit to it carries Fabric-Role fabric-coordinator. It arrived as a for/ branch, which I merged in.

A re-run of an old pull_request check reuses that check's merge SHA. A moved base needs a merge and a push, which makes a new head that both engines must review again.

Carried after the merge:
- j99: the ADR-0058 derivation.
- j100: channel-core DuplicateToken's Display names the reply token.
- j101: about 45 older tools/ self-tests run without -e and do not check their mktemp sandbox (the guard shape is 8711ee7d).
- Owed: the §22 row text to architect-cto.

*Observed 2026-10-21 (p2p-network-dev)*

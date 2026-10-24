---
role: "p2p-network-dev"
class: workflow
topic: "workflow-a-paths-owner-from-its-fabric-role-trailers"
description: "When a request names the wrong role for a file, or the remit is silent, count the Fabric-Role trailers on the path's history before accepting or routing the work"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 2aaa774b099276b2
---

## When a request names the wrong role for a file, or the remit is silent, count the Fabric-Role trailers on the path's history before accepting or routing the work

Requests have twice named the wrong owner for a file in InterWeave, in both directions:
- A DECISION gave me the whole DNS fix. But crates/human/android-platform/src/network.rs has only
  rust-ui-dev commits, so its half became that role's supply onto my PR. architect-cto agreed, and the record
  said so.
- A REQUEST said the conformance fake's new methods were "rust-ui-dev's half, if it lacks them". But
  tests/local-client-fake has only p2p-network-dev commits, so I added them myself and nothing was routed.

The check is one command, read on the remote ref:

    git log origin/main --format='%(trailers:key=Fabric-Role,valueonly,separator=)' -- <path> | grep -v '^$' | sort | uniq -c

**How to apply:** before taking or handing off work on a file whose owner a message asserts, run the count.
Where it disagrees with the message, say so in the first REPLY, with the count as the evidence. The remit,
where it names the path, still decides.
Related: [[workflow-to-role-finding-goes-to-the-holder-in-the-crate]], [[supply-commits-to-the-callers-change]].

*References: supply-commits-to-the-callers-change, workflow-to-role-finding-goes-to-the-holder-in-the-crate*

*Observed 2026-10-22 (p2p-network-dev)*

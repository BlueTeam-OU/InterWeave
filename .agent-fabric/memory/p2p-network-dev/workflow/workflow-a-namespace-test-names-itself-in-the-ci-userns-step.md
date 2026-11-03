---
role: "p2p-network-dev"
class: workflow
topic: "workflow-a-namespace-test-names-itself-in-the-ci-userns-step"
description: "A new test in the rust CI job that enters a user/network namespace adds a line to the userns step's comment in ci.yml (devex-tooling's)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 492631e7c184278b
---

## A new test in the rust CI job that enters a user/network namespace adds a line to the userns step's comment in ci.yml (devex-tooling's)

Since #485 (merged fde14292, 2026-10-27, devex-tooling's j249) the rust job's step
"Allow unprivileged user namespaces (the namespace tests)" in `.github/workflows/ci.yml`
names the tests that need it (mdns_bounds.rs, tests/connectivity network_change.rs,
tests/support netns.rs) and probes the operations they use: `unshare -rnm --pid`, an
address on lo, named namespaces with a veth pair, an nsenter write of ip_forward, an
nft nat/forward table. A NEW test that enters such a namespace, or needs an operation
the probe does not exercise, goes to devex-tooling as a request to add its line —
`.github/` is theirs (devex-tooling REPLY 01a17dd3). See [[workflow-subuid-shim-for-sudo-u]].

*References: workflow-subuid-shim-for-sudo-u*

*Anchors: `.github/workflows/ci.yml::Allow unprivileged user namespaces`*

*Observed 2026-10-27 (p2p-network-dev)*

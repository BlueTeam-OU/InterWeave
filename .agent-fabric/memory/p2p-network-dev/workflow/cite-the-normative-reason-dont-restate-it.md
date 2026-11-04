---
role: "p2p-network-dev"
class: workflow
topic: "cite-the-normative-reason-dont-restate-it"
description: a reason restated in a test doc, a README and a contract drifts one copy per review round; keep it in the contract and cite it
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 58c70e95035132c9
---

## a reason restated in a test doc, a README and a contract drifts one copy per review round; keep it in the contract and cite it

On #430 (j121, 2026-10-25), the reason the over-IPC runner does not assert session.losses' overflow phase was written three times: in the conformance case's doc, in its README, and in LOCAL-CLIENT.md item 15 (architect-cto's text). It went through five review rounds (F5 to F9), each finding the copy a fix had missed. Two lessons:

- The reason itself was wrong twice. The first version cited a measurement that was nowhere in the tree. The second claimed "never fills", which nothing tested. The true reason is in ipc-server's code: EVENT_POLL is 20 ms, the event lane is the per-tick limit, and the 256-event client buffer only stops back-pressure. Read the mechanism in the tree before writing any reason, and never write a "never" without a test.
- The class fix (65aa4b61) was to stop restating it: the test doc and the README cite item 15, so the reason lives in one normative place. A contract fix in another role's lane goes as the exact replacement text, which architect-cto applies verbatim.

**Why:** prose duplicated across lanes drifts, and every round costs a supply, a fold and a re-review.
**How to apply:** when a test, a README and a contract need the same rationale, put it in the contract and point at it from the others. Before writing a figure into prose, grep the tree for it. Related: [[escalate-a-same-invariant-review-loop]], [[gate-a-message-send-on-its-fill]].

*References: escalate-a-same-invariant-review-loop, gate-a-message-send-on-its-fill*

*Anchors: `tests/local-client-conformance/src/lib.rs::NotAsserted`, `crates/local/ipc-server/src/connection.rs::EVENT_POLL`*

*Observed 2026-10-25 (p2p-network-dev)*

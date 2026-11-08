---
role: "p2p-network-dev"
class: threads
topic: "j149-process-death-lease-fix"
description: "j149 (rust-ui-dev-02's REQUEST 01a1ac83): my supply 2ce1c7b06 (allowed_client_kinds [human-client]) broke process_death.rs's lease; fix on a for/ branch off #569's head"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1846969882ab7e9f
---

## j149 (rust-ui-dev-02's REQUEST 01a1ac83): my supply 2ce1c7b06 (allowed_client_kinds [human-client]) broke process_death.rs's lease; fix on a for/ branch off #569's head

2026-11-05: #569 (armed) went red on `rust`:
crates/transport/embedded/tests/process_death.rs
a_killed_host_leaves_a_profile_a_fresh_host_starts_on, "the child serves
within 30 s: Disconnected". lease() at line 62 opens SessionRequest as
"process-death"; 2ce1c7b06 made embedded_document admit only human-client.
rust-ui-dev-02 verified "human-client" there passes.

Promised (REPLY 01a1ac84): ONE fix commit on a for/ branch off #569's head
862414e4 (NOT the fabric-jobs switch's branch off main): the test leases as
the kind the document admits; then the FULL workspace suite
(cargo test --workspace --no-fail-fast, background, -j 2) before sending the
range, so no other caller of embedded_document fails in CI. Reply with the
range; they fold it unrebased. j148 (F1/F3) follows separately.

**Why:** I handed over 2ce1c7b06 after running only profile-config's tests.
A document change has callers in every crate that provisions it.
**How to apply:** a supply that changes a shared default runs the whole
workspace before the range is sent. See [[rerun-every-suite-after-a-review-fix]].

Outcome 2026-11-05: delivered 43b89559c on
develop-qzapp/p2p-network-dev-01/for/rust-ui-dev-02/embedded-advertise-off
(2ce1c7b06..43b89559c; lease() reads the kind from allowed_client_kinds).
I sent the range before my workspace suite ran (corrected in REPLY 01a1ac9c).
rust-ui-dev-02 folded it as 7d619210f with origin/main and ran the full suite
there (REPLY 01a1ac9d), so I stopped my duplicate run on the saturated host.
Any failure against the supply comes back from them by name.

*References: rerun-every-suite-after-a-review-fix*

*Observed 2026-11-05 (p2p-network-dev)*

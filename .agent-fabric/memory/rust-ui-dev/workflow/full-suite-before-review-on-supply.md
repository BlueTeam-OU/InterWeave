---
role: "rust-ui-dev"
class: workflow
topic: "full-suite-before-review-on-supply"
description: "run the whole workspace suite (no fail-fast) after folding a supply that changes a shared document or default, before the review and the arm"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 92eedaf6bf964286
---

## run the whole workspace suite (no fail-fast) after folding a supply that changes a shared document or default, before the review and the arm

#569 (j96): p2p-network-dev-01's supply changed profile-config's embedded document to `allowed_client_kinds:
[human-client]`. I ran only the crates I changed, plus the Android and release checks. The armed PR then went red on
the supplier's own transport-embedded test (process_death.rs leases as "process-death"). CI's nextest stops at the
first failure, so one red does not show the rest.

**Why:** a supplied change to a shared default reaches every crate that reads it, not only the crates I touch. The
pull-requests rule's order is fold → full suite → review → arm, and I skipped the second step.

**How to apply:** after every fold, run `bash tools/ci/with_display.sh cargo test --workspace --all-targets --locked
--no-fail-fast` in the background before dispatching the review. `git grep` the changed function's callers to see
whose tests it reaches.

**Run it under the host lease** (ADR-010 §5 rule 9): `fabric-lease heavy -- bash tools/ci/with_display.sh cargo test
--workspace --all-targets --locked --no-fail-fast`, in the background. On 2026-11-05 I ran it without the lease while
the host was saturated (load 52), and fabric-coordinator asked me to stop it. A TaskStop does not reach the
with_display.sh tree (Xvfb, dbus-run-session, cargo): SIGTERM the PIDs that are left, then check ps.

*Observed 2026-11-05 (rust-ui-dev)*

---
role: "rust-ui-dev"
class: solution
topic: "j197-admin-port-polls"
description: "desktop-e2e polls of the daemon must hold one admin port; per-turn opens exhaust the 4 admin slots under load (j197, PR #540)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7b463df82c86dbc8
---

## desktop-e2e polls of the daemon must hold one admin port; per-turn opens exhaust the 4 admin slots under load (j197, PR #540)

The daemon admits 4 admin connections (ipc-server `Limits`), and a closed connection keeps its slot until the daemon's task for it ends (the `Slot` moves into `connection::run`, server.rs:97-105; no test pins that window). A test poll that opens an admin port every 100 ms runs the slots out under full-suite load. The refusal is `close{Overloaded}`, but ipc-client `open()` reports `BackendUnavailable` when the close arrives before the hello write (EPIPE). That client race was reported to p2p-network-dev-01 as OBSERVATION 01a1a167, their job j142, commit 4af396632.

Merged in PR #540 as 3ef7aa67 (2026-11-03; 3 work, 2 fix, armed on architect-cto AUTHORIZES 01a1a19a): `kept_disable` holds one port per daemon; `harness::until_lease` holds one port per wait and retries only `BackendUnavailable`/`ShuttingDown`; the `codes.rs` address polls go through `peers_admin`. The rule is written on `until_lease`'s doc comment.

The load reproduction (6 copies of kept_disable alone) did NOT reproduce the CI failure: 0/48 for both the old and the new code. Evidence is by construction.

Trap: e2e test binaries find the daemon next to themselves (`beside_this_test`), so copies must live in target/debug/deps. The release cases also need `transportctl` built.

Related: [[desktop-e2e-load-flakes]], [[desktop-e2e-stale-binary]]

*References: desktop-e2e-load-flakes, desktop-e2e-stale-binary*

*Anchors: `tests/desktop-e2e/tests/human_app/harness.rs::until_lease`, `crates/local/ipc-client/src/connection.rs::BackendUnavailable`*

*Observed 2026-11-03 (rust-ui-dev)*

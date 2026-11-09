---
role: "p2p-network-dev"
class: solution
topic: "a-released-port-0-bind-is-not-a-dead-port"
description: "A dead-address fixture made by binding port 0 and releasing it races other tests' listeners (CI runs nextest, one process per test); use tests/support's free_port, which answers below the port-0 range under a claim"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 54180aa5b194f724
---

## A dead-address fixture made by binding port 0 and releasing it races other tests' listeners (CI runs nextest, one process per test); use tests/support's free_port, which answers below the port-0 range under a claim

A fixture that binds `127.0.0.1:0` (a `TcpListener`, or a libp2p swarm bound and dropped), reads the port and releases it gives an address nobody listens on only at that moment. CI's `rust` job runs the workspace tests under cargo-nextest, one process per test, so any other test process's port-0 listener can land on that port while the test is still dialling it. The dial then fails with `WrongPeerId` (class `IdentityMismatch`), the gate quarantines the address (`AddressQuarantined`, 30 min) instead of scheduling a peer retry, and a test about refused dials sees a different event.

Measured on j136 (2026-10-28). Merge-group run 40676203372 of #501 failed `relay_client::a_relay_that_is_also_a_data_plane_peer_is_still_retried_by_the_reconnect_scheduler` with "Unexpected peer ID" at the released port about 30 s in. A planted second relay on the freed port at 10 s reproduced the same panic locally. The observer's four-event summary of that panic was truncated: read the job log (`gh api repos/BlueTeam-OU/InterWeave/actions/jobs/<id>/logs`) before diagnosing.

**How to apply:** for a dead address or a listen address chosen ahead of time, use `interweave_test_support::net::free_port(ip)`. It answers below `ip_local_port_range` (where no port-0 bind can land) and holds a per-account file claim on the port for the rest of the process. dcutr.rs's `dead_on` already did this. The class is grep-able as `TcpListener::bind(... 0)` followed by `.port()` with the listener not kept, and as `bound(&mut relay_server(..))` on a temporary. j136 moved relay_client, relayed_paths, direct-v2 stage6 and ipc-v2 wire to `free_port` (#504, merged efcbf4a1 on 2026-10-28; armed below the minimum on architect-cto's word as a CI-blocking fix). j137 finished the class in #507 (merged 0c957701, 2026-10-28; armed below the minimum on architect-cto's word): composition.rs (two sites), status_surface.rs and gate_notes.rs, after #503 (j88) merged. A comment above a free_port call that claims the port is reserved must say "this account and `TMPDIR`" and cite free_port_answers_below_the_port_0_range, free_port_holds_the_claim_on_what_it_answers and a_port_another_process_claims_is_refused_until_it_ends — two review rounds on #507 were those two omissions. A full TcpListener::bind sweep found no other released site: the rest keep their listener alive while dialled.

Not the class: a `/ip4/127.0.0.1/tcp/1` literal (autonat_misbehaving_server.rs, autonat_outcome_wire.rs) — port 1 is privileged and no test process can bind it. dcutr.rs's `dead_on` was already `free_port`.

*Anchors: `tests/support/src/net.rs::free_port`, `tests/connectivity/tests/relay_client.rs::down_relay_address`*

*Observed 2026-10-28 (p2p-network-dev)*

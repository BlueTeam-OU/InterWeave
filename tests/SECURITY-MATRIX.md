# Security matrix: Stage 18's gate

Stage 18's exit gate (plan section 21) is this table. Each row is one
threat from `architecture/docs/architecture/threat-model.md` or one
item of section 21's list. It names every test that guards the threat,
wherever that test lives. The gate is met when every test the table
names exists and is green on `main`, and no row is empty.

`tests/security` (the network half, S1-S8) and `tests/security-local`
(the daemon's sockets, transportctl and the stores, S9-S11) hold only
what no lower layer can prove completely (plan section 25). A test that
re-proves a lower-stage test belongs in that stage's suite, and this
table names it there. The table sits outside both crates so that each
holder can add rows without entering the other's crate.

## Row grammar

```text
| id | guards | step | tests | limit |
```

- **id**: `S21-NN` for section 21's list, in its order. `TM-NN` for
  threat-model.md's rows, numbered in document order across its
  tables. `C-NN` for an item carried into section 21 from an earlier
  stage. An id is never reused or renumbered.
- **guards**: the threat-model row or section 21 item, in its own words.
- **step**: the plan step that owns the row's open work, and its
  holder (`01` / `02` for the two p2p-network-dev holders), or `—`
  once nothing is owed.
- **tests**: every guarding test as `` `path::fn` ``, the path relative
  to the repository root, separated by `; `. A test inside a
  `#[cfg(test)]` module is named by its source file. `owed: <what>`
  marks work a step still has to land. A row whose tests cell holds
  only `owed:` is still empty, and so is a cell holding `—`.
- **limit**: what the named tests do not prove (the residual the
  threat model accepts, a recorded measurement, or a platform the
  runner lacks), or `—`.

## Section 21

| id | guards | step | tests | limit |
|---|---|---|---|---|
| S21-01 | pre-Noise handshake floods | — | `tests/security/tests/preauth_flood.rs::one_source_flooding_at_the_shipped_limits_holds_its_share_and_shuts_out_nobody_else`; `tests/security/tests/preauth_flood.rs::a_distributed_flood_fills_the_global_ceiling_and_the_timeout_gives_it_back`; `tests/security/tests/preauth_flood.rs::the_global_start_rate_refuses_a_fresh_source_once_the_window_is_spent`; `tests/connectivity/tests/stage5_dial_admission.rs::a_source_past_its_pre_auth_rate_is_refused_before_noise`; `tests/connectivity/tests/stage5_dial_admission.rs::a_completed_handshake_gives_its_pre_auth_slot_back`; `tests/connectivity/tests/stage5_dial_admission.rs::a_handshake_abandoned_mid_flight_does_not_hold_its_slot`; `tests/connectivity/tests/stage5_dial_admission.rs::a_handshake_that_never_speaks_is_dropped_on_the_timeout`; `tests/connectivity/tests/stage5_dial_admission.rs::a_handshake_killed_by_the_timeout_gives_its_slot_back`; `crates/transport/runtime/src/preauth.rs::one_source_cannot_spend_the_whole_global_budget`; `crates/transport/runtime/src/preauth.rs::the_global_ceiling_holds_across_sources`; `crates/transport/libp2p/src/preauth_gate.rs::a_relayed_inbound_is_charged_to_the_relay_not_the_source`; `tests/security/tests/relayed_preauth.rs::fresh_identities_through_one_relay_share_its_bucket_and_a_direct_peer_from_its_host_does_not` | A distributed flood that fills the global ceiling refuses an honest peer too, until the 10 s timeout reclaims the slots (measured by `a_distributed_flood_fills_the_global_ceiling_and_the_timeout_gives_it_back`). |
| S21-02 | trusted-peer direct floods | S2 (01) | `tests/direct-v2/tests/stage6_ingress_rate_limits.rs::a_trusted_peer_is_refused_once_its_burst_is_spent`; `tests/direct-v2/tests/stage6_ingress_rate_limits.rs::a_peer_cannot_mint_allowance_by_inventing_source_endpoints`; `tests/direct-v2/tests/stage6_ingress_rate_limits.rs::a_flooding_peer_does_not_spend_a_quiet_peers_allowance`; `tests/direct-v2/tests/stage6_ingress_rate_limits.rs::the_global_bucket_bounds_peers_that_are_each_within_their_own`; `tests/pubsub/tests/stage7_broadcast_over_the_wire.rs::a_broadcast_flood_does_not_wedge_the_direct_path`; owed: the shipped per-peer and global buckets, and several trusted peers on one IP | — |
| S21-03 | GossipSub duplicate/signature poisoning | S3 (01) | `tests/pubsub/tests/stage7_broadcast_over_the_wire.rs::invalid_signature_traffic_cannot_poison_the_cache_for_authentic_traffic`; `tests/pubsub/tests/stage7_broadcast_over_the_wire.rs::an_unauthorized_publisher_is_ignored_not_delivered_and_not_relayed_further`; `tests/pubsub/tests/stage7_broadcast_over_the_wire.rs::a_signed_but_malformed_envelope_is_reject_and_does_not_wedge_later_valid_traffic`; `tests/pubsub/tests/stage7_broadcast_over_the_wire.rs::an_infrastructure_only_peer_cannot_join_the_mesh`; owed: one publisher reusing another's envelope id and sequence, and a forged source, over the wire | A publisher can still suppress or replace its own repeated ids (threat-model.md). |
| S21-04 | EndpointId probing/squatting/metadata abuse | S4 (01) | `tests/endpoint-routing/tests/stage8_endpoint_directory.rs::a_hostile_directory_response_is_a_protocol_violation`; `tests/endpoint-routing/tests/stage8_endpoint_directory.rs::a_refusal_carries_no_endpoint_list`; `tests/endpoint-routing/tests/stage8_endpoint_directory.rs::a_disabled_directory_still_charges_the_query_rate`; `tests/endpoint-routing/tests/stage8_endpoint_directory.rs::the_configured_query_rate_is_honoured`; owed: `no_route` byte-identical for its four causes, an untrusted peer refused at the wire, a probe sweep bounded | Timing may still distinguish the `no_route` causes; constant-time evaluation is not promised (threat-model.md). |
| S21-05 | address poisoning of trusted PeerIds | S5 (01) | `tests/connectivity/tests/stage5_dial_admission.rs::an_identity_mismatch_quarantines_the_address_and_not_the_peer`; `tests/connectivity/tests/stage5_dial_admission.rs::a_poisoned_address_does_not_suppress_the_peer_s_good_route`; `crates/transport/runtime/src/connection_policy.rs::address_poisoning_cannot_suppress_a_known_good_route`; owed: a poisoned address from each learning source (Identify, Kademlia, mDNS, cache) | Many poisoned addresses still spend the bounded dial budget (threat-model.md). |
| S21-06 | Kademlia poisoning/eclipsing bounds | S6 (01) | `crates/transport/libp2p/tests/kademlia_driver.rs::an_offer_is_judged_against_the_runtimes_operator_set_and_counted`; `crates/transport/libp2p/tests/kademlia_driver.rs::the_gate_refuses_the_walks_dial_to_a_stranger`; `crates/transport/libp2p/tests/kademlia_driver.rs::two_network_ids_never_mix`; `crates/transport/libp2p/src/runtime/kademlia_driver.rs::inbound_record_writes_are_counted_never_stored`; owed: record-write and query floods from real peers, a driver event flood, repeated bootstrap failure, the N/M eclipse run | The eclipse ratio and bootstrap diversity are RECORDED with no threshold (R2), and owed with their N and M. No diversity floor is configured, and kademlia-integration.md section 18 makes no Byzantine-resistance claim. |
| S21-07 | relay smuggling/protocol-class violations | S7 (01) | `crates/transport/libp2p/tests/relay_hop_gate.rs::the_relay_offers_hop_only_while_it_holds_a_verified_direct_address`; `crates/transport/libp2p/tests/relay_hop_gate.rs::with_the_gate_open_the_ceilings_are_exact_and_a_stranger_is_offered_nothing`; `tests/connectivity/tests/relay_server.rs::the_relay_server_retains_authorized_peers_and_serves_nobody_without_a_verified_address`; `tests/connectivity/tests/relayed_paths.rs::an_infrastructure_only_source_over_a_circuit_is_refused_at_the_destination_with_the_servers_on`; `crates/transport/libp2p/src/class_gate.rs::an_infrastructure_only_peer_is_offered_nothing_and_the_inner_behaviour_is_never_asked`; owed: unauthorized and anonymous reservations, a non-hop protocol over a circuit | A relay still observes connection metadata and can deny service (threat-model.md). |
| S21-08 | AutoNAT dial-back SSRF | S8 (01) | `crates/transport/libp2p/tests/autonat_server_wire.rs::the_wrapper_refuses_a_loopback_dial_back_and_no_socket_is_opened`; `tests/connectivity/tests/autonat_server.rs::an_infrastructure_only_client_is_served_and_its_loopback_target_is_refused_before_any_socket`; owed: private, ULA, link-local, multicast, unrelated-public and DNS targets and the source-equality rule, each with a probeable control | An authorized client can still ask for permitted ports on its own observed public IP within quotas (threat-model.md). |
| S21-09 | admin/data socket spoofing | S9 (02) | `tests/ipc-v2/tests/wire.rs::each_socket_refuses_the_other_domains_methods`; `tests/ipc-v2/tests/wire.rs::each_handshake_refusal_has_its_code`; `crates/local/ipc-server/src/server.rs::a_peer_of_another_uid_is_closed_before_hello_and_counted`; owed: a spoofed `client.kind`, a real foreign uid under `sudo -u` (R4) | Hostile code running as the same user can open the admin socket (threat-model.md, rows 28 and 39). SPIKE-005 stays conditional: awaits the owner's word (R3). |
| S21-10 | human-store/storage failure | S10 (02, with rust-ui-dev) | `crates/human/store/tests/durable_write_fault.rs::recheck_health_stays_degraded_while_durable_writes_fail`; owed: profile state under a write fault, a corrupt store, a failed migration | — |
| S21-11 | identity recovery failure/tamper | S11 (02) | `crates/identity/profile-identity/tests/identity_lifecycle.rs::a_missing_key_is_an_error_and_never_a_new_identity`; `crates/identity/profile-identity/tests/identity_lifecycle.rs::a_world_readable_key_is_refused_rather_than_repaired`; `crates/identity/profile-identity/tests/required_conformance.rs::one_substituted_word_fails_the_checksum`; owed: a tampered phrase and a wrong PeerId through transportctl, the phrase absent from every captured stream | — |
| S21-12 | Android key/backup/recovery failure cases | S12 (02) | `crates/transport/embedded/tests/custody.rs::an_invalidated_or_missing_key_requires_recovery_and_mints_nothing`; owed: the device cases, waiting on the instrumentation | — |

## Carried into section 21

| id | guards | step | tests | limit |
|---|---|---|---|---|
| C-01 | `HandshakeSlot::source`: the bucket a reclaimed handshake was counted under, attributable in a log (Stage 13, R6) | — | `crates/transport/libp2p/src/preauth_gate.rs::a_handshake_that_ends_unestablished_is_logged_by_its_bucket_alone`; `crates/transport/runtime/src/preauth.rs::ipv6_is_bucketed_by_prefix_so_an_address_range_is_not_free` | The line is debug level, so an operator sees it only with the profile's log level at debug. |
| C-02 | SPIKE-005, a hostile same-uid process (Stage 13) | owner | owed: the owner's word on R3 | — |
| C-03 | the foreign-uid peer refused on a real OS (Stage 13) | S9 (02) | `crates/local/ipc-server/src/server.rs::a_peer_of_another_uid_is_closed_before_hello_and_counted`; owed: the real-uid run (R4) | The existing test injects the uid. |
| C-04 | the Windows binding: named pipe, ACL, peer identity; BSL-1.0 through `arboard` (Stage 15) | owner | owed: the owner's word on R5 | No Windows runner exists. |
| C-05 | the Claude Code Channel bridge on a second network and against the human client's facade, plain and `;ce=br` (Stage 16) | S13 (02) | owed: the namespace run (R7) | — |

## threat-model.md

Owed: each row's guarding tests, added by the step its threat belongs
to. A row no step covers is raised to architect-cto rather than left
empty.

| id | guards | step | tests | limit |
|---|---|---|---|---|
| TM-01 | Rogue peer | — | owed | — |
| TM-02 | Discovery poisoning | S5 (01) | owed | — |
| TM-03 | Trusted-PeerId address poisoning/backoff pollution | S5 (01) | owed (S21-05's tests) | — |
| TM-04 | Malicious bootstrap/Kademlia | S6 (01) | owed (S21-06's tests) | — |
| TM-05 | Replay | — | owed | — |
| TM-06 | Pre-Noise handshake flood | — | `tests/security/tests/preauth_flood.rs::one_source_flooding_at_the_shipped_limits_holds_its_share_and_shuts_out_nobody_else`; `tests/security/tests/preauth_flood.rs::a_distributed_flood_fills_the_global_ceiling_and_the_timeout_gives_it_back`; `tests/security/tests/preauth_flood.rs::the_global_start_rate_refuses_a_fresh_source_once_the_window_is_spent` (and S21-01's) | Distributed-source floods and NAT collateral; the source IP is unavailable on some paths (threat-model.md). |
| TM-07 | Flood/oversized | S2 (01) | owed | — |
| TM-08 | Slow endpoint consumer | — | owed | — |
| TM-09 | Slow broadcast consumer | — | owed | — |
| TM-10 | Local IPC attack | S9 (02) | owed | — |
| TM-11 | Endpoint squatting | S4 (01) | owed | — |
| TM-12 | Endpoint source spoof by ordinary local caller | S9 (02) | owed | — |
| TM-13 | Endpoint label used as authorization principal | S4 (01) | owed | — |
| TM-14 | Remote source-endpoint spoof | S4 (01) | owed | — |
| TM-15 | Endpoint enumeration | S4 (01) | owed | — |
| TM-16 | Endpoint probing oracle | S4 (01) | owed | — |
| TM-17 | Remote transport-metadata injection | S4 (01) | owed | — |
| TM-18 | Default-route confusion | — | owed | — |
| TM-19 | Endpoint ACL widens trust | — | owed | — |
| TM-20 | Offline mailbox creep | — | owed | — |
| TM-21 | Admin confused deputy | S9 (02) | owed | — |
| TM-22 | Private-key theft | S11 (02) | owed | — |
| TM-23 | Recovery-phrase theft | S11 (02) | owed | — |
| TM-24 | Recovery typo/wrong phrase | S11 (02) | owed | — |
| TM-25 | Android mnemonic UI exfiltration | S12 (02) | owed | — |
| TM-26 | Android system backup/privacy leak | S12 (02) | owed | — |
| TM-27 | Topic enumeration/confidentiality | — | owed | — |
| TM-28 | GossipSub message-ID suppression | S3 (01) | owed | — |
| TM-29 | GossipSub trust asymmetry | S3 (01) | owed | — |
| TM-30 | Prompt injection | — | owed | — |
| TM-31 | Address SSRF-like abuse | S8 (01) | owed | — |
| TM-32 | Log leakage | — | owed | — |
| TM-33 | malicious/compromised relay | S7 (01) | owed | — |
| TM-34 | malicious AutoNAT observer | S8 (01) | owed | — |
| TM-35 | infrastructure privilege escalation | S7 (01) | owed | — |
| TM-36 | relay exhaustion / abusive reservations | S7 (01) | owed | — |
| TM-37 | hole-punch abuse | S7 (01) | owed | — |
| TM-38 | stale relay address advertisement | S7 (01) | owed | — |
| TM-39 | authorized AutoNAT client uses probe server as SSRF scanner | S8 (01) | owed (S21-08's tests) | — |
| TM-40 | implicit Identify turns friend into relay/probe | S7 (01) | owed | — |
| TM-41 | circuit flood hides original source IP | — | `tests/security/tests/relayed_preauth.rs::fresh_identities_through_one_relay_share_its_bucket_and_a_direct_peer_from_its_host_does_not`; `crates/transport/libp2p/src/preauth_gate.rs::a_relayed_inbound_is_charged_to_the_relay_not_the_source` | A malicious relay can spend its own bucket and its share of the global caps until it is blocked (threat-model.md). |
| TM-42 | Android remote message triggers admin action | S12 (02) | owed | — |
| TM-43 | Android key extraction from filesystem | S12 (02) | owed | — |
| TM-44 | remote sender attempts to force message persistence | S10 (02, with rust-ui-dev) | owed | — |
| TM-45 | logs/notifications/search indexes become shadow chat archive | S10 (02, with rust-ui-dev) | owed | — |
| TM-46 | restored pending outbox replays old sends | S10 (02, with rust-ui-dev) | owed | — |
| TM-47 | OS kills Android runtime | S12 (02) | owed | — |
| TM-48 | cloned recovery phrase used on two active devices | S11 (02) | owed | — |
| TM-49 | an account owning an ancestor of a private file's directory, or a link on its path, swaps that directory | S10 (02) | owed | — |
| TM-50 | durable trust overlay tampered with or deleted | S10 (02) | owed | — |
| TM-51 | durable endpoint overlay tampered with or deleted | S10 (02) | owed | — |

Rows with `—` in the step column are covered by no S1-S13 step yet.
They go to architect-cto, who assigns each one or records it as met by
a lower stage.

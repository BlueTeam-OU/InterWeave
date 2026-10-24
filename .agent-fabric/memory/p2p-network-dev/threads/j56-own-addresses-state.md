---
role: "p2p-network-dev"
class: threads
topic: "j56-own-addresses-state"
description: "j56 admin.peers.own_addresses (IPC 2.8) + j61 item-13 conformance case on one branch; what is built, proved, owed (2026-10-22)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 8b40deb5d5b4e514
---

## j56 admin.peers.own_addresses (IPC 2.8) + j61 item-13 conformance case on one branch; what is built, proved, owed (2026-10-22)

Branch develop-qzapp/p2p-network-dev-02/feat/j56-admin-peers-for-the-screens-the, 2026-10-22: 14 work commits, main folded at 8f881c9c, no PR yet.

- Shape approved at 3e0cf423 (ipc/peers-own-addresses), sent to rust-ui-dev-01 (REPLY 01a16345-862c) and rust-ui-dev-02 (01a16345-fcc1).
- Selector: transport-libp2p runtime/own_addresses.rs `select` (one place: /p2p suffix, foreign peer dropped, entry_grammar + special_use_host floor, dedupe, 64 bound); AutoNAT `advertised` kept in confirmation order; ActiveListeners became an ordered newtype (Swarm's listener table is a HashMap).
- "relay-only" defined in LOCAL-IPC as no confirmed-external row surviving the floor; asked architect-cto-01 (REPLY 01a163ff) to object or not, and whether own_addresses gets its own LOCAL-CLIENT §7 item.
- j61 (architect-cto REQUEST 01a163fd): item 13 case `bootstrap_addresses_answer_alike_at_every_binding` + `own_addresses_are_what_an_add_takes`, green at fake/in-process/IPC/embedded; mutations of each mapping red. Item 13's text is on #373 — this PR arms only after #373 lands.
- Unprovable on this host: confirmed_external and relay_circuit rows end to end (no public address); unit tests only.

PR #378 opened 2026-10-22 at head 1489b716; three review rounds (r1 F1 P3 + F2 P2 fixed; r2 F3 not-real on adjudication; r3 clean at 7df69cc8); xtask ci green at 1489b716; 20 work / 2 fix. Still to do before arm: wait #373, fold architect-cto's for/p2p-network-dev-02/j56-item-14-relay-only (f9341372, ade7daba; item 14 = show side, relay-only defined), full suite, re-review fold range.
- 2026-10-22 19:30: #373 merged; main + ade7daba (item 14) folded locally as af0948dd, unpushed; item 14 checked against tests. j62 takes 46a90f24 + fc015886 (architect-cto REPLY 01a16512).
- First opened (main folded); review round 1 dispatched; PR number sent to all three (INFO 01a16467-*). Arms only after #373.
- j62 (rust-ui-dev-01 REQUEST 01a16466): scan all-or-none -> option (b) admin.peers.add_addresses 1..=16 at minor 2.9, the next batch after #378 (REPLY 01a16467-feba); schema commit first on a fresh branch off main.

See [[j54-bootstrap-overlay-state]], [[commit-before-mutation]].

*References: commit-before-mutation, j54-bootstrap-overlay-state*

*Observed 2026-10-22 (p2p-network-dev)*

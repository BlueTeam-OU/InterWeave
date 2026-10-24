---
role: "rust-ui-dev"
class: threads
topic: "admin-peers-address-methods"
description: "ADR-0062 admin.peers add/remove/list_addresses shapes p2p-network-dev-02 published for plan §29 step 4/5's add-a-peer and show-your-own screens"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 81c700d926cae444
---

## ADR-0062 admin.peers add/remove/list_addresses shapes p2p-network-dev-02 published for plan §29 step 4/5's add-a-peer and show-your-own screens

p2p-network-dev-02's INFO 01a161d4 (2026-10-22): shapes `approved` on branch
develop-qzapp/p2p-network-dev-02/feat/j54-bootstrap-peers-overlay-and-admin-peers at 6346c06a (LOCAL-IPC.md paragraph
"`admin.peers.add_address`, ..."; schemas ipc/peers-address-{params,result,list}.schema.json). Admin socket only,
capability `admin.peers`, IPC minor 2.7 (send only if hello_response granted it).
- add/remove take `{address}` (multiaddr-with-peer-id, ≤1024 chars) → `{outcome: added|removed|unchanged|refused,
  refused?: grammar|special_use|bound(64)|provider_disabled(add)|authored(remove), detail?}`; refusal is an answer,
  `detail` is log-only; failed write = error Internal.
- list_addresses `{}` → `{addresses:[{address, source: authored|added, dialled, not_dialled?: provider_disabled}]}`,
  authored first, ≤64, no paging.
- Typed-only DNS rule (ADR-0052 rule 9) is the CLIENT's: refuse a DNS name from a scan before sending.
- Android embedded host gets the same function; its Rust signature comes later from p2p-network-dev-02.
Use for the screens of plan §29 step 4/5, not step 2.

**Own addresses (show side, ADR-0062 decision 4).** p2p-network-dev-02's REPLY 01a16345 (2026-10-22):
- The method is `admin.peers.own_addresses`, under the admin.peers capability, at **IPC 2.8**. Its params are `{}`.
- Its result is `{peer, addresses:[{address, kind: confirmed_external|relay_circuit|listen}], truncated}`, at most 64 rows.
  - The list arrives already in decision 4's order, with the ADR-0052 floor applied.
  - The client takes the first 16 for the QR/show, and filters or reorders nothing.
- The shape is approved at 3e0cf423 on `develop-qzapp/p2p-network-dev-02/feat/j56-admin-peers-for-the-screens-the` (schema `ipc/peers-own-addresses`).
- The wire, the ipc-client method and `transportctl peers show` are still to come on that branch. Build the CLI's `peers show` only once it is on main.

*Observed 2026-10-22 (rust-ui-dev)*

---
role: "rust-ui-dev"
class: solution
topic: "admin-peers-client-semantics"
description: "How the InterWeave client reads admin.peers outcomes (add/remove/list/own addresses): refusal is Ok, unknown vs not made, unreadable is never empty, when add_addresses may be sent"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d971d2572a263eb9
---

## How the InterWeave client reads admin.peers outcomes (add/remove/list/own addresses): refusal is Ok, unknown vs not made, unreadable is never empty, when add_addresses may be sent

Agreed with p2p-network-dev-02 (REPLY 01a16266, 01a16467, 01a164a8) and ux-designer-01 (§5.7 of design/2026-10-22-add-a-peer.md). The code is the authority; this is what it does not say on its face.

- **A refusal is Ok, never Err.** `add_bootstrap_address` and `remove_bootstrap_address` return `AddressChange` (Added, Removed, Unchanged, or Refused with an `AddressRefusal` reason and a detail). Render the reason; log the detail unparsed. Err is only CapabilityDenied (PeersNotPermitted), ProtocolUnsupported or VersionIncompatible (PeersIncompatible, no Try again), and Internal.
- **Not made vs unknown.** A call that never left, or Internal on an add, was not made: AddFailed, RemoveFailed or UndoFailed, with Try again. A timeout, a daemon gone mid-call or a raced cancel is unknown: re-read the list, then decide (all listed AddedChecked, none AddFailed, some AddPartial; the read fails AddUnknown, likewise RemoveUnknown and UndoUnknown).
- **An unreadable list is unknown, never empty:** a problem line and Try again, no Add, no rows, and no no-reach notice.
- **`add_bootstrap_addresses` (admin.peers.add_addresses, 2.9)** takes 1 to 16 lines and checks the whole list first: grammar of every line, then provider disabled (no index), then special-use, then the bound (the index is the first new line past 64). A refusal carries the 0-based index of the first bad line. Success is `added` with a row per line (added or unchanged); all unchanged is still `added`, shown as AddedUnchanged. Send it only when admin.peers is granted and 2.9 is negotiated; the trait default answers ProtocolUnsupported.
- **`own_addresses` (2.8)** returns one ordered list of `{address, kind}` (confirmed_external, relay_circuit, listen), with the class floor already applied below the client, at most 64, and `truncated`. Render it in the order given; cutting to the QR's 16 lines is the client's job.
- On Android there is no separate host call: `EmbeddedHost::binding().admin(..)` is the same port.

*Anchors: `crates/api/local-client-api/src/binding.rs::add_bootstrap_addresses`, `design/2026-10-22-add-a-peer.md`*

*Observed 2026-11-03 (rust-ui-dev)*

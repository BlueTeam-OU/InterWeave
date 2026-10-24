---
role: "p2p-network-dev"
class: solution
topic: "admin-call-order-across-connections"
description: whether an admin set whose answer was lost can land after a later read on another IPC connection
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 4f6d418dd8d11a23
---

## whether an admin set whose answer was lost can land after a later read on another IPC connection

It can (read at origin/main 2026-10-23). The composition driver runs one FIFO queue (`mpsc::channel(64)`, runtime.rs) and handles each request to completion, so a set and a list are ordered by when each enters that queue. Nothing orders entry across connections: each connection's server task (ipc-server connection.rs) reads its frames and spawns the port call, and the call enters the queue only when that task runs. A client closing the connection does not withdraw a frame already written; at EOF, end() aborts the call, so the set either never happens or happens unseen. A deadline `Timeout` discards the outcome without stopping the call. The earliest read a client can trust is one sent after the DAEMON has ended the set's connection: end() joins the aborted calls before closing. Told to rust-ui-dev-01 (REPLY 01a16662) for #384's pending Trust; a daemon-side bound would be a contract change, architect-cto's.

*Observed 2026-10-23 (p2p-network-dev)*

---
role: "p2p-network-dev"
class: solution
topic: "non-reading-ipc-client-is-closed-by-write-stall"
description: "a test that overflows an IPC client's session queue by not reading must bound the queue small, or the write stall closes the client first"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 290ba827dea79fe8
---

## a test that overflows an IPC client's session queue by not reading must bound the queue small, or the write stall closes the client first

To overflow a data client's session queue over real IPC (tests/ipc-v2 wire.rs,
PR #554), the client stops reading — but the server pumps the queue into the
event lane (sized like the queue) and the socket first, and `WRITE_STALL` (10 s)
closes a client that takes no frame, ending its session. At the default bound
256 (lane 256 + queue 256) 1000 publishes of 48 KiB never produced a drop in
57 s. With `CompositionOptions.queue_bound = 16` (the profile's least) the first
drop comes within a few dozen publishes, ~4.7 s, and each further broadcast then
drops exactly one (assertable as +k). `Node::start_bounded` in wire.rs takes the
bound. Also: a `human-client` hello naming endpoint `agent` with only
`commands` is closed CapabilityDenied there; an unleased `broadcaster` with
`["commands"]` publishes fine.

*Anchors: `tests/ipc-v2/tests/wire.rs::start_bounded`, `crates/local/ipc-server/src/hello.rs::WRITE_STALL`*

*Observed 2026-11-04 (p2p-network-dev)*

# ipc-client

The desktop IPC client (major 3, ADR-0067 decision 7): the neutral local-session binding over the daemon's data and admin Unix sockets.

**Current status:** Stage 13, active workspace member. `IpcBinding` implements `DataSessionBinding` and `AdminBinding` (`interweave-local-client-api`), so a local application holds the same `DataSessionPort` and `AdminPort` whether its binding is in-process or IPC; `tests/local-client-conformance` runs the same generic functions against both. Like the server, it depends on nothing under `crates/transport/*` and on no libp2p crate. Unix only.

## What differs from the in-process binding

All seven are of the wire, and the contracts name them: `LOCAL-IPC.md` the first three at A 2026-09-30, and the fourth and fifth in §Version negotiation and phases (major 3); `LOCAL-CLIENT.md` the sixth under "Taking events" and `LOCAL-IPC.md` the seventh in §Disconnect/reconnect and optional keepalive (both A 2026-10-08):

- Events are pushed, so `events` returns what has arrived; one the daemon admitted may still be on its way.
- The receive buffer is bounded at the granted `event_queue`. A full buffer stops the client reading its socket, so a response queued behind undrained events waits for them, and past the keepalive miss threshold the daemon closes the connection as wedged. Draining events is part of holding a lease.
- The grouped order of `events` (session notices, then direct, then broadcast) holds within one server pump; across pumps batches are read as they arrive, so a notice pumped after a direct message follows it.
- An admin port names every capability it wants in its first hello: each is a 3.0 capability, so the capability rule's probe connection (`LOCAL-IPC.md` §Version negotiation) is owed for none, and the binding keeps no learnt minor. Major 2's probe for `admin.trust` (2.1) and `admin.peers` (2.7) went with major 2; a capability a later minor brings needs it back. `trust` reads `admin.trust.list` page by page; pages are read against the live policy, not a snapshot, and every row says where its peer comes from: a row that does not is refused `Internal`, since an unknown is never shown as a value (ADR-0028 and ADR-0017 A 2026-10-07). `set_trust` carries no row and reads `trust-set-result`.
- A session that declines route notices cannot be opened: major 3 has no hello that asks for one, every session taking them, so `open` refuses the request `ProtocolUnsupported` with nothing sent. In process the composition honours it.
- What arrived before the end is still delivered: once the session has ended, a positive `max` takes what waits and then answers the end, while `events(0)` answers the end at once. In process the queues are the runtime's and go with it.
- The connection can end `Timeout` on the client's own bound: once a `ping` has been read, `CLIENT_SILENCE_TIMEOUT` (120 s) with no frame read ends it, and every waiting call answers `Timeout` with it. Before any ping, only the caller's own timeouts and OS liveness bound a silent daemon.

## What it does not decide

Every refusal is the daemon's answer, carried back as its code. The one read that never reaches the daemon is `events`, which takes from what was already pushed; its capability is judged locally, as the in-process binding judges it. The daemon's `server_state` is held beside the event buffer as the session's newest `ServerState`, replaced rather than queued and taken first; `ready()` looks at both and waits on a wake the reader gives for each event buffered, each state held and the connection's end, taking nothing.

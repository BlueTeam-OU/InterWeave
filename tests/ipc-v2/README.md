# ipc-v2

The desktop IPC wire suite, major 3 (ADR-0067 decision 7; the crate keeps the name it had on major 2): raw frames over real Unix sockets to `interweave-ipc-server`, serving a runtime composed from a profile.

**Current status:** Stage 13, active workspace member, Unix only. It covers:
- **wire:** every frame the server writes validates against `ipc/frame.schema.json`, and the golden client frames are accepted;
- **handshake:** each refusal closes with its own code -- a major-2 hello `VersionIncompatible`, a label claim `InvalidArgument`;
- **authority:** the data and admin sockets refuse each other's methods;
- **limits:** the client ceilings hold;
- **keepalive:** expiry releases the lease on the real runtime.

Cancellation and request concurrency are pinned in the server's own tests against a scripted binding; a real runtime answers too quickly to hold a request in flight.

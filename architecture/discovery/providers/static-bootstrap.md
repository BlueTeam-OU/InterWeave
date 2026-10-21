# StaticBootstrapDiscovery

Purpose: configured reachability entry points.

Example hint:

```text
/dns4/bootstrap.example.net/tcp/4001/p2p/<PeerId>
```

## Semantics

Static peers are candidate addresses with configured provenance. They are **not**:

- identity authorities;
- trust roots;
- membership servers;
- coordinators;
- brokers;
- message stores;
- channel owners;
- required permanent infrastructure after peers learn alternative reachability.

A configured bootstrap PeerId still requires an explicit trust rule before ConnectionManager may establish/retain an ordinary v1 data-plane connection. Configuration does not grant trust.

## DNS ownership

`StaticBootstrapDiscovery` validates and emits configured `/dns4`/`/dns6` multiaddresses **without eagerly resolving them**. Its health covers configuration parsing, provider lifecycle, and its ability to emit configured candidate observations.

DNS resolution occurs when the libp2p/ConnectionManager dial path consumes the multiaddress. DNS lookup failures are therefore **dial/connection diagnostics**, not discovery-provider health failures. ConnectionManager applies its normal bounded retry/backoff policy. This separation avoids making the provider claim visibility into failures that occur only during dialing.

**Built 2026-09-20 — the paragraph above holds.** Two dated paragraphs stood here from 2026-09-19 recording that the build did not honour it: the Swarm was built `with_tcp` alone, a `/dns4` or `/dns6` dial failed `MultiaddrNotSupported`, which `dialing.rs` classifies as structural, so `record_permanent_failure` forgot the address instead of retrying it, and `profile-config` refused a configured `dns4`/`dns6` host (`AddressHostNotBuilt`) as the repository's rule for a capability the build omits. They said the change that builds the DNS transport into the Swarm deletes them, and it has: the builder is `with_tcp(...)` then `.with_dns()` (`crates/transport/libp2p/src/runtime/mod.rs`), wrapping the base transport in `libp2p_dns::tokio::Transport::system`; the `dns` feature is on the root manifest's list and `profile-config`'s `DIALABLE_HOST_PROTOCOLS` carries `dns4` and `dns6`, held together by `tools/checks/check_dialable_hosts.sh`; and `crates/transport/libp2p/tests/dns_transport.rs` is the test the Stage 12 precondition named — it starts the real runtime, dials a `/dns4` name under RFC 6761's never-resolving `.invalid` and asserts positively on the resolver diagnostic, because with `.with_dns()` removed the same test fails (measured, both strings recorded beside the assertion). A `/dns4` dial therefore no longer produces `MultiaddrNotSupported`, and the structural classifier was REWRITTEN for it rather than left alone (PR #111 commits c060ce8 and 8c11772): the DNS wrap re-shapes EVERY dial's error into `Other`, so a classifier that matched the variant stopped classifying anything; it now asks the address — an unknown protocol, or no hop this build can dial — and runs on every failed attempt, names included — so a lookup failure is an ordinary dial diagnostic the ConnectionManager retries under its bounded policy, which is what ADR-0010 and the paragraph above say. The refusal is lifted: a configured `dns4` bootstrap peer, static server or static relay validates, and the six shipped examples that name `/dns4` hosts no longer draw it (whether each validates now turns only on the providers it enables). **One deployment-visible change rides with it**: `Transport::system` reads the host's resolver configuration at construction, so a host with none, as first built, refused to START (`SubstrateError::Transport`). Since PR #111 commit 816e477 the runtime reads the resolver configuration itself — once, at start — and on a host with none builds the transport on an EMPTY resolver: the node starts, a name fails at dial as an ordinary lookup failure, and `SwarmEvent::ResolverUnavailable` says why, once. A profile that names no DNS host loses nothing on such a host, which the startup refusal had taken from it; one that does learns at its first dial and from that event. `AddressHostNotBuilt` stays in the vocabulary with no constructible input — every host the grammar accepts is now dialable — dormant for the next host the vocabulary admits before its transport, which is the sequence `dns4` has just completed; the three assertions that rested on it were removed rather than re-pinned through the grammar, whose refusal answers a different question. NOT MEASURED: a successful resolution end to end. The test proves the transport is built by naming the failure a resolver produces; nothing composes this provider before Stage 12, so no configured name is dialled today.

**A 2026-10-14.** "Once, at start" is the HOST read. Where the host read is gated off (Android, #277), the platform's view supplies the resolver's facts and replaces them live when the network's change (CONNECTIVITY.md §14, A 2026-10-14); strict Private DNS fails closed there, so an Android-facing profile names its static servers, relays and bootstrap peers by IP literal, or accepts that names resolve only off strict mode.

## Configuration

Default max entries: 64, authored entries and the bootstrap-peers overlay's together (A 2026-10-21, ADR-0062: `<state>/bootstrap-overlay.json`, the addresses a person adds through `admin.peers.add_address` or the embedded host, with `remove_address` and `list_addresses` beside it, ADR-0028 A 2026-10-21 (ii); the provider emits them beside the authored ones with the same configured provenance and the same rules — they pass the operator door whatever their class (ADR-0052 rule 9), so the add method applying ADR-0052 rule 2's special-use floor to every component, whoever the caller, is their one class gate (rule 8's named exception, A 2026-10-21); the composition builds the provider from the overlay alone when the profile authors none, at priority 30, and an authored `enabled: false` refuses the add, while entries already present under one load, are emitted by nothing, raise a load-time diagnostic naming the disabled provider and are listed as not dialled). Invalid PeerId/multiaddress syntax fails config validation, an overlay entry outside the grammar fails the load by name, and a composed count above 64 at start is FATAL naming the overlay. A valid DNS multiaddress whose hostname later fails to resolve does not invalidate the provider configuration; the dial attempt reports `PeerUnreachable`/address-resolution diagnostics as appropriate. That has held since 2026-09-20, when the DNS transport was built into the Swarm; §DNS ownership records the gap that stood before it.

### The accepted address vocabulary

"Invalid multiaddress syntax" above needs a vocabulary to be checkable, and this is it. A configured entry is a DIRECT address, `/<host>/<value>/<transport>/<port>/p2p/<PeerId>`, or — since ADR-0052 rule 9 (A 2026-10-09) — a CIRCUIT ROUTE, `<relay route>/p2p/<relay PeerId>/p2p-circuit/p2p/<PeerId>`, the relay route a direct address in the same grammar; two shape conditions hold for a circuit route: the prefix ends in the relay's `/p2p/<relay PeerId>`, and exactly `/p2p/<PeerId>` follows `/p2p-circuit` — a foreign destination, a second `/p2p-circuit` or a circuit without the relay id is refused by shape, named (the `p2p-circuit` marker is read at a protocol position only, so a host named `p2p-circuit` stays a direct address). A circuit route is the operator's route to a relay-only peer and belongs to this provider's entries alone: `relay.client.static_relays` and `autonat.client.static_servers` take the direct form only and refuse a circuit route by role (a relay, or an AutoNAT server, must be reached directly, not through a circuit). Whether this node may dial THROUGH the relay is the gate's question at dial time (`RelayCircuit`, ADR-0036), never this grammar's. For a direct address, and for the relay route of a circuit:

- **host** is one of `ip4`, `ip6`, `dns4`, `dns6` — all four dialable by the build since 2026-09-20 (§DNS ownership); a name here is the OPERATOR's and is resolved when the dial path consumes it, which is the resolver policy ADR-0052 rule 2 states — a name a PEER supplies is refused at the address-class boundary and never reaches the resolver;
- **transport** is `tcp` — the only transport the substrate builds;
- **port** is `0..=65535`;
- an `ip4` value is a dotted quad, an `ip6` value is hexadecimal, and a DNS name is **not resolved at validation** — resolution happens when the dial path consumes it (§DNS ownership), never here.

This is a decision, not a description of what a multiaddr parser happens to accept, and it is deliberately narrower than the general multiaddr grammar: a profile naming a transport this build cannot dial is a configuration error an operator should read at validation, not a dial failure later. The set widens in the change that adds the transport — a new listen or dial capability and the configuration that may name it belong in the same commit.

It is spelled out in `interweave-profile-config` rather than delegated to the backend's parser. A configuration crate that pulled in a networking stack to name four protocols would invert the layering `crates/api` exists to hold, and would make the accepted set a property of a dependency rather than of this document.

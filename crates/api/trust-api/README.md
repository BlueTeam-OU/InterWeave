# trust-api

Peer trust/policy input-output types; no discovery or UI semantics.

**Current status:** Stage 1, active workspace member. Types and decisions only — nothing here dials, connects, or knows what a connection is.

## What it decides

One question: *may this peer use the application data plane?* Authenticating a PeerId proves who a peer is; it does not decide whether they may do anything, and ADR-0012 keeps those apart.

## Three properties are structural, not documented

**Deny by default.** `PeerTrustPolicy::default()` is an empty allowlist admitting nobody, and there is deliberately no `allow_all` constructor — that default was rejected by ADR-0012, and a convenience constructor is how a rejected default comes back: first in a test, then a fixture, then a shipped profile.

**Narrowing only, inside the admission set.** An authored `EndpointTrustPolicy` subtracts from the operator's list and can never add to it. The intersection happens inside `PeerTrustPolicy::decide_for_endpoint`, so there is no ordering a caller can pick that lets an authored policy admit a peer the operator's list refused. `is_subset_of` additionally lets configuration validation reject a widening subset at load, where the operator can still see it. An endpoint's own set (`EndpointSet`, ADR-0067 decision 6) is the exception by design: it replaces the operator's list at its endpoint and may hold a peer the operator never trusted, so it is part of the admission set (`PeerTrustPolicy::admission`, the union bounded as one set) that decides the connection. `PeerTrustPolicy::decide_at_endpoint` applies the order — own set, else the authored policy — and is judged against the operator's list, never the admission set, so an inheriting endpoint trusts nothing a person trusted in an identity.

**Infrastructure is a different type.** `InfrastructureSet` is not a flag on the trust policy. A relay or AutoNAT server is authorized for reachability control and nothing else (ADR-0036) — no GossipSub, direct v2, endpoint directory, Kademlia routing, or Channel delivery. It even returns a plain `bool` rather than a `TrustDecision`, so the two answers cannot be passed interchangeably at a call site. That interchange *is* the confused deputy the separation prevents.

## Why `TrustDecision` is not a `bool`

A boolean at a security boundary reads identically whether it means allowed or denied, and the reason has to be reconstructed for every diagnostic. `DenyReason` travels with the decision instead — and is explicitly local: `NotAllowlisted` versus `NarrowedByEndpoint` must never reach the direct wire, where distinguishing them would tell a probing peer which endpoints exist and how they are configured.

# ADR-0031 — amendment history

### Amendment 2026-11-22 — The directory protocol id is `/interweave/endpoints/2.0.0` from the Stage 22 release

The decision named the protocol `/interweave/endpoints/1.0.0`. ADR-0067 makes every EndpointId the derived name of its endpoint's key and its decision 7 (A 2026-11-22) gives every protocol whose frames carry one a major, so the directory is `/interweave/endpoints/2.0.0` from the Stage 22 release and `1.0.0` is not spoken beside it. Nothing this record decided changes: who may query, which endpoints are listed, that no metadata beyond the EndpointId is exposed, and the bounds. The layout (an entry of exactly 52 bytes, a largest response of 1710) is `transport/libp2p/ENDPOINTS.md`'s.

# ADR-0012 — amendment notes

### Amendment 2026-10-28 — PeerTrustPolicy is the admission set the composition derives from the operator's list and the endpoint overlay's sets

ADR-0067 decision 6 makes trust per endpoint: each identity's trusted
set lives in the endpoint overlay (ADR-0028), the operator's list stays
what `config.yaml` and the trust overlay make it, and the transport
admits a connection under the union of the two, derived at load and on
every overlay write. The implementation-implications sentence that
read "EndpointRegistry applies only an additional narrowing filter and
must never widen PeerTrustPolicy" now says which set that policy is,
so that a reader building admission from this record alone does not
refuse a peer that only an identity trusts. The rule that trust-core
has no discovery or endpoint dependency holds as written: the
composition root reads both overlays and hands trust-core the derived
set and each endpoint's effective set, as it hands it the operator's
list today. Nothing else of this record changes: deny by default, the
static allowlist, discovery never mutating it, the local privileged
administration path, and the ADR-0036 set kept apart. Propagation: the
implementation-implications sentence; this note; the log row; the
digest entry.

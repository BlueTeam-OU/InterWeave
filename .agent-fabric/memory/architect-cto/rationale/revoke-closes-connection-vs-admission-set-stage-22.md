---
role: "architect-cto"
class: rationale
topic: "revoke-closes-connection-vs-admission-set-stage-22"
description: "Open item (2026-11-04, from #559's review §2): TRANSPORT.md L238 and ADR-0037 say a revoke closes the peer's connections; ADR-0067 d.6 says a peer leaves the admission set only when no set holds it — from the Stage 22 release the close is…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d7a1a2966bf721af
---

## Open item (2026-11-04, from #559's review §2): TRANSPORT.md L238 and ADR-0037 say a revoke closes the peer's connections; ADR-0067 d.6 says a peer leaves the admission set only when no set holds it — from the Stage 22 release the close is "when the peer leaves the admission set", unamended anywhere

What the tree says today (origin/main d834ecf21): `architecture/contracts/TRANSPORT.md`
line 238 "Revoking a connected peer also closes its data-plane connection and produces
`PeerDisconnected { reason_class: policy }`"; ADR-0037 §Decision (A 2026-10-03), its
history note and the digest row "a revoking set closes the peer's connections". Both
describe `admin.trust.set` on the node's one list, true until the Stage 22 release.

ADR-0067 decision 6 (unamended on this point): the admission set is the union of the
operator's list and every endpoint's overlay set; "a peer leaves it when it is in no
set"; admission at the connection is the node's. So from Stage 22 an endpoint-less
revoke of a peer an identity's set still holds closes nothing, and an identity's
revoke (the endpoint dimension) closes nothing while any set holds the peer.

**Why:** the #559 blind review (5772385912) found the two texts disagree once §8
says "connections, which the node admits for a peer any endpoint trusts"; it is
pre-existing, outside #559's range, and a contract rule (mine) — p2p-network-dev
builds `PeerTrustPolicy` from d.6 in Stage 22's batch and will read TRANSPORT.md.

**How to apply:** before Stage 22 opens (or with its first p2p PR), amend ADR-0067
d.6 with one clause — the connection closes, `PeerDisconnected { policy }`, when the
peer LEAVES THE ADMISSION SET, by whichever write removed its last set — and qualify
TRANSPORT.md L238 and ADR-0037 "until the Stage 22 release; from it, when the peer
leaves the admission set (ADR-0067 d.6)". Also settle the review's §3 risk: write
"a peer in the admission set" rather than "a peer any endpoint trusts" where the
operator's list may have no inheriting endpoint. Three-part amendment + digest row.

*Observed 2026-11-04 (architect-cto)*

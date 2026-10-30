---
role: "architect-cto"
class: solution
topic: "empty-response-stream-is-peer-unreachable-2026-10-26"
description: "InterWeave direct v2 and endpoint directory — a response stream ending with zero bytes is PeerUnreachable (the remote never answered), a short field means at least one byte; ruled 2026-10-26 on #460 F3, supplied on a for/ branch"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - bb5a8a6194341fad
---

## InterWeave direct v2 and endpoint directory — a response stream ending with zero bytes is PeerUnreachable (the remote never answered), a short field means at least one byte; ruled 2026-10-26 on #460 F3, supplied on a for/ branch

Ruled 2026-10-26 (j392, p2p-network-dev-01's OBSERVATION 01a1799b on
InterWeave #460 F3): on `/interweave/direct/2.0.0` and the endpoint
directory protocol, a response stream that ends with NO bytes is the
exchange not completing — the responder read the request and never
answered (its own `request_timeout` firing first, SPIKE-002 finding 1's
race, which the requester reads as `Io(Eof)`; or a dropped response
channel; yamux 0.14 sends FIN not RST after the request was read to EOF)
— and is `PeerUnreachable`, in TRANSPORT.md's outcome-unknown class. A
"short field", an unassigned code or a trailing byte means at least one
byte arrived and stays a local `ProtocolViolation`. Both classes are
outcome-unknown, so no client delivery state changes.

Written in DIRECT.md §Response byte layout and §Peer not connected,
transport/libp2p/ENDPOINTS.md after the unassigned-reason paragraph,
contracts/ENDPOINTS.md (step 8, directory validation bullet) and
TRANSPORT.md's PeerUnreachable sources, each citing the tests on #460
(stage6_malformed_frames.rs OwnTimeout/Omitted, stage8's empty frame).
Supplied as a for/ branch from #460's head
(`for/p2p-network-dev-01/empty-response-unreachable`, a2a905b1 plus the
alignment commit); no schema, fixture or ADR change — the wire shape is
unchanged and the class is local.

**Why:** a contract sentence listing causes must name only the cases its
tests prove (the PR's own review narrowed the code comment the same way
in b37ce502); "any other zero-byte end reads the same" covers the rest.

**Host note:** this login's Fedora rustc is 1.99.0 with no rustup, and
the pin went to 1.99.1 on 2026-10-24 (8d488f38), so `cargo xtask checks`
refuses here; the doc checks run as bare scripts. See
[[rustsec-zbus-family-ignores-and-arming-below-minimum]] for the other
CI-outage shape.

**CLOSED 22:05Z:** #460 (12 work, 3 fix, 2 merge) merged; a2a905b1 and e3ba33cb on origin/main.

*References: rustsec-zbus-family-ignores-and-arming-below-minimum*

*Anchors: `crates/transport/libp2p/src/direct_codec.rs::refuse_an_unanswered_exchange`, `architecture/transport/libp2p/DIRECT.md::ends with no bytes at all is not a short field`, `architecture/contracts/TRANSPORT.md::ended with no bytes`*

*Observed 2026-10-26 (architect-cto)*

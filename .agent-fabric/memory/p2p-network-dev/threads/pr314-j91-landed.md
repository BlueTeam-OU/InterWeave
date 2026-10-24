---
role: "p2p-network-dev"
class: threads
topic: "pr314-j91-landed"
description: "#314 merged 400011af 2026-10-20 (j91): deny.toml judges aarch64-linux-android; unused allows/exceptions deny; armed on architect-cto's below-minimum word"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6794f98aa34845cb
---

## #314 merged 400011af 2026-10-20 (j91): deny.toml judges aarch64-linux-android; unused allows/exceptions deny; armed on architect-cto's below-minimum word

PR #314 merged as 400011af on 2026-10-20 (job j91, ADR-0057 decision 3's deny.toml half).
deny.toml's [graph] targets now holds aarch64-linux-android beside x86_64; the two Android Slint
crates have per-crate Royalty-free exceptions; [licenses] sets unused-allowed-license and
unused-license-exception = "deny" (see [[cargo-deny-unmatched-exception-only-warns]]).
Armed at 1 work, 1 fix on architect-cto's word (DECISION 01a15809, AUTHORIZES header,
`fabric-pr arm --authorization <id>`): architect-cto is InterWeave's below-minimum arming
authority (settings repos/InterWeave.json), and gives the word only after reading the posted review.

Nothing carried. Open elsewhere: apps/human-android/STORE-CHECK.md's "Open" paragraph is now stale
(rust-ui-dev-01 told, OBSERVATION 01a155a2-f6af); the native Skia library's notices are plan §20
step 10's.

*References: cargo-deny-unmatched-exception-only-warns*

*Observed 2026-10-20 (p2p-network-dev)*

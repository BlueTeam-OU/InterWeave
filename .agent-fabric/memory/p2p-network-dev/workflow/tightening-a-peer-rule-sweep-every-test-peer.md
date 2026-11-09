---
role: "p2p-network-dev"
class: workflow
topic: "tightening-a-peer-rule-sweep-every-test-peer"
description: "When a rule starts refusing a peer shape (e.g. a relay that refuses ping), sweep every test peer of that shape, tests/support included; and xtask ci's cargo test stops at the first failing binary"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - bd02e3733098037d
---

## When a rule starts refusing a peer shape (e.g. a relay that refuses ping), sweep every test peer of that shape, tests/support included; and xtask ci's cargo test stops at the first failing binary

j126 (2026-10-25): CONNECTIVITY.md §14 A 2026-10-25 (b) made a client close a relay control peer that refuses ping. I gave ping to the bare relays in the relay suites (8d05ac4a) but missed `tests/support/src/e2e/relay.rs`, the relay every android-e2e and desktop-e2e relayed suite shares; android-e2e human_chat then failed ("D receiving R's two did not happen in time") and was fixed by e0f43879.

**Why:** I swept by the pattern `relay::Behaviour::new`; the shared helper builds the production `ServerField` through `relay_server_driver::build_behaviour`, so it never matched.

**How to apply:** before committing a rule that refuses a peer shape, grep for every way that peer is constructed (the crate type, the production builder, the shared harness helpers under tests/support), not one constructor. Also: `cargo xtask ci` runs `cargo test --workspace` WITHOUT `--no-fail-fast`, so one failing binary hides every binary after it (5 of ~all ran); after a failure, rerun the workspace tests with `--no-fail-fast`. cargo-deny lives in `~/.cargo/bin`, which a PATH built for the scratchpad toolchain drops: add it, or the two deny checks exit 2 as "not installed".

*Anchors: `tests/support/src/e2e/relay.rs::build_behaviour`, `crates/transport/libp2p/src/relay_keepalive.rs::Refused`*

*Observed 2026-10-25 (p2p-network-dev)*

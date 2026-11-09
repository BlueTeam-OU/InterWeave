---
role: "devex-tooling"
class: solution
topic: "rustup-toolchain-install-self-updates"
description: "rustup toolchain install also self-updates rustup (1.29.1 → 1.29.2 at j214), drifting from the registry/android pins' RUSTUP_VERSION"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 43a091cef13ed088
---

## rustup toolchain install also self-updates rustup (1.29.1 → 1.29.2 at j214), drifting from the registry/android pins' RUSTUP_VERSION

`rustup toolchain install 1.99.1` on develop-qzapp (2026-10-24, j214) printed
"checking for self-update" and moved this account's rustup from 1.29.1 to
1.29.2 — and set the new toolchain as the account default. agent-fabric's
projects/registry.json rustup row and InterWeave's
tools/host/android/android-toolchain.pins both name 1.29.1, so after any
toolchain bump an account's rustup no longer matches its pin. Before a bump,
`rustup set auto-self-update disable` keeps it; fabric-tools still reported
`ok` (its proof does not compare the version).

Same bump: clippy 1.99 added pedantic `assert_is_empty` (163 sites in
InterWeave, allowed workspace-wide on the owner's word, commit 587ba286)
and `double_must_use` (4 sites, fixed at the sites). A bump's clippy run
must be `--keep-going` without `-D warnings` to see every site: with
`-D warnings` a failing API crate hides everything downstream of it (12
of 167 seen). See [[host-rust-floats-rustup-via-fabric-tools]].

*References: host-rust-floats-rustup-via-fabric-tools*

*Anchors: `tools/host/android/android-toolchain.pins::RUSTUP_VERSION`, `Cargo.toml::assert_is_empty`, `agent-fabric:projects/registry.json::rustup-init`*

*Observed 2026-10-24 (devex-tooling)*

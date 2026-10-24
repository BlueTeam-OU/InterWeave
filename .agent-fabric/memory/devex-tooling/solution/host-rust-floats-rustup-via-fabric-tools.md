---
role: "devex-tooling"
class: solution
topic: "host-rust-floats-rustup-via-fabric-tools"
description: "develop-qzapp's system rust floats with dnf (1.99.0 on 2026-10-19, pin 1.98.1); fabric-tools --install rustup gives an account the pin, but links only cargo/rustc/rustup"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - eae0d113696ee4d0
---

## develop-qzapp's system rust floats with dnf (1.99.0 on 2026-10-19, pin 1.98.1); fabric-tools --install rustup gives an account the pin, but links only cargo/rustc/rustup

develop-qzapp's /usr/bin cargo, rustc, clippy-driver and rustfmt are Fedora packages (cargo-1.99.0-1.fc43
on 2026-10-19), which a dnf update moves with no commit in InterWeave. InterWeave's rust-toolchain.toml
pin (1.98.1) is honoured only by rustup. The fix for an account is `fabric-tools --install rustup`, run from an
InterWeave working copy. It installs rustup 1.29.1 into ~/.cargo and links ONLY cargo, rustc and rustup into
~/.local/bin, which comes before /usr/bin. The first cargo run auto-installs the pinned channel.
`cargo clippy`/`cargo fmt` are then the pin's because cargo finds a subcommand in $CARGO_HOME/bin (rustup's
proxies) BEFORE PATH, and rustup's proxy prepends $CARGO_HOME/bin. It is NOT because the toolchain bin goes on
PATH (strace-measured, #311 round 1 F1). Even /usr/bin/cargo runs rustup's clippy that way. Moving CARGO_HOME
makes `cargo clippy` Fedora's 0.1.99. So neither PATH's tool nor `rustup which` predicts what cargo runs: ask
`cargo clippy --version` / `cargo fmt --version`. A rustup build prints "(hash date)", which must match rustc's;
a distribution build prints none (check_rustc_pin.sh since #311).

**Why:** the drift showed up as clippy 1.99's double_must_use failing trust-api locally while CI was green
(observation 01a154ad from p2p-network-dev-01).

**How to apply:** when a local lint differs from CI, run `bash tools/checks/check_rustc_pin.sh` first.
To know which clippy or rustfmt is in use, ask cargo, never `rustup which`.

*Anchors: `tools/checks/check_rustc_pin.sh::cargo_says`*

*Observed 2026-10-19 (devex-tooling)*

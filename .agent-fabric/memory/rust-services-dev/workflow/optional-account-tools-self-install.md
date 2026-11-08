---
role: "rust-services-dev"
class: workflow
topic: "optional-account-tools-self-install"
description: "cargo xtask checks exits 2 on check_unused_dependencies — cargo-machete is an optional account tool, installed per login with fabric-tools --install"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - d354e8e559588a33
---

## cargo xtask checks exits 2 on check_unused_dependencies — cargo-machete is an optional account tool, installed per login with fabric-tools --install

agent-fabric `projects/registry.json` lists cargo-machete (0.9.2, sha256-pinned) as an `"optional": true` account tool twice: interweave's row for p2p-network-dev, rust-ui-dev and devex-tooling — NOT rust-services-dev — and agent-fabric-gateway's row for rust-services-dev and devex-tooling (devex-tooling's reply and my request to fabric-coordinator both had the two swapped). fabric-tools reports an optional tool but never installs it unasked, while cargo-deny (not optional) is installed. fabric-coordinator's job j605 (reply 01a1a782, 2026-11-04) drops "optional" from interweave's row and adds rust-services-dev to it, after its PR #482; until that commit lands, self-install. So on a fresh login `check_unused_dependencies.sh` exits 2 ("not installed"; the guard refuses to pass without running). Fix: `fabric-tools --install cargo-machete` (into ~/.local/bin). devex-tooling's reply of 2026-11-04 (01a1a75a); installed that day.

The self-test `test_check_unused_dependencies.sh` builds a fixture workspace outside the repo, so `rust-toolchain.toml` does not apply, and this login has no rustup default: run it with `RUSTUP_TOOLCHAIN=<rust-toolchain.toml channel>` (1.99.1 then).

**Why:** cost a question to devex-tooling to learn the tool was self-install.
**How to apply:** any exit 2 "X is not installed" from a tree check — look for the tool's registry row and `fabric-tools --install` it before asking.

*Anchors: `tools/checks/check_unused_dependencies.sh::cargo-machete is not installed`, `agent-fabric:projects/registry.json::cargo-machete`*

*Observed 2026-11-04 (rust-services-dev)*

---
role: "devex-tooling"
class: workflow
topic: "ci-warnings-scan-reads-text"
description: "fabric-ci-warnings [log] findings are text matches: a test name or banner with \"deprecat\"/\"warn\" is a finding"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 290ef237566bddfd
---

## fabric-ci-warnings [log] findings are text matches: a test name or banner with "deprecat"/"warn" is a finding

fabric-ci-warnings' `[log]` level matches words on unmarked log lines. Known
false positives: Gradle 9.6's welcome banner ("Important project hierarchy
lookup deprecations"), and a passing self-test whose case name said
"deprecated" (InterWeave fefa661c7 renamed it). Write case names and log lines
of your own without "deprecated" or "warning" unless they are one. The
rustup fix for the auto-install deprecation is `rustup toolchain install
--no-self-update` (no name: installs rust-toolchain.toml's, a no-op when it is
there), never `rustup show active-toolchain ||`. See
[[rustup-toolchain-install-self-updates]].

*References: rustup-toolchain-install-self-updates*

*Observed 2026-11-04 (devex-tooling)*

---
role: "architect-cto"
class: solution
topic: "dev-profile-line-tables-build-size-2026-10-21"
description: "InterWeave dev builds at line tables only (j210, owner's ask via fabric-coordinator 2026-10-20): the measured target sizes, why a dependency-only override recovers little, what was checked about backtraces"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d2051f9b28a3ef73
---

## InterWeave dev builds at line tables only (j210, owner's ask via fabric-coordinator 2026-10-20): the measured target sizes, why a dependency-only override recovers little, what was checked about backtraces

Owner's ask (REQUEST 01a15b80, fabric-coordinator, 2026-10-20): decide
whether InterWeave's 60-75 GB per-worktree cargo target shrinks. Decided
2026-10-21: `[profile.dev] debug = "line-tables-only"` in the root
Cargo.toml, landed on my PR BlueTeam-OU/InterWeave#326 with the figures in
the manifest's comment.

Measured on develop-qzapp, `cargo build --workspace --all-targets` from a
clean target, head 48d187bb:

| dev profile | target | 223 executables | incremental | rlibs | wall |
|---|---|---|---|---|---|
| cargo default | 60 GB | 48 GB | 7.5 GB | 3.5 GB | 67 min |
| `[profile.dev.package."*"] debug = "line-tables-only"` | 48 GB | 36 GB | 7.5 GB | 2.8 GB | 52 min |
| `[profile.dev] debug = "line-tables-only"` | 25 GB | 17 GB | 3.9 GB | 2.1 GB | 37 min |

**Why the dependency-only override recovers little:** the DWARF in a test
binary is mostly emitted while compiling the workspace crates, where the
generic code of tokio, libp2p and Slint is instantiated and takes the
workspace's debug level. With dependencies at line tables, one test
binary still held 291 MB of debug sections against 32 MB of code
(`.debug_str` 162 MB). Under line tables everywhere it holds 102 MB
(`.debug_str` 46, `.debug_line` 23, `.debug_info` 17).

**Checked:** a panic backtrace keeps file:line under line-tables-only and
under `split-debuginfo=unpacked` alike on this toolchain (rustc 1.99,
small program). No gdb or lldb is installed on the host, so the locals
line tables drop are read by nobody here. Incremental applies to
workspace members only, so `incremental = false` for dependencies is a
no-op. A one-off full-debug build: `CARGO_PROFILE_DEV_DEBUG=full`.

**How to apply:** the fleet's other half is fabric-jobs switch cleaning a
left job's target (python-dev-03 j75); a shared CARGO_TARGET_DIR per login
stays rejected (rust-ui-dev slice shared-target-worktree: binaries bake in
CARGO_MANIFEST_DIR). Related: [[rustsec-zbus-family-ignores-and-arming-below-minimum]].

**Landed:** 54e97f07 + review fixes 57552ca8, c0312bd3 on #326 (2 work,
2 fix); two blind reviews posted (round 1: README over-claimed device-run
records and Keystore custody, Stage 18 date 2026-10-11 not 10-19,
incremental applies to path packages incl. patched third_party, name the
host; round 2 clean). Owner's word asked in REPLY 01a15c3a (j210).
Review risk carried: CI cache key hashes rust-toolchain.toml, Cargo.lock
and the ISO week, not Cargo.toml, so one cold dependency rebuild per
runner cache until the week rolls over.

*References: rustsec-zbus-family-ignores-and-arming-below-minimum*

*Anchors: `Cargo.toml::[profile.dev]`, `Cargo.toml::line-tables-only`*

*Observed 2026-10-21 (architect-cto)*

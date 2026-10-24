---
role: "rust-services-dev"
class: solution
topic: "plugin-version-from-workspace"
description: "How the Claude plugin and claude-channel carry the ADR-0065 release version: build.sh, release archive, --version stamp"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 3a456781e6d11a44
---

## How the Claude plugin and claude-channel carry the ADR-0065 release version: build.sh, release archive, --version stamp

From InterWeave #398 (j46, 2026-10-23):
- `packaging/claude-plugin/build.sh` reads `[workspace.package] version` from the root Cargo.toml with python tomllib,
  not `cargo metadata`, because `apps/claude-channel/tests/packaging_build.rs` builds in a scratch repo that is no cargo
  workspace. It writes that version only into the STAGED plugin.json; the committed one keeps the placeholder 0.0.0.
  `--version <v>` refuses a value that differs from it, and `tools/ci/release_linux_archive.sh` passes its `<version>`.
  So that archive's self-test must build at the workspace version, not a made-up one.
- `claude-channel --version` prints `<version>+<12-hex commit>`, stamped by `apps/claude-channel/build.rs`. The id is fixed
  at 12 hex digits because git's default abbreviation grows with object count, so a shallow CI clone and a full clone
  would differ: `--short=12` is only a minimum, so the full id is cut to 12. The rerun triggers come from
  `git rev-parse --path-format=absolute --git-path`, which works in worktrees. It watches only paths that exist, since a
  missing one reruns every build: logs/HEAD (a packed branch moves no other file), HEAD, packed-refs, the branch ref's
  nearest existing ancestor directory (reflogs off + packed branch), and `reftable/` under both the git dir and the
  common dir (the reftable backend). p2p-network-dev-01 measured the last two as STALE; they are fixed in 29708852 and
  4c6b6585. #398 merged 2026-10-23 as 356c4d13. `tools/stamp/commit_stamp.rs` (#399) is the shared copy of these rules,
  and j47 points claude-channel at it.
- Whether `claude plugin update` reports an upgrade rests on SPIKE-001 S20 case B: a higher plugin.json version is
  reported. A rebuild at the same version reports nothing but still runs in place. See [[claude-plugin-runtime-facts]].

*References: claude-plugin-runtime-facts*

*Anchors: `packaging/claude-plugin/build.sh::tomllib`, `apps/claude-channel/build.rs::reftable`, `tools/ci/release_linux_archive.sh::--version`*

*Observed 2026-10-23 (rust-services-dev)*

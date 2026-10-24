---
role: "rust-ui-dev"
class: solution
topic: "previous-build-axis-exists"
description: "Before saying InterWeave has no \"previous build\" for upgrade/rollback, read testing.md §Compatibility fixtures — a real one is built in CI"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e13a062ffa3fbd6f
---

## Before saying InterWeave has no "previous build" for upgrade/rollback, read testing.md §Compatibility fixtures — a real one is built in CI

InterWeave has a real "previous build" even with no release tag: testing.md §Compatibility
fixtures (A 2026-10-08) defines the upgrade matrix's previous-build axis, entries in
`tools/ci/previous-builds.txt` (first: `stage13-45ba3928`, #165's merge), built by
`tools/ci/build_previous_builds.sh <dir>` (release, own worktree, ~3 min) into
`<dir>/<label>/{transport-daemon,transportctl}` — no human-desktop — and run by the `rust`
job's upgrade-matrix step via `INTERWEAVE_PREVIOUS_BUILDS`.

On j30 (2026-10-20) I told architect-cto "the previous build has no referent" because
`git tag` was empty; they ruled on that, and I had to bring it back and get it re-ruled.
**How to apply:** grep `previous-builds`, `upgrade_matrix` and testing.md before any claim
about what versions exist to test against.

Measured with it: the Stage 13 daemon starts on a HEAD tree, serves the same PeerId,
writes nothing, and IGNORES both overlays (endpoint overlay measured: disabled endpoint
reads enabled; trust overlay absent from that build's source, so a revoked peer is allowed
again). That led to the profile-format marker ruling ([[profile-format-marker]]).
Its socket path must be under SUN_LEN: run a hand test from `mktemp -d`, not the scratchpad.

*References: profile-format-marker*

*Anchors: `tools/ci/previous-builds.txt::stage13-45ba3928`, `tools/ci/build_previous_builds.sh::INTERWEAVE_PREVIOUS_BUILDS`*

*Observed 2026-10-20 (rust-ui-dev)*

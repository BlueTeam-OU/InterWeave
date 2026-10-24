---
role: "devex-tooling"
class: workflow
topic: "release-logic-lives-in-tools-ci"
description: "editing release.yml (or any workflow step with a case/flag mapping)? the logic goes in a tools/ci script with a mocked-gh self-test, or review files a P2"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f3dfdcc046df98c4
---

## editing release.yml (or any workflow step with a case/flag mapping)? the logic goes in a tools/ci script with a mocked-gh self-test, or review files a P2

In InterWeave, release.yml's header promises "no release logic of its own; every step a tools/ci script with a self-test beside it". On #394 (2026-10-23) I put a five-line `case "$PRERELEASE"` → `--prerelease --latest=false` mapping inline in the Publishing step; the blind review filed it P2 (judged real): no test read it, and `true) kind=()` passed every suite. Fixed by moving the whole `gh release create` call into `tools/ci/release_publish.sh`, tested with `gh` mocked on PATH recording argv (`test_release_publish.sh`).

Since #394 the tag rule (SemVer 2.0.0 grammar, no build metadata, equal to the workspace version via `cargo metadata --no-deps`) is `tools/ci/release_tag.sh`, and `tools/checks/check_workspace_version.sh` keeps `[workspace.package] version` the only version of the 57 members (spikes are their own workspaces and are excluded). Also measured there: without `--latest`, the API default makes a newly published release the latest. The platform does not choose.

**Why:** a workflow line is read by actionlint and nothing else, so any mapping inline in it is untested by construction.
**How to apply:** before adding a conditional or a flag choice to a workflow `run:` block, write it as a tools/ci script with a self-test (mock the CLI on PATH) and make the step a single call. Related: [[ci-command-line-is-read-by-guards]], [[new-guard-needs-ci-and-xtask]].

*References: ci-command-line-is-read-by-guards, new-guard-needs-ci-and-xtask*

*Anchors: `interweave:.github/workflows/release.yml`, `interweave:tools/ci/release_publish.sh`, `interweave:tools/ci/release_tag.sh`*

*Observed 2026-10-23 (devex-tooling)*

---
role: "rust-ui-dev"
class: solution
topic: "j71-fleetdeck-linux-only"
description: "j71 FleetDeck Debian/Fedora-only: part 1 (release manifest test) merged as #20; part 2 is removing macOS/Windows code, scripts and docs"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-12"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f4bf8acd7dd6ce3a
---

## j71 FleetDeck Debian/Fedora-only: part 1 (release manifest test) merged as #20; part 2 is removing macOS/Windows code, scripts and docs

Owner's rule 2026-10-11: FleetDeck ships for Debian and Fedora only. devex-tooling's #19 (2 work, 8 fix) made CI/release Linux-only (check legs ubuntu, debian-12, fedora-42).

Part 1 done: BlueTeam-OU/agent-fabric-fleetdeck#20 (1 work, 2 fix) merged ee22e618. src/update.rs: RELEASED_TARGETS = linux-x86_64, linux-aarch64; assert_release_manifest checks assets named agent-fabric-fleetdeck-<target> and per-release sha256, exercised on synthetic manifests (latest.json is 0.0.0).

Part 2 (next, devex OBSERVATION 01a12a70): src/platform/{macos,windows}, vendor/portable-pty's Windows half, distribution/install.ps1 + install.cmd, scripts/*.ps1, scripts/package_windows_conpty.py and its maintenance-test, docs/next/website install.mdx (+ ja/) which still teach herdr-<target> and mac/win downloads, src/update.rs windows branches (e.g. line ~535 preview_windows...). Read the cfg gates and Cargo targets first; check overlap with rust-ui-dev-02 (holds src/app/api/tabs.rs, src/api/schema*, src/cli/{tab,spec}.rs, src/workspace.rs tab move, fleet-deck/fabric_deck.py; branch develop-qzapp/rust-ui-dev-02/feat/tab-move-workspace, PR #16). The fork's CLAUDE.md: platform code isolated in src/platform/<os>.rs.

Gotchas: the repo was renamed; keep origin until fabric-coordinator says the registry update is on this account; fabric-pr post-review needs GH_REPO=BlueTeam-OU/agent-fabric-fleetdeck (HTTP 307 otherwise). Conventional commit subjects from the first commit (see [[herdr-conventional-commits]]). Owner arms under 8 work commits: give a short script path (~/armNN.sh) that waits for checks then merges --merge.

*References: herdr-conventional-commits*

*Observed 2026-10-11 (rust-ui-dev)*

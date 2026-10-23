#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# packaging/claude-plugin/build.sh
#
# Assemble the Claude Code marketplace from the bridge's release build:
# this directory's marketplace manifest and plugin, with the built
# `claude-channel` copied to `interweave/bin/`, where the plugin's
# `.mcp.json` starts it (`${CLAUDE_PLUGIN_ROOT}/bin/claude-channel`).
#
#   bash packaging/claude-plugin/build.sh [--version <v>] <out-dir> [<claude-channel binary>]
#
# The binary defaults to the workspace's release build
# (`cargo build --release -p interweave-claude-channel`). The output is a
# directory `claude plugin marketplace add <out-dir>` accepts. It is
# assembled outside the tree because the binary is never committed, and
# <out-dir> must not exist yet, so a stale binary is never shipped beside a
# fresh manifest. What is copied is the files git tracks under
# `.claude-plugin/marketplace.json` and `interweave/`, as they stand in the
# working tree; an untracked or ignored file there fails the build.
#
# The plugin's `version` is the release version (ADR-0065 decision 5):
# `[workspace.package] version` in the repository's Cargo.toml, which every
# first-party crate inherits (tools/checks/check_workspace_version.sh), the
# bridge's included. The committed plugin.json carries the placeholder
# 0.0.0 and the assembled one carries that version, so `claude plugin
# update` sees a release as an upgrade. `--version <v>` is the release
# run's own version; a build where it is not the workspace's is refused.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo="$(cd "$here/../.." && pwd -P)"

usage="usage: $0 [--version <v>] <out-dir> [<claude-channel binary>]"
release=""
if [[ "${1-}" == --version ]]; then
    [[ $# -ge 2 && -n "$2" ]] || { echo "$usage" >&2; exit 2; }
    release="$2"
    shift 2
fi
[[ $# -ge 1 && $# -le 2 ]] || { echo "$usage" >&2; exit 2; }
out="$1"
binary="${2:-$repo/target/release/claude-channel}"

[[ -x "$binary" ]] || { echo "$0: no executable bridge at $binary: cargo build --release -p interweave-claude-channel" >&2; exit 1; }
[[ ! -e "$out" ]] || { echo "$0: $out exists; give a new directory" >&2; exit 1; }
# The parent is resolved physically, both sides alike, and must exist: an
# unresolvable parent once made the case word "/", which matched nothing,
# and `set -e` does not fire inside a case word -- so the guard failed open
# and the binary was installed into the tree.
parent="$(cd "$(dirname "$out")" 2>/dev/null && pwd -P)" \
    || { echo "$0: the parent of $out does not exist; create it outside the repository" >&2; exit 1; }
case "$parent/" in
    "$repo"/*) echo "$0: $out is inside the repository; the binary is never committed" >&2; exit 1 ;;
esac

# Read with tomllib (Python 3.11) rather than `cargo metadata`, which would
# need every workspace member's manifest beside it; the two agree while
# check_workspace_version.sh holds every member to this one value. A
# missing python3 or tomllib is said as that, not as a manifest's fault.
python3 -c 'import tomllib' \
    || { echo "$0: python3 with tomllib (3.11 or later) is needed to read the workspace version" >&2; exit 1; }
version="$(python3 -c '
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    print(tomllib.load(f)["workspace"]["package"]["version"])' "$repo/Cargo.toml")" \
    || { echo "$0: no [workspace.package] version in $repo/Cargo.toml" >&2; exit 1; }
[[ -n "$version" ]] || { echo "$0: an empty [workspace.package] version in $repo/Cargo.toml" >&2; exit 1; }
[[ -z "$release" || "$release" == "$version" ]] \
    || { echo "$0: the release version $release is not the workspace version $version" >&2; exit 1; }

# The package is exactly what git tracks here, plus the one binary installed
# below. A whole-directory copy shipped anything a developer had left under
# interweave/ -- a stale binary in its ignored bin/, which git status never
# shows -- so any file here that git does not track (ignored ones included)
# fails the build instead of shipping or being skipped silently
# (apps/claude-channel/tests/packaging_build.rs).
stray="$(git -C "$here" ls-files --others -- .claude-plugin interweave)" \
    || { echo "$0: cannot list $here with git; build from a checkout" >&2; exit 1; }
[[ -z "$stray" ]] || { printf '%s: untracked files in the package source, remove them:\n%s\n' "$0" "$stray" >&2; exit 1; }
tracked=()
while IFS= read -r -d '' f; do tracked+=("$f"); done \
    < <(git -C "$here" ls-files -z -- .claude-plugin/marketplace.json interweave)
[[ ${#tracked[@]} -gt 0 ]] || { echo "$0: git tracks no package files under $here" >&2; exit 1; }

# Assembled beside <out-dir> and renamed into place only once complete, so a
# failed run -- a tracked file deleted from the working tree, a failed
# install -- leaves no partial <out-dir> to be refused as existing next time
# (apps/claude-channel/tests/packaging_build.rs). The trap is set first, so
# no exit after mktemp leaves the staging directory.
stage=""
trap '[[ -n "$stage" ]] && rm -rf "$stage"' EXIT
stage="$(mktemp -d "$parent/.$(basename "$out").XXXXXX")"
chmod 0755 "$stage"
for f in "${tracked[@]}"; do
    mkdir -p "$stage/$(dirname "$f")"
    cp -p "$here/$f" "$stage/$f"
done
# The staged copy is rewritten, never the tracked one. json.dump with this
# indent reproduces the committed file byte for byte, so the version is
# the only change (apps/claude-channel/tests/packaging_build.rs).
python3 -c '
import json, sys
path, version = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as f:
    manifest = json.load(f)
manifest["version"] = version
with open(path, "w", encoding="utf-8") as f:
    f.write(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")' \
    "$stage/interweave/.claude-plugin/plugin.json" "$version"
mkdir -p "$stage/interweave/bin"
install -m 0755 "$binary" "$stage/interweave/bin/claude-channel"
mv "$stage" "$out"
trap - EXIT
echo "$out"

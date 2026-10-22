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
#   bash packaging/claude-plugin/build.sh <out-dir> [<claude-channel binary>]
#
# The binary defaults to the workspace's release build
# (`cargo build --release -p interweave-claude-channel`). The output is a
# directory `claude plugin marketplace add <out-dir>` accepts. It is
# assembled outside the tree because the binary is never committed, and
# <out-dir> must not exist yet, so a stale binary is never shipped beside a
# fresh manifest.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo="$(cd "$here/../.." && pwd -P)"

[[ $# -ge 1 && $# -le 2 ]] || { echo "usage: $0 <out-dir> [<claude-channel binary>]" >&2; exit 2; }
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

mkdir -p "$out/.claude-plugin"
cp "$here/.claude-plugin/marketplace.json" "$out/.claude-plugin/"
cp -R "$here/interweave" "$out/interweave"
mkdir -p "$out/interweave/bin"
install -m 0755 "$binary" "$out/interweave/bin/claude-channel"
echo "$out"

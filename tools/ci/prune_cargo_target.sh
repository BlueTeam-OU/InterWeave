#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/prune_cargo_target.sh
#
# >>> help
# Reduce a cargo target directory to what is worth caching: the builds of
# the DEPENDENCIES, never this workspace's own.
#
#   tools/ci/prune_cargo_target.sh <target-dir>
#
# The `rust` job caches its target directory so a run compiles only what
# changed. Everything a workspace member built is dead weight there: its
# sources come out of a fresh checkout with new mtimes, so cargo rebuilds
# it on every run whatever is restored, and its test executables are most
# of the directory (each links the whole graph, with debuginfo). So, in
# every profile directory under <target-dir> (one holding deps/), this
# removes:
#
#   deps/         every file a workspace target built: <name>-<hash>[.ext],
#                 and lib<name>-<hash>.{rlib,rmeta,so,a} for a library;
#   .fingerprint/ and build/
#                 every <package>-<hash> directory of a workspace package;
#   the rest      every other entry of the profile directory (the uplifted
#                 binaries and their .d files, examples/, incremental/);
#
# and <target-dir>/tmp, CARGO_TARGET_TMPDIR's scratch from the tests.
#
# A name is a workspace member's package name or one of its targets'
# names, as `cargo metadata --no-deps` reports them, with `-` read as `_`
# (rustc's spelling of a crate name). A dependency that happens to share
# such a name is removed too and rebuilt on the next run: slower, never
# wrong, since cargo rebuilds whatever is missing.
#
# Needs: cargo (run from the repository root, for `cargo metadata`), jq.
#
# Exit codes:
#   0  pruned (the sizes before and after are printed)
#   1  cargo metadata failed, or named no workspace member
#   2  usage, or <target-dir> is not a directory
# <<< help

set -euo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd -- "$SCRIPT_DIR/../.." && pwd )"

case "${1:-}" in
    -h|--help) sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'; exit 0 ;;
esac
[[ $# -eq 1 ]] || { echo "prune_cargo_target: usage: $0 <target-dir> (try --help)" >&2; exit 2; }
TARGET="$1"
[[ -d "$TARGET" ]] || { echo "prune_cargo_target: $TARGET is not a directory" >&2; exit 2; }

# Filtered into a variable, never piped into a reader that can stop early:
# under pipefail a SIGPIPE upstream would read as cargo failing.
metadata="$(cd "$ROOT" && cargo metadata --no-deps --format-version 1 --locked)" \
    || { echo "prune_cargo_target: cargo metadata failed" >&2; exit 1; }
names="$(jq -r '.packages[] | .name, .targets[].name' <<<"$metadata" | tr - _ | sort -u)"
[[ -n "$names" ]] || { echo "prune_cargo_target: cargo metadata named no workspace member" >&2; exit 1; }
declare -A WORKSPACE=()
while IFS= read -r n; do WORKSPACE["$n"]=1; done <<<"$names"

is_workspace() { [[ -n "${WORKSPACE[${1//-/_}]+x}" ]]; }

before="$(du -sh "$TARGET" | cut -f1)"
removed=0
remove() { rm -rf -- "$1"; removed=$((removed + 1)); }

shopt -s nullglob dotglob
# <target-dir>/<profile>/deps, and <target-dir>/<triple>/<profile>/deps
# under --target. Never <target-dir>/deps: <target-dir> is no profile, and
# the last loop below would empty it.
for deps in "$TARGET"/*/deps "$TARGET"/*/*/deps; do
    profile="${deps%/deps}"
    for f in "$deps"/*; do
        base="${f##*/}"
        # <name>-<16 hex>, then an extension or nothing (a test executable).
        [[ "$base" =~ ^(.+)-[0-9a-f]{16}(\..*)?$ ]] || continue
        name="${BASH_REMATCH[1]}" ext="${BASH_REMATCH[2]}"
        # Only a library's own files carry the lib prefix; the .d beside
        # it does not, so `libp2p-<hash>.d` is the crate libp2p.
        case "$ext" in .rlib|.rmeta|.so|.a) name="${name#lib}" ;; esac
        if is_workspace "$name"; then remove "$f"; fi
    done
    for d in "$profile"/.fingerprint/* "$profile"/build/*; do
        base="${d##*/}"
        [[ "$base" =~ ^(.+)-[0-9a-f]{16}$ ]] || continue
        if is_workspace "${BASH_REMATCH[1]}"; then remove "$d"; fi
    done
    for e in "$profile"/*; do
        case "${e##*/}" in deps|.fingerprint|build) ;; *) remove "$e" ;; esac
    done
done
[[ -e "$TARGET/tmp" ]] && remove "$TARGET/tmp"

echo "prune_cargo_target: $TARGET $before -> $(du -sh "$TARGET" | cut -f1), $removed entries removed"

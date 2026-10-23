#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_tag.sh
#
# >>> help
# The release run's tag rule (ADR-0065 decisions 5 and 6): is this tag a
# release of the commit it names?
#
#   tools/ci/release_tag.sh <tag>
#   tools/ci/release_tag.sh <tag> --root <dir>
#
# A tag is `v` and a Semantic Versioning 2.0.0 version with NO build
# metadata: v<MAJOR>.<MINOR>.<PATCH>, each a number with no leading zero,
# optionally followed by `-` and pre-release identifiers -- dot-separated,
# each non-empty and of [0-9A-Za-z-], a numeric one with no leading zero
# (v0.4.0-rc.1, v1.0.0-alpha.beta, v1.0.0-0.3.7, v1.0.0-x-y). A `+` and
# anything after it is refused, since a tag carries none (decision 6).
#
# The version must be the workspace's: every workspace member's version
# as `cargo metadata --no-deps` reads it from <dir> (default: this
# repository), which check_workspace_version.sh keeps to one value
# inherited from [workspace.package]. A tag whose version is not that
# value names a release the commit does not build.
#
# On success it prints, for $GITHUB_OUTPUT:
#   version=<the version, without the v>
#   prerelease=<true if it has pre-release identifiers, else false>
# A refused tag is printed %q-quoted, as a dispatched one is typed text
# and a newline in it would otherwise start a workflow command of its own.
#
# What it does not check: that the tag is on main, and from which ref a
# dry run was dispatched -- release.yml's tag job does, with git.
#
# Exit codes:
#   0  the tag is a release of the workspace version
#   1  refused: not the grammar, or not the workspace's version
#   2  usage, or cargo metadata failed
# <<< help

set -euo pipefail
# [A-Za-z] is ASCII only in the C locale.
export LC_ALL=C

usage() { echo "usage: release_tag.sh <tag> [--root <dir>]" >&2; exit 2; }
case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
case $# in
1) ;;
3) [[ "$2" == --root ]] || usage; root="$3" ;;
*) usage ;;
esac
tag="$1"

num='(0|[1-9][0-9]*)'
# A pre-release identifier: numeric with no leading zero, or
# alphanumeric with at least one non-digit (the specification's §9).
ident="($num|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
grammar="^v$num\\.$num\\.$num(-$ident(\\.$ident)*)?\$"
[[ "$tag" =~ $grammar ]] || {
    printf '::error::tag %q is not v<MAJOR>.<MINOR>.<PATCH>[-<pre-release>] (Semantic Versioning 2.0.0, no build metadata)\n' "$tag"
    exit 1
}
version="${tag#v}"

metadata="$(cargo metadata --no-deps --format-version 1 --manifest-path "$root/Cargo.toml")" \
    || { echo "::error::cargo metadata failed in $root"; exit 2; }
workspace="$(printf '%s' "$metadata" | python3 -c '
import json, sys
m = json.load(sys.stdin)
members = set(m["workspace_members"])
print("\n".join(sorted({p["version"] for p in m["packages"] if p["id"] in members})))')"
[[ -n "$workspace" && "$workspace" != *$'\n'* ]] || {
    printf '::error::the workspace members carry more than one version (%s); check_workspace_version.sh says which\n' \
        "${workspace//$'\n'/, }"
    exit 1
}
[[ "$version" == "$workspace" ]] || {
    printf '::error::tag %q is not the workspace version %s ([workspace.package] version in Cargo.toml at this commit)\n' \
        "$tag" "$workspace"
    exit 1
}

echo "version=$version"
if [[ "$version" == *-* ]]; then echo "prerelease=true"; else echo "prerelease=false"; fi

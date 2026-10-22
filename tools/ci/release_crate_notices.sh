#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_crate_notices.sh
#
# >>> help
# Write the third-party notices of the crates one package compiles for one
# target, as a values file of the release page (ADR-0058 decision 5: the
# Android page carries every crate compiled into the native library, made
# by the same tool under the same rule as the Linux archive's):
#
#   tools/ci/release_crate_notices.sh <target> <package> <out.json>
#
# <out.json> is {"crate_notices": ...}, release_notices.py --json's page
# section, for release_page.py beside the release's other values. Which
# crates is `cargo tree --locked -e normal --target <target> -p <package>`;
# what each is, `cargo metadata --locked --filter-platform <target>` -- the
# lines release_linux_archive.sh runs for the archive, or the files
# RELEASE_COMPILED_LIST and RELEASE_METADATA_JSON name (the self-test's
# stand-ins, so nothing is compiled or resolved).
#
# REFUSED, writing nothing and removing an earlier run's <out.json>, when
# cargo fails or release_notices.py refuses (a crate with no notice it can
# ship, a compiled list that does not name the package, ...): its message
# says which.
#
# Exit codes:
#   0  written
#   1  a refusal, or cargo failed
#   2  usage
# <<< help

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

usage() {
    echo "usage: release_crate_notices.sh <target> <package> <out.json>" >&2
    exit 2
}
die() { echo "release_crate_notices: $*" >&2; exit 1; }

case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
[[ $# -eq 3 ]] || usage
target="$1" package="$2" out="$3"

# Every failure from here removes <out.json>, an earlier run's included:
# a stale file would be published as this release's notices.
scratch=""
finish() {
    local status=$?
    if [[ -n "$scratch" ]]; then rm -rf "$scratch"; fi
    if (( status != 0 )); then rm -f -- "$out"; fi
}
trap finish EXIT
scratch="$(mktemp -d)"

metadata="${RELEASE_METADATA_JSON-}"
if [[ -z "$metadata" ]]; then
    metadata="$scratch/metadata.json"
    (cd "$root" && cargo metadata --locked --format-version 1 --filter-platform "$target") > "$metadata" \
        || die "cargo metadata failed"
fi
compiled="${RELEASE_COMPILED_LIST-}"
if [[ -z "$compiled" ]]; then
    compiled="$scratch/compiled.txt"
    (cd "$root" && cargo tree --locked -e normal --target "$target" --prefix none -f '{p}' -p "$package") \
        > "$compiled" || die "cargo tree failed"
fi
python3 "$here/release_notices.py" --json crate_notices "$metadata" "$compiled" "$out" "$package" >&2 \
    || die "the crate notices of $package for $target could not be written"
echo "crate_notices $out"

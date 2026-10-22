#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_crate_notices.sh
#
# >>> help
# Write the third-party notices of the crates one package compiles for one
# target as a release asset, and the page's value that links to it
# (ADR-0058 decision 5, A 2026-10-22 (ii): the Android crate notices are
# about six times what a release body takes, so they travel as a file
# beside the APK, and the page holds the file's link and hash):
#
#   tools/ci/release_crate_notices.sh <target> <package> <version> <out-dir> <download-url>
#
# writes into <out-dir>
#   THIRD-PARTY-NOTICES-android.md          release_notices.py's page, made
#                                           by the tool and rule the Linux
#                                           archive's notices are
#   THIRD-PARTY-NOTICES-android.md.sha256   in sha256sum's format
#   crate-notices.json   {"crate_notices": the file's URL,
#                         "crate_notices_sha256": the file's SHA-256,
#                         "check:crate_notices_version": <version>}
#                        for release_page.py, which binds both values to
#                        the page's version as it binds the APK's hash
#
# <download-url> is the release's download base, the file's link being
# <download-url>/THIRD-PARTY-NOTICES-android.md. Which crates is `cargo
# tree --locked -e normal --target <target> -p <package>`; what each is,
# `cargo metadata --locked --filter-platform <target>` -- the lines
# release_linux_archive.sh runs for the archive, or the files
# RELEASE_COMPILED_LIST and RELEASE_METADATA_JSON name (the self-test's
# stand-ins, so nothing is compiled or resolved).
#
# REFUSED when <version> is not MAJOR.MINOR.PATCH, when cargo fails, or
# when release_notices.py refuses (a crate with no notice it can ship, a
# compiled list that does not name the package, ...): its message says
# which. On any failure none of the three outputs is left in <out-dir>:
# all three are written in scratch and moved together at the end, and a
# failure removes every one of their names, an earlier run's included.
#
# Exit codes:
#   0  written
#   1  a refusal, or cargo failed
#   2  usage
# <<< help

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
asset=THIRD-PARTY-NOTICES-android.md

usage() {
    echo "usage: release_crate_notices.sh <target> <package> <version> <out-dir> <download-url>" >&2
    exit 2
}
die() { echo "release_crate_notices: $*" >&2; exit 1; }

case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
[[ $# -eq 5 ]] || usage
target="$1" package="$2" version="$3" out_dir="$4" url="$5"

outputs=("$asset" "$asset.sha256" crate-notices.json)
# Every failure from here removes the outputs' names, an earlier run's
# included: a stale file would be published as this release's notices.
scratch=""
finish() {
    local status=$? f
    if [[ -n "$scratch" ]]; then rm -rf "$scratch"; fi
    if (( status != 0 )); then
        for f in "${outputs[@]}"; do rm -f -- "$out_dir/$f"; done
    fi
}
trap finish EXIT
# After the trap, so a refused version also removes an earlier run's files.
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] \
    || die "version '$version' is not MAJOR.MINOR.PATCH"
scratch="$(mktemp -d)"
mkdir -p "$out_dir"

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
python3 "$here/release_notices.py" "$metadata" "$compiled" "$scratch/$asset" "$package" >&2 \
    || die "the crate notices of $package for $target could not be written"
(cd "$scratch" && sha256sum -- "$asset" > "$asset.sha256")
sha="$(cut -d' ' -f1 "$scratch/$asset.sha256")"
python3 - "$scratch/crate-notices.json" "$url/$asset" "$sha" "$version" <<'PY'
import json, sys
out, link, sha, version = sys.argv[1:]
# The URL and the hash as values, as run.json's apk_url and
# android-values.json's apk_sha256 are; the sentence is the template's.
json.dump({"crate_notices": link, "crate_notices_sha256": sha,
           "check:crate_notices_version": version}, open(out, "w"))
PY
for f in "${outputs[@]}"; do mv -fT -- "$scratch/$f" "$out_dir/$f"; done
echo "crate_notices $out_dir/$asset"
echo "crate_notices_sha256 $sha"

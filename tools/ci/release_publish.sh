#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_publish.sh
#
# >>> help
# Step 9 of the release run: create the tag's release from its page and
# its files (packaging/android/README.md, "The release run").
#
#   tools/ci/release_publish.sh <tag> <version> <prerelease> <page.md> <in-dir>
#
# <prerelease> is the tag job's output, from tools/ci/release_tag.sh:
#   true   the release is created with --prerelease --latest=false: a
#          pre-release tag (v0.4.0-rc.1) is marked as one and never made
#          the repository's latest release (ADR-0065 decision 6);
#   false  neither flag: the API's own default, which makes a newly
#          published release the latest one;
# anything else is refused before gh runs.
#
# --verify-tag: the release is the pushed tag's, never one gh would
# create at another commit. The files are the signed APK, the archive,
# the Android crate notices and the .sha256 of each, read from <in-dir>
# by the names the build jobs give them.
#
# Exit codes:
#   0  gh created the release
#   1  <prerelease> is neither true nor false, or gh failed
#   2  usage
# <<< help

set -euo pipefail

case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
[[ $# -eq 5 ]] || { echo "usage: release_publish.sh <tag> <version> <prerelease> <page.md> <in-dir>" >&2; exit 2; }
tag="$1" version="$2" prerelease="$3" page="$4" in="$5"

case "$prerelease" in
true) kind=(--prerelease --latest=false) ;;
false) kind=() ;;
*) printf '::error::prerelease is %q, neither true nor false\n' "$prerelease"; exit 1 ;;
esac

gh release create "$tag" --verify-tag "${kind[@]}" --title "InterWeave $version" \
    --notes-file "$page" \
    "$in"/interweave-*-arm64-v8a.apk "$in"/interweave-*-arm64-v8a.apk.sha256 \
    "$in"/interweave-*-x86_64-unknown-linux-gnu.tar.gz "$in"/interweave-*-x86_64-unknown-linux-gnu.tar.gz.sha256 \
    "$in"/THIRD-PARTY-NOTICES-android.md "$in"/THIRD-PARTY-NOTICES-android.md.sha256

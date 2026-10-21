#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/skia_prebuilt.sh
#
# >>> help
# The Android build's Skia, pinned (ADR-0057 decision 4, A 2026-10-20).
# skia-bindings downloads a prebuilt archive in its build script and
# verifies nothing about it, and when the download fails it builds Skia
# from source instead without saying so in the result. So the build is
# handed one archive this script has checked, and its output is read
# afterwards to show that archive was the one taken.
#
#   tools/ci/skia_prebuilt.sh fetch <dir>
#       Download the archive tools/ci/skia-binaries.pin names into <dir>
#       (or reuse one already there), check its SHA-256, and print
#           SKIA_BINARIES_URL=file://<absolute path>
#       for the build's environment. REFUSED, with no archive left in
#       <dir>, when the pin's version is not the skia-bindings version
#       Cargo.lock builds (a bump without a new pin), or the file's
#       SHA-256 is not the pin's.
#
#   tools/ci/skia_prebuilt.sh check <target-dir> [<profile>]
#       After the Android build: read EVERY skia-bindings build output
#       under <target-dir>/aarch64-linux-android/<profile>/ (default
#       release, the release run's; a `cargo check` writes debug) and
#       REFUSE unless each one asked for the pin's version and archive
#       key (a feature change moves the key), read a file:// path whose
#       file has the pin's SHA-256, and said DOWNLOAD AND INSTALL
#       SUCCEEDED -- and none started a full build. Every output, not the
#       newest: cargo does not rerun a build script whose inputs are
#       unchanged, so an older output in a reused target may be the one
#       the build linked; a stale one fails the run, and the release run
#       builds in a clean target.
#
# The release run runs fetch before the build and check after it. The
# android check job is to read the same pin once its workflow calls
# this (devex-tooling's j182), so one pin line will govern both.
#
# Environment, for the self-test: SKIA_PIN_FILE (the pin),
# SKIA_PIN_LOCK (Cargo.lock), SKIA_PIN_SOURCE (a directory holding the
# archive, read instead of the release URL).
#
# Exit codes:
#   0  fetched and checked / the build took the pinned archive
#   1  a refusal above (named on stderr)
#   2  usage, or the pin or Cargo.lock unreadable
# <<< help

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
pin_file="${SKIA_PIN_FILE:-$here/skia-binaries.pin}"
lock_file="${SKIA_PIN_LOCK:-$here/../../Cargo.lock}"
release_base="https://github.com/rust-skia/skia-binaries/releases/download"

usage() {
    echo "usage: skia_prebuilt.sh fetch <dir> | check <target-dir> [<profile>]" >&2
    exit 2
}
die() { echo "skia_prebuilt: $*" >&2; exit 1; }

case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
fetch) [[ $# -eq 2 ]] || usage ;;
check) [[ $# -eq 2 || $# -eq 3 ]] || usage ;;
*) usage ;;
esac

[[ -r "$pin_file" ]] || { echo "skia_prebuilt: cannot read the pin $pin_file" >&2; exit 2; }
pin_lines="$(grep -v -e '^#' -e '^[[:space:]]*$' "$pin_file" || true)"
[[ "$(wc -l <<<"$pin_lines")" == 1 && -n "$pin_lines" ]] \
    || { echo "skia_prebuilt: $pin_file must hold exactly one pin line" >&2; exit 2; }
read -r tag archive sha extra <<<"$pin_lines"
[[ -z "${extra-}" && "$archive" == skia-binaries-*-aarch64-linux-android*.tar.gz \
    && "$sha" =~ ^[0-9a-f]{64}$ ]] \
    || { echo "skia_prebuilt: the pin line is not <version> <skia-binaries-...-aarch64-linux-android-....tar.gz> <sha256>" >&2; exit 2; }
key="${archive#skia-binaries-}"
key="${key%.tar.gz}"

fetch() {
    local dir="$1"
    [[ -r "$lock_file" ]] || { echo "skia_prebuilt: cannot read $lock_file" >&2; exit 2; }
    # A refusal removes an earlier fetch's archive of the pin's name too, so
    # what is left in <dir> after a refused fetch is never taken for one.
    refuse() { rm -f -- "$dir/$archive"; die "$@"; }
    local locked
    locked="$(awk '/^name = "skia-bindings"$/ { getline; gsub(/version = |"/, ""); print }' "$lock_file")"
    [[ -n "$locked" ]] || refuse "Cargo.lock builds no skia-bindings"
    [[ "$locked" == "$tag" ]] \
        || refuse "Cargo.lock builds skia-bindings $locked but the pin is for $tag: renew tools/ci/skia-binaries.pin"

    mkdir -p "$dir"
    local path
    path="$(cd "$dir" && pwd)/$archive"
    if [[ ! -f "$path" ]] || [[ "$(sha256sum "$path" | cut -d' ' -f1)" != "$sha" ]]; then
        rm -f "$path"
        if [[ -n "${SKIA_PIN_SOURCE-}" ]]; then
            cp -- "$SKIA_PIN_SOURCE/$archive" "$path.part" || { rm -f "$path.part"; die "no $archive in $SKIA_PIN_SOURCE"; }
        else
            curl -sSLf --retry 3 -o "$path.part" "$release_base/$tag/$archive" \
                || { rm -f "$path.part"; die "cannot download $release_base/$tag/$archive"; }
        fi
        local got
        got="$(sha256sum "$path.part" | cut -d' ' -f1)"
        if [[ "$got" != "$sha" ]]; then
            rm -f "$path.part"
            die "$archive has SHA-256 $got, not the pin's $sha"
        fi
        mv -f "$path.part" "$path"
    fi
    echo "SKIA_BINARIES_URL=file://$path"
}

check_one() {
    local output="$1" from read
    if grep -q '^STARTING A FULL BUILD' "$output"; then
        die "skia-bindings built Skia from source ($output): the pinned archive was not taken"
    fi
    grep -qxF "TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: $tag/$key" "$output" \
        || die "skia-bindings asked for $(sed -n 's/^TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: //p' "$output" | head -n 1), not the pin's $tag/$key ($output): renew the pin"
    from="$(sed -n 's/^  FROM: //p' "$output")"
    [[ "$(wc -l <<<"$from")" == 1 && "$from" == file:///*/"$archive" ]] \
        || die "skia-bindings did not read a local file named $archive ($output)"
    # The file the bindings read is hashed here, whatever directory
    # fetch used: a same-named file fetch never checked is refused.
    read="${from#file://}"
    [[ -f "$read" && "$(sha256sum "$read" | cut -d' ' -f1)" == "$sha" ]] \
        || die "skia-bindings read $read, which is not the pin's archive by SHA-256 ($output)"
    grep -qx 'DOWNLOAD AND INSTALL SUCCEEDED' "$output" \
        || die "skia-bindings did not install the pinned archive ($output)"
}

check() {
    local target="$1" profile="${2:-release}" outputs output count=0
    local dir="$target/aarch64-linux-android/$profile/build"
    [[ -d "$dir" ]] || die "no aarch64-linux-android $profile build under $target"
    outputs="$(find "$dir" -mindepth 2 -maxdepth 2 -path '*/skia-bindings-*/output' -type f | sort)"
    [[ -n "$outputs" ]] || die "no skia-bindings build output under $dir"
    while IFS= read -r output; do
        check_one "$output"
        count=$((count + 1))
    done <<<"$outputs"
    echo "skia-bindings took the pinned archive $archive ($count output(s) under $dir)"
}

"$1" "$2" "${3-}"

#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/minisign_install.sh
#
# >>> help
# Install the pinned minisign (tools/ci/minisign.pin):
#
#   tools/ci/minisign_install.sh [--pin FILE] [--url URL] <dir>
#
# Downloads minisign-<version>-linux.tar.gz from jedisct1/minisign's
# release of that version (--url: another location of the same file, a
# file:// one in the self-test), checks its SHA-256 against the pin, and
# installs its x86_64 executable as <dir>/minisign, then asks it its
# version. Prints `minisign <path>`.
#
# REFUSED, with nothing left at <dir>/minisign, when the download's
# SHA-256 is not the pin's, the archive holds no x86_64 minisign, or the
# installed program does not report the pinned version.
#
# Exit codes:
#   0  installed
#   1  a refusal above, or the download failed
#   2  usage, or the pin is not one <version> <sha256> line
# <<< help

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
pin_file="$here/minisign.pin"
url=""

usage() { echo "usage: minisign_install.sh [--pin FILE] [--url URL] <dir>" >&2; exit 2; }
die() { echo "::error::minisign_install: $*"; exit 1; }

case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
while [[ "${1-}" == --* ]]; do
    (( $# >= 2 )) || usage
    case "$1" in
    --pin) pin_file="$2" ;;
    --url) url="$2" ;;
    *) usage ;;
    esac
    shift 2
done
[[ $# -eq 1 && -n "$1" ]] || usage
dest="$1"

mapfile -t lines < <(grep -v -e '^#' -e '^[[:space:]]*$' "$pin_file" 2>/dev/null || true)
read -r version sha extra <<<"${lines[0]-}" || true
if (( ${#lines[@]} != 1 )) || [[ -n "${extra-}" || ! "${version-}" =~ ^[0-9]+(\.[0-9]+)+$ || ! "${sha-}" =~ ^[0-9a-f]{64}$ ]]; then
    echo "minisign_install: $pin_file is not one <version> <sha256> line" >&2
    exit 2
fi
asset="minisign-$version-linux.tar.gz"
[[ -n "$url" ]] || url="https://github.com/jedisct1/minisign/releases/download/$version/$asset"

scratch="$(mktemp -d)"
installed=0
finish() {
    rm -rf "$scratch"
    # A refused install leaves no program a later step could take for it.
    (( installed )) || rm -f -- "$dest/minisign"
}
trap finish EXIT

curl -fsSL --retry 3 -o "$scratch/$asset" "$url" || die "could not download $url"
got="$(sha256sum "$scratch/$asset" | cut -d' ' -f1)"
[[ "$got" == "$sha" ]] || die "$asset has SHA-256 $got, not the pinned $sha"
# The archive also carries macOS resource forks (._*); only the one
# executable is taken.
tar -xzf "$scratch/$asset" -C "$scratch" minisign-linux/x86_64/minisign 2>/dev/null \
    || die "$asset holds no minisign-linux/x86_64/minisign"
mkdir -p "$dest"
install -m 0755 "$scratch/minisign-linux/x86_64/minisign" "$dest/minisign"
reported="$("$dest/minisign" -v 2>&1 || true)"
[[ "$reported" == "minisign $version" ]] || die "the installed program reports '$reported', not minisign $version"
installed=1
echo "minisign $dest/minisign"

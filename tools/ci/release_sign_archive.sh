#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_sign_archive.sh
#
# >>> help
# Sign release files with the archive key, and check each signature with
# the pinned public key:
#
#   tools/ci/release_sign_archive.sh [--keys FILE] [--values FILE] <file>...
#
# Writes <file>.minisig beside each <file> (the release run: the Linux
# archive and its .sha256). The secret key and its password come from
# the environment, as the release environment's secrets:
#   INTERWEAVE_RELEASE_MINISIGN_KEY    the secret key file's contents
#                                      (`minisign -G`'s .key, both lines)
#   INTERWEAVE_RELEASE_MINISIGN_PASS   its password; empty for a key made
#                                      without one (-W)
# The key is written to a file only the job's user reads, removed however
# the script ends; the password reaches minisign on its standard input,
# never on a command line. MINISIGN names the program (default: minisign
# on PATH; tools/ci/minisign_install.sh installs the pinned one).
#
# Each signature is then verified with --keys' archive.minisign_public_key
# (default: packaging/release-keys.toml beside this script's repository;
# the sign job passes main's): a key in the environment that is not the
# pinned one signs nothing that is kept.
#
# --values writes {"linux_minisign_public_key": <that key>} to FILE once
# every signature verified, for release_page.py: the page's check names
# the key the signatures were verified with, never one read elsewhere.
#
# REFUSED, with none of the .minisig files it was to write left behind,
# when no archive key is pinned (the release fails closed), the
# environment holds no secret key, the password does not open it, or a
# signature does not verify with the pinned key; nor the --values file.
# Prints
# `minisig <file>.minisig` for each.
#
# Exit codes:
#   0  every file is signed and its signature verified
#   1  a refusal above
#   2  usage, or the pin cannot be read
# <<< help

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
keys="$here/../../packaging/release-keys.toml"
minisign="${MINISIGN:-minisign}"
values=""

usage() { echo "usage: release_sign_archive.sh [--keys FILE] [--values FILE] <file>..." >&2; exit 2; }
die() { echo "::error::release_sign_archive: $*"; exit 1; }

case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
esac
while [[ "${1-}" == --* ]]; do
    (( $# >= 2 )) || usage
    case "$1" in
    --keys) keys="$2" ;;
    --values) values="$2" ;;
    *) usage ;;
    esac
    shift 2
done
(( $# >= 1 )) || usage
files=("$@")
for f in "${files[@]}"; do [[ -f "$f" ]] || die "no file $f"; done

written=()
scratch=""
ok=0
finish() {
    local f
    if [[ -n "$scratch" ]]; then rm -rf "$scratch"; fi
    if (( ! ok )); then
        for f in "${written[@]}"; do rm -f -- "$f"; done
        if [[ -n "$values" ]]; then rm -f -- "$values"; fi
    fi
}
trap finish EXIT

rc=0
pub="$(python3 "$here/release_keys.py" "$keys" archive.minisign_public_key 2>&1)" || rc=$?
if (( rc == 1 )); then die "${pub#release_keys: }"; fi
if (( rc != 0 )); then echo "release_sign_archive: ${pub#release_keys: }" >&2; exit 2; fi
[[ -n "${INTERWEAVE_RELEASE_MINISIGN_KEY-}" ]] \
    || die "the release environment has no INTERWEAVE_RELEASE_MINISIGN_KEY"
command -v "$minisign" >/dev/null || die "no $minisign to sign with"

scratch="$(mktemp -d)"
( umask 077 && printf '%s\n' "$INTERWEAVE_RELEASE_MINISIGN_KEY" > "$scratch/archive.key" )
for f in "${files[@]}"; do
    written+=("$f.minisig")
    printf '%s\n' "${INTERWEAVE_RELEASE_MINISIGN_PASS-}" \
        | "$minisign" -S -s "$scratch/archive.key" -m "$f" -x "$f.minisig" >/dev/null 2>"$scratch/err" \
        || die "minisign could not sign $f with the environment's key: $(tail -n 1 "$scratch/err")"
    "$minisign" -V -q -P "$pub" -m "$f" -x "$f.minisig" >/dev/null 2>&1 \
        || die "$f's signature does not verify with the pinned archive key: the environment's key is another"
done
if [[ -n "$values" ]]; then
    printf '{"linux_minisign_public_key": "%s"}\n' "$pub" > "$values" \
        || die "could not write $values"
fi
ok=1
for f in "${files[@]}"; do echo "minisig $f.minisig"; done

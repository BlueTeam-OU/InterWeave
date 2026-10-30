#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_minisign_install.sh
#
# Self-test for minisign_install.sh, against stand-in archives served as
# file:// URLs, so nothing is downloaded: an archive whose SHA-256 is the
# pin's installs and prints its path; one of another SHA-256, one with no
# x86_64 executable, and one whose program reports another version are
# refused with nothing left at <dir>/minisign -- an earlier install's
# copy included; a pin of two lines or a short hash is exit 2.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/minisign_install.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_minisign_install: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# archive <name> <reported version> [<dir in the archive>]: a stand-in
# release archive whose minisign prints that version.
archive() {
    local d="$SANDBOX/build-$1"
    mkdir -p "$d/minisign-linux/${3:-x86_64}"
    printf '#!/bin/sh\necho "minisign %s"\n' "$2" > "$d/minisign-linux/${3:-x86_64}/minisign"
    chmod 0755 "$d/minisign-linux/${3:-x86_64}/minisign"
    tar -czf "$SANDBOX/$1.tar.gz" -C "$d" minisign-linux
}
sha() { sha256sum "$SANDBOX/$1.tar.gz" | cut -d' ' -f1; }
archive good 0.12
archive old 0.11
archive arm 0.12 aarch64
DEST="$SANDBOX/bin"
run() {  # run <pin line> <archive> [<dest>]
    printf '# a comment\n\n%s\n' "$1" > "$SANDBOX/pin"
    status=0
    out="$(bash "$UNDER_TEST" --pin "$SANDBOX/pin" --url "file://$SANDBOX/$2.tar.gz" "${3-$DEST}" 2>&1)" || status=$?
}
expect() {  # expect <label> <status> [<output fragment>]
    if [[ $status == "$2" && ( -z "${3-}" || "$out" == *"$3"* ) ]]; then pass "$1"
    else fail "$1" "status $status, wanted $2; output: $out"; fi
}

echo "minisign_install.sh:"
run "0.12 $(sha good)" good
expect "the pinned archive installs" 0 "minisign $DEST/minisign"
[[ "$("$DEST/minisign" -v)" == "minisign 0.12" ]] && pass "and the installed program is its" \
    || fail "the installed program is not the archive's"
run "0.12 $(sha old)" good
expect "an archive of another SHA-256 is refused" 1 "not the pinned"
[[ ! -e "$DEST/minisign" ]] && pass "and the earlier install's program is gone" \
    || fail "a refused install left $DEST/minisign"
run "0.12 $(sha old)" old
expect "a program reporting another version is refused" 1 "reports 'minisign 0.11', not minisign 0.12"
[[ ! -e "$DEST/minisign" ]] && pass "and is not left installed" || fail "a refused program was left installed"
run "0.12 $(sha arm)" arm
expect "an archive with no x86_64 minisign is refused" 1 "holds no minisign-linux/x86_64/minisign"
run "0.12 $(sha good)" missing
expect "a download that fails is refused" 1 "could not download"
run "0.12 $(sha good)
0.12 $(sha good)" good
expect "a pin of two lines is exit 2" 2 "is not one <version> <sha256> line"
run "0.12 abc" good
expect "a short hash is exit 2" 2 "is not one <version> <sha256> line"
run "0.12 $(sha good)" good ""
expect "no directory is usage" 2 "usage"

if (( failures > 0 )); then
    echo "test_minisign_install: $failures failed" >&2
    exit 1
fi
echo "test_minisign_install: all passed"

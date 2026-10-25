#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_linux_run.sh
#
# Self-test for release_linux_run.sh, over stand-in archives whose
# programs are scripts: an archive whose four print a version and exit 0
# and whose human-desktop exits 64 passes, naming each; the loader's
# refusal (a GLIBC_ version not found, a missing shared library) fails
# even with the wanted exit, as do another exit, a missing program and a
# program that is not executable, and every failing program is named, not
# only the first; a file that is not an archive, and an archive of two
# directories, are exit 2. That LD_BIND_NOW reaches the program is read
# from a stand-in that prints it.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_linux_run.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_linux_run: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

N=interweave-0.1.1-x86_64-unknown-linux-gnu
# stage: a passing archive's tree; each case then edits it.
stage() {
    rm -rf "$SANDBOX/t"
    mkdir -p "$SANDBOX/t/$N/bin" "$SANDBOX/t/$N/claude-plugin/interweave/bin"
    local p
    for p in bin/transport-daemon bin/transportctl bin/interweave-human claude-plugin/interweave/bin/claude-channel; do
        printf '#!/bin/sh\n[ "$1" = --version ] || exit 2\necho "%s 0.1.1 bind=$LD_BIND_NOW"\n' "${p##*/}" > "$SANDBOX/t/$N/$p"
    done
    printf '#!/bin/sh\necho "usage: human-desktop ..." >&2\nexit 64\n' > "$SANDBOX/t/$N/bin/human-desktop"
    chmod 0755 "$SANDBOX/t/$N/bin/"* "$SANDBOX/t/$N/claude-plugin/interweave/bin/claude-channel"
}
put() {  # put <path in the archive> <script body>
    printf '#!/bin/sh\n%s\n' "$2" > "$SANDBOX/t/$N/$1"
    chmod 0755 "$SANDBOX/t/$N/$1"
}
run() {
    tar -czf "$SANDBOX/a.tar.gz" -C "$SANDBOX/t" .
    status=0
    out="$(bash "$UNDER_TEST" "$SANDBOX/a.tar.gz" 2>&1)" || status=$?
}
expect() {  # expect <label> <status> [<output fragment>]
    if [[ $status == "$2" && ( -z "${3-}" || "$out" == *"$3"* ) ]]; then pass "$1"
    else fail "$1" "status $status, wanted $2; output: $out"; fi
}

echo "release_linux_run.sh:"
stage
run
expect "every program running passes" 0 "every program ran"
[[ "$out" == *"ran bin/human-desktop (exit 64)"* && "$out" == *"ran claude-plugin/interweave/bin/claude-channel --version (exit 0)"* ]] \
    && pass "  and each is named with its exit" || fail "  the programs are not each named" "$out"
put bin/transportctl '[ "$LD_BIND_NOW" = 1 ] || exit 3; echo ok'
run
expect "LD_BIND_NOW=1 reaches the program" 0

stage
put bin/human-desktop "echo \"bin/human-desktop: /lib/x86_64-linux-gnu/libm.so.6: version \\\`GLIBC_2.43' not found\" >&2; exit 64"
run
expect "the loader's version refusal fails, even with the wanted exit" 1 "bin/human-desktop exited 64, not 64"
stage
put bin/transport-daemon "echo 'transport-daemon: error while loading shared libraries: libfoo.so.1' >&2; exit 0"
run
expect "a missing shared library fails" 1 "bin/transport-daemon --version exited 0"
stage
put bin/interweave-human 'exit 127'
put claude-plugin/interweave/bin/claude-channel 'exit 1'
run
expect "another exit fails" 1 "bin/interweave-human --version exited 127, not 0"
[[ "$out" == *"claude-channel --version exited 1, not 0"* && "$out" == *"2 program(s) did not run"* ]] \
    && pass "  and every failing program is named, not only the first" || fail "  only one failure named" "$out"
stage
put bin/human-desktop 'exit 0'
run
expect "human-desktop exiting 0 with no argument fails (its main is to refuse)" 1 "exited 0, not 64"
stage
rm "$SANDBOX/t/$N/claude-plugin/interweave/bin/claude-channel"
run
expect "a missing program fails" 1 "holds no executable claude-plugin/interweave/bin/claude-channel"
stage
chmod 0644 "$SANDBOX/t/$N/bin/transportctl"
run
expect "a program that is not executable fails" 1 "holds no executable bin/transportctl"

stage
mkdir -p "$SANDBOX/t/second"
run
expect "an archive of two directories is exit 2" 2 "does not hold one directory"
printf 'not gzip\n' > "$SANDBOX/b.tar.gz"
status=0; out="$(bash "$UNDER_TEST" "$SANDBOX/b.tar.gz" 2>&1)" || status=$?
expect "a file that is not an archive is exit 2" 2 "does not unpack"
status=0; out="$(bash "$UNDER_TEST" 2>&1)" || status=$?
expect "no argument is usage" 2 "usage"

if (( failures > 0 )); then
    echo "test_release_linux_run: $failures failed" >&2
    exit 1
fi
echo "test_release_linux_run: all passed"

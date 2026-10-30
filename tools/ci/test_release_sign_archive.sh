#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_sign_archive.sh
#
# Self-test for release_sign_archive.sh with the real minisign (CI
# installs the pinned one with minisign_install.sh; the suite fails, never
# skips, without it) and throwaway keys made at test time: two files
# signed with the pinned key's secret half are written a .minisig each,
# which minisign verifies with the public key, a key made without a
# password included; every refusal leaves no .minisig behind -- no
# archive key pinned (fail closed), no secret key in the environment, a
# wrong password, an environment key that is not the pinned one, a file
# that does not exist. The password never appears in what it prints.
# --values writes the pinned key for the page once every signature
# verified, and a refusal leaves no values file, an earlier one included.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_sign_archive.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }
MS="$(command -v minisign)" \
    || { echo "test_release_sign_archive: no minisign on PATH (tools/ci/minisign_install.sh)" >&2; exit 1; }

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_sign_archive: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

PASS='correct horse 1'
printf '%s\n%s\n' "$PASS" "$PASS" | "$MS" -G -p "$SANDBOX/a.pub" -s "$SANDBOX/a.key" >/dev/null 2>&1 \
    || { echo "test_release_sign_archive: minisign could not make a key" >&2; exit 1; }
printf 'other\nother\n' | "$MS" -G -p "$SANDBOX/b.pub" -s "$SANDBOX/b.key" >/dev/null 2>&1
"$MS" -G -W -p "$SANDBOX/n.pub" -s "$SANDBOX/n.key" </dev/null >/dev/null 2>&1
pub() { sed -n 2p "$SANDBOX/$1.pub"; }
pin() { printf '[archive]\nminisign_public_key = "%s"\n' "$1" > "$SANDBOX/keys.toml"; }

F1="$SANDBOX/interweave-0.1.1-x86_64-unknown-linux-gnu.tar.gz"
F2="$F1.sha256"
printf 'archive bytes\n' > "$F1"
printf 'hash line\n' > "$F2"
run() {  # run <key file or ""> <password> <file>...
    local key="$1" pw="$2"; shift 2
    rm -f "$F1.minisig" "$F2.minisig"
    status=0
    out="$(INTERWEAVE_RELEASE_MINISIGN_KEY="$( [[ -n "$key" ]] && cat "$SANDBOX/$key.key")" \
        INTERWEAVE_RELEASE_MINISIGN_PASS="$pw" MINISIGN="$MS" \
        bash "$UNDER_TEST" --keys "$SANDBOX/keys.toml" "$@" 2>&1)" || status=$?
}
expect() {  # expect <label> <status> [<output fragment>]
    if [[ $status == "$2" && ( -z "${3-}" || "$out" == *"$3"* ) ]]; then pass "$1"
    else fail "$1" "status $status, wanted $2; output: $out"; fi
}
none_left() {
    [[ ! -e "$F1.minisig" && ! -e "$F2.minisig" ]] && pass "  and no .minisig is left" \
        || fail "  a refusal left a .minisig"
}

echo "release_sign_archive.sh:"
pin "$(pub a)"
run a "$PASS" "$F1" "$F2"
expect "both files are signed with the pinned key" 0 "minisig $F2.minisig"
"$MS" -V -q -p "$SANDBOX/a.pub" -m "$F1" -x "$F1.minisig" && "$MS" -V -q -p "$SANDBOX/a.pub" -m "$F2" -x "$F2.minisig" \
    && pass "  and each signature verifies with the public key" || fail "  a signature does not verify"
[[ "$out" != *"$PASS"* ]] && pass "  and the password is not printed" || fail "  the password was printed" "$out"
printf 'tampered\n' >> "$F1"
"$MS" -V -q -p "$SANDBOX/a.pub" -m "$F1" -x "$F1.minisig" 2>/dev/null \
    && fail "control: a changed file still verified" || pass "control: the signature does not verify a changed file"
printf 'archive bytes\n' > "$F1"
V="$SANDBOX/signing-values.json"
status=0
out="$(INTERWEAVE_RELEASE_MINISIGN_KEY="$(cat "$SANDBOX/a.key")" INTERWEAVE_RELEASE_MINISIGN_PASS="$PASS" MINISIGN="$MS" \
    bash "$UNDER_TEST" --keys "$SANDBOX/keys.toml" --values "$V" "$F1" 2>&1)" || status=$?
expect "--values with a good signature" 0
[[ "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["linux_minisign_public_key"])' "$V" 2>/dev/null)" == "$(pub a)" ]] \
    && pass "  writes the pinned key as linux_minisign_public_key" || fail "  the values file is not the pinned key" "$(cat "$V" 2>&1)"
status=0
out="$(INTERWEAVE_RELEASE_MINISIGN_KEY="$(cat "$SANDBOX/b.key")" INTERWEAVE_RELEASE_MINISIGN_PASS=other MINISIGN="$MS" \
    bash "$UNDER_TEST" --keys "$SANDBOX/keys.toml" --values "$V" "$F1" 2>&1)" || status=$?
expect "--values with a key that is not the pinned one" 1 "does not verify"
[[ ! -e "$V" ]] && pass "  leaves no values file, the earlier one removed" || fail "  a refusal left the values file"
pin "$(pub n)"
run n "" "$F1"
expect "a key made without a password signs with an empty one" 0 "minisig $F1.minisig"

pin ""
run a "$PASS" "$F1" "$F2"
expect "no archive key pinned fails closed" 1 "pins no archive.minisign_public_key"
none_left
pin "$(pub a)"
run "" "$PASS" "$F1"
expect "no secret key in the environment is refused" 1 "has no INTERWEAVE_RELEASE_MINISIGN_KEY"
none_left
run a "wrong" "$F1" "$F2"
expect "a wrong password is refused" 1 "could not sign"
none_left
run b other "$F1" "$F2"
expect "an environment key that is not the pinned one is refused" 1 "does not verify with the pinned archive key"
none_left
run a "$PASS" "$F1" "$SANDBOX/absent"
expect "a file that does not exist is refused" 1 "no file"
none_left
run a "$PASS"
expect "no file is usage" 2 "usage"

if (( failures > 0 )); then
    echo "test_release_sign_archive: $failures failed" >&2
    exit 1
fi
echo "test_release_sign_archive: all passed"

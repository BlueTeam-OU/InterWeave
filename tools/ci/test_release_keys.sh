#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_keys.sh
#
# Self-test for release_keys.py: each field is printed in its shape (a
# fingerprint uppercased, one armoured block, a minisign Ed25519 key),
# and refused when empty -- the release fails closed -- or of another
# shape: a long key id for a fingerprint, two armoured blocks, a minisign
# key too short, of another algorithm, or with a character outside
# base64; a file that is not TOML and an unknown field are exit 2.
# The armoured block is a throwaway key made in a sandboxed keyring.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
KEYS_PY="$SCRIPT_DIR/release_keys.py"
[[ -f "$KEYS_PY" ]] || { echo "test: $KEYS_PY not found" >&2; exit 1; }
for t in gpg gpgconf python3; do
    command -v "$t" >/dev/null || { echo "test_release_keys: no $t on PATH" >&2; exit 1; }
done

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_keys: no sandbox (mktemp failed)" >&2; exit 1; }
export GNUPGHOME="$SANDBOX/gnupg"
mkdir -m 0700 "$GNUPGHOME"
trap 'gpgconf --kill all >/dev/null 2>&1; rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }
expect() {  # expect <label> <status> [<output fragment>]
    if [[ $status == "$2" && ( -z "${3-}" || "$out" == *"$3"* ) ]]; then pass "$1"
    else fail "$1" "status $status, wanted $2; output: $out"; fi
}
gpg --batch --quiet --passphrase '' --quick-gen-key 'pinned <a@example.invalid>' ed25519 sign never 2>/dev/null \
    || { echo "test_release_keys: gpg could not make a key" >&2; exit 1; }
A="$(gpg --batch --with-colons --list-keys 2>/dev/null | awk -F: '$1 == "fpr" { print $10; exit }')"

echo "release_keys.py:"
keys() {  # keys <field> <toml body>: sets out and status
    printf '%s\n' "$2" > "$SANDBOX/k.toml"
    status=0
    out="$(python3 "$KEYS_PY" "$SANDBOX/k.toml" "$1" 2>&1)" || status=$?
}
MS="RW$(printf 'x%.0s' $(seq 53))!"
# A well-formed minisign key: "Ed", an 8-byte id and 32 key bytes.
MS_GOOD="$(python3 -c 'import base64; print(base64.b64encode(b"Ed" + bytes(range(40))).decode())')"
keys archive.minisign_public_key "[archive]
minisign_public_key = \"$MS_GOOD\""
expect "a minisign Ed25519 public key is printed" 0 "$MS_GOOD"
MS_SHORT="${MS_GOOD:0:40}"
keys archive.minisign_public_key "[archive]
minisign_public_key = \"$MS_SHORT\""
expect "a short minisign key is refused" 1 "is not a minisign public key"
MS_ALGO="$(python3 -c 'import base64; print(base64.b64encode(b"ED" + bytes(range(40))).decode())')"
keys archive.minisign_public_key "[archive]
minisign_public_key = \"$MS_ALGO\""
expect "a minisign key of another algorithm is refused" 1 "is not"
keys archive.minisign_public_key "[archive]
minisign_public_key = \"$MS\""
expect "a minisign key of the right length with a character outside base64 is refused" 1 "is not a minisign public key"
keys archive.minisign_public_key "[archive]
minisign_public_key = \"\""
expect "no archive key pinned fails closed" 1 "pins no archive.minisign_public_key"
keys tag.fingerprint "[tag]
fingerprint = \"${A:24}\""
expect "a long key id is refused as a fingerprint" 1 "is not 40 hex digits"
keys tag.fingerprint "[tag]
fingerprint = \"${A,,}\""
[[ $status == 0 && "$out" == "$A" ]] && pass "a lowercase fingerprint is printed uppercased" \
    || fail "a lowercase fingerprint is printed uppercased" "status $status; output: $out"
BLOCK="$(gpg --batch --armor --export "$A")"
keys tag.public_key "[tag]
public_key = \"\"\"
$BLOCK
$BLOCK
\"\"\""
expect "two armoured blocks are refused" 1 "is not one armoured PGP public key block"
keys tag.public_key "[tag]
public_key = \"\"\"
$BLOCK
\"\"\""
expect "control: one armoured block is printed" 0 "BEGIN PGP PUBLIC KEY BLOCK"
keys tag.nothing "[tag]"
expect "an unknown field is usage" 2 "usage"
keys tag.fingerprint "not = [toml"
expect "a file that is not TOML is exit 2" 2 "cannot read"

if (( failures > 0 )); then
    echo "test_release_keys: $failures failed" >&2
    exit 1
fi
echo "test_release_keys: all passed"

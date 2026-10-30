#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_verify_tag.sh
#
# Self-test for release_verify_tag.sh, with throwaway
# Ed25519 keys made in a sandboxed keyring at test time and a throwaway
# repository: a tag signed by the pinned key passes, by its signing
# subkey too, a lowercase pin included; every refusal sits beside that
# control -- no key pinned (fail closed), a lightweight tag, an unsigned
# annotated tag, another key's signature, a tag at another commit, a pin
# whose public key is not its fingerprint's, a block of two keys, a
# revoked key, a short commit id, a pin that is not TOML.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_verify_tag.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }
for t in gpg gpgconf git python3; do
    command -v "$t" >/dev/null || { echo "test_release_verify_tag: no $t on PATH" >&2; exit 1; }
done

failures=0
SANDBOX="$(mktemp -d)"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_release_verify_tag: no sandbox (mktemp failed)" >&2; exit 1; }
# The signer's keyring: never the person's running the test.
export GNUPGHOME="$SANDBOX/gnupg"
# Nor the person's git config: a commit.gpgsign there would sign the
# fixtures' commits with their own key, or fail to.
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
mkdir -m 0700 "$GNUPGHOME"
trap 'gpgconf --kill all >/dev/null 2>&1; rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

# newkey <uid> [<usage>]: a passphrase-less Ed25519 primary key; prints
# its fingerprint.
newkey() {
    gpg --batch --quiet --passphrase '' --quick-gen-key "$1" ed25519 "${2:-sign}" never 2>/dev/null \
        || { echo "test_release_verify_tag: gpg could not make a key" >&2; exit 1; }
    gpg --batch --with-colons --list-keys "$1" 2>/dev/null | awk -F: '$1 == "fpr" { print $10; exit }'
}
A="$(newkey 'pinned <a@example.invalid>')"
B="$(newkey 'other <b@example.invalid>')"
# C certifies only; its subkey signs.
C="$(newkey 'subkey <c@example.invalid>' cert)"
gpg --batch --quiet --passphrase '' --quick-add-key "$C" ed25519 sign never 2>/dev/null
R="$(newkey 'revoked <r@example.invalid>')"

REPO="$SANDBOX/repo"
git init -q "$REPO"
g() { git -C "$REPO" -c user.name=t -c user.email=t@example.invalid -c gpg.format=openpgp -c gpg.program=gpg "$@"; }
g commit -q --allow-empty -m one
ONE="$(g rev-parse HEAD)"
g commit -q --allow-empty -m two
TWO="$(g rev-parse HEAD)"
g tag -s -u "$A" -m 'signed by A' vA "$TWO"
g tag -s -u "$B" -m 'signed by B' vB "$TWO"
g tag -s -u "$C" -m 'signed by C' vC "$TWO"
g tag -s -u "$R" -m 'signed by R' vR "$TWO"
g tag -a -m 'unsigned' vU "$TWO"
g tag vL "$TWO"
g tag -s -u "$A" -m 'at one' vOne "$ONE"
# R revoked after it signed: the certificate gpg wrote when it made R.
sed 's/^:-----BEGIN/-----BEGIN/' "$GNUPGHOME/openpgp-revocs.d/$R.rev" | gpg --batch --quiet --import 2>/dev/null

# pin <fingerprint> <key>...: a pin of that fingerprint and those keys'
# public block, as packaging/release-keys.toml holds it.
pin() {
    local fpr="$1"; shift
    local block=""
    [[ -n "${1-}" ]] && block="$(gpg --batch --armor --export "$@")"
    python3 - "$SANDBOX/keys.toml" "$fpr" "$block" <<'EOF'
import sys
path, fpr, block = sys.argv[1:]
with open(path, "w") as f:
    f.write('[tag]\nfingerprint = "%s"\npublic_key = """\n%s\n"""\n[archive]\nminisign_public_key = ""\n' % (fpr, block))
EOF
}
run() {
    status=0
    out="$(bash "$UNDER_TEST" --keys "$SANDBOX/keys.toml" --repo "$REPO" "$@" 2>&1)" || status=$?
}
expect() {  # expect <label> <status> [<output fragment>]
    if [[ $status == "$2" && ( -z "${3-}" || "$out" == *"$3"* ) ]]; then pass "$1"
    else fail "$1" "status $status, wanted $2; output: $out"; fi
}

echo "release_verify_tag.sh:"
pin "$A" "$A"
run vA "$TWO"
expect "a tag signed by the pinned key passes" 0 "tag_signer $A"
pin "${A,,}" "$A"
run vA "$TWO"
expect "a lowercase pinned fingerprint passes" 0 "tag_signer $A"
pin "$C" "$C"
run vC "$TWO"
expect "a tag signed by the pinned key's signing subkey passes, naming the primary" 0 "tag_signer $C"

pin "" ""
run vA "$TWO"
expect "no key pinned fails closed" 1 "pins no tag.fingerprint"
pin "$A" "$A"
run vL "$TWO"
expect "a lightweight tag is refused" 1 "is not an annotated tag"
run vU "$TWO"
expect "an unsigned annotated tag is refused" 1 "does not verify against the pinned key"
run vB "$TWO"
expect "a tag signed by another key is refused" 1 "does not verify against the pinned key"
run vOne "$TWO"
expect "a tag at another commit is refused" 1 "points at $ONE, not $TWO"
run vA "${TWO:0:12}"
expect "a short commit id is refused" 1 "is not a full commit id"
run vNone "$TWO"
expect "a missing tag is refused" 1 "is not an annotated tag"
pin "$A" "$B"
run vA "$TWO"
expect "a pin whose key is not its fingerprint's is refused" 1 "is $B, not the pinned fingerprint $A"
pin "$A" "$A" "$B"
run vA "$TWO"
expect "a pinned block of two keys is refused" 1 "holds 2 primary keys, not one"
pin "$R" "$R"
run vR "$TWO"
expect "a tag signed by a revoked key is refused" 1 "vR"
[[ "$out" != *"tag_signer"* ]] && pass "and names no signer" || fail "a revoked key named a signer" "$out"

run vA
expect "one argument is usage" 2 "usage"
printf 'not = [toml\n' > "$SANDBOX/keys.toml"
run vA "$TWO"
expect "a pin that is not TOML is exit 2" 2 "cannot read"

if (( failures > 0 )); then
    echo "test_release_verify_tag: $failures failed" >&2
    exit 1
fi
echo "test_release_verify_tag: all passed"

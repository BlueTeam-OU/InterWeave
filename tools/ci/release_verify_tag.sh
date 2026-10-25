#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_verify_tag.sh
#
# >>> help
# Refuse a release tag that the pinned tag key did not sign:
#
#   tools/ci/release_verify_tag.sh [--keys FILE] [--repo DIR] <tag> <commit>
#
# --keys is the pin (default: packaging/release-keys.toml beside this
# script's repository); --repo the repository holding the tag (default:
# the current directory). The tag must be an annotated tag object at
# refs/tags/<tag> pointing at <commit>, a full commit id, and signed by
# the key whose primary fingerprint the pin names.
#
# The pin's public key is imported into a keyring made for this run and
# removed after it, so neither the runner's nor a person's keyring
# answers; that keyring must hold exactly the pinned key. git verify-tag
# runs against it, and gpg's status lines are read, not its exit alone:
# the signature must be GOODSIG and VALIDSIG with the pinned primary
# fingerprint, and no BADSIG, ERRSIG, EXPSIG, EXPKEYSIG or REVKEYSIG.
# An expired or revoked key is a refusal, not a warning.
#
# Prints `tag_signer <fingerprint>` on success.
#
# Exit codes:
#   0  the tag is signed by the pinned key and points at <commit>
#   1  a refusal above, or no key pinned (the release fails closed)
#   2  usage, or the pin cannot be read
# <<< help

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
keys="$here/../../packaging/release-keys.toml"
repo="."

usage() {
    echo "usage: release_verify_tag.sh [--keys FILE] [--repo DIR] <tag> <commit>" >&2
    exit 2
}
die() { echo "::error::release_verify_tag: $*"; exit 1; }

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
    --repo) repo="$2" ;;
    *) usage ;;
    esac
    shift 2
done
[[ $# -eq 2 ]] || usage
tag="$1" commit="$2"
[[ "$commit" =~ ^[0-9a-f]{40}$ ]] || die "$(printf '%q' "$commit") is not a full commit id"

# release_keys.py: 1 is a refusal (the release fails closed), 2 a pin it
# cannot read.
read_key() {
    local rc=0 out
    out="$(python3 "$here/release_keys.py" "$keys" "$1" 2>&1)" || rc=$?
    # To stderr: this runs inside the caller's $(...), whose stdout is
    # the value.
    if (( rc == 1 )); then echo "::error::release_verify_tag: ${out#release_keys: }" >&2; exit 1; fi
    if (( rc != 0 )); then echo "release_verify_tag: ${out#release_keys: }" >&2; exit 2; fi
    printf '%s\n' "$out"
}
fpr="$(read_key tag.fingerprint)"
pubkey="$(read_key tag.public_key)"

ref="refs/tags/$tag"
[[ "$(git -C "$repo" cat-file -t "$ref" 2>/dev/null)" == tag ]] \
    || die "$(printf '%q' "$tag") is not an annotated tag in $repo"
target="$(git -C "$repo" rev-parse --verify --quiet "$ref^{commit}")" \
    || die "$(printf '%q' "$tag") points at no commit"
[[ "$target" == "$commit" ]] || die "$(printf '%q' "$tag") points at $target, not $commit"

GNUPGHOME="$(mktemp -d)"
export GNUPGHOME
finish() {
    gpgconf --kill all >/dev/null 2>&1 || true
    rm -rf "$GNUPGHOME"
}
trap finish EXIT
chmod 0700 "$GNUPGHOME"

printf '%s\n' "$pubkey" | gpg --batch --quiet --import 2>/dev/null \
    || die "the pinned tag.public_key does not import"
# Every primary key the block brought, by fingerprint: one, and the pin's.
mapfile -t primaries < <(gpg --batch --with-colons --list-keys \
    | awk -F: '$1 == "pub" { want = 1; next } want && $1 == "fpr" { print $10; want = 0 }')
(( ${#primaries[@]} == 1 )) \
    || die "the pinned tag.public_key holds ${#primaries[@]} primary keys, not one"
[[ "${primaries[0]}" == "$fpr" ]] \
    || die "the pinned tag.public_key is ${primaries[0]}, not the pinned fingerprint $fpr"
# Ultimate trust in this keyring only, so that gpg reports the signature
# as from a known key rather than an unverified one; what makes it
# trustworthy is the pin, checked above.
printf '%s:6:\n' "$fpr" | gpg --batch --quiet --import-ownertrust 2>/dev/null

# The user's own git config never decides the format or the program.
status="$(git -C "$repo" -c gpg.format=openpgp -c gpg.program=gpg verify-tag --raw "$ref" 2>&1)" \
    || die "$(printf '%q' "$tag") does not verify against the pinned key: $(printf '%s' "$status" | grep -E '^\[GNUPG:\] (BADSIG|ERRSIG|EXPSIG|EXPKEYSIG|REVKEYSIG|NO_PUBKEY)' | head -n 1)"
if printf '%s\n' "$status" | grep -qE '^\[GNUPG:\] (BADSIG|ERRSIG|EXPSIG|EXPKEYSIG|REVKEYSIG) '; then
    die "$(printf '%q' "$tag"): $(printf '%s\n' "$status" | grep -E '^\[GNUPG:\] (BADSIG|ERRSIG|EXPSIG|EXPKEYSIG|REVKEYSIG) ' | head -n 1)"
fi
printf '%s\n' "$status" | grep -q '^\[GNUPG:\] GOODSIG ' || die "$(printf '%q' "$tag") carries no good signature"
# VALIDSIG's tenth argument is the primary key's fingerprint, the signing
# subkey's being the first.
signer="$(printf '%s\n' "$status" | awk '$1 == "[GNUPG:]" && $2 == "VALIDSIG" { print $12; exit }')"
[[ "$signer" == "$fpr" ]] || die "$(printf '%q' "$tag") is signed by ${signer:-no key}, not the pinned $fpr"
echo "tag_signer $signer"

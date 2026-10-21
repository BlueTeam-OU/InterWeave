#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_skia_prebuilt.sh
#
# Self-test for skia_prebuilt.sh, offline.
#
# fetch reads a stand-in archive from a sandbox directory (SKIA_PIN_SOURCE)
# against a sandbox pin and Cargo.lock; check reads build outputs laid out
# where cargo puts a build script's, in the shape skia-bindings 0.153.3's
# build script wrote on 2026-10-20 when it took the pinned archive
# (`cargo check --target aarch64-linux-android -p interweave-human-android`
# with fetch's SKIA_BINARIES_URL: the TRYING line named the pin's key, FROM
# the file, then DOWNLOAD AND INSTALL SUCCEEDED). The happy path of each
# is the positive control for its refusals. The repository's own pin is
# parsed too, and its version must be the one Cargo.lock builds.
#
# Exit codes:
#   0  all assertions passed
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"
UNDER_TEST="$SCRIPT_DIR/skia_prebuilt.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
[[ -n "$SANDBOX" && -d "$SANDBOX" ]] || { echo "test_skia_prebuilt: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

KEY="0123456789abcdef0123-aarch64-linux-android-ganesh-gl-pdf"
ARCHIVE="skia-binaries-$KEY.tar.gz"
mkdir -p "$SANDBOX/source"
printf 'stand-in archive bytes\n' > "$SANDBOX/source/$ARCHIVE"
SHA="$(sha256sum "$SANDBOX/source/$ARCHIVE" | cut -d' ' -f1)"

pin() { printf '# a comment\n\n%s\n' "$1" > "$SANDBOX/pin"; }
lock() {
    printf '[[package]]\nname = "skia-bindings"\nversion = "%s"\nsource = "registry"\n' "$1" > "$SANDBOX/Cargo.lock"
}
run() {
    SKIA_PIN_FILE="$SANDBOX/pin" SKIA_PIN_LOCK="$SANDBOX/Cargo.lock" SKIA_PIN_SOURCE="$SANDBOX/source" \
        bash "$UNDER_TEST" "$@" >"$SANDBOX/stdout" 2>"$SANDBOX/err"
}

echo "skia_prebuilt.sh fetch:"

pin "0.9.1 $ARCHIVE $SHA"
lock 0.9.1
D="$SANDBOX/dl"
if run fetch "$D" && [[ "$(cat "$SANDBOX/stdout")" == "SKIA_BINARIES_URL=file://$D/$ARCHIVE" ]] \
    && cmp -s "$D/$ARCHIVE" "$SANDBOX/source/$ARCHIVE"; then
    pass "fetches the pinned archive and prints its file:// URL"
else
    fail "a pinned archive was not fetched" "$(cat "$SANDBOX/stdout" "$SANDBOX/err")"
fi
rm -f "$SANDBOX/source/$ARCHIVE.moved"
mv "$SANDBOX/source/$ARCHIVE" "$SANDBOX/source/$ARCHIVE.moved"
if run fetch "$D"; then
    pass "an archive already there with the pin's hash is reused"
else
    fail "a good archive already in place was fetched again" "$(cat "$SANDBOX/err")"
fi
mv "$SANDBOX/source/$ARCHIVE.moved" "$SANDBOX/source/$ARCHIVE"

printf 'tampered\n' > "$D/$ARCHIVE"
if run fetch "$D" && cmp -s "$D/$ARCHIVE" "$SANDBOX/source/$ARCHIVE"; then
    pass "an archive already there with another hash is replaced"
else
    fail "a tampered archive in place was kept" "$(cat "$SANDBOX/err")"
fi

# refused <what> <needle> <dir>
refused() {
    local what="$1" needle="$2" dir="$3"
    # An earlier fetch's archive of the pin's name, which a refusal must
    # remove.
    mkdir -p "$dir"
    printf 'an earlier fetch\n' > "$dir/$ARCHIVE"
    if run fetch "$dir"; then
        fail "$what: accepted"
    elif ls "$dir"/skia-binaries-* >/dev/null 2>&1; then
        fail "$what: refused but left an archive" "$(ls "$dir")"
    elif ! grep -qF -- "$needle" "$SANDBOX/err"; then
        fail "$what: refused without saying $needle" "$(cat "$SANDBOX/err")"
    else
        pass "$what: refused, an earlier archive removed"
    fi
}

pin "0.9.1 $ARCHIVE $(printf '0%.0s' {1..64})"
refused "an archive whose SHA-256 is not the pin's" "not the pin's" "$SANDBOX/dl2"
pin "0.9.1 $ARCHIVE $SHA"
lock 0.9.2
refused "a pin for another skia-bindings version than Cargo.lock's" \
    "builds skia-bindings 0.9.2 but the pin is for 0.9.1" "$SANDBOX/dl3"
lock 0.9.1
mv "$SANDBOX/source/$ARCHIVE" "$SANDBOX/source/$ARCHIVE.gone"
refused "an archive that cannot be fetched" "no $ARCHIVE" "$SANDBOX/dl4"
mv "$SANDBOX/source/$ARCHIVE.gone" "$SANDBOX/source/$ARCHIVE"

for bad in "0.9.1 $ARCHIVE" "0.9.1 $ARCHIVE $SHA extra" "0.9.1 skia-binaries-x-x86_64-unknown-linux-gnu.tar.gz $SHA"; do
    pin "$bad"
    run fetch "$SANDBOX/dl5"
    rc=$?
    if [[ $rc -eq 2 ]]; then pass "a malformed pin line is a usage error (2): ${bad:0:40}..."
    else fail "a malformed pin line exited $rc: $bad"; fi
done
printf '0.9.1 %s %s\n0.9.1 %s %s\n' "$ARCHIVE" "$SHA" "$ARCHIVE" "$SHA" > "$SANDBOX/pin"
run fetch "$SANDBOX/dl5"
[[ $? -eq 2 ]] && pass "two pin lines are a usage error (2)" || fail "two pin lines were accepted"

echo "skia_prebuilt.sh check:"
pin "0.9.1 $ARCHIVE $SHA"
T="$SANDBOX/target"
# output <profile> <hash-dir> <TRYING key> <FROM url> <last line>
output() {
    local d="$T/aarch64-linux-android/$1/build/skia-bindings-$2"
    mkdir -p "$d"
    cat > "$d/output" <<EOF
cargo:rerun-if-env-changed=FORCE_SKIA_BUILD
TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: $3
cargo:rerun-if-env-changed=SKIA_BINARIES_URL
  FROM: $4
UNPACKING ARCHIVE INTO: $d/out/skia
$5
cargo:rustc-link-lib=static=skia
EOF
}
# fetch's file, with the pin's bytes.
good_from="file://$SANDBOX/dl/$ARCHIVE"
cmp -s "$SANDBOX/dl/$ARCHIVE" "$SANDBOX/source/$ARCHIVE" || fail "the fetched file is not the stand-in"

output release aaaa "0.9.1/$KEY" "$good_from" "DOWNLOAD AND INSTALL SUCCEEDED"
if run check "$T"; then pass "the build took the pinned archive"
else fail "a build that took the pinned archive was refused" "$(cat "$SANDBOX/err")"; fi

# checked <what> <needle> [profile]: refused while a bad release output
# sits beside the good one, which is removed after.
checked() {
    if run check "$T" ${3-}; then fail "$1: accepted"
    elif ! grep -qF -- "$2" "$SANDBOX/err"; then fail "$1: refused without saying $2" "$(cat "$SANDBOX/err")"
    else pass "$1: refused"; fi
    rm -rf "$T/aarch64-linux-android/release/build/skia-bindings-bbbb"
}
output release bbbb "0.9.1/$KEY" "$good_from" "DOWNLOAD AND INSTALL FAILED: x"
printf 'STARTING A FULL BUILD\n' >> "$T/aarch64-linux-android/release/build/skia-bindings-bbbb/output"
checked "a build that fell back to a source build, beside a good one" "built Skia from source"
output release bbbb "0.9.1/${KEY}-vulkan" "file://$SANDBOX/dl/skia-binaries-$KEY-vulkan.tar.gz" "DOWNLOAD AND INSTALL SUCCEEDED"
checked "a build asking for another key (a feature change)" "not the pin's 0.9.1/$KEY"
output release bbbb "0.9.1/$KEY" "https://github.com/rust-skia/skia-binaries/releases/download/0.9.1/$ARCHIVE" \
    "DOWNLOAD AND INSTALL SUCCEEDED"
checked "a build that downloaded instead of reading a local file" "did not read a local file"
mkdir -p "$SANDBOX/elsewhere"
printf 'other bytes\n' > "$SANDBOX/elsewhere/$ARCHIVE"
output release bbbb "0.9.1/$KEY" "file://$SANDBOX/elsewhere/$ARCHIVE" "DOWNLOAD AND INSTALL SUCCEEDED"
checked "a same-named file fetch never checked" "not the pin's archive by SHA-256"
output release bbbb "0.9.1/$KEY" "file://$SANDBOX/gone/$ARCHIVE" "DOWNLOAD AND INSTALL SUCCEEDED"
checked "a file that is not there" "not the pin's archive by SHA-256"
output release bbbb "0.9.1/$KEY" "$good_from" "DOWNLOAD AND INSTALL FAILED: unpack"
checked "a build whose install of the archive failed" "did not install"

# The profile is the release's: a good debug output, however new, does
# not vouch for a release output that fell back.
rm -rf "$T"
output release aaaa "0.9.1/$KEY" "$good_from" "DOWNLOAD AND INSTALL FAILED: x"
printf 'STARTING A FULL BUILD\n' >> "$T/aarch64-linux-android/release/build/skia-bindings-aaaa/output"
output debug cccc "0.9.1/$KEY" "$good_from" "DOWNLOAD AND INSTALL SUCCEEDED"
checked "a fallen-back release beside a good debug build" "built Skia from source"
if run check "$T" debug; then pass "the debug profile, named, is checked on its own"
else fail "a good debug output was refused under check <target> debug" "$(cat "$SANDBOX/err")"; fi

# Only the bindings' own outputs are read, wherever the target lies: under
# a directory whose name starts skia-bindings-, another crate's output
# (no Skia lines at all) is not taken for one.
T_SAVED="$T"
T="$SANDBOX/skia-bindings-check/target"
output release aaaa "0.9.1/$KEY" "$good_from" "DOWNLOAD AND INSTALL SUCCEEDED"
mkdir -p "$T/aarch64-linux-android/release/build/libc-dddd"
printf 'cargo:rustc-cfg=freebsd11\n' > "$T/aarch64-linux-android/release/build/libc-dddd/output"
if run check "$T" && grep -q '(1 output(s)' "$SANDBOX/stdout"; then
    pass "under a skia-bindings-* ancestor, only the bindings' output is read"
else
    fail "a target under a skia-bindings-* directory was misread" "$(cat "$SANDBOX/err" "$SANDBOX/stdout")"
fi
T="$T_SAVED"

rm -rf "$T"
mkdir -p "$T/aarch64-linux-android/release/build"
if run check "$T"; then fail "a target with no skia-bindings output was accepted"
else pass "a target with no skia-bindings output is refused"; fi

echo "the repository's pin:"
real_tag="$(grep -v -e '^#' -e '^[[:space:]]*$' "$SCRIPT_DIR/skia-binaries.pin" | cut -d' ' -f1)"
real_lock="$(awk '/^name = "skia-bindings"$/ { getline; gsub(/version = |"/, ""); print }' "$ROOT/Cargo.lock")"
if [[ -n "$real_tag" && "$real_tag" == "$real_lock" ]]; then
    pass "tools/ci/skia-binaries.pin is for the skia-bindings Cargo.lock builds ($real_lock)"
else
    fail "tools/ci/skia-binaries.pin is for '$real_tag', Cargo.lock builds '$real_lock'"
fi

if (( failures )); then
    echo "test_skia_prebuilt.sh: $failures failure(s)" >&2
    exit 1
fi
echo "test_skia_prebuilt.sh: all assertions passed"

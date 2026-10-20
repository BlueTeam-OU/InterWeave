#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# tools/ci/release_android_apk.sh
#
# >>> help
# Sign the release APK and print what the release page publishes of it
# (ADR-0057 decision 1, apps/human-android/RELEASE.md, plan section 22).
#
#   tools/ci/release_android_apk.sh [--first-release] <version> <unsigned.apk> <out-dir>
#
# <unsigned.apk> is Gradle's assembleRelease output
# (app/build/outputs/apk/release/app-release-unsigned.apk). Before
# anything is signed it is REFUSED when:
#   - <version> is not MAJOR.MINOR.PATCH, or is not the APK's versionName
#     (the version is a commit in android/app/build.gradle.kts before the
#     tag, never a value this run invents);
#   - its package is not org.interweave.human, or its versionCode is not
#     a positive integer;
#   - it is debuggable, or carries native code for an ABI other than
#     arm64-v8a, or none.
# Then it is aligned (4-byte, 16 KiB pages for the native library),
# signed with APK Signature Scheme v2 and v3 and no v1 signature, and
# verified; the signing certificate's SHA-256 must equal
# INTERWEAVE_RELEASE_CERT_SHA256 -- the same on every release, so a run
# signing with any other key fails -- unless --first-release, which
# prints the fingerprint for the owner to record instead, and is refused
# when a fingerprint is given too.
#
# Writes into <out-dir>:
#   interweave-<version>-arm64-v8a.apk          the signed APK
#   interweave-<version>-arm64-v8a.apk.sha256   in sha256sum's format
#   android-values.json                         {apk_sha256, cert_sha256,
#                                               check:apk_version} for
#                                               release_page.py
# all three or none: once <version> and the APK are found, a run that
# fails at any step, a refusal or a write, leaves none of them, a
# previous run's of the same names included.
#
# The key is read from the environment and never from a file in the
# tree, and its passwords reach apksigner as env: references, never on
# a command line another process can read:
#   INTERWEAVE_RELEASE_KEYSTORE        path to the keystore
#   INTERWEAVE_RELEASE_KEY_ALIAS       the key's alias in it
#   INTERWEAVE_RELEASE_KEYSTORE_PASS   the keystore's password
#   INTERWEAVE_RELEASE_KEY_PASS        the key's, if it differs
#   INTERWEAVE_RELEASE_CERT_SHA256     the expected fingerprint (lowercase
#                                      hex, no colons), unless --first-release
#
# Needs the Android SDK build tools (aapt2, zipalign, apksigner): the
# directory ANDROID_BUILD_TOOLS names, else the newest under
# ${ANDROID_HOME:-$ANDROID_SDK_ROOT}/build-tools.
#
# Exit codes:
#   0  signed and verified; the fingerprint and hash are printed
#   1  a refusal above, or a tool failed (named on stderr)
#   2  usage, or a variable or a tool missing
# <<< help

set -euo pipefail

usage() {
    echo "usage: release_android_apk.sh [--first-release] <version> <unsigned.apk> <out-dir>" >&2
    exit 2
}
die() { echo "release_android_apk: $*" >&2; exit 1; }

first_release=0
case "${1-}" in
-h | --help)
    sed -n '/^# >>> help$/,/^# <<< help$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
    exit 0
    ;;
--first-release) first_release=1; shift ;;
esac
[[ $# -eq 3 ]] || usage
version="$1" unsigned="$2" out_dir="$3"

[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] \
    || die "version '$version' is not MAJOR.MINOR.PATCH"
[[ -f "$unsigned" ]] || die "no APK at $unsigned"

mkdir -p "$out_dir"
apk_name="interweave-$version-arm64-v8a.apk"
outputs=("$apk_name" "$apk_name.sha256" android-values.json)
# Everything is made in a work directory on the out-dir's filesystem and
# moved out together at the end; a failure at any step removes every one
# of the outputs' names from the out-dir, what this run moved and what an
# earlier run left, so a run that fails leaves no APK, hash or values file.
work="$(mktemp -d "$out_dir/.release.XXXXXX")"
finish() {
    local status=$? f
    rm -rf "$work"
    if (( status != 0 )); then
        for f in "${outputs[@]}"; do rm -f -- "$out_dir/$f"; done
    fi
}
trap finish EXIT

for var in INTERWEAVE_RELEASE_KEYSTORE INTERWEAVE_RELEASE_KEY_ALIAS INTERWEAVE_RELEASE_KEYSTORE_PASS; do
    [[ -n "${!var-}" ]] || { echo "release_android_apk: $var is not set" >&2; exit 2; }
done
[[ -f "$INTERWEAVE_RELEASE_KEYSTORE" ]] || die "no keystore at INTERWEAVE_RELEASE_KEYSTORE"
expected_cert=""
if (( first_release )) && [[ -n "${INTERWEAVE_RELEASE_CERT_SHA256-}" ]]; then
    # A run given the release key's fingerprint is not the first release:
    # --first-release would sign with whatever key it holds and publish
    # that key's fingerprint as the release key's.
    echo "release_android_apk: --first-release with INTERWEAVE_RELEASE_CERT_SHA256 set: give one or the other" >&2
    exit 2
fi
if (( ! first_release )); then
    expected_cert="${INTERWEAVE_RELEASE_CERT_SHA256-}"
    [[ -n "$expected_cert" ]] || {
        echo "release_android_apk: INTERWEAVE_RELEASE_CERT_SHA256 is not set (--first-release prints it)" >&2
        exit 2
    }
    [[ "$expected_cert" =~ ^[0-9a-f]{64}$ ]] \
        || die "INTERWEAVE_RELEASE_CERT_SHA256 is not 64 lowercase hex digits"
fi

tools="${ANDROID_BUILD_TOOLS-}"
if [[ -z "$tools" ]]; then
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT-}}"
    [[ -n "$sdk" && -d "$sdk/build-tools" ]] || {
        echo "release_android_apk: no ANDROID_BUILD_TOOLS, and no build-tools under ANDROID_HOME" >&2
        exit 2
    }
    tools="$(find "$sdk/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1)"
fi
for t in aapt2 zipalign apksigner; do
    [[ -x "$tools/$t" ]] || { echo "release_android_apk: no $t in $tools" >&2; exit 2; }
done

# What the APK says it is, before any key is touched.
badging="$("$tools/aapt2" dump badging "$unsigned")" || die "aapt2 cannot read $unsigned"
package_line="$(grep -m1 '^package:' <<<"$badging" || true)"
name="$(sed -n "s/.* name='\([^']*\)'.*/\1/p" <<<"$package_line")"
code="$(sed -n "s/.* versionCode='\([^']*\)'.*/\1/p" <<<"$package_line")"
vname="$(sed -n "s/.* versionName='\([^']*\)'.*/\1/p" <<<"$package_line")"
[[ "$name" == org.interweave.human ]] || die "package is '$name', not org.interweave.human"
[[ "$code" =~ ^[1-9][0-9]*$ ]] || die "versionCode '$code' is not a positive integer"
[[ "$vname" == "$version" ]] \
    || die "the APK's versionName is '$vname', not $version: set it in android/app/build.gradle.kts before tagging"
! grep -q '^application-debuggable' <<<"$badging" || die "the APK is debuggable: a debug build is never published"
abis="$(sed -n "s/^native-code: //p" <<<"$badging")"
[[ "$abis" == "'arm64-v8a'" ]] || die "native code is for [${abis:-none}], not arm64-v8a alone"

signed="$work/$apk_name"
aligned="$work/aligned.apk"
"$tools/zipalign" -f -P 16 4 "$unsigned" "$aligned" || die "zipalign failed"

sign_args=(
    --ks "$INTERWEAVE_RELEASE_KEYSTORE"
    --ks-key-alias "$INTERWEAVE_RELEASE_KEY_ALIAS"
    --ks-pass env:INTERWEAVE_RELEASE_KEYSTORE_PASS
)
if [[ -n "${INTERWEAVE_RELEASE_KEY_PASS-}" ]]; then
    sign_args+=(--key-pass env:INTERWEAVE_RELEASE_KEY_PASS)
else
    sign_args+=(--key-pass env:INTERWEAVE_RELEASE_KEYSTORE_PASS)
fi
"$tools/apksigner" sign "${sign_args[@]}" \
    --v1-signing-enabled false --v2-signing-enabled true --v3-signing-enabled true \
    --out "$signed" "$aligned" || die "apksigner sign failed"
rm -f "$signed.idsig"

# The signed file as a person's phone will judge it.
"$tools/zipalign" -c -P 16 4 "$signed" >/dev/null || die "the signed APK is not aligned"
# Verified as for API 24, the oldest that reads v2: at the APK's own
# minimum (30) apksigner checks v3 alone and reports v2 false although
# the block is there, so this is what proves both were written.
verify="$("$tools/apksigner" verify -v --print-certs --min-sdk-version 24 "$signed")" \
    || die "apksigner verify failed"
for scheme in "v2 scheme (APK Signature Scheme v2): true" "v3 scheme (APK Signature Scheme v3): true" \
    "v1 scheme (JAR signing): false"; do
    grep -qF "Verified using $scheme" <<<"$verify" || die "not verified as: $scheme"
done
# The signer count and the certificate are read the same way from every
# apksigner: up to 36.1 a certificate line is "Signer #1 certificate ...",
# from 37.0 "V3.0 Signer: certificate ...", and both print "Number of
# signers". A line in neither shape gives no certificate, which is refused.
# A version may print the certificate once per scheme it verified, so the
# lines are taken as a set: one certificate however often it is printed.
signers="$(sed -n 's/^Number of signers: //p' <<<"$verify")"
[[ "$signers" == 1 ]] || die "${signers:-no} signers, not one"
cert="$(sed -En 's/^(Signer #[0-9]+|V[0-9.]+ Signer:) certificate SHA-256 digest: ([0-9a-f]{64})$/\2/p' <<<"$verify" | sort -u)"
[[ "$cert" =~ ^[0-9a-f]{64}$ ]] || die "apksigner verify named no single signing certificate: '${cert//$'\n'/ }'"
if (( ! first_release )) && [[ "$cert" != "$expected_cert" ]]; then
    die "signed with certificate $cert, not the release key's $expected_cert"
fi

(cd "$work" && sha256sum -- "$apk_name" > "$apk_name.sha256")
apk_sha="$(cut -d' ' -f1 "$work/$apk_name.sha256")"
printf '{"apk_sha256": "%s", "cert_sha256": "%s", "check:apk_version": "%s"}\n' \
    "$apk_sha" "$cert" "$version" > "$work/android-values.json"
for f in "${outputs[@]}"; do mv -fT -- "$work/$f" "$out_dir/$f"; done

echo "signed $out_dir/$apk_name (versionCode $code)"
echo "apk_sha256  $apk_sha"
echo "cert_sha256 $cert"
if (( first_release )); then
    echo "first release: record cert_sha256 as INTERWEAVE_RELEASE_CERT_SHA256 for every later run"
fi

#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_release_android_apk.sh
#
# Self-test for release_android_apk.sh, with the real build tools.
#
# Each case links a minimal APK with aapt2 (a manifest, and a stand-in
# native library stored where Gradle puts the real one) and signs it
# with a TEST-ONLY keystore made here by keytool, deleted with the
# sandbox. The happy path is the positive control: the signed APK
# verifies with v2 and v3 and no v1, its hash file checks, and the
# fingerprint the script prints is the keystore's. Each refusal leaves
# no signed APK behind. apksigner runs behind a wrapper that logs its
# arguments, so the test sees that no password reached a command line.
#
# Needs the Android SDK build tools (aapt2, zipalign, apksigner), an
# android.jar, keytool and python3. Without them it SKIPS, except under
# CI, where ubuntu-latest carries the SDK and a skip would mean the job
# stopped exercising the script.
#
# Exit codes:
#   0  all assertions passed, or skipped outside CI
#   1  one or more failed

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
UNDER_TEST="$SCRIPT_DIR/release_android_apk.sh"
[[ -f "$UNDER_TEST" ]] || { echo "test: $UNDER_TEST not found" >&2; exit 1; }

skip() {
    if [[ -n "${CI-}" ]]; then
        echo "test_release_android_apk.sh: $1 under CI" >&2
        exit 1
    fi
    echo "test_release_android_apk.sh: SKIPPED, $1"
    exit 0
}

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-/opt/android-sdk}}"
[[ -d "$sdk/build-tools" ]] || skip "no Android SDK build tools under $sdk"
real_tools="$(find "$sdk/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1)"
jar="$(find "$sdk/platforms" -mindepth 2 -maxdepth 2 -name android.jar 2>/dev/null | sort -V | tail -n 1)"
[[ -n "$jar" ]] || skip "no android.jar under $sdk/platforms"
for t in aapt2 zipalign apksigner; do
    [[ -x "$real_tools/$t" ]] || skip "no $t in $real_tools"
done
command -v keytool >/dev/null || skip "no keytool"
command -v python3 >/dev/null || skip "no python3"

failures=0
SANDBOX="$(realpath -- "$(mktemp -d)")"
trap 'rm -rf "$SANDBOX"' EXIT
pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ $1" >&2; printf '%s\n' "${2:-}" | sed 's/^/      /' >&2
         failures=$((failures + 1)); }

TOOLS="$SANDBOX/tools"
mkdir -p "$TOOLS"
ln -s "$real_tools/aapt2" "$TOOLS/aapt2"
ln -s "$real_tools/zipalign" "$TOOLS/zipalign"
cat > "$TOOLS/apksigner" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$SANDBOX/apksigner-argv"
exec "$real_tools/apksigner" "\$@"
EOF
chmod +x "$TOOLS/apksigner"

# TEST-ONLY key material: two throwaway keystores, never a release key.
PASS="test-only-$RANDOM-$RANDOM"
for alias in rel other; do
    keytool -genkeypair -keystore "$SANDBOX/$alias.p12" -storetype PKCS12 \
        -storepass "$PASS" -alias "$alias" -keyalg EC -groupname secp256r1 \
        -dname "CN=test-only-$alias" -validity 2 >/dev/null 2>&1 \
        || skip "keytool cannot make a keystore"
done
fingerprint() {
    keytool -exportcert -keystore "$SANDBOX/$1.p12" -storepass "$PASS" -alias "$1" 2>/dev/null \
        | sha256sum | cut -d' ' -f1
}
REL_CERT="$(fingerprint rel)"

# make_apk <file> <package> <versionName> <debuggable> <abi>...
make_apk() {
    local out="$1" package="$2" vname="$3" debuggable="$4"; shift 4
    local d="$SANDBOX/build-$RANDOM"
    mkdir -p "$d"
    cat > "$d/AndroidManifest.xml" <<EOF
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="$package" android:versionCode="7" android:versionName="$vname">
  <application android:hasCode="false" android:debuggable="$debuggable"/>
</manifest>
EOF
    "$real_tools/aapt2" link -o "$out" --manifest "$d/AndroidManifest.xml" -I "$jar" \
        --min-sdk-version 30 --target-sdk-version 36 2>/dev/null || return 1
    python3 - "$out" "$@" <<'PY'
import os, sys, zipfile
with zipfile.ZipFile(sys.argv[1], "a") as z:
    for abi in sys.argv[2:]:
        z.writestr(zipfile.ZipInfo(f"lib/{abi}/libinterweave_human_android.so"),
                   os.urandom(4096), compress_type=zipfile.ZIP_STORED)
PY
}

# entries <dir> <glob>: whether the directory holds a name the glob matches.
entries() { compgen -G "$1/$2" >/dev/null; }

OUT="$SANDBOX/out"
sign() {
    rm -rf "$OUT"
    ANDROID_BUILD_TOOLS="${BT:-$TOOLS}" \
    INTERWEAVE_RELEASE_KEYSTORE="$SANDBOX/${KEY:-rel}.p12" \
    INTERWEAVE_RELEASE_KEY_ALIAS="${KEY:-rel}" \
    INTERWEAVE_RELEASE_KEYSTORE_PASS="$PASS" \
    INTERWEAVE_RELEASE_CERT_SHA256="${CERT-$REL_CERT}" \
        bash "$UNDER_TEST" "$@" >"$SANDBOX/stdout" 2>"$SANDBOX/err"
}

echo "release_android_apk.sh:"

make_apk "$SANDBOX/good.apk" org.interweave.human 0.4.1 false arm64-v8a \
    || skip "aapt2 cannot link a minimal APK"
SIGNED="$OUT/interweave-0.4.1-arm64-v8a.apk"

if sign 0.4.1 "$SANDBOX/good.apk" "$OUT"; then
    verify="$("$real_tools/apksigner" verify -v --min-sdk-version 24 "$SIGNED" 2>/dev/null)"
    if grep -qF "v1 scheme (JAR signing): false" <<<"$verify" \
        && grep -qF "v2 scheme (APK Signature Scheme v2): true" <<<"$verify" \
        && grep -qF "v3 scheme (APK Signature Scheme v3): true" <<<"$verify"; then
        pass "signs with v2 and v3 and no v1"
    else
        fail "the signed APK's schemes are not v2+v3 without v1" "$verify"
    fi
    if (cd "$OUT" && sha256sum -c --quiet "interweave-0.4.1-arm64-v8a.apk.sha256"); then
        pass "the .sha256 file checks against the signed APK"
    else
        fail "the .sha256 file does not check"
    fi
    want="{\"apk_sha256\": \"$(sha256sum "$SIGNED" | cut -d' ' -f1)\", \"cert_sha256\": \"$REL_CERT\", \"check:apk_version\": \"0.4.1\"}"
    if [[ "$(cat "$OUT/android-values.json")" == "$want" ]]; then
        pass "android-values.json carries the file's hash and the key's fingerprint"
    else
        fail "android-values.json is not the expected one" "$(cat "$OUT/android-values.json")"
    fi
    if "$real_tools/zipalign" -c -P 16 4 "$SIGNED" >/dev/null 2>&1; then
        pass "the signed APK is aligned for 16 KiB pages"
    else
        fail "the signed APK is not aligned"
    fi
    if entries "$OUT" '.[!.]*' || entries "$OUT" '..?*'; then
        fail "a temporary file is left in the output directory" "$(ls -A "$OUT")"
    else
        pass "no temporary file is left behind"
    fi
else
    fail "a well-formed release APK was refused" "$(cat "$SANDBOX/err")"
fi

if grep -qF -- "$PASS" "$SANDBOX/apksigner-argv" 2>/dev/null; then
    fail "a password reached apksigner's command line"
elif grep -q 'env:INTERWEAVE_RELEASE_KEYSTORE_PASS' "$SANDBOX/apksigner-argv" 2>/dev/null; then
    pass "the passwords reach apksigner as env: references only"
else
    fail "apksigner was not seen taking the keystore password" "$(cat "$SANDBOX/apksigner-argv" 2>/dev/null)"
fi

if CERT="" sign --first-release 0.4.1 "$SANDBOX/good.apk" "$OUT" \
    && grep -qF "cert_sha256 $REL_CERT" "$SANDBOX/stdout"; then
    pass "--first-release signs and prints the key's fingerprint"
else
    fail "--first-release did not sign and print the fingerprint" "$(cat "$SANDBOX/err")"
fi

# refused <what> <needle> <sign args...>
refused() {
    local what="$1" needle="$2"; shift 2
    if sign "$@"; then
        fail "$what: accepted"
    elif [[ -e "$SIGNED" ]] || ls "$OUT"/*.apk >/dev/null 2>&1; then
        fail "$what: refused but left a signed APK"
    elif ! grep -qF -- "$needle" "$SANDBOX/err"; then
        fail "$what: refused without saying $needle" "$(cat "$SANDBOX/err")"
    else
        pass "$what: refused, nothing signed"
    fi
}

KEY=other refused "a key other than the release key" "not the release key's" \
    0.4.1 "$SANDBOX/good.apk" "$OUT"
refused "a tag that is not the APK's versionName" "versionName is '0.4.1', not 0.4.2" \
    0.4.2 "$SANDBOX/good.apk" "$OUT"
refused "a version that is not MAJOR.MINOR.PATCH" "not MAJOR.MINOR.PATCH" \
    v0.4.1 "$SANDBOX/good.apk" "$OUT"
make_apk "$SANDBOX/pkg.apk" org.interweave.other 0.4.1 false arm64-v8a
refused "another package" "not org.interweave.human" 0.4.1 "$SANDBOX/pkg.apk" "$OUT"
make_apk "$SANDBOX/dbg.apk" org.interweave.human 0.4.1 true arm64-v8a
refused "a debuggable APK" "debuggable" 0.4.1 "$SANDBOX/dbg.apk" "$OUT"
make_apk "$SANDBOX/abi.apk" org.interweave.human 0.4.1 false arm64-v8a x86_64
refused "native code for a second ABI" "not arm64-v8a alone" 0.4.1 "$SANDBOX/abi.apk" "$OUT"
make_apk "$SANDBOX/none.apk" org.interweave.human 0.4.1 false
refused "no native code" "not arm64-v8a alone" 0.4.1 "$SANDBOX/none.apk" "$OUT"
CERT=ABC refused "an expected fingerprint that is not lowercase hex" "64 lowercase hex" \
    0.4.1 "$SANDBOX/good.apk" "$OUT"

# apksigner's verify output as each version prints it. Up to 36.1 a
# certificate line reads "Signer #1 certificate SHA-256 digest: ...", from
# 37.0 "V3.0 Signer: certificate SHA-256 digest: ...". A stand-in rewrites
# the real tool's output with REWRITE (a sed -E script), so the shape this
# host's tool does not print is run too, and so are shapes no version
# should print.
RW="$SANDBOX/rewrite"
mkdir -p "$RW"
ln -s "$real_tools/aapt2" "$RW/aapt2"
ln -s "$real_tools/zipalign" "$RW/zipalign"
cat > "$RW/apksigner" <<EOF
#!/usr/bin/env bash
[[ "\$1" == verify ]] || exec "$real_tools/apksigner" "\$@"
out="\$("$real_tools/apksigner" "\$@")" || exit \$?
sed -E "\$REWRITE" <<<"\$out"
EOF
chmod +x "$RW/apksigner"
for shape in old new; do
    if [[ $shape == old ]]; then
        export REWRITE='s/^V[0-9.]+ Signer: certificate /Signer #1 certificate /'
    else
        export REWRITE='s/^Signer #1 certificate /V3.0 Signer: certificate /'
    fi
    if BT="$RW" sign 0.4.1 "$SANDBOX/good.apk" "$OUT" \
        && grep -qF "\"cert_sha256\": \"$REL_CERT\"" "$OUT/android-values.json"; then
        pass "apksigner's $shape certificate lines give the release key's fingerprint"
    else
        fail "apksigner's $shape certificate lines were not read" "$(cat "$SANDBOX/err")"
    fi
done
REWRITE='s/^Number of signers: 1$/Number of signers: 2/' BT="$RW" \
    refused "apksigner reporting two signers" "2 signers, not one" 0.4.1 "$SANDBOX/good.apk" "$OUT"
REWRITE='/^Number of signers:/d' BT="$RW" \
    refused "apksigner reporting no signer count" "no signers, not one" 0.4.1 "$SANDBOX/good.apk" "$OUT"
REWRITE='s/^(Signer #1|V[0-9.]+ Signer:) certificate SHA-256/Signer one certificate SHA-256/' BT="$RW" \
    refused "certificate lines in a shape not known" "named no single signing certificate" \
    0.4.1 "$SANDBOX/good.apk" "$OUT"
REWRITE='s/^((Signer #1|V[0-9.]+ Signer:) certificate SHA-256 digest: )([0-9a-f]{64})$/\1\3\nV3.1 Signer: certificate SHA-256 digest: '"$(printf 'e%.0s' {1..64})"'/' \
    BT="$RW" refused "two different certificates" "named no single signing certificate" \
    0.4.1 "$SANDBOX/good.apk" "$OUT"
unset REWRITE

# The last write fails, after the APK and its hash are moved: an `mv` on
# PATH that refuses the values file. Nothing of the run is left.
STUB="$SANDBOX/stub"
mkdir -p "$STUB"
cat > "$STUB/mv" <<EOF
#!/bin/sh
for last; do :; done
case "\$last" in *-values.json) echo "mv: refused by the test" >&2; exit 1 ;; esac
exec $(command -v mv) "\$@"
EOF
chmod +x "$STUB/mv"
rm -rf "$OUT"
if PATH="$STUB:$PATH" ANDROID_BUILD_TOOLS="$TOOLS" \
    INTERWEAVE_RELEASE_KEYSTORE="$SANDBOX/rel.p12" INTERWEAVE_RELEASE_KEY_ALIAS=rel \
    INTERWEAVE_RELEASE_KEYSTORE_PASS="$PASS" INTERWEAVE_RELEASE_CERT_SHA256="$REL_CERT" \
    bash "$UNDER_TEST" 0.4.1 "$SANDBOX/good.apk" "$OUT" >/dev/null 2>"$SANDBOX/err"; then
    fail "a run whose values file cannot be moved succeeded"
elif ! grep -q 'refused by the test' "$SANDBOX/err"; then
    fail "the stub was not reached at the last move" "$(cat "$SANDBOX/err")"
elif [[ -e "$SIGNED" || -e "$SIGNED.sha256" ]] || entries "$OUT" '.release*'; then
    fail "a run that failed at its last move left an APK, a hash or its work" "$(ls -A "$OUT")"
else
    pass "a run that fails at its last move leaves no APK, no hash, no work directory"
fi
rm -rf "$OUT"
mkdir -p "$OUT"
printf 'a previous run\n' > "$SIGNED"
ANDROID_BUILD_TOOLS="$TOOLS" INTERWEAVE_RELEASE_KEYSTORE="$SANDBOX/other.p12" \
    INTERWEAVE_RELEASE_KEY_ALIAS=other INTERWEAVE_RELEASE_KEYSTORE_PASS="$PASS" \
    INTERWEAVE_RELEASE_CERT_SHA256="$REL_CERT" \
    bash "$UNDER_TEST" 0.4.1 "$SANDBOX/good.apk" "$OUT" >/dev/null 2>&1
[[ ! -e "$SIGNED" ]] && pass "a refused run leaves no previous run's APK of the same name" \
    || fail "a refused run left a previous run's APK"

# --first-release with a fingerprint given, signing with another key:
# refused before anything is signed, whatever the key.
KEY=other sign --first-release 0.4.1 "$SANDBOX/good.apk" "$OUT"
rc=$?
if [[ $rc -eq 2 ]] && grep -qF "give one or the other" "$SANDBOX/err" && [[ ! -e "$SIGNED" && ! -e "$OUT/android-values.json" ]]; then
    pass "--first-release with a fingerprint given is a usage error (2), nothing signed"
else
    fail "--first-release with a fingerprint given exited $rc" "$(cat "$SANDBOX/err")"
fi

CERT="" sign 0.4.1 "$SANDBOX/good.apk" "$OUT"
rc=$?
if [[ $rc -eq 2 ]] && grep -qF "INTERWEAVE_RELEASE_CERT_SHA256 is not set" "$SANDBOX/err"; then
    pass "without --first-release, an unset expected fingerprint is a usage error (2)"
else
    fail "an unset expected fingerprint exited $rc" "$(cat "$SANDBOX/err")"
fi

if (( failures )); then
    echo "test_release_android_apk.sh: $failures failure(s)" >&2
    exit 1
fi
echo "test_release_android_apk.sh: all assertions passed"

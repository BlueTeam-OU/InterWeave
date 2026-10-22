#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
# tools/ci/test_android_ci_toolchain.sh
#
# Self-test for android_ci_toolchain.sh, with the download, sdkmanager,
# rustup and cargo faked: the JDK is taken only at its pinned checksum and
# version, every SDK package only at its pinned revision, the Rust target
# and cargo-ndk version are the pins', what later steps need reaches
# GITHUB_ENV and GITHUB_PATH, and every failure is loud and named.
set -uo pipefail
HERE="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
SUT="$HERE/android_ci_toolchain.sh"
PASS=0; FAIL=0
ok()  { PASS=$((PASS+1)); printf '  ok   %s\n' "$1"; }
bad() { FAIL=$((FAIL+1)); printf '  FAIL %s\n' "$1"; [[ -n "${2:-}" ]] && printf '%s\n' "$2" | sed 's/^/       /' | head -6; }

S="$(mktemp -d)"
[[ -n "$S" && -d "$S" ]] || { echo "test_android_ci_toolchain: no sandbox (mktemp failed)" >&2; exit 1; }
trap 'rm -rf "$S"' EXIT

# A JDK tarball whose release file names the pinned version, and one that
# names another; the pins carry the first one's checksum.
mkjdk() {  # <name> <version>
    mkdir -p "$S/build/$1/jdk-x/bin"
    printf 'JAVA_RUNTIME_VERSION="%s"\n' "$2" > "$S/build/$1/jdk-x/release"
    printf '#!/bin/sh\n' > "$S/build/$1/jdk-x/bin/javac"; chmod +x "$S/build/$1/jdk-x/bin/javac"
    tar -czf "$S/$1.tar.gz" -C "$S/build/$1" jdk-x
}
mkjdk good 17.0.20.1+1
mkjdk other 21.0.1+12
good_sum="$(sha256sum "$S/good.tar.gz" | cut -d' ' -f1)"
other_sum="$(sha256sum "$S/other.tar.gz" | cut -d' ' -f1)"
cat > "$S/pins" <<P
PKG_1=platform-tools@37.0.1
PKG_2=platforms;android-30@3
PKG_6=ndk;28.2.13676358@28.2.13676358
HOST_PKG_1=cmake;3.22.1@3.22.1
JDK_VERSION=17.0.20.1+1
JDK_URL=https://example.invalid/good.tar.gz
JDK_SHA256=$good_sum
RUST_ANDROID_TARGET=aarch64-linux-android
CARGO_NDK_VERSION=4.1.2
P
# FETCH copies the tarball the URL names out of the sandbox.
cat > "$S/fetch" <<'F'
#!/usr/bin/env bash
[ -z "${FAKE_FETCH_FAIL:-}" ] || exit 22
cp "$FAKE_DIR/$(basename "$1")" "$2"
F
# sdkmanager records its argv and installs each package at FAKE_REV_<n>
# or the pinned revision, by writing its source.properties.
cat > "$S/sdkmanager" <<'M'
#!/usr/bin/env bash
echo "$*" > "$FAKE_DIR/sdkm.args"
echo call >> "$FAKE_DIR/sdkm.calls"
[ -z "${FAKE_SDKM_FAIL:-}" ] || exit 1
# FAKE_SDKM_FAIL_FIRST=<n>: the first n calls fail as a corrupt download
# does in repository-core's installer: the archive staged under .temp/ (and
# .downloadIntermediates/), and at the install path only .installer/.
if [ "$(wc -l < "$FAKE_DIR/sdkm.calls")" -le "${FAKE_SDKM_FAIL_FIRST:-0}" ]; then
    for d in .temp .downloadIntermediates; do mkdir -p "$ANDROID_HOME/$d"; echo corrupt > "$ANDROID_HOME/$d/ndk.zip"; done
    mkdir -p "$ANDROID_HOME/ndk/28.2.13676358/.installer"; echo op > "$ANDROID_HOME/ndk/28.2.13676358/.installer/.installData"
    for p in "$ANDROID_HOME"/.temp "$ANDROID_HOME"/.downloadIntermediates "$ANDROID_HOME"/ndk/28.2.13676358; do
        find "$p" -type f >> "$FAKE_DIR/sdkm.seen"; done
    echo "Error on ZipFile unknown archive" >&2; exit 1
fi
[ -e "$ANDROID_HOME/.temp" ] && echo "staged archive still there" >> "$FAKE_DIR/sdkm.leftover"
[ -e "$ANDROID_HOME/.downloadIntermediates" ] && echo "intermediates still there" >> "$FAKE_DIR/sdkm.leftover"
[ -e "$ANDROID_HOME/ndk/28.2.13676358" ] && echo "the NDK's .installer still there" >> "$FAKE_DIR/sdkm.leftover"
head -c 64 >/dev/null  # some licence answers; it stops reading, as sdkmanager does
shift
for pkg in "$@"; do
    rev="$(grep -F "=$pkg@" "$PINS_FOR_FAKE" | sed 's/.*@//')"
    [ "$pkg" = "${FAKE_STALE_PKG:-}" ] && rev=0.0.1
    mkdir -p "$ANDROID_HOME/${pkg//;//}"
    printf 'Pkg.Revision = %s\n' "$rev" > "$ANDROID_HOME/${pkg//;//}/source.properties"
done
M
cat > "$S/rustup" <<'R'
#!/usr/bin/env bash
echo "$*" >> "$FAKE_DIR/rustup.args"
case "${1:-}" in
    show) [ -z "${FAKE_NO_ACTIVE:-}" ] ;;
    target) [ -z "${FAKE_TARGET_FAIL:-}" ] ;;
esac
R
cat > "$S/cargo" <<'C'
#!/usr/bin/env bash
echo "$*" > "$FAKE_DIR/cargo.args"
C
chmod +x "$S/fetch" "$S/sdkmanager" "$S/rustup" "$S/cargo"

run() {  # [VAR=value...] -- <component>...
    local envs=()
    while (( $# )) && [[ "$1" != -- ]]; do envs+=("$1"); shift; done; shift
    rm -rf "$S/sdk" "$S/runner"; mkdir -p "$S/sdk" "$S/runner"
    if [[ " ${envs[*]} " == *" FAKE_PLANT_PLATFORM=1 "* ]]; then
        mkdir -p "$S/sdk/platforms/android-30"; printf 'Pkg.Revision = 3\n' > "$S/sdk/platforms/android-30/source.properties"
    fi
    if [[ " ${envs[*]} " == *" FAKE_PLANT_GROUPING=1 "* ]]; then
        mkdir -p "$S/sdk/extras/google/m2repository"; printf 'Pkg.Revision = 58\n' > "$S/sdk/extras/google/m2repository/source.properties"
    fi
    if [[ " ${envs[*]} " == *" FAKE_PLANT_STALE_NDK=1 "* ]]; then
        mkdir -p "$S/sdk/ndk/28.2.13676358"; printf 'Pkg.Revision = 1.0\n' > "$S/sdk/ndk/28.2.13676358/source.properties"
    fi
    if [[ " ${envs[*]} " == *" FAKE_PLANT_SDKM=1 "* ]]; then
        mkdir -p "$S/sdk/cmdline-tools/latest/bin"; cp "$S/sdkmanager" "$S/sdk/cmdline-tools/latest/bin/sdkmanager"
    fi
    : > "$S/runner/env"; : > "$S/runner/path"; rm -f "$S"/*.args "$S"/sdkm.calls "$S"/sdkm.leftover "$S"/sdkm.seen; : > "$S/sdkm.calls"
    env -u ANDROID_NDK_HOME ANDROID_TOOLCHAIN_PINS="${PINS:-$S/pins}" PINS_FOR_FAKE="${PINS:-$S/pins}" \
        FETCH="$S/fetch" SDKMANAGER="$S/sdkmanager" RUSTUP="$S/rustup" CARGO="$S/cargo" \
        FAKE_DIR="$S" ANDROID_HOME="$S/sdk" RUNNER_TEMP="$S/runner" \
        GITHUB_ENV="$S/runner/env" GITHUB_PATH="$S/runner/path" SDK_RETRY_DELAY=0 "${envs[@]}" \
        bash "$SUT" "$@" 2>&1
}

echo "android_ci_toolchain: the JDK by its checksum and version"
out="$(run -- jdk)"; rc=$?
[[ $rc -eq 0 ]] && ok "the pinned JDK: exit 0" || bad "rc $rc" "$out"
grep -qx "JAVA_HOME=$S/runner/android-ci-jdk/jdk-x" "$S/runner/env" && ok "JAVA_HOME reaches GITHUB_ENV" || bad "GITHUB_ENV" "$(cat "$S/runner/env")"
grep -qx "$S/runner/android-ci-jdk/jdk-x/bin" "$S/runner/path" && ok "its bin reaches GITHUB_PATH" || bad "GITHUB_PATH" "$(cat "$S/runner/path")"
grep -q "JDK 17.0.20.1+1 at" <<<"$out" && ok "the log names the JDK version" || bad "version not named" "$out"
sed "s|good.tar.gz|other.tar.gz|" "$S/pins" > "$S/pins-swapped"
out="$(PINS="$S/pins-swapped" run -- jdk)"; rc=$?
[[ $rc -eq 1 && "$out" == *"sha256 is $other_sum, not the pinned $good_sum"* ]] \
    && ok "a tarball at another checksum: exit 1, both sums named" || bad "rc $rc" "$out"
sed "s|good.tar.gz|other.tar.gz|; s|^JDK_SHA256=.*|JDK_SHA256=$other_sum|" "$S/pins" > "$S/pins-otherver"
out="$(PINS="$S/pins-otherver" run -- jdk)"; rc=$?
[[ $rc -eq 1 && "$out" == *"not 17.0.20.1+1"* ]] && ok "a checksum-true tarball of another version: exit 1" || bad "rc $rc" "$out"
out="$(run FAKE_FETCH_FAIL=1 -- jdk)"; rc=$?
[[ $rc -eq 1 && "$out" == *"could not download the JDK"* ]] && ok "a failed download: exit 1" || bad "rc $rc" "$out"
grep -v '^JDK_SHA256=' "$S/pins" > "$S/pins-nosum"
out="$(PINS="$S/pins-nosum" run -- jdk)"; rc=$?
[[ $rc -eq 2 && "$out" == *"no JDK_SHA256"* ]] && ok "pins without a checksum: exit 2" || bad "rc $rc" "$out"

echo "android_ci_toolchain: every SDK package at its pinned revision"
out="$(run -- sdk)"; rc=$?
[[ $rc -eq 0 && "$out" == *"3 SDK package(s) at their pinned revisions"* ]] && ok "all three installed and held: exit 0" || bad "rc $rc" "$out"
[[ "$(cat "$S/sdkm.args" 2>/dev/null)" == "--install platform-tools platforms;android-30 ndk;28.2.13676358" ]] \
    && ok "sdkmanager gets every PKG_<n>, by name, in one call, and no HOST_PKG_<n>" || bad "sdkmanager argv" "$(cat "$S/sdkm.args" 2>/dev/null)"
out="$(run FAKE_STALE_PKG='platforms;android-30' -- sdk)"; rc=$?
[[ $rc -eq 1 && "$out" == *"platforms;android-30 is revision '0.0.1'"*"not the pinned 3"* ]] \
    && ok "a package sdkmanager installed at another revision: exit 1, named" || bad "rc $rc" "$out"
out="$(run FAKE_SDKM_FAIL=1 -- sdk)"; rc=$?
[[ $rc -eq 1 && "$out" == *"sdkmanager could not install"*"(attempt 3 of 3)"* ]] && ok "sdkmanager failing every time: exit 1 after the third attempt, named" || bad "rc $rc" "$out"
[[ "$(wc -l < "$S/sdkm.calls")" -eq 3 ]] && ok "  and it was called three times, no more" || bad "calls" "$(wc -l < "$S/sdkm.calls")"
out="$(run FAKE_SDKM_FAIL_FIRST=1 -- sdk)"; rc=$?
[[ $rc -eq 0 && "$out" == *"3 SDK package(s) at their pinned revisions"* ]] && ok "one corrupt download, then a good one: exit 0, every package held to its revision" || bad "rc $rc" "$out"
[[ "$out" == *"::warning::android_ci_toolchain: sdkmanager failed"*"(attempt 1 of 3)"* ]] && ok "  the failed attempt is a warning in the log, numbered" || bad "no warning" "$out"
[[ -s "$S/sdkm.seen" && ! -s "$S/sdkm.leftover" ]] && ok "  the staged archive, the intermediates and the NDK's .installer were cleared before the retry" || bad "leftover" "seen: $(cat "$S/sdkm.seen" 2>/dev/null) left: $(cat "$S/sdkm.leftover" 2>/dev/null)"
out="$(run FAKE_SDKM_FAIL_FIRST=2 SDK_INSTALL_ATTEMPTS=2 -- sdk)"; rc=$?
[[ $rc -eq 1 && "$out" == *"(attempt 2 of 2)"* ]] && ok "SDK_INSTALL_ATTEMPTS bounds the tries: two failures of two is exit 1" || bad "rc $rc" "$out"
for bad_pin in 'PKG_9=@1' 'PKG_9=platforms' 'PKG_9=ndk;28.2@' 'PKG_9=../x@1' 'PKG_9=.;platforms@3' 'PKG_9=..;sdk@1' 'PKG_9=.;cmdline-tools;latest@99' 'PKG_9=ndk;..@1'; do
    { cat "$S/pins"; echo "$bad_pin"; } > "$S/pins-badform"
    out="$(PINS="$S/pins-badform" run FAKE_SDKM_FAIL=1 -- sdk)"; rc=$?
    [[ $rc -eq 2 && "$out" == *"pin '$bad_pin'"*"is not <package>@<revision>"* && ! -s "$S/sdkm.calls" ]] \
        && ok "a malformed pin ($bad_pin): exit 2, named, sdkmanager never run (#347 F1)" || bad "rc $rc calls $(wc -l < "$S/sdkm.calls")" "$out"
done
# The sdkmanager in use inside a pinned package that is not at its revision:
# the retry keeps that directory, so the next attempt still has a binary.
{ cat "$S/pins"; echo 'PKG_9=cmdline-tools;latest@99'; } > "$S/pins-tools"
out="$(PINS="$S/pins-tools" run FAKE_SDKM_FAIL_FIRST=1 FAKE_STALE_PKG='cmdline-tools;latest' \
    SDKMANAGER="$S/sdk/cmdline-tools/latest/bin/sdkmanager" FAKE_PLANT_SDKM=1 -- sdk)"; rc=$?
[[ "$(wc -l < "$S/sdkm.calls")" -eq 2 && "$out" != *"No such file"* ]] \
    && ok "a stale package holding the sdkmanager in use is not removed: the retry runs it again" || bad "calls $(wc -l < "$S/sdkm.calls")" "$out"
# A category pinned with a revision passes the form but is no package: the
# retry never removes it, however sdkmanager fails (#347 round 2 F1, F2).
{ cat "$S/pins"; echo 'PKG_9=platforms@3'; } > "$S/pins-category"
out="$(PINS="$S/pins-category" run FAKE_SDKM_FAIL=1 FAKE_PLANT_PLATFORM=1 -- sdk)"; rc=$?
[[ $rc -eq 1 && -f "$S/sdk/platforms/android-30/source.properties" ]] \
    && ok "a category pin (platforms@3): the retry leaves platforms/ and what it holds, and the run is red" || bad "rc $rc" "$(ls -R "$S/sdk" 2>&1 | head -5) $out"
# A grouping two segments deep (extras;google) holds packages and no
# source.properties of its own: a pin naming it never empties it.
{ cat "$S/pins"; echo 'PKG_9=extras;google@1'; } > "$S/pins-grouping"
out="$(PINS="$S/pins-grouping" run FAKE_SDKM_FAIL=1 FAKE_PLANT_GROUPING=1 -- sdk)"; rc=$?
[[ $rc -eq 1 && -f "$S/sdk/extras/google/m2repository/source.properties" ]] \
    && ok "a grouping pin (extras;google@1): the packages under it are kept, and the run is red (#347 round 3 F1)" || bad "rc $rc" "$out"
# An installed package at another revision keeps its content through a retry
# (only an empty directory is removed); the revision check then says so.
out="$(run FAKE_SDKM_FAIL=1 FAKE_PLANT_STALE_NDK=1 -- sdk)"; rc=$?
[[ $rc -eq 1 && -f "$S/sdk/ndk/28.2.13676358/source.properties" ]] \
    && ok "a stale package with content: kept through every retry, never removed recursively" || bad "rc $rc" "$out"
out="$(run SDK_INSTALL_ATTEMPTS=0 -- sdk)"; rc=$?
[[ $rc -eq 2 && "$out" == *"not a positive count"* ]] && ok "SDK_INSTALL_ATTEMPTS=0: exit 2, never a run that tries nothing" || bad "rc $rc" "$out"
out="$(env -u ANDROID_HOME bash -c "ANDROID_TOOLCHAIN_PINS='$S/pins' SDKMANAGER='$S/sdkmanager' bash '$SUT' sdk" 2>&1)"; rc=$?
[[ $rc -eq 2 && "$out" == *"ANDROID_HOME is not set"* ]] && ok "no ANDROID_HOME: exit 2" || bad "rc $rc" "$out"
grep -v '^PKG_' "$S/pins" > "$S/pins-nopkg"
out="$(PINS="$S/pins-nopkg" run -- sdk)"; rc=$?
[[ $rc -eq 2 && "$out" == *"no PKG_<n> package"* ]] && ok "pins without packages: exit 2" || bad "rc $rc" "$out"

echo "android_ci_toolchain: the Rust target and cargo-ndk are the pins'"
out="$(run -- rust cargo-ndk)"; rc=$?
[[ $rc -eq 0 ]] && ok "rust and cargo-ndk: exit 0" || bad "rc $rc" "$out"
grep -qx "target add aarch64-linux-android" "$S/rustup.args" 2>/dev/null && ok "rustup adds RUST_ANDROID_TARGET" || bad "rustup argv" "$(cat "$S/rustup.args" 2>/dev/null)"
[[ "$(cat "$S/cargo.args" 2>/dev/null)" == "install cargo-ndk --version =4.1.2 --locked" ]] \
    && ok "cargo installs cargo-ndk at exactly CARGO_NDK_VERSION, --locked" || bad "cargo argv" "$(cat "$S/cargo.args" 2>/dev/null)"
out="$(run FAKE_NO_ACTIVE=1 -- rust)"; rc=$?
[[ $rc -eq 0 ]] && grep -qx "toolchain install" "$S/rustup.args" 2>/dev/null \
    && ok "no active toolchain: rust-toolchain.toml's is installed explicitly" || bad "rc $rc" "$(cat "$S/rustup.args" 2>/dev/null; echo "$out")"
out="$(run FAKE_TARGET_FAIL=1 -- rust)"; rc=$?
[[ $rc -eq 1 && "$out" == *"could not add the target aarch64-linux-android"* ]] && ok "a target rustup cannot add: exit 1" || bad "rc $rc" "$out"

echo "android_ci_toolchain: usage"
out="$(run -- )"; rc=$?
[[ $rc -eq 2 && "$out" == *"name at least one component"* ]] && ok "no component: exit 2" || bad "rc $rc" "$out"
out="$(run -- jdk gradle)"; rc=$?
[[ $rc -eq 2 && "$out" == *"unknown component 'gradle'"* ]] && ok "an unknown component: exit 2 before anything installs" || bad "rc $rc" "$out"
[[ ! -e "$S/runner/android-ci-jdk" ]] && ok "  and nothing was installed" || bad "installed before refusing"

echo
if (( FAIL == 0 )); then echo "test_android_ci_toolchain: OK — $PASS assertion(s)."
else echo "test_android_ci_toolchain: FAILED — $FAIL of $((PASS + FAIL))."; exit 1; fi

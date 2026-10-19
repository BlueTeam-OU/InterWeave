#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
#
# Self-test for check_android_no_play_services.sh.
#
# Every case starts from a copy of the repository's own Gradle tree --
# the lockfiles the guard reads in CI, not a shape written for the test
# -- and changes one thing in it.

set -uo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" && pwd )"
GUARD="$SCRIPT_DIR/check_android_no_play_services.sh"
REPO="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
REAL="$REPO/apps/human-android/android"
[ -f "$GUARD" ] || { echo "test: guard not found at $GUARD" >&2; exit 1; }
[ -f "$REAL/app/gradle.lockfile" ] || { echo "test: no real lockfile at $REAL/app/gradle.lockfile" >&2; exit 1; }

failures=0
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

ok()  { echo "  ✓ $1"; }
bad() { echo "  ✗ $1" >&2; failures=$((failures + 1)); }

run_code() { bash "$GUARD" --root "$1" >/dev/null 2>&1; printf '%s' "$?"; }
run()      { bash "$GUARD" --root "$1" 2>&1; }

# A fresh copy of the real Gradle tree's lockfiles and build scripts.
# mktemp, not a counter: tree runs in $(...), so a counter would never
# advance and every case would share one directory.
tree() {
    local r g
    r="$(mktemp -d "$TMP/r.XXXXXX")"
    g="$r/apps/human-android/android"
    mkdir -p "$g/app"
    cp "$REAL/build.gradle.kts" "$REAL/buildscript-gradle.lockfile" "$REAL/settings-gradle.lockfile" "$g/"
    cp "$REAL/app/build.gradle.kts" "$REAL/app/gradle.lockfile" "$g/app/"
    printf '%s' "$r"
}
# Adds one artifact line to a lockfile, above its `empty=` line as
# Gradle writes it.
add() { sed -i "/^empty=/i $2" "$1"; }

expect_fail() {  # <root> <what> <text the output must name>
    local out code
    out="$(run "$1")"; code="$(run_code "$1")"
    if [ "$code" = "1" ]; then ok "$2"; else bad "$2: exit $code, want 1: $out"; fi
    case "$out" in
        *"$3"*) ok "  and names $3" ;;
        *) bad "  should name '$3': $out" ;;
    esac
}
expect_pass() {  # <root> <what>
    if [ "$(run_code "$1")" = "0" ]; then ok "$2"; else bad "$2 should pass: $(run "$1")"; fi
}

printf 'test_check_android_no_play_services\n'

echo "a copy of the real tree passes"
R="$(tree)"
expect_pass "$R" "the real lockfiles, copied"

echo "each banned group on the shipped classpath fails"
for coord in \
    com.google.android.gms:play-services-base:18.5.0 \
    com.google.firebase:firebase-messaging:24.1.0 \
    com.google.android.play:integrity:1.4.0 \
    com.android.billingclient:billing:7.1.1 \
    com.google.gms:google-services:4.4.2; do
    R="$(tree)"
    add "$R/apps/human-android/android/app/gradle.lockfile" "$coord=releaseRuntimeClasspath"
    expect_fail "$R" "${coord%%:*} on releaseRuntimeClasspath" "$coord on releaseRuntimeClasspath"
done

echo "a subgroup of a banned group is that group"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.firebase.crashlytics:x:1.0=debugCompileClasspath"
expect_fail "$R" "com.google.firebase.crashlytics" "com.google.firebase.crashlytics:x:1.0 on debugCompileClasspath"

echo "the Google services plugin on the buildscript classpath fails (ADR-0057 A 2026-10-19)"
R="$(tree)"
add "$R/apps/human-android/android/buildscript-gradle.lockfile" "com.google.gms:google-services:4.4.2=classpath"
expect_fail "$R" "com.google.gms on classpath" "buildscript-gradle.lockfile: com.google.gms:google-services:4.4.2 on classpath"

echo "a group that only shares a prefix is not banned"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gmsx:x:1.0=releaseRuntimeClasspath"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.player:x:1.0=releaseRuntimeClasspath"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.firebaseui:x:1.0=releaseRuntimeClasspath"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.gmsx:x:1.0=releaseRuntimeClasspath"
expect_pass "$R" "com.google.android.gmsx, com.google.android.player, com.google.firebaseui, com.google.gmsx"

echo "lint and test configurations may carry them"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gms:a:1.0=androidLintTool,lintChecks,debugLintChecksClasspath"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.firebase:b:1.0=debugUnitTestRuntimeClasspath,debugAndroidTestCompileClasspath,androidTestUtil"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.play:c:1.0=unified-test-platform-gradle-work-action"
expect_pass "$R" "lint, unit-test, instrumented-test and test-platform configurations"

echo "a line shared by lint and the application names the application half only"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gms:d:1.0=androidLintTool,releaseCompileClasspath"
out="$(run "$R")"
expect_fail "$R" "androidLintTool,releaseCompileClasspath" "com.google.android.gms:d:1.0 on releaseCompileClasspath"
case "$out" in *"androidLintTool"*) bad "  should not report the lint configuration: $out" ;; *) ok "  and leaves the lint half out" ;; esac

echo "a configuration nobody has named yet is the application (fail-closed)"
R="$(tree)"
add "$R/apps/human-android/android/app/gradle.lockfile" "com.google.android.gms:e:1.0=stagingRuntimeClasspath"
expect_fail "$R" "an unknown configuration" "on stagingRuntimeClasspath"

echo "the build's plugin classpath is not exempt"
R="$(tree)"
add "$R/apps/human-android/android/buildscript-gradle.lockfile" "com.google.android.gms:oss-licenses-plugin:0.10.6=classpath"
expect_fail "$R" "a Play-services plugin on the buildscript classpath" "buildscript-gradle.lockfile: com.google.android.gms:oss-licenses-plugin:0.10.6 on classpath"

echo "a lockfile in a new subproject is read"
R="$(tree)"
mkdir -p "$R/apps/human-android/android/lib"
printf '// x\n' > "$R/apps/human-android/android/lib/build.gradle.kts"
printf 'com.android.billingclient:billing:7.1.1=releaseRuntimeClasspath\nempty=\n' > "$R/apps/human-android/android/lib/gradle.lockfile"
expect_fail "$R" "lib/gradle.lockfile" "lib/gradle.lockfile: com.android.billingclient:billing:7.1.1"

echo "the lock itself must be trustworthy"
R="$(tree)"; rm "$R/apps/human-android/android/app/gradle.lockfile"
expect_fail "$R" "no app lockfile" "app/gradle.lockfile: missing"
R="$(tree)"; sed -i 's/releaseRuntimeClasspath/releaseRuntimeClasspathX/g' "$R/apps/human-android/android/app/gradle.lockfile"
expect_fail "$R" "a lockfile that does not cover the shipped classpath" "names no releaseRuntimeClasspath"
R="$(tree)"; sed -i 's/^\([[:space:]]*\)lockAllConfigurations()/\1\/\/ lockAllConfigurations()/' "$R/apps/human-android/android/build.gradle.kts"
expect_fail "$R" "lockAllConfigurations() commented out" "no lockAllConfigurations()"
R="$(tree)"; sed -i 's/lockAllConfigurations()/lockAllConfigurations()\n        lockMode.set(LockMode.LENIENT)/' "$R/apps/human-android/android/build.gradle.kts"
expect_fail "$R" "LockMode.LENIENT" "LockMode.LENIENT"
R="$(tree)"; mkdir -p "$R/apps/human-android/android/lib"; printf '// x\n' > "$R/apps/human-android/android/lib/build.gradle.kts"
expect_fail "$R" "a subproject with no lockfile" "android/lib: a project with no gradle.lockfile"

echo "the real repository passes its own guard"
if [ "$(run_code "$REPO")" = "0" ]; then ok "this repository"; else bad "this repository fails: $(run "$REPO")"; fi

echo "--help quotes the decision, its amendment and the five groups"
help="$(bash "$GUARD" --help 2>&1)"; code=$?
[ "$code" = 0 ] && ok "--help exits 0" || bad "--help exit $code"
for s in 'ADR-0056 decision 2' 'com.google.android.gms' 'com.google.firebase' 'com.google.android.play' 'com.android.billingclient' 'com.google.gms`' 'Amendment 2026-10-19' 'Lint and'; do
    case "$help" in *"$s"*) ok "  says $s" ;; *) bad "  --help lacks '$s'" ;; esac
done

echo "invocation errors"
[ "$(run_code "$TMP/nope")" = "2" ] && ok "no such directory" || bad "missing root should exit 2"
if timeout 5 bash "$GUARD" --root >/dev/null 2>&1; then
    bad "--root with no value should be an invocation error"
elif [ "$?" = 124 ]; then
    bad "--root with no value hung instead of failing"
else
    ok "--root with no value"
fi

echo
if [ "$failures" -eq 0 ]; then
    echo "test_check_android_no_play_services: OK — all assertions passed."
    exit 0
fi
echo "test_check_android_no_play_services: FAILED — $failures assertion(s) failed." >&2
exit 1
